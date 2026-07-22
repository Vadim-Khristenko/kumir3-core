//! Подсветка кода на Кумире.
//!
//! Разбор здесь свой, а не лексером языка: подсвечивать нужно и незаконченную
//! строку, которую человек ещё набирает, — лексер на ней сообщил бы об ошибке.
//! Перечни ключевых слов и встроенных функций берутся из `shared::constants`,
//! то есть из того же источника, что и сам язык: добавленное слово
//! подсвечивается само, без правки этого файла.

use ratatui::prelude::*;
use shared::constants::{builtins, keywords};

use super::theme;

/// Типы языка подсвечиваются иначе, чем остальные ключевые слова: в тексте
/// программы они читаются как «существительные» среди «глаголов».
const TYPE_WORDS: &[&str] = &[
    "цел",
    "вещ",
    "лог",
    "сим",
    "лит",
    "таб",
    "любой",
    "пустота",
    "указатель",
    "перечисление",
    "авто",
    "необязательно",
];

/// Разбивает строку кода на окрашенные куски.
pub(crate) fn highlight(line: &str) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        // Комментарий: от `|` до конца строки. Проверяется первым — внутри
        // комментария больше ничего не разбирается.
        if ch == '|' {
            spans.push(Span::styled(
                chars[i..].iter().collect::<String>(),
                Style::default().fg(theme::COMMENT),
            ));
            break;
        }

        // Строка или символ. Незакрытая кавычка тоже подсвечивается: человек
        // ещё печатает, и «поломанная» подсветка мешала бы читать.
        if ch == '"' || ch == '\'' {
            let quote = ch;
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != quote {
                i += 1;
            }
            if i < chars.len() {
                i += 1;
            }
            spans.push(Span::styled(
                chars[start..i].iter().collect::<String>(),
                Style::default().fg(theme::STRING),
            ));
            continue;
        }

        if ch.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.') {
                i += 1;
            }
            spans.push(Span::styled(
                chars[start..i].iter().collect::<String>(),
                Style::default().fg(theme::NUMBER),
            ));
            continue;
        }

        if is_word_char(ch) {
            let start = i;
            while i < chars.len() && is_word_char(chars[i]) {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            spans.push(Span::styled(word.clone(), word_style(&word)));
            continue;
        }

        // Всё прочее — знаки препинания и пробелы — цвета не меняет.
        let start = i;
        while i < chars.len()
            && !is_word_char(chars[i])
            && !chars[i].is_ascii_digit()
            && chars[i] != '"'
            && chars[i] != '\''
            && chars[i] != '|'
        {
            i += 1;
        }
        spans.push(Span::raw(chars[start..i].iter().collect::<String>()));
    }

    spans
}

/// Символ, из которых состоят имена: буквы любого алфавита, цифры и `_`.
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn word_style(word: &str) -> Style {
    let lower = word.to_lowercase();
    if TYPE_WORDS.contains(&lower.as_str()) {
        Style::default().fg(theme::TYPE)
    } else if keywords::is_keyword(&lower) {
        Style::default().fg(theme::KEYWORD)
    } else if builtins::builtin_category(word).is_some() {
        Style::default().fg(theme::BUILTIN)
    } else {
        Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Собирает текст обратно: подсветка не имеет права терять символы.
    fn joined(line: &str) -> String {
        highlight(line)
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>()
    }

    #[test]
    fn podsvetka_sohranyaet_tekst() {
        for line in [
            "цел x := 5",
            "вывод \"Ответ: \", x * 2   | комментарий",
            "нц для i от 1 до 3",
            "",
            "   ",
            "лямбда(x) -> x + 1",
            "|только комментарий",
        ] {
            assert_eq!(joined(line), line, "потерян текст строки «{line}»");
        }
    }

    /// Незакрытая кавычка встречается на каждом втором нажатии клавиши.
    #[test]
    fn nezakrytaya_kavychka_ne_lomaet_razbor() {
        assert_eq!(joined("вывод \"нача"), "вывод \"нача");
    }

    #[test]
    fn slova_poluchayut_svoi_cveta() {
        let style_of = |line: &str, word: &str| {
            highlight(line)
                .into_iter()
                .find(|s| s.content.as_ref() == word)
                .unwrap_or_else(|| panic!("в «{line}» нет куска «{word}»"))
                .style
        };

        assert_eq!(style_of("цел x", "цел").fg, Some(theme::TYPE));
        assert_eq!(style_of("нц пока x", "пока").fg, Some(theme::KEYWORD));
        assert_eq!(style_of("длина(s)", "длина").fg, Some(theme::BUILTIN));
        assert_eq!(style_of("моя_переменная", "моя_переменная").fg, None);
    }

    #[test]
    fn kommentarij_pogloshchaet_ostatok_stroki() {
        let spans = highlight("x := 1 | тут цел и вывод — просто слова");
        let last = spans.last().expect("комментарий разобран");
        assert_eq!(last.style.fg, Some(theme::COMMENT));
        assert!(last.content.contains("просто слова"));
    }
}
