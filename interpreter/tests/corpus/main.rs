// =============================================================================
//     ИНТЕГРАЦИОННЫЙ ТЕСТ: УЧЕБНЫЙ КОРПУС ПРОГРАММ (KITE 18)
// =============================================================================
// Раннер самопроверяемых программ. Обходит `examples/corpus/`, извлекает
// кейсы текстовым сканером директив, собирает исходный текст «прелюдия + код
// кейса», исполняет его настоящим интерпретатором и сравнивает результат с
// эталоном.
//
// Запуск: `cargo test -p kumir3-interpreter --test corpus`
// Перезапись эталонов: `KUMIR3_BLESS=1 cargo test -p kumir3-interpreter --test corpus`

mod bless;
mod compare;
mod directives;
mod exec;
mod unit;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use compare::{Comparison, ExpectedError, Tolerance};
use directives::{Case, CaseFile, Problem};

/// Каталог корпуса (KITE 18 § 3.8).
fn corpus_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("у каталога крейта есть родитель")
        .join("examples")
        .join("corpus")
}

/// Двоичный файл интерпретатора, собираемый Cargo для этого теста.
fn interpreter_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_interpreter-cli"))
}

/// Рекурсивно собирает файлы `*.kum` корпуса в устойчивом порядке.
fn collect_kum_files(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::path);
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_kum_files(&path, out)?;
        } else if path.extension().is_some_and(|e| e == "kum") {
            out.push(path);
        }
    }
    Ok(())
}

// -----------------------------------------------------------------------------
//                     ПРОВЕРКА ТРЕБОВАНИЙ К ФАЙЛУ КОРПУСА
// -----------------------------------------------------------------------------

/// Проверяет размещение и оформление файла (KITE 18 § 3.8, § 3.10).
fn check_file_requirements(root: &Path, file: &CaseFile) -> Vec<Problem> {
    let mut problems = Vec::new();
    let relative = file.path.strip_prefix(root).unwrap_or(&file.path);
    let segments: Vec<String> = relative
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();

    // § 3.10, п. 1: имя файла — латиница в kebab-case.
    if let Some(name) = file.path.file_stem().map(|s| s.to_string_lossy()) {
        let ok = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            && !name.starts_with('-')
            && !name.ends_with('-');
        if !ok {
            problems.push(Problem {
                line: 0,
                message: format!(
                    "имя файла «{name}» должно записываться латиницей в kebab-case (§ 3.10)"
                ),
            });
        }
    }

    // § 3.10, п. 2: первая строка — краткое описание задачи (комментарий).
    match file.lines.first() {
        Some(first) if first.trim_start().starts_with('|') => {}
        _ => problems.push(Problem {
            line: 1,
            message: "первая строка файла должна быть комментарием с кратким описанием \
                      задачи (§ 3.10, п. 2)"
                .to_string(),
        }),
    }

    // § 3.8: уровень обязателен, метаданные соответствуют пути.
    let level_dir = segments.first().cloned().unwrap_or_default();
    let expected_level = level_dir
        .split('-')
        .next()
        .filter(|d| d.len() == 1 && d.chars().all(|c| c.is_ascii_digit()))
        .map(str::to_string);

    match &file.meta.level {
        None => problems.push(Problem {
            line: 0,
            message: "файл корпуса обязан нести директиву «| УРОВЕНЬ:» (§ 3.8)".to_string(),
        }),
        Some(level) => {
            if let Some(expected) = &expected_level
                && level != expected
            {
                problems.push(Problem {
                    line: 0,
                    message: format!(
                        "«| УРОВЕНЬ: {level}» расходится с размещением: каталог «{level_dir}» \
                         задаёт уровень {expected} (§ 3.8)"
                    ),
                });
            }
        }
    }

    // § 3.8: второй сегмент пути — категория.
    if segments.len() >= 3
        && let Some(category) = &file.meta.category
    {
        let dir = &segments[1];
        if category != dir {
            problems.push(Problem {
                line: 0,
                message: format!(
                    "«| КАТЕГОРИЯ: {category}» расходится с размещением: каталог «{dir}» (§ 3.8)"
                ),
            });
        }
    }

    // § 3.1, п. 4: ровно одна из директив эталона в каждом кейсе.
    for case in &file.cases {
        if case.meta.expected_output.is_none() && case.meta.expected_error.is_none() {
            problems.push(Problem {
                line: case.line,
                message: format!(
                    "кейс «{}» не задаёт ни «| ОЖИДАЕМЫЙ ВЫВОД:», ни «| ОЖИДАЕМАЯ ОШИБКА:» \
                     (§ 3.3)",
                    case.name
                ),
            });
        }
        if case.name.trim().is_empty() {
            problems.push(Problem {
                line: case.line,
                message: "директива «| КЕЙС:» должна нести описание кейса (§ 3.3)".to_string(),
            });
        }
    }

    if file.cases.is_empty() {
        problems.push(Problem {
            line: 0,
            message: "в файле корпуса нет ни одного кейса «| КЕЙС:» (§ 3.1)".to_string(),
        });
    }

    problems
}

