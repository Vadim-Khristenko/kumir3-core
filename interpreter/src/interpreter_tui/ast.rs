use crate::terminal::{init_terminal, restore_terminal};
use crate::ui::OutputLine;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use shared::parser::parse;
use std::fs;
use std::io::Stdout;
use std::path::PathBuf;
use std::time::Duration;

pub(crate) fn run_ast_tui(path: &PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Файл не найден: {}", path.display()));
    }

    let source = fs::read_to_string(path).map_err(|e| format!("Ошибка чтения: {}", e))?;

    let program = parse(&source).map_err(|e| format!("Ошибка парсинга: {}", e))?;

    let mut output: Vec<OutputLine> = vec![
        OutputLine::Header("  AST Программы".to_string()),
        OutputLine::System("═".repeat(50)),
        OutputLine::System(String::new()),
        OutputLine::Header("  Импорты:".to_string()),
    ];
    if program.imports.is_empty() {
        output.push(OutputLine::Normal("    (нет)".to_string()));
    } else {
        for import in &program.imports {
            output.push(OutputLine::Code(format!("    {:?}", import)));
        }
    }
    output.push(OutputLine::System(String::new()));

    output.push(OutputLine::Header("  Алгоритмы:".to_string()));
    if program.algorithms.is_empty() {
        output.push(OutputLine::Normal("    (нет)".to_string()));
    } else {
        for alg in &program.algorithms {
            let params: Vec<String> = alg
                .params
                .iter()
                .map(|p| format!("{}: {:?}", p.name, p.type_kind))
                .collect();
            let ret = alg
                .return_type
                .as_ref()
                .map(|t| format!(" -> {:?}", t))
                .unwrap_or_default();
            output.push(OutputLine::Success(format!(
                "    алг {}({}){}",
                alg.name,
                params.join(", "),
                ret
            )));
        }
    }
    output.push(OutputLine::System(String::new()));

    output.push(OutputLine::Header("  Классы:".to_string()));
    if program.classes.is_empty() {
        output.push(OutputLine::Normal("    (нет)".to_string()));
    } else {
        for class in &program.classes {
            output.push(OutputLine::Success(format!(
                "    класс {} [{} полей, {} методов]",
                class.name,
                class.fields.len(),
                class.methods.len()
            )));
        }
    }
    output.push(OutputLine::System(String::new()));

    if program.main.is_some() {
        output.push(OutputLine::Header("  Главный алгоритм:".to_string()));
        output.push(OutputLine::Success("    определён".to_string()));
    }

    output.push(OutputLine::System(String::new()));
    output.push(OutputLine::System("  ↑↓ прокрутка | Q выход".to_string()));

    let mut terminal = init_terminal()?;
    let mut scroll: usize = 0;

    loop {
        terminal
            .draw(|frame| {
                let area = frame.area();

                let block = Block::default()
                    .title(" AST ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Magenta));

                let inner = block.inner(area);
                frame.render_widget(block, area);

                let visible_height = inner.height as usize;
                let total = output.len();
                let start = scroll.min(total.saturating_sub(1));
                let end = (start + visible_height).min(total);

                let items: Vec<ListItem> = output[start..end]
                    .iter()
                    .map(|line| ListItem::new(line.to_styled_line()))
                    .collect();

                let list = List::new(items);
                frame.render_widget(list, inner);

                if total > visible_height {
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
                    let mut state = ScrollbarState::new(total).position(scroll);
                    frame.render_stateful_widget(scrollbar, area, &mut state);
                }
            })
            .map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())?
            && let Event::Key(key) = event::read().map_err(|e| e.to_string())?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Up | KeyCode::Char('k') => scroll = scroll.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    scroll = scroll.saturating_add(1).min(output.len().saturating_sub(1))
                }
                KeyCode::PageUp => scroll = scroll.saturating_sub(10),
                KeyCode::PageDown => {
                    scroll = scroll
                        .saturating_add(10)
                        .min(output.len().saturating_sub(1))
                }
                _ => {}
            }
        }
    }

    restore_terminal()?;
    Ok(())
}
