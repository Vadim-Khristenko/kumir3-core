//! Палитра интерфейса.
//!
//! Цвета собраны в одном месте, чтобы панели не расходились между собой.
//! Используются только цвета из палитры терминала (`Indexed`), а не 24-битные:
//! так интерфейс подхватывает тему терминала пользователя и остаётся читаемым
//! и на светлом фоне, и на тёмном.

use ratatui::style::{Color, Modifier, Style};

/// Основной акцент: рамка активной панели, заголовки.
pub(crate) const ACCENT: Color = Color::Cyan;
/// Спокойный акцент: рамки неактивных панелей, подписи.
pub(crate) const MUTED: Color = Color::DarkGray;
/// Успешное завершение, значения.
pub(crate) const OK: Color = Color::Green;
/// Ошибки.
pub(crate) const ERROR: Color = Color::Red;
/// Предупреждения и режим продолжения ввода.
pub(crate) const WARN: Color = Color::Yellow;

// --- Подсветка кода ---

/// Ключевое слово языка (`алг`, `нц`, `если`).
pub(crate) const KEYWORD: Color = Color::Magenta;
/// Имя типа (`цел`, `лит`, `таб`).
pub(crate) const TYPE: Color = Color::Blue;
/// Встроенная функция (`длина`, `корень`).
pub(crate) const BUILTIN: Color = Color::Cyan;
/// Числовой литерал.
pub(crate) const NUMBER: Color = Color::LightYellow;
/// Строковый и символьный литерал.
pub(crate) const STRING: Color = Color::LightGreen;
/// Комментарий.
pub(crate) const COMMENT: Color = Color::DarkGray;

/// Стиль рамки панели: активная выделяется цветом, а не только заголовком.
pub(crate) fn border(active: bool) -> Style {
    if active {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(MUTED)
    }
}

/// Стиль заголовка панели.
pub(crate) fn title(active: bool) -> Style {
    if active {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    }
}

/// Стиль подписи клавиши в нижней строке.
pub(crate) fn key_hint() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Стиль пояснения рядом с подписью клавиши.
pub(crate) fn key_label() -> Style {
    Style::default().fg(MUTED)
}
