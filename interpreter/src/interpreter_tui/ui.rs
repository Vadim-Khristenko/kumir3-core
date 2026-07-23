//! Строки вывода консоли.
//!
//! У каждой строки есть вид, а у вида — знак на левом поле и цвет. Знак
//! важнее цвета: он отличает ввод от результата, а ошибку от предупреждения
//! даже там, где цвет не помогает — в монохромном терминале, при копировании
//! в текст, при дальтонизме.

use ratatui::prelude::*;

use super::theme;

#[derive(Clone)]
pub(crate) enum OutputLine {
    /// То, что напечатала программа.
    Normal(String),
    /// Ошибка выполнения или разбора.
    Error(String),
    /// Значение выражения или успешно выполненная команда.
    Success(String),
    /// Предупреждение.
    Warning(String),
    /// Эхо набранной строки.
    Input(String),
    /// Сообщение самой консоли.
    System(String),
    /// Заголовок раздела.
    Header(String),
    /// Фрагмент кода в сообщении консоли.
    Code(String),
}

impl OutputLine {
    /// Знак на левом поле — два столбца вместе с отбивкой.
    fn gutter(&self) -> &'static str {
        match self {
            Self::Input(_) => "› ",
            Self::Error(_) => "✗ ",
            Self::Success(_) => "⇒ ",
            Self::Warning(_) => "! ",
            Self::Normal(_) | Self::System(_) | Self::Header(_) | Self::Code(_) => "  ",
        }
    }

    fn color(&self) -> Color {
        match self {
            Self::Normal(_) => Color::White,
            Self::Error(_) => theme::ERROR,
            Self::Success(_) => theme::OK,
            Self::Warning(_) => theme::WARN,
            Self::Input(_) => theme::MUTED,
            Self::System(_) | Self::Header(_) => theme::ACCENT,
            Self::Code(_) => theme::KEYWORD,
        }
    }

    fn text(&self) -> &str {
        match self {
            Self::Normal(s)
            | Self::Error(s)
            | Self::Success(s)
            | Self::Warning(s)
            | Self::Input(s)
            | Self::System(s)
            | Self::Header(s)
            | Self::Code(s) => s,
        }
    }

    pub(crate) fn to_styled_line(&self) -> Line<'_> {
        // Эхо ввода подсвечивается как код: набранное читается так же, как в
        // строке ввода, и глазу не приходится перестраиваться.
        if let Self::Input(text) = self {
            let mut spans = vec![Span::styled(
                self.gutter(),
                Style::default().fg(theme::MUTED),
            )];
            spans.extend(super::syntax::highlight(text));
            return Line::from(spans);
        }

        let color = self.color();
        let mut style = Style::default().fg(color);
        if matches!(self, Self::Header(_)) {
            style = style.add_modifier(Modifier::BOLD);
        }

        Line::from(vec![
            Span::styled(self.gutter(), Style::default().fg(color)),
            Span::styled(self.text(), style),
        ])
    }
}
