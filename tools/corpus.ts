#!/usr/bin/env bun
/**
 * Проверка учебного корпуса по-файлово — то же, что делает раннер
 * `cargo test -p kumir3-interpreter --test corpus`, но без пересборки крейта.
 *
 * Смысл: писать кейсы корпуса можно только тогда, когда обратная связь приходит
 * за секунды. Полный раннер поднимает весь тест целиком и не умеет показывать
 * один файл, поэтому автор кейса ждёт минуты ради одной строки эталона.
 *
 *   bun tools/corpus.ts                       — весь корпус
 *   bun tools/corpus.ts examples/corpus/1-osnovy
 *   bun tools/corpus.ts путь/к/файлу.kum      — один файл, с разбором расхождений
 *   bun tools/corpus.ts --кратко              — только итог, без разборов
 *
 * Правила разбора директив и сравнения вывода повторяют KITE 18 § 3.1–3.7 и
 * модули `interpreter/tests/corpus/`. Раннер на Rust остаётся источником истины:
 * этот скрипт — инструмент автора, а не замена тесту.
 */

import { mkdtemp, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, join, relative, sep } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const CORPUS = join(ROOT, "examples", "corpus");
const BINARY = join(ROOT, "target", "debug", "interpreter-cli.exe");
const DEFAULT_TIMEOUT_MS = 5000;

const argv = Bun.argv.slice(2);
const BRIEF = argv.includes("--кратко") || argv.includes("--brief");
const targets = argv.filter((a) => !a.startsWith("--"));

const c = {
  dim: (s: string) => `\x1b[2m${s}\x1b[0m`,
  red: (s: string) => `\x1b[31m${s}\x1b[0m`,
  green: (s: string) => `\x1b[32m${s}\x1b[0m`,
  yellow: (s: string) => `\x1b[33m${s}\x1b[0m`,
  bold: (s: string) => `\x1b[1m${s}\x1b[0m`,
};

// =============================================================================
//                          РАЗБОР ДИРЕКТИВ (KITE 18 § 3.2–3.4)
// =============================================================================

const KEYS = [
  "ФАЙЛ",
  "АВТОР",
  "УРОВЕНЬ",
  "КАТЕГОРИЯ",
  "ТЕГИ",
  "КЕЙС",
  "ВВОД",
  "АРГУМЕНТЫ",
  "ОЖИДАЕМЫЙ ВЫВОД",
  "ОЖИДАЕМАЯ ОШИБКА",
  "КОД ВОЗВРАТА",
  "ДОПУСК",
  "ТАЙМАУТ",
  "ПРОПУСТИТЬ",
  "ТОЛЬКО",
] as const;

type Key = (typeof KEYS)[number];

/** Ключи, значение которых продолжается блоками `| текст` (§ 3.4). */
const MULTILINE = new Set<Key>(["ВВОД", "ОЖИДАЕМЫЙ ВЫВОД", "ОЖИДАЕМАЯ ОШИБКА"]);

/** Директивы файла, которым нечего делать внутри кейса (§ 3.3). */
const FILE_ONLY = new Set<Key>(["ФАЙЛ", "АВТОР"]);

/** Директивы кейса: до первой `| КЕЙС:` они относиться не к чему (§ 3.3). */
const CASE_ONLY = new Set<Key>([
  "ВВОД",
  "АРГУМЕНТЫ",
  "ОЖИДАЕМЫЙ ВЫВОД",
  "ОЖИДАЕМАЯ ОШИБКА",
  "КОД ВОЗВРАТА",
  "ПРОПУСТИТЬ",
  "ТОЛЬКО",
]);

/** Виды ошибок исполнения KITE 14 § 3.2. */
const ERROR_KINDS = new Set([
  "DivisionByZero",
  "Overflow",
  "UndefinedVariable",
  "UndefinedAlgorithm",
  "UndefinedType",
  "TypeMismatch",
  "IndexOutOfBounds",
  "ArgumentCount",
  "AssertionFailed",
  "IOError",
  "UserException",
  "NotImplemented",
  "Other",
]);

type Meta = {
  level?: string;
  category?: string;
  input?: string;
  args: string[];
  expectedOutput?: string;
  expectedError?: string;
  exitCode?: number;
  tolerance?: string;
  timeoutMs?: number;
  skip?: string;
  only: boolean;
};

type Case = { name: string; line: number; meta: Meta; code: number[] };
type Problem = { line: number; message: string };
type CaseFile = {
  path: string;
  lines: string[];
  meta: Meta;
  prelude: number[];
  cases: Case[];
  problems: Problem[];
};

