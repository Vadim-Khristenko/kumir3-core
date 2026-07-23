//! Console output lines.
//!
//! Each line has a type, and each type has a gutter symbol and color. The symbol
//! is more important than color: it distinguishes input from result, error from
//! warning even where color fails — on monochrome terminals, when copying to
//! text, or for colorblindness.

use ratatui::prelude::*;

use super::theme;

#[derive(Clone)]
pub(crate) enum OutputLine {
    /// What the program printed.
    Normal(String),
    /// Execution or parse error.
    Error(String),
    /// Expression value or successful command.
    Success(String),
    /// Warning message.
    Warning(String),
    /// Echo of input line.
    Input(String),
    /// Console's own message.
    System(String),
    /// Section header.
    Header(String),
    /// Code fragment in console message.
    Code(String),
}

impl OutputLine {
    /// Gutter symbol — two columns with padding.
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
        // Input echo is highlighted as code: typed text reads the same in the
        // input line, so the eye doesn't have to adjust.
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
