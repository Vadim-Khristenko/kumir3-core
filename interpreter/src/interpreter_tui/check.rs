use crate::terminal::{init_terminal, restore_terminal};
use crate::ui::OutputLine;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem};
use shared::parser::parse;
use std::fs;
use std::io::Stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub(crate) fn run_check_tui(path: &PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Файл не найден: {}", path.display()));
    }

    let source = fs::read_to_string(path).map_err(|e| format!("Ошибка чтения: {}", e))?;

    let start = Instant::now();
    let result = parse(&source);
    let elapsed = start.elapsed();

    let mut output: Vec<OutputLine> = Vec::new();
    let is_ok;

    output.push(OutputLine::Header(format!(
        "  Проверка: {}",
        path.display()
    )));
    output.push(OutputLine::System("─".repeat(60)));

    match result {
        Ok(program) => {
            is_ok = true;
            output.push(OutputLine::Success("  ✓ Синтаксис корректный".to_string()));
            output.push(OutputLine::System(String::new()));
            output.push(OutputLine::Normal(format!(
                "  Алгоритмов: {}",
                program.algorithms.len()
            )));
            output.push(OutputLine::Normal(format!(
                "  Классов:    {}",
                program.classes.len()
            )));
            output.push(OutputLine::Normal(format!(
                "  Импортов:   {}",
                program.imports.len()
            )));
            if program.main.is_some() {
                output.push(OutputLine::Success(
                    "  Главный алгоритм: определён".to_string(),
                ));
            }
        }
        Err(e) => {
            is_ok = false;
            output.push(OutputLine::Error("  ✗ Ошибка синтаксиса".to_string()));
            output.push(OutputLine::System(String::new()));
            output.push(OutputLine::Error(format!("  {}", e)));
        }
    }

    output.push(OutputLine::System(String::new()));
    output.push(OutputLine::System(format!(
        "  Время парсинга: {:.3} мс",
        elapsed.as_secs_f64() * 1000.0
    )));
    output.push(OutputLine::System(String::new()));
    output.push(OutputLine::System(
        "  Нажмите Enter или Q для выхода".to_string(),
    ));

    let mut terminal = init_terminal()?;
    let result = run_simple_viewer(
        &mut terminal,
        &output,
        is_ok,
        if is_ok {
            " Синтаксис OK "
        } else {
            " Ошибка синтаксиса "
        },
    );
    restore_terminal()?;
    result
}

fn run_simple_viewer(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    output: &[OutputLine],
    is_ok: bool,
    title: &str,
) -> Result<(), String> {
    loop {
        terminal
            .draw(|frame| {
                let area = frame.area();
                let border_color = if is_ok { Color::Green } else { Color::Red };

                let block = Block::default()
                    .title(title)
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color));

                let inner = block.inner(area);
                frame.render_widget(block, area);

                let items: Vec<ListItem> = output
                    .iter()
                    .map(|line| ListItem::new(line.to_styled_line()))
                    .collect();

                let list = List::new(items);
                frame.render_widget(list, inner);
            })
            .map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())?
            && let Event::Key(key) = event::read().map_err(|e| e.to_string())?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => break,
                _ => {}
            }
        }
    }
    Ok(())
}