function emptyMeta(): Meta {
  return { args: [], only: false };
}

function cloneMeta(m: Meta): Meta {
  return { ...m, args: [...m.args] };
}

/** Разбирает `| КЛЮЧ: значение`; первый пробел после `|` и после `:` — разделители. */
function parseDirective(line: string): [Key, string] | null {
  const rest = line.trimStart();
  if (!rest.startsWith("| ")) return null;
  const body = rest.slice(2);
  const colon = body.indexOf(":");
  if (colon < 0) return null;
  const head = body.slice(0, colon);
  if (!(KEYS as readonly string[]).includes(head)) return null;
  let value = body.slice(colon + 1);
  if (value.startsWith(" ")) value = value.slice(1);
  return [head as Key, value];
}

/** Разбирает строку продолжения `| текст`; одинокий `|` даёт пустую строку. */
function parseContinuation(line: string): string | null {
  if (parseDirective(line)) return null;
  const rest = line.trimStart();
  if (!rest.startsWith("|")) return null;
  const body = rest.slice(1);
  if (body === "") return "";
  if (!body.startsWith(" ")) return null;
  return body.slice(1);
}

/**
 * Опечатка в ключе не должна молча превращать кейс в комментарий, поэтому
 * строка с прописной «головой» до двоеточия считается неизвестным ключом.
 */
function looksLikeUnknownKey(line: string): string | null {
  const rest = line.trimStart();
  if (!rest.startsWith("| ")) return null;
  const body = rest.slice(2);
  const colon = body.indexOf(":");
  if (colon < 0) return null;
  const head = body.slice(0, colon);
  if (!head || [...head].length > 40) return null;
  if (!/\p{L}/u.test(head)) return null;
  if (/\p{Ll}/u.test(head)) return null;
  if ((KEYS as readonly string[]).includes(head)) return null;
  return head;
}

function parseCaseFile(path: string, source: string): CaseFile {
  const lines = source.replace(/\r\n/g, "\n").split("\n");
  const fileMeta = emptyMeta();
  const prelude: number[] = [];
  const cases: Case[] = [];
  const problems: Problem[] = [];

  let i = 0;
  while (i < lines.length) {
    const directive = parseDirective(lines[i]);
    if (!directive) {
      const unknown = looksLikeUnknownKey(lines[i]);
      if (unknown) {
        problems.push({
          line: i + 1,
          message: `ключ «${unknown}» отсутствует в перечне KITE 18 § 3.2`,
        });
      }
      if (cases.length) cases[cases.length - 1].code.push(i);
      else prelude.push(i);
      i += 1;
      continue;
    }

    const [key, first] = directive;
    const start = i;
    const parts: string[] = [];
    if (first !== "") parts.push(first);
    i += 1;
    if (MULTILINE.has(key)) {
      while (i < lines.length) {
        const text = parseContinuation(lines[i]);
        if (text === null) break;
        parts.push(text);
        i += 1;
      }
    }
    const value = parts.join("\n");

    if (key === "КЕЙС") {
      cases.push({ name: value, line: start + 1, meta: cloneMeta(fileMeta), code: [] });
      continue;
    }

    // Область действия ключа. Без этой проверки эталон, поставленный до первой
    // `| КЕЙС:`, молча достался бы всем кейсам файла разом.
    const inCase = cases.length > 0;
    if (FILE_ONLY.has(key) && inCase) {
      problems.push({
        line: start + 1,
        message: `директива «${key}» относится к файлу и не может стоять внутри кейса`,
      });
    } else if (CASE_ONLY.has(key) && !inCase) {
      problems.push({
        line: start + 1,
        message: `директива «${key}» относится к кейсу и должна стоять после «| КЕЙС:»`,
      });
    }

    const target = inCase ? cases[cases.length - 1].meta : fileMeta;
    apply(target, key, value, start + 1, problems);
  }

  return { path, lines, meta: fileMeta, prelude, cases, problems };
}

