// =============================================================================
//        МОДУЛЬ: ТЕКСТОВЫЙ СКАНЕР ДИРЕКТИВ КЕЙСОВ (KITE 18 § 3.1–3.5)
// =============================================================================
// Сканер работает по строкам исходного файла и НЕ задействует разбор языка
// (KITE 18 § 3.11, п. 2): файл, не исполняемый текущим интерпретатором, всё
// равно ДОЛЖЕН разбираться как набор кейсов.

use std::fmt;
use std::path::{Path, PathBuf};

// -----------------------------------------------------------------------------
//                                  КЛЮЧИ
// -----------------------------------------------------------------------------

/// Ключ директивы (KITE 18 § 3.2, § 3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    File,
    Author,
    Level,
    Category,
    Tags,
    Case,
    Input,
    Args,
    ExpectedOutput,
    ExpectedError,
    ExitCode,
    Tolerance,
    Timeout,
    Skip,
    Only,
}

/// Область действия ключа (столбец «Область» таблицы KITE 18 § 3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scope {
    /// Только файловая директива.
    FileOnly,
    /// Файловая директива, переопределяемая в кейсе.
    FileAndCase,
    /// Только директива кейса.
    CaseOnly,
}

impl Key {
    /// Разбирает имя ключа. Перечень закрыт (KITE 18 § 3.2).
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "ФАЙЛ" => Self::File,
            "АВТОР" => Self::Author,
            "УРОВЕНЬ" => Self::Level,
            "КАТЕГОРИЯ" => Self::Category,
            "ТЕГИ" => Self::Tags,
            "КЕЙС" => Self::Case,
            "ВВОД" => Self::Input,
            "АРГУМЕНТЫ" => Self::Args,
            "ОЖИДАЕМЫЙ ВЫВОД" => Self::ExpectedOutput,
            "ОЖИДАЕМАЯ ОШИБКА" => Self::ExpectedError,
            "КОД ВОЗВРАТА" => Self::ExitCode,
            "ДОПУСК" => Self::Tolerance,
            "ТАЙМАУТ" => Self::Timeout,
            "ПРОПУСТИТЬ" => Self::Skip,
            "ТОЛЬКО" => Self::Only,
            _ => return None,
        })
    }

    /// Имя ключа так, как оно записывается в файле.
    pub fn name(self) -> &'static str {
        match self {
            Self::File => "ФАЙЛ",
            Self::Author => "АВТОР",
            Self::Level => "УРОВЕНЬ",
            Self::Category => "КАТЕГОРИЯ",
            Self::Tags => "ТЕГИ",
            Self::Case => "КЕЙС",
            Self::Input => "ВВОД",
            Self::Args => "АРГУМЕНТЫ",
            Self::ExpectedOutput => "ОЖИДАЕМЫЙ ВЫВОД",
            Self::ExpectedError => "ОЖИДАЕМАЯ ОШИБКА",
            Self::ExitCode => "КОД ВОЗВРАТА",
            Self::Tolerance => "ДОПУСК",
            Self::Timeout => "ТАЙМАУТ",
            Self::Skip => "ПРОПУСТИТЬ",
            Self::Only => "ТОЛЬКО",
        }
    }

    /// Многострочные ключи (KITE 18 § 3.3, столбец «Значение»).
    ///
    /// Продолжение блоками `| текст` (§ 3.4) принимается только для них: иначе
    /// обычный комментарий, стоящий сразу за однострочной директивой, был бы
    /// поглощён её значением.
    fn is_multiline(self) -> bool {
        matches!(
            self,
            Self::Input | Self::ExpectedOutput | Self::ExpectedError
        )
    }

    fn scope(self) -> Scope {
        match self {
            Self::File | Self::Author => Scope::FileOnly,
            Self::Level | Self::Category | Self::Tags | Self::Tolerance | Self::Timeout => {
                Scope::FileAndCase
            }
            _ => Scope::CaseOnly,
        }
    }
}

// -----------------------------------------------------------------------------
//                          РАЗБОР ОТДЕЛЬНЫХ СТРОК
// -----------------------------------------------------------------------------

/// Разбирает строку-директиву `| КЛЮЧ: значение` (KITE 18 § 3.2).
///
/// Возвращает ключ и значение первой строки. Первый пробел после `|` и первый
/// пробел после `:` — разделители и в значение не входят (§ 3.4).
pub fn parse_directive_line(line: &str) -> Option<(Key, String)> {
    let rest = line.trim_start().strip_prefix('|')?;
    let rest = rest.strip_prefix(' ')?;
    let (head, value) = rest.split_once(':')?;
    let key = Key::parse(head)?;
    let value = value.strip_prefix(' ').unwrap_or(value);
    Some((key, value.to_string()))
}

