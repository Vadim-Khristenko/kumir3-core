// =============================================================================
//            МОДУЛЬ: СРАВНЕНИЕ ВЫВОДА И ДОПУСКИ (KITE 18 § 3.6, § 3.7)
// =============================================================================

use std::fmt;

// -----------------------------------------------------------------------------
//                                  ДОПУСКИ
// -----------------------------------------------------------------------------

/// Режим сравнения из директивы `| ДОПУСК:` (KITE 18 § 3.6).
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Tolerance {
    /// `точно` — посимвольное равенство после нормализации.
    #[default]
    Exact,
    /// `пробелы` — пробелы и табуляции схлопываются, хвостовые отбрасываются.
    Whitespace,
    /// `вещ:<eps>` — числа сравниваются с допуском, прочий текст точно.
    Float(f64),
    /// `регэксп` — эталон является регулярным выражением.
    Regex,
}

impl fmt::Display for Tolerance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact => write!(f, "точно"),
            Self::Whitespace => write!(f, "пробелы"),
            Self::Float(eps) => write!(f, "вещ:{eps}"),
            Self::Regex => write!(f, "регэксп"),
        }
    }
}

impl Tolerance {
    /// Разбирает значение директивы `| ДОПУСК:`.
    pub fn parse(value: &str) -> Result<Self, String> {
        let value = value.trim();
        match value {
            "точно" => Ok(Self::Exact),
            "пробелы" => Ok(Self::Whitespace),
            "регэксп" => Ok(Self::Regex),
            _ => match value.strip_prefix("вещ:") {
                Some(eps) => eps.trim().parse::<f64>().map(Self::Float).map_err(|_| {
                    format!("«ДОПУСК: {value}» — после «вещ:» ожидалось число, например вещ:1e-9")
                }),
                None => Err(format!(
                    "«ДОПУСК: {value}» — известны режимы: точно, пробелы, вещ:<eps>, регэксп"
                )),
            },
        }
    }
}

// -----------------------------------------------------------------------------
//                                НОРМАЛИЗАЦИЯ
// -----------------------------------------------------------------------------

/// Приводит `CRLF` к `LF` и отбрасывает завершающие переводы строки
/// (KITE 18 § 3.6, первый абзац).
pub fn normalize(text: &str) -> String {
    let text = text.replace("\r\n", "\n");
    text.trim_end_matches('\n').to_string()
}

/// Схлопывает пробелы и табуляции режима `пробелы`.
fn collapse_spaces(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            let mut out = String::with_capacity(line.len());
            let mut in_space = false;
            for ch in line.chars() {
                if ch == ' ' || ch == '\t' {
                    in_space = true;
                    continue;
                }
                if in_space {
                    out.push(' ');
                }
                in_space = false;
                out.push(ch);
            }
            out.truncate(out.trim_end().len());
            out
        })
        .collect::<Vec<_>>()
        .join("\n")
}

// -----------------------------------------------------------------------------
//                          ЧИСЛОВОЙ РЕЖИМ (вещ:<eps>)
// -----------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
enum Token {
    Number(f64),
    Text(String),
}

/// Разбивает строку на числа и промежуточный текст.
fn tokenize_numbers(text: &str) -> Vec<Token> {
    let chars: Vec<char> = text.chars().collect();
    let mut tokens = Vec::new();
    let mut text_buf = String::new();
    let mut i = 0usize;

    while i < chars.len() {
        if let Some((value, next)) = scan_number(&chars, i) {
            if !text_buf.is_empty() {
                tokens.push(Token::Text(std::mem::take(&mut text_buf)));
            }
            tokens.push(Token::Number(value));
            i = next;
            continue;
        }
        text_buf.push(chars[i]);
        i += 1;
    }
    if !text_buf.is_empty() {
        tokens.push(Token::Text(text_buf));
    }
    tokens
}

