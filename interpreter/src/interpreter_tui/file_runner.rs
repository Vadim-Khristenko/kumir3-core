use crate::interpreter::Interpreter;
use crate::terminal::{init_terminal, restore_terminal};
use crate::ui::OutputLine;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use shared::types::Value;
use std::fs;
use std::io::Stdout;
use std::path::PathBuf;
use std::time::{Duration, Instant};

pub(crate) struct FileRunnerApp {
    output: Vec<OutputLine>,
    scroll: usize,
    finished: bool,
    error: Option<String>,
}

pub(crate) fn run_file_tui(path: &PathBuf, debug: bool, show_time: bool) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Файл не найден: {}", path.display()));
    }

    let source = fs::read_to_string(path).map_err(|e| format!("Ошибка чтения файла: {}", e))?;

    let mut app = FileRunnerApp {
        output: Vec::new(),
        scroll: 0,
        finished: false,
        error: None,
    };

    app.output
        .push(OutputLine::Header(format!("  Файл: {}", path.display())));
    app.output.push(OutputLine::System("─".repeat(60)));

    let mut interpreter = Interpreter::new();
    interpreter.set_debug_mode(debug);

    if let Some(parent) = path.parent() {
        interpreter.set_base_dir(parent);
    }

    let start = Instant::now();
    let result = interpreter.run(&source);
    let elapsed = start.elapsed();

    let output = interpreter.get_output();
    if !output.is_empty() {
        for line in output.lines() {
            app.output.push(OutputLine::Normal(format!("  {}", line)));
        }
    }

    app.output.push(OutputLine::System("─".repeat(60)));

    match result {
        Ok(value) => {
            if !matches!(value, Value::Null) {
                app.output
                    .push(OutputLine::Success(format!("  Результат: {:?}", value)));
            } else {
                app.output
                    .push(OutputLine::Success("  Выполнено успешно".to_string()));
            }
        }
        Err(e) => {
            app.error = Some(format!("{}", e));
            app.output
                .push(OutputLine::Error(format!("  Ошибка: {}", e)));
        }
    }

    if show_time {
        app.output.push(OutputLine::System(format!(
            "  Время: {:.3} мс",
            elapsed.as_secs_f64() * 1000.0
        )));
    }

    app.output.push(OutputLine::System(String::new()));
    app.output.push(OutputLine::System(
        "  Нажмите Enter или Q для выхода".to_string(),
    ));
    app.finished = true;

    let mut terminal = init_terminal()?;
    let result = run_file_loop(&mut terminal, &mut app);
    restore_terminal()?;
    result
}

fn run_file_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut FileRunnerApp,
) -> Result<(), String> {
    loop {
        terminal
            .draw(|frame| draw_file_runner(frame, app))
            .map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())?
            && let Event::Key(key) = event::read().map_err(|e| e.to_string())?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter if app.finished => break,
                KeyCode::Up | KeyCode::Char('k') => app.scroll = app.scroll.saturating_sub(1),
                KeyCode::Down | KeyCode::Char('j') => {
                    app.scroll = app
                        .scroll
                        .saturating_add(1)
                        .min(app.output.len().saturating_sub(1))
                }
                KeyCode::PageUp => app.scroll = app.scroll.saturating_sub(10),
                KeyCode::PageDown => {
                    app.scroll = app
                        .scroll
                        .saturating_add(10)
                        .min(app.output.len().saturating_sub(1))
                }
                _ => {}
            }
        }
    }
    Ok(())
}

fn draw_file_runner(frame: &mut Frame, app: &FileRunnerApp) {
    let area = frame.area();

    let title = if app.error.is_some() {
        " Выполнение (ОШИБКА) "
    } else {
        " Выполнение "
    };
    let border_color = if app.error.is_some() {
        Color::Red
    } else {
        Color::Green
    };

    let block = Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible_height = inner.height as usize;
    let total = app.output.len();
    let start = app.scroll.min(total.saturating_sub(1));
    let end = (start + visible_height).min(total);

    let items: Vec<ListItem> = app.output[start..end]
        .iter()
        .map(|line| ListItem::new(line.to_styled_line()))
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);

    if total > visible_height {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        let mut state = ScrollbarState::new(total).position(app.scroll);
        frame.render_stateful_widget(scrollbar, area, &mut state);
    }
}
