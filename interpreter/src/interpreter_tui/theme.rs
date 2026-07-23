//! Interface color palette.
//!
//! Colors centralized so panels stay visually consistent. Uses only terminal
//! palette colors (`Indexed`), not 24-bit, so the interface adopts the user's
//! terminal theme and remains readable on light and dark backgrounds.

use ratatui::style::{Color, Modifier, Style};

/// Main accent: active panel frame, titles.
pub(crate) const ACCENT: Color = Color::Cyan;
/// Muted accent: inactive panel frames, labels.
pub(crate) const MUTED: Color = Color::DarkGray;
/// Success, values.
pub(crate) const OK: Color = Color::Green;
/// Errors.
pub(crate) const ERROR: Color = Color::Red;
/// Warnings and input continuation mode.
pub(crate) const WARN: Color = Color::Yellow;

// --- Code highlighting ---

/// Language keyword (`алг`, `нц`, `если`).
pub(crate) const KEYWORD: Color = Color::Magenta;
/// Type name (`цел`, `лит`, `таб`).
pub(crate) const TYPE: Color = Color::Blue;
/// Builtin function (`длина`, `корень`).
pub(crate) const BUILTIN: Color = Color::Cyan;
/// Numeric literal.
pub(crate) const NUMBER: Color = Color::LightYellow;
/// String and character literal.
pub(crate) const STRING: Color = Color::LightGreen;
/// Comment.
pub(crate) const COMMENT: Color = Color::DarkGray;

/// Panel frame style: active one gets color, not just title.
pub(crate) fn border(active: bool) -> Style {
    if active {
        Style::default().fg(ACCENT)
    } else {
        Style::default().fg(MUTED)
    }
}

/// Panel title style.
pub(crate) fn title(active: bool) -> Style {
    if active {
        Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(MUTED)
    }
}

/// Key label style in the hint bar.
pub(crate) fn key_hint() -> Style {
    Style::default().fg(ACCENT).add_modifier(Modifier::BOLD)
}

/// Explanation text next to key label.
pub(crate) fn key_label() -> Style {
    Style::default().fg(MUTED)
}