/// Пытается прочитать число, начинающееся в позиции `start`.
///
/// Знак принимается, только если слева нет буквы или цифры: так `x-1`
/// разбирается как текст `x-` и число `1`, а `= -1` — как число `-1`.
fn scan_number(chars: &[char], start: usize) -> Option<(f64, usize)> {
    let mut i = start;
    let signed = matches!(chars[i], '+' | '-');
    if signed {
        let left_is_word = start
            .checked_sub(1)
            .map(|p| chars[p].is_alphanumeric())
            .unwrap_or(false);
        if left_is_word {
            return None;
        }
        i += 1;
    }
    if i >= chars.len() || !chars[i].is_ascii_digit() {
        return None;
    }
    while i < chars.len() && chars[i].is_ascii_digit() {
        i += 1;
    }
    if i + 1 < chars.len() && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
        i += 1;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
    }
    // Показатель степени: e / E, необязательный знак, цифры.
    if i < chars.len() && matches!(chars[i], 'e' | 'E') {
        let mut j = i + 1;
        if j < chars.len() && matches!(chars[j], '+' | '-') {
            j += 1;
        }
        if j < chars.len() && chars[j].is_ascii_digit() {
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    let literal: String = chars[start..i].iter().collect();
    literal.parse::<f64>().ok().map(|v| (v, i))
}

fn float_equal(expected: &str, actual: &str, eps: f64) -> bool {
    let lhs = tokenize_numbers(expected);
    let rhs = tokenize_numbers(actual);
    if lhs.len() != rhs.len() {
        return false;
    }
    lhs.iter().zip(rhs.iter()).all(|pair| match pair {
        (Token::Number(a), Token::Number(b)) => (a - b).abs() <= eps,
        (Token::Text(a), Token::Text(b)) => a == b,
        _ => false,
    })
}

// -----------------------------------------------------------------------------
//                                 СРАВНЕНИЕ
// -----------------------------------------------------------------------------

/// Результат сравнения вывода с эталоном.
#[derive(Debug)]
pub enum Comparison {
    Equal,
    /// Расхождение: пояснение для отчёта.
    Different(String),
}

/// Сравнивает эталон и полученный вывод в заданном режиме.
pub fn compare(expected_raw: &str, actual_raw: &str, tolerance: &Tolerance) -> Comparison {
    let expected = normalize(expected_raw);
    let actual = normalize(actual_raw);

    let equal = match tolerance {
        Tolerance::Exact => expected == actual,
        Tolerance::Whitespace => collapse_spaces(&expected) == collapse_spaces(&actual),
        Tolerance::Float(eps) => float_equal(&expected, &actual, *eps),
        Tolerance::Regex => match regex::Regex::new(&format!("(?s)\\A(?:{expected})\\z")) {
            Ok(re) => re.is_match(&actual),
            Err(e) => {
                return Comparison::Different(format!(
                    "эталон не является регулярным выражением: {e}"
                ));
            }
        },
    };

    if equal {
        Comparison::Equal
    } else {
        Comparison::Different(diff(&expected, &actual, tolerance))
    }
}

/// Построчная разница эталона и полученного вывода (KITE 18 § 3.11, п. 6).
pub fn diff(expected: &str, actual: &str, tolerance: &Tolerance) -> String {
    let exp: Vec<&str> = expected.split('\n').collect();
    let act: Vec<&str> = actual.split('\n').collect();

    let mut out = String::new();
    out.push_str(&format!("      режим сравнения: {tolerance}\n"));

    if *tolerance == Tolerance::Regex {
        out.push_str("      эталон-регулярное выражение не совпало с выводом целиком\n");
    }

    let width = exp.len().max(act.len());
    for i in 0..width {
        let e = exp.get(i).copied();
        let a = act.get(i).copied();
        let same = match (e, a) {
            (Some(e), Some(a)) => match tolerance {
                Tolerance::Whitespace => collapse_spaces(e) == collapse_spaces(a),
                Tolerance::Float(eps) => float_equal(e, a, *eps),
                _ => e == a,
            },
            _ => false,
        };
        let mark = if same { ' ' } else { '≠' };
        out.push_str(&format!(
            "      {mark} строка {:>3}\n          ожидалось: {}\n          получено:  {}\n",
            i + 1,
            show(e),
            show(a),
        ));
    }
    out
}

fn show(line: Option<&str>) -> String {
    match line {
        Some(text) => format!("«{text}»"),
        None => "<строки нет>".to_string(),
    }
}

// -----------------------------------------------------------------------------
//                             ОЖИДАЕМАЯ ОШИБКА
// -----------------------------------------------------------------------------

/// Ожидаемая ошибка: вид из перечня KITE 14 и необязательная подстрока
/// сообщения после `/` (KITE 18 § 3.7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedError {
    pub kind: String,
    pub message_part: Option<String>,
}

/// Виды ошибок исполнения (KITE 14 § 3.2), они же `RuntimeErrorKind`.
pub const ERROR_KINDS: &[&str] = &[
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
];

impl ExpectedError {
    pub fn parse(value: &str) -> Result<Self, String> {
        let (kind, message_part) = match value.split_once('/') {
            Some((k, m)) => (k.trim().to_string(), Some(m.trim().to_string())),
            None => (value.trim().to_string(), None),
        };
        if !ERROR_KINDS.contains(&kind.as_str()) {
            return Err(format!(
                "«{kind}» не является видом ошибки исполнения KITE 14; известны: {}",
                ERROR_KINDS.join(", ")
            ));
        }
        Ok(Self {
            kind,
            message_part: message_part.filter(|m| !m.is_empty()),
        })
    }
}