// -----------------------------------------------------------------------------
//                            ИСПОЛНЕНИЕ ОДНОГО КЕЙСА
// -----------------------------------------------------------------------------

/// Итог одного кейса.
enum CaseResult {
    Passed,
    Skipped(String),
    Failed(String),
    /// Эталон перезаписан режимом § 3.11.
    Blessed,
}

fn run_case(
    binary: &Path,
    file: &CaseFile,
    case: &Case,
    blessings: &mut bless::Blessings,
) -> CaseResult {
    if let Some(reason) = &case.meta.skip {
        return CaseResult::Skipped(reason.clone());
    }

    let tolerance = match case.meta.tolerance.as_deref() {
        None => Tolerance::Exact,
        Some(value) => match Tolerance::parse(value) {
            Ok(t) => t,
            Err(e) => return CaseResult::Failed(format!("      {e}\n")),
        },
    };

    let expected_error = match case.meta.expected_error.as_deref() {
        None => None,
        Some(value) => match ExpectedError::parse(value) {
            Ok(e) => Some(e),
            Err(e) => return CaseResult::Failed(format!("      {e}\n")),
        },
    };

    let source = file.assemble(case);
    let source_path = match exec::write_temp_source(&source) {
        Ok(p) => p,
        Err(e) => return CaseResult::Failed(format!("      не удалось записать программу: {e}\n")),
    };

    let timeout = Duration::from_millis(case.meta.timeout_ms.unwrap_or(exec::DEFAULT_TIMEOUT_MS));
    let input = case.meta.input.clone().unwrap_or_default();
    let input = if input.is_empty() {
        String::new()
    } else {
        format!("{}\n", input.trim_end_matches('\n'))
    };

    let outcome = match exec::run_program(binary, &source_path, &input, &case.meta.args, timeout) {
        Ok(o) => o,
        Err(e) => {
            return CaseResult::Failed(format!("      не удалось запустить программу: {e}\n"));
        }
    };
    let _ = std::fs::remove_file(&source_path);

    let mut report = String::new();

    if outcome.timed_out {
        return CaseResult::Failed(format!(
            "      программа не завершилась за {} мс и была остановлена (§ 3.3, ТАЙМАУТ)\n\
             {}",
            timeout.as_millis(),
            indent_block("частичный вывод", &outcome.stdout),
        ));
    }

    // § 3.7, п. 5: успех при ожидаемой ошибке и наоборот.
    match (&expected_error, outcome.failed()) {
        (Some(expected), false) => {
            report.push_str(&format!(
                "      ожидалась ошибка «{}», но программа завершилась успешно\n",
                expected.kind
            ));
        }
        (Some(expected), true) => {
            let kind = outcome.error_kind.as_deref().unwrap_or("<не сообщён>");
            if kind != expected.kind {
                report.push_str(&format!(
                    "      вид ошибки: ожидался «{}», получен «{kind}»\n",
                    expected.kind
                ));
            }
            if let Some(part) = &expected.message_part {
                let message = outcome.error_message.clone().unwrap_or_default();
                if !message.contains(part.as_str()) {
                    report.push_str(&format!(
                        "      сообщение об ошибке не содержит подстроку «{part}»\n\
                         \x20         сообщение: «{message}»\n"
                    ));
                }
            }
        }
        (None, true) => {
            report.push_str(&format!(
                "      ожидался успешный запуск, но программа завершилась ошибкой\n\
                 \x20         {}\n",
                outcome
                    .error_message
                    .clone()
                    .unwrap_or_else(|| outcome.stderr.trim().to_string()),
            ));
        }
        (None, false) => {}
    }

    // § 3.3: код возврата проверяется, когда задан явно.
    if let Some(expected_code) = case.meta.exit_code {
        let actual = outcome.exit_code;
        if actual != Some(expected_code) {
            report.push_str(&format!(
                "      код возврата: ожидался {expected_code}, получен {}\n",
                actual.map_or("<нет>".to_string(), |c| c.to_string()),
            ));
        }
    }

    // § 3.6 / § 3.7, п. 4: сравнение вывода.
    if let Some(expected_output) = &case.meta.expected_output {
        match compare::compare(expected_output, &outcome.stdout, &tolerance) {
            Comparison::Equal => {}
            Comparison::Different(text) => {
                if bless::enabled()
                    && tolerance != Tolerance::Regex
                    && let Some(span) = case.meta.expected_output_span
                {
                    blessings.record(
                        &file.path,
                        bless::Edit {
                            span,
                            actual: outcome.stdout.clone(),
                        },
                    );
                    return CaseResult::Blessed;
                }
                report.push_str("      вывод не совпал с эталоном:\n");
                report.push_str(&text);
            }
        }
    }

    if report.is_empty() {
        CaseResult::Passed
    } else {
        CaseResult::Failed(report)
    }
}