function apply(meta: Meta, key: Key, value: string, line: number, problems: Problem[]): void {
  switch (key) {
    case "ФАЙЛ":
    case "АВТОР":
    case "ТЕГИ":
      break;
    case "УРОВЕНЬ":
      meta.level = value.trim();
      break;
    case "КАТЕГОРИЯ":
      meta.category = value.trim();
      break;
    case "ВВОД":
      meta.input = value;
      break;
    case "АРГУМЕНТЫ":
      meta.args = value.split(/\s+/).filter(Boolean);
      break;
    case "ОЖИДАЕМЫЙ ВЫВОД":
      meta.expectedOutput = value;
      break;
    case "ОЖИДАЕМАЯ ОШИБКА":
      meta.expectedError = value.trim();
      break;
    case "КОД ВОЗВРАТА": {
      const code = Number.parseInt(value.trim(), 10);
      if (Number.isNaN(code)) problems.push({ line, message: `«КОД ВОЗВРАТА: ${value}» — ожидалось целое` });
      else meta.exitCode = code;
      break;
    }
    case "ДОПУСК":
      meta.tolerance = value.trim();
      break;
    case "ТАЙМАУТ": {
      const ms = Number.parseInt(value.trim(), 10);
      if (Number.isNaN(ms)) problems.push({ line, message: `«ТАЙМАУТ: ${value}» — ожидалось число мс` });
      else meta.timeoutMs = ms;
      break;
    }
    case "ПРОПУСТИТЬ":
      meta.skip = value.trim();
      break;
    case "ТОЛЬКО":
      meta.only = true;
      break;
    case "КЕЙС":
      break;
  }
}

/** Исходный текст кейса: прелюдия файла + код кейса (§ 3.1, п. 5). */
function assemble(file: CaseFile, kase: Case): string {
  const out: string[] = [];
  for (const i of file.prelude) out.push(file.lines[i]);
  for (const i of kase.code) out.push(file.lines[i]);
  return out.join("\n") + "\n";
}

// =============================================================================
//                    ТРЕБОВАНИЯ К ФАЙЛУ КОРПУСА (KITE 18 § 3.8, § 3.10)
// =============================================================================

function checkFile(file: CaseFile): Problem[] {
  const problems: Problem[] = [];
  const rel = relative(CORPUS, file.path);
  const segments = rel.split(sep);

  const name = basename(file.path, ".kum");
  if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(name)) {
    problems.push({ line: 0, message: `имя файла «${name}» должно быть латиницей в kebab-case (§ 3.10)` });
  }

  if (!file.lines[0]?.trimStart().startsWith("|")) {
    problems.push({ line: 1, message: "первая строка — комментарий с описанием задачи (§ 3.10, п. 2)" });
  }

  const levelDir = segments[0] ?? "";
  const digit = levelDir.split("-")[0];
  const expectedLevel = digit.length === 1 && /^\d$/.test(digit) ? digit : undefined;
  if (!file.meta.level) {
    problems.push({ line: 0, message: "файл обязан нести директиву «| УРОВЕНЬ:» (§ 3.8)" });
  } else if (expectedLevel && file.meta.level !== expectedLevel) {
    problems.push({
      line: 0,
      message: `«| УРОВЕНЬ: ${file.meta.level}» расходится с каталогом «${levelDir}» (§ 3.8)`,
    });
  }

  if (segments.length >= 3 && file.meta.category && file.meta.category !== segments[1]) {
    problems.push({
      line: 0,
      message: `«| КАТЕГОРИЯ: ${file.meta.category}» расходится с каталогом «${segments[1]}» (§ 3.8)`,
    });
  }

  for (const kase of file.cases) {
    // Сравнение с undefined, а не проверка на истинность: пустой эталон —
    // законное ожидание «программа ничего не печатает», а пустая строка ложна.
    if (kase.meta.expectedOutput === undefined && kase.meta.expectedError === undefined) {
      problems.push({
        line: kase.line,
        message: `кейс «${kase.name}» не задаёт ни «| ОЖИДАЕМЫЙ ВЫВОД:», ни «| ОЖИДАЕМАЯ ОШИБКА:» (§ 3.3)`,
      });
    }
    if (!kase.name.trim()) {
      problems.push({ line: kase.line, message: "директива «| КЕЙС:» должна нести описание (§ 3.3)" });
    }
    if (kase.meta.expectedError && !ERROR_KINDS.has(kase.meta.expectedError.split("/")[0].trim())) {
      problems.push({
        line: kase.line,
        message: `неизвестный вид ошибки «${kase.meta.expectedError}»; известны: ${[...ERROR_KINDS].join(", ")}`,
      });
    }
  }

  if (!file.cases.length) {
    problems.push({ line: 0, message: "в файле нет ни одного кейса «| КЕЙС:» (§ 3.1)" });
  }

  return problems;
}

// =============================================================================
//                        СРАВНЕНИЕ ВЫВОДА (KITE 18 § 3.6)
// =============================================================================

const NUMBER = /-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?/g;

function normalize(text: string): string {
  return text.replace(/\r\n/g, "\n").replace(/\n+$/, "");
}

function collapseSpaces(text: string): string {
  return text
    .split("\n")
    .map((line) => line.replace(/[ \t]+/g, " ").trimEnd())
    .join("\n");
}