/// Разбирает строку продолжения `| текст` (KITE 18 § 3.4).
///
/// Одинокий `|` даёт пустую строку значения. Строка, являющаяся директивой,
/// продолжением не считается.
pub fn parse_continuation_line(line: &str) -> Option<String> {
    if parse_directive_line(line).is_some() {
        return None;
    }
    let rest = line.trim_start().strip_prefix('|')?;
    if rest.is_empty() {
        return Some(String::new());
    }
    Some(rest.strip_prefix(' ')?.to_string())
}

/// Похожа ли строка на директиву с ключом вне перечня § 3.2.
///
/// Эвристика: голова до двоеточия записана прописными буквами и коротка.
/// Нужна, чтобы опечатка в ключе не превращала кейс в молчаливый комментарий.
fn looks_like_unknown_key(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix('|')?;
    let rest = rest.strip_prefix(' ')?;
    let (head, _) = rest.split_once(':')?;
    if head.is_empty() || head.chars().count() > 40 {
        return None;
    }
    if !head.chars().any(char::is_alphabetic) {
        return None;
    }
    if head.chars().any(char::is_lowercase) {
        return None;
    }
    if Key::parse(head).is_some() {
        return None;
    }
    Some(head.to_string())
}

// -----------------------------------------------------------------------------
//                             МЕТАДАННЫЕ И КЕЙСЫ
// -----------------------------------------------------------------------------

/// Собранные значения директив одной области (файла либо кейса).
#[derive(Debug, Clone, Default)]
pub struct Meta {
    pub file: Option<String>,
    pub author: Option<String>,
    pub level: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub input: Option<String>,
    pub args: Vec<String>,
    pub expected_output: Option<String>,
    /// Полуинтервал строк файла, занятых директивой `| ОЖИДАЕМЫЙ ВЫВОД:`
    /// вместе с блоками продолжения. Нужен режиму перезаписи (§ 3.11).
    pub expected_output_span: Option<(usize, usize)>,
    pub expected_error: Option<String>,
    pub exit_code: Option<i32>,
    pub tolerance: Option<String>,
    pub timeout_ms: Option<u64>,
    pub skip: Option<String>,
    pub only: bool,
}

/// Один тест-кейс файла.
#[derive(Debug, Clone)]
pub struct Case {
    /// Описание из `| КЕЙС:`.
    pub name: String,
    /// Номер строки директивы `| КЕЙС:` (нумерация с единицы).
    pub line: usize,
    /// Слитые метаданные файла и кейса.
    pub meta: Meta,
    /// Индексы строк файла, составляющих код кейса.
    pub code_lines: Vec<usize>,
}

/// Разобранный файл корпуса.
#[derive(Debug, Clone)]
pub struct CaseFile {
    pub path: PathBuf,
    /// Строки файла без завершающих переводов строки.
    pub lines: Vec<String>,
    pub meta: Meta,
    /// Индексы строк прелюдии (всё до первой директивы `| КЕЙС:`).
    pub prelude_lines: Vec<usize>,
    pub cases: Vec<Case>,
    /// Нарушения формата, найденные сканером.
    pub problems: Vec<Problem>,
}

/// Нарушение формата кейса или требований к файлу корпуса.
#[derive(Debug, Clone)]
pub struct Problem {
    /// Номер строки (с единицы), 0 — проблема всего файла.
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "строка {}: {}", self.line, self.message)
        }
    }
}

impl CaseFile {
    /// Собирает исходный текст кейса: прелюдия файла + код кейса
    /// (KITE 18 § 3.1, п. 5).
    pub fn assemble(&self, case: &Case) -> String {
        let mut out = String::new();
        for &i in &self.prelude_lines {
            out.push_str(&self.lines[i]);
            out.push('\n');
        }
        for &i in &case.code_lines {
            out.push_str(&self.lines[i]);
            out.push('\n');
        }
        out
    }

    /// Кейсы к исполнению с учётом директивы `| ТОЛЬКО:` (§ 3.3).
    pub fn selected_cases(&self) -> Vec<&Case> {
        let has_only = self.cases.iter().any(|c| c.meta.only);
        self.cases
            .iter()
            .filter(|c| !has_only || c.meta.only)
            .collect()
    }
}