fn indent_block(title: &str, text: &str) -> String {
    if text.trim().is_empty() {
        return format!("      {title}: <пусто>\n");
    }
    let body: String = text
        .replace("\r\n", "\n")
        .trim_end_matches('\n')
        .split('\n')
        .map(|l| format!("          {l}\n"))
        .collect();
    format!("      {title}:\n{body}")
}

// -----------------------------------------------------------------------------
//                                  ГЛАВНЫЙ ТЕСТ
// -----------------------------------------------------------------------------

#[test]
fn korpus_programm_prohodit() {
    let root = corpus_root();
    assert!(
        root.is_dir(),
        "каталог корпуса не найден: {} (KITE 18 § 3.8)",
        root.display()
    );

    let mut files = Vec::new();
    collect_kum_files(&root, &mut files).expect("каталог корпуса читается");
    assert!(
        !files.is_empty(),
        "в каталоге {} нет ни одной программы *.kum",
        root.display()
    );

    let binary = interpreter_binary();
    let mut failures: Vec<String> = Vec::new();
    let mut passed = 0usize;
    let mut skipped: Vec<String> = Vec::new();
    let mut blessed = 0usize;
    let mut blessings = bless::Blessings::default();
    let mut sources: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();

    for path in &files {
        let source = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("файл корпуса {} не читается: {e}", path.display()));
        let file = directives::parse_case_file(path, &source);
        sources.insert(path.clone(), file.lines.clone());

        let relative = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();

        let mut problems = file.problems.clone();
        problems.extend(check_file_requirements(&root, &file));
        if !problems.is_empty() {
            let body: String = problems.iter().map(|p| format!("      {p}\n")).collect();
            failures.push(format!("  {relative}: файл оформлен неверно\n{body}"));
            continue;
        }

        for case in file.selected_cases() {
            match run_case(&binary, &file, case, &mut blessings) {
                CaseResult::Passed => passed += 1,
                CaseResult::Blessed => blessed += 1,
                CaseResult::Skipped(reason) => {
                    skipped.push(format!("  {relative} :: {} — {reason}", case.name));
                }
                CaseResult::Failed(report) => {
                    failures.push(format!("  {relative} :: {}\n{report}", case.name));
                }
            }
        }
    }

    if bless::enabled() && !blessings.is_empty() {
        let report = blessings.apply(&sources).expect("эталоны записываются");
        for line in report {
            println!("перезапись: {line}");
        }
    }

    println!(
        "корпус: файлов {}, кейсов пройдено {}, пропущено {}, перезаписано {}",
        files.len(),
        passed,
        skipped.len(),
        blessed
    );
    for line in &skipped {
        println!("пропущен:{line}");
    }

    assert!(
        failures.is_empty(),
        "не пройдено кейсов корпуса: {}\n\n{}",
        failures.len(),
        failures.join("\n")
    );
}