/** Сравнение с числовым допуском: числа — по eps, промежуточный текст — точно. */
function compareFloat(expected: string, actual: string, eps: number): boolean {
  const expNums = expected.match(NUMBER) ?? [];
  const actNums = actual.match(NUMBER) ?? [];
  if (expNums.length !== actNums.length) return false;
  for (let i = 0; i < expNums.length; i += 1) {
    if (Math.abs(Number(expNums[i]) - Number(actNums[i])) > eps) return false;
  }
  return expected.replace(NUMBER, "#") === actual.replace(NUMBER, "#");
}

function compare(expected: string, actual: string, tolerance: string | undefined): true | string {
  const exp = normalize(expected);
  const act = normalize(actual);
  const mode = (tolerance ?? "точно").trim();

  if (mode === "точно") {
    return exp === act ? true : diff(exp, act);
  }
  if (mode === "пробелы") {
    return collapseSpaces(exp) === collapseSpaces(act) ? true : diff(exp, act);
  }
  if (mode === "регэксп") {
    return new RegExp(`^${exp}$`, "s").test(act) ? true : diff(exp, act);
  }
  if (mode.startsWith("вещ:")) {
    const eps = Number(mode.slice(4).trim());
    if (Number.isNaN(eps)) return `неизвестный допуск «${mode}»`;
    return compareFloat(exp, act, eps) ? true : diff(exp, act);
  }
  return `неизвестный допуск «${mode}»: точно, пробелы, вещ:<eps>, регэксп`;
}

/** Построчный разбор расхождения: без него правка эталона превращается в гадание. */
function diff(expected: string, actual: string): string {
  const exp = expected.split("\n");
  const act = actual.split("\n");
  const rows: string[] = [];
  for (let i = 0; i < Math.max(exp.length, act.length); i += 1) {
    const e = exp[i];
    const a = act[i];
    if (e === a) {
      rows.push(`      ${c.dim(`  ${e}`)}`);
    } else {
      if (e !== undefined) rows.push(`      ${c.red(`- ${e}`)}`);
      if (a !== undefined) rows.push(`      ${c.green(`+ ${a}`)}`);
    }
  }
  return rows.join("\n");
}

// =============================================================================
//                              ИСПОЛНЕНИЕ КЕЙСА
// =============================================================================

type Outcome = {
  stdout: string;
  stderr: string;
  exitCode: number | null;
  timedOut: boolean;
  errorKind?: string;
  errorMessage?: string;
};

async function runProgram(sourcePath: string, input: string, args: string[], timeoutMs: number): Promise<Outcome> {
  const cmd = [BINARY, sourcePath, "--error-kind", ...(args.length ? ["--", ...args] : [])];
  const proc = Bun.spawn(cmd, {
    cwd: ROOT,
    stdin: new TextEncoder().encode(input),
    stdout: "pipe",
    stderr: "pipe",
  });

  let timedOut = false;
  const timer = setTimeout(() => {
    timedOut = true;
    proc.kill();
  }, timeoutMs);

  const [stdout, stderr] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  const exitCode = await proc.exited;
  clearTimeout(timer);

  let errorKind: string | undefined;
  let errorMessage: string | undefined;
  for (const line of stderr.split("\n")) {
    const trimmed = line.trim();
    if (trimmed.startsWith("[вид ошибки] ")) errorKind = trimmed.slice("[вид ошибки] ".length).trim();
    else if (trimmed.startsWith("Ошибка выполнения: ")) errorMessage = trimmed.slice("Ошибка выполнения: ".length).trim();
  }

  return { stdout, stderr, exitCode: timedOut ? null : exitCode, timedOut, errorKind, errorMessage };
}

type CaseResult = { status: "ok" | "skip" | "fail"; report?: string };

