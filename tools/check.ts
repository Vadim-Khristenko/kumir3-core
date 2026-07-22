#!/usr/bin/env bun
/**
 * Локальная проверка проекта — то же, что делает CI, одной командой.
 *
 * Смысл: узнавать о поломке до отправки, а не через десять минут после.
 * Порядок шагов — от самого дешёвого к самому долгому, поэтому опечатка в
 * форматировании обнаруживается за секунды и не ждёт полной сборки.
 *
 *   bun tools/check.ts              — всё
 *   bun tools/check.ts --fast       — без сборки релиза и примеров
 *   bun tools/check.ts --fix        — сначала починить форматирование
 *   bun tools/check.ts --примеры    — только прогон программ на КуМире
 *
 * Программы КуМира прогоняются двумя способами: корпус — раннером с проверкой
 * эталонов, а examples/ и kumir-examples/ — просто на запуск, чтобы поймать
 * программы, переставшие исполняться после изменения языка.
 */

import { readdir } from "node:fs/promises";
import { join, relative } from "node:path";

const ROOT = new URL("..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1");
const args = new Set(Bun.argv.slice(2));

const FAST = args.has("--fast");
const FIX = args.has("--fix");
const ONLY_EXAMPLES = args.has("--примеры") || args.has("--examples");

const c = {
  dim: (s: string) => `\x1b[2m${s}\x1b[0m`,
  red: (s: string) => `\x1b[31m${s}\x1b[0m`,
  green: (s: string) => `\x1b[32m${s}\x1b[0m`,
  yellow: (s: string) => `\x1b[33m${s}\x1b[0m`,
  bold: (s: string) => `\x1b[1m${s}\x1b[0m`,
};

type Step = { name: string; cmd: string[]; skip?: boolean };

const results: { name: string; ok: boolean; ms: number; output: string }[] = [];

/** Запускает шаг, показывая его вывод только при неудаче. */
async function run(step: Step): Promise<boolean> {
  if (step.skip) {
    console.log(`${c.dim("○")} ${step.name} ${c.dim("— пропущен")}`);
    return true;
  }

  process.stdout.write(`${c.dim("…")} ${step.name}`);
  const started = performance.now();
  const proc = Bun.spawn(step.cmd, {
    cwd: ROOT,
    stdout: "pipe",
    stderr: "pipe",
    env: { ...process.env, CARGO_TERM_COLOR: "always" },
  });
  const [stdout, stderr, code] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
    proc.exited,
  ]);
  const ms = Math.round(performance.now() - started);
  const ok = code === 0;
  const output = stdout + stderr;

  process.stdout.write("\r\x1b[2K");
  console.log(`${ok ? c.green("✓") : c.red("✗")} ${step.name} ${c.dim(`${ms} мс`)}`);
  if (!ok) console.log(output.trimEnd().split("\n").map((l) => "    " + l).join("\n"));

  results.push({ name: step.name, ok, ms, output });
  return ok;
}

/**
 * Все программы КуМира вне корпуса — он проверяется своим раннером.
 *
 * Каталог с `kumir.toml` — это библиотека, а не программа: её файлы
 * предназначены для подключения и точки входа не имеют, поэтому запускать их
 * напрямую бессмысленно.
 */
async function findPrograms(): Promise<string[]> {
  const found: string[] = [];
  const walk = async (dir: string) => {
    let entries;
    try {
      entries = await readdir(dir, { withFileTypes: true });
    } catch {
      return;
    }
    if (entries.some((e) => e.isFile() && e.name === "kumir.toml")) return;

    for (const e of entries) {
      const full = join(dir, e.name);
      if (e.isDirectory()) {
        if (e.name === "corpus" || e.name === "target" || e.name.startsWith(".")) continue;
        await walk(full);
      } else if (e.name.endsWith(".kum")) {
        found.push(full);
      }
    }
  };
  await walk(join(ROOT, "examples"));
  await walk(join(ROOT, "kumir-examples"));
  return found.sort();
}

/**
 * Прогоняет программы на запуск.
 *
 * Ввод закрыт, а время ограничено: программа, ждущая ввода, иначе повесила бы
 * проверку. Программа, которой ввод нужен, — не сломана, поэтому таймаут
 * считается пропуском, а не поломкой; такие программы место которым в корпусе,
 * где ввод можно задать.
 */
async function runPrograms(): Promise<boolean> {
  const binary = join(ROOT, "target", "debug", process.platform === "win32" ? "interpreter-cli.exe" : "interpreter-cli");
  if (!(await Bun.file(binary).exists())) {
    console.log(`${c.red("✗")} Программы КуМира: не собран ${relative(ROOT, binary)}`);
    return false;
  }

  const programs = await findPrograms();
  const failed: { path: string; output: string }[] = [];
  let waiting = 0;

  for (const path of programs) {
    const proc = Bun.spawn([binary, path], {
      cwd: ROOT,
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    });
    const timer = setTimeout(() => proc.kill(), 5000);
    const [stderr, code] = await Promise.all([
      new Response(proc.stderr).text(),
      proc.exited,
    ]);
    clearTimeout(timer);

    if (code === 0) continue;
    // Программа, оборвавшаяся на закрытом вводе, ждёт данных — это не поломка.
    if (code === null || /ввод|stdin/i.test(stderr)) {
      waiting += 1;
      continue;
    }
    failed.push({ path: relative(ROOT, path), output: stderr.trim() });
  }

  const ok = failed.length === 0;
  const note = waiting > 0 ? c.dim(`, ${waiting} ждут ввода`) : "";
  console.log(
    `${ok ? c.green("✓") : c.red("✗")} Программы КуМира ${c.dim(`${programs.length} шт.${note}`)}`,
  );
  for (const f of failed) {
    console.log(`    ${c.red(f.path)}`);
    console.log(`      ${f.output.split("\n")[0]}`);
  }

  results.push({ name: "Программы КуМира", ok, ms: 0, output: "" });
  return ok;
}

// ---------------------------------------------------------------------------

console.log(c.bold("\nПроверка Kumir 3\n"));

let allOk = true;

if (!ONLY_EXAMPLES) {
  if (FIX) allOk = (await run({ name: "Форматирование (правка)", cmd: ["cargo", "fmt", "--all"] })) && allOk;

  const steps: Step[] = [
    { name: "Формат", cmd: ["cargo", "fmt", "--all", "--", "--check"], skip: FIX },
    { name: "Линтер", cmd: ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"] },
    { name: "Тесты", cmd: ["cargo", "test", "--workspace", "--locked"] },
    { name: "Сборка релиза", cmd: ["cargo", "build", "--release", "--workspace", "--locked"], skip: FAST },
  ];

  for (const step of steps) {
    const ok = await run(step);
    allOk = ok && allOk;
    // Дальше идти незачем: и линтер, и тесты требуют собирающегося кода.
    if (!ok && (step.name === "Формат" || step.name === "Линтер")) break;
  }
}

if (!FAST || ONLY_EXAMPLES) {
  allOk = (await runPrograms()) && allOk;
}

const total = results.reduce((s, r) => s + r.ms, 0);
console.log();
if (allOk) {
  console.log(c.green(c.bold("Всё в порядке")) + c.dim(` · ${(total / 1000).toFixed(1)} с`));
} else {
  const bad = results.filter((r) => !r.ok).map((r) => r.name);
  console.log(c.red(c.bold("Не прошло: ")) + bad.join(", "));
}
console.log();

process.exit(allOk ? 0 : 1);