// -----------------------------------------------------------------------------
//                                   СКАНЕР
// -----------------------------------------------------------------------------

/// Разбирает текст файла в набор кейсов.
///
/// Сканер устойчив к неразбираемым программам: код кейса собирается как текст
/// и синтаксически не проверяется.
pub fn parse_case_file(path: &Path, source: &str) -> CaseFile {
    let lines: Vec<String> = source
        .replace("\r\n", "\n")
        .split('\n')
        .map(str::to_string)
        .collect();

    let mut file_meta = Meta::default();
    let mut prelude_lines = Vec::new();
    let mut cases: Vec<Case> = Vec::new();
    let mut problems = Vec::new();

    let mut i = 0usize;
    while i < lines.len() {
        let Some((key, first)) = parse_directive_line(&lines[i]) else {
            if let Some(head) = looks_like_unknown_key(&lines[i]) {
                problems.push(Problem {
                    line: i + 1,
                    message: format!(
                        "ключ «{head}» отсутствует в перечне KITE 18 § 3.2; \
                         допустимы только известные ключи"
                    ),
                });
            }
            match cases.last_mut() {
                Some(case) => case.code_lines.push(i),
                None => prelude_lines.push(i),
            }
            i += 1;
            continue;
        };

        // Собираем значение: первая строка плюс блоки продолжения (§ 3.4).
        let start = i;
        let mut parts: Vec<String> = Vec::new();
        if !first.is_empty() {
            parts.push(first);
        }
        i += 1;
        if key.is_multiline() {
            while i < lines.len() {
                let Some(text) = parse_continuation_line(&lines[i]) else {
                    break;
                };
                parts.push(text);
                i += 1;
            }
        }
        let value = parts.join("\n");
        let span = (start, i);

        if key == Key::Case {
            cases.push(Case {
                name: value,
                line: start + 1,
                meta: file_meta.clone(),
                code_lines: Vec::new(),
            });
            continue;
        }

        let in_case = !cases.is_empty();
        match (key.scope(), in_case) {
            (Scope::FileOnly, true) => problems.push(Problem {
                line: start + 1,
                message: format!(
                    "директива «{}» относится к файлу и не может стоять внутри кейса",
                    key.name()
                ),
            }),
            (Scope::CaseOnly, false) => problems.push(Problem {
                line: start + 1,
                message: format!(
                    "директива «{}» относится к кейсу и должна стоять после «| КЕЙС:»",
                    key.name()
                ),
            }),
            _ => {}
        }

        let target = match cases.last_mut() {
            Some(case) => &mut case.meta,
            None => &mut file_meta,
        };
        apply(target, key, &value, span, start + 1, &mut problems);
    }

    CaseFile {
        path: path.to_path_buf(),
        lines,
        meta: file_meta,
        prelude_lines,
        cases,
        problems,
    }
}

fn apply(
    meta: &mut Meta,
    key: Key,
    value: &str,
    span: (usize, usize),
    line: usize,
    problems: &mut Vec<Problem>,
) {
    match key {
        Key::File => meta.file = Some(value.to_string()),
        Key::Author => meta.author = Some(value.to_string()),
        Key::Level => meta.level = Some(value.trim().to_string()),
        Key::Category => meta.category = Some(value.trim().to_string()),
        Key::Tags => {
            meta.tags = value
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
        }
        Key::Case => unreachable!("КЕЙС обрабатывается вызывающей стороной"),
        Key::Input => meta.input = Some(value.to_string()),
        Key::Args => {
            meta.args = value
                .split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>();
        }
        Key::ExpectedOutput => {
            meta.expected_output = Some(value.to_string());
            meta.expected_output_span = Some(span);
        }
        Key::ExpectedError => meta.expected_error = Some(value.trim().to_string()),
        Key::ExitCode => match value.trim().parse::<i32>() {
            Ok(code) => meta.exit_code = Some(code),
            Err(_) => problems.push(Problem {
                line,
                message: format!("«КОД ВОЗВРАТА: {value}» — ожидалось целое число"),
            }),
        },
        Key::Tolerance => meta.tolerance = Some(value.trim().to_string()),
        Key::Timeout => match value.trim().parse::<u64>() {
            Ok(ms) => meta.timeout_ms = Some(ms),
            Err(_) => problems.push(Problem {
                line,
                message: format!("«ТАЙМАУТ: {value}» — ожидалось число миллисекунд"),
            }),
        },
        Key::Skip => meta.skip = Some(value.trim().to_string()),
        Key::Only => meta.only = true,
    }
}