async function runCase(dir: string, file: CaseFile, kase: Case, index: number): Promise<CaseResult> {
  if (kase.meta.skip) return { status: "skip", report: kase.meta.skip };

  const sourcePath = join(dir, `kejs-${index}.kum`);
  await writeFile(sourcePath, assemble(file, kase), "utf8");

  const rawInput = kase.meta.input ?? "";
  const input = rawInput === "" ? "" : `${rawInput.replace(/\n+$/, "")}\n`;
  const timeout = kase.meta.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const outcome = await runProgram(sourcePath, input, kase.meta.args, timeout);

  const failed = outcome.timedOut || outcome.exitCode !== 0;
  const lines: string[] = [];

  if (outcome.timedOut) {
    return { status: "fail", report: `      программа не завершилась за ${timeout} мс` };
  }

  const expected = kase.meta.expectedError;
  if (expected) {
    const [wantKind, wantPart] = expected.split("/").map((s) => s.trim());
    if (!failed) {
      lines.push(`      ожидалась ошибка «${wantKind}», но программа завершилась успешно`);
    } else {
      const kind = outcome.errorKind ?? "<не сообщён>";
      if (kind !== wantKind) lines.push(`      вид ошибки: ожидался «${wantKind}», получен «${kind}»`);
      if (wantPart && !(outcome.errorMessage ?? "").includes(wantPart)) {
        lines.push(`      сообщение не содержит «${wantPart}»`);
        lines.push(`      сообщение: «${outcome.errorMessage ?? ""}»`);
      }
    }
  } else if (failed) {
    lines.push("      ожидался успешный запуск, но программа завершилась ошибкой");
    lines.push(`         ${outcome.errorMessage ?? outcome.stderr.trim()}`);
  }

  if (kase.meta.exitCode !== undefined && outcome.exitCode !== kase.meta.exitCode) {
    lines.push(`      код возврата: ожидался ${kase.meta.exitCode}, получен ${outcome.exitCode}`);
  }

  if (kase.meta.expectedOutput !== undefined) {
    const verdict = compare(kase.meta.expectedOutput, outcome.stdout, kase.meta.tolerance);
    if (verdict !== true) {
      lines.push("      вывод не совпал с эталоном:");
      lines.push(verdict);
    }
  }

  return lines.length ? { status: "fail", report: lines.join("\n") } : { status: "ok" };
}

// =============================================================================
//                                    ОБХОД
// =============================================================================

async function collectKum(dir: string, out: string[]): Promise<void> {
  const entries = await readdir(dir, { withFileTypes: true });
  entries.sort((a, b) => a.name.localeCompare(b.name));
  for (const entry of entries) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) await collectKum(path, out);
    else if (entry.name.endsWith(".kum")) out.push(path);
  }
}

async function resolveTargets(): Promise<string[]> {
  if (!targets.length) {
    const out: string[] = [];
    await collectKum(CORPUS, out);
    return out;
  }
  const out: string[] = [];
  for (const target of targets) {
    const path = target.match(/^[A-Za-z]:/) ? target : join(ROOT, target);
    const stat = await Bun.file(path).exists();
    if (stat && path.endsWith(".kum")) out.push(path);
    else await collectKum(path, out);
  }
  return out;
}

async function main(): Promise<void> {
  if (!(await Bun.file(BINARY).exists())) {
    console.error(c.red(`нет двоичного файла ${relative(ROOT, BINARY)}`));
    console.error(c.dim("собрать: cargo build -p kumir3-interpreter --bin interpreter-cli"));
    process.exit(1);
  }

  const files = await resolveTargets();
  const dir = await mkdtemp(join(tmpdir(), "kumir3-korpus-"));

  let passed = 0;
  let index = 0;
  const skipped: string[] = [];
  const failures: string[] = [];

  for (const path of files) {
    const source = await Bun.file(path).text();
    const file = parseCaseFile(path, source);
    const rel = relative(CORPUS, path).replaceAll(sep, "/");

    const problems = [...file.problems, ...checkFile(file)];
    if (problems.length) {
      const body = problems.map((p) => `      ${p.line ? `строка ${p.line}: ` : ""}${p.message}`).join("\n");
      failures.push(`  ${c.bold(rel)}: файл оформлен неверно\n${body}`);
      continue;
    }

    const hasOnly = file.cases.some((k) => k.meta.only);
    for (const kase of file.cases) {
      if (hasOnly && !kase.meta.only) continue;
      index += 1;
      const result = await runCase(dir, file, kase, index);
      if (result.status === "ok") passed += 1;
      else if (result.status === "skip") skipped.push(`  ${rel} :: ${kase.name} — ${result.report}`);
      else failures.push(`  ${c.bold(rel)} :: ${kase.name}\n${result.report}`);
    }
  }

  await rm(dir, { recursive: true, force: true });

  if (!BRIEF) {
    for (const line of skipped) console.log(`${c.yellow("○")}${line}`);
    for (const line of failures) console.log(`${c.red("✗")}${line}\n`);
  }

  const total = passed + skipped.length + failures.length;
  console.log(
    `корпус: файлов ${files.length}, кейсов ${total} — ` +
      `${c.green(`пройдено ${passed}`)}, ${c.yellow(`пропущено ${skipped.length}`)}, ` +
      `${failures.length ? c.red(`не пройдено ${failures.length}`) : "не пройдено 0"}`,
  );

  process.exit(failures.length ? 1 : 0);
}

await main();
