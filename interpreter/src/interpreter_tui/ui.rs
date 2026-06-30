use ratatui::prelude::*;

#[derive(Clone)]
pub(crate) enum OutputLine {
    Normal(String),
    Error(String),
    Success(String),
    Warning(String),
    Input(String),
    System(String),
    Header(String),
    Code(String),
}

impl OutputLine {
    pub(crate) fn to_styled_line(&self) -> Line<'_> {
        match self {
            OutputLine::Normal(s) => {
                Line::from(s.as_str()).style(Style::default().fg(Color::White))
            }
            OutputLine::Error(s) => Line::from(s.as_str()).style(Style::default().fg(Color::Red)),
            OutputLine::Success(s) => {
                Line::from(s.as_str()).style(Style::default().fg(Color::Green))
            }
            OutputLine::Warning(s) => {
                Line::from(s.as_str()).style(Style::default().fg(Color::Yellow))
            }
            OutputLine::Input(s) => {
                Line::from(s.as_str()).style(Style::default().fg(Color::Yellow))
            }
            OutputLine::System(s) => Line::from(s.as_str()).style(Style::default().fg(Color::Cyan)),
            OutputLine::Header(s) => {
                Line::from(s.as_str()).style(Style::default().fg(Color::Cyan).bold())
            }
            OutputLine::Code(s) => {
                Line::from(s.as_str()).style(Style::default().fg(Color::Magenta))
            }
        }
    }
}
