//! Kumir 3 Interpreter - TUI
//!
//! Интерпретатор языка Кумир с полным TUI интерфейсом:
//! - Выполнение .kum файлов
//! - Интерактивный режим (REPL)
//! - Отладка программ
//! - Проверка синтаксиса
//! - Просмотр AST

use clap::{Parser, Subcommand};
use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    prelude::*,
    widgets::{
        Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
    },
};
use std::fs;
use std::io::{Stdout, stdout};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use kumir3_corelib::interpreter::Interpreter;
pub use kumir3_corelib::interpreter::cli::{Cli, Commands};
use kumir3_corelib::shared::parser::parse;

// =============================================================================
//                              MAIN
// =============================================================================

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Run { file, debug, time }) => run_file_tui(&file, debug, time),
        Some(Commands::Repl { debug }) => run_repl_tui(debug),
        Some(Commands::Check { file }) => run_check_tui(&file),
        Some(Commands::Ast { file }) => run_ast_tui(&file),
        Some(Commands::Info) => run_info_tui(),
        None => {
            if let Some(file) = cli.file {
                run_file_tui(&file, cli.debug, cli.time)
            } else {
                run_repl_tui(cli.debug)
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Ошибка: {}", e);
        std::process::exit(1);
    }
}

// =============================================================================
//                              TUI INFRASTRUCTURE
// =============================================================================

fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, String> {
    enable_raw_mode().map_err(|e| e.to_string())?;
    stdout()
        .execute(EnterAlternateScreen)
        .map_err(|e| e.to_string())?;
    Terminal::new(CrosstermBackend::new(stdout())).map_err(|e| e.to_string())
}

fn restore_terminal() -> Result<(), String> {
    disable_raw_mode().map_err(|e| e.to_string())?;
    stdout()
        .execute(LeaveAlternateScreen)
        .map_err(|e| e.to_string())?;
    Ok(())
}

// =============================================================================
//                              OUTPUT LINE TYPE
// =============================================================================

#[derive(Clone)]
enum OutputLine {
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
    fn to_styled_line(&self) -> Line<'_> {
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

// =============================================================================
//                              INFO TUI
// =============================================================================

fn run_info_tui() -> Result<(), String> {
    let mut terminal = init_terminal()?;
    let result = run_info_loop(&mut terminal);
    restore_terminal()?;
    result
}

fn run_info_loop(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), String> {
    loop {
        terminal
            .draw(|frame| draw_info(frame))
            .map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => break,
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

fn draw_info(frame: &mut Frame) {
    let area = frame.area();

    let block = Block::default()
        .title(" KUMIR 3 INTERPRETER ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(12),
            Constraint::Length(7),
            Constraint::Min(1),
        ])
        .split(inner);

    // Заголовок
    let title = Paragraph::new(vec![
        Line::from("╔══════════════════════════════════════════════════════════╗")
            .style(Style::default().fg(Color::Cyan)),
        Line::from("║              KUMIR 3 - Интерпретатор                     ║")
            .style(Style::default().fg(Color::Cyan).bold()),
        Line::from("╚══════════════════════════════════════════════════════════╝")
            .style(Style::default().fg(Color::Cyan)),
    ]);
    frame.render_widget(title, chunks[0]);

    // Версия
    let version = Paragraph::new(vec![Line::from(vec![
        Span::styled("  Версия: ", Style::default().fg(Color::White).bold()),
        Span::styled(env!("CARGO_PKG_VERSION"), Style::default().fg(Color::Green)),
    ])]);
    frame.render_widget(version, chunks[1]);

    // Возможности
    let features = Paragraph::new(vec![
        Line::from(Span::styled(
            "  Возможности:",
            Style::default().fg(Color::White).bold(),
        )),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Базовые типы: цел, вещ, лит, лог, сим"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Массивы и таблицы"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Алгоритмы с параметрами"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("ООП: классы, объекты, методы"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Перечисления и сопоставление с образцом"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Обработка исключений (попытка/перехват)"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Импорт модулей (.kum файлы)"),
        ]),
        Line::from(vec![
            Span::styled("    [+] ", Style::default().fg(Color::Green)),
            Span::raw("Стандартные библиотеки"),
        ]),
    ]);
    frame.render_widget(features, chunks[2]);

    // Использование
    let usage = Paragraph::new(vec![
        Line::from(Span::styled(
            "  Использование:",
            Style::default().fg(Color::White).bold(),
        )),
        Line::from(vec![
            Span::styled(
                "    kumir3-interpreter ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("<файл.kum>", Style::default().fg(Color::Cyan)),
            Span::raw("     Выполнить файл"),
        ]),
        Line::from(vec![
            Span::styled(
                "    kumir3-interpreter ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("repl", Style::default().fg(Color::Cyan)),
            Span::raw("             Интерактивный режим"),
        ]),
        Line::from(vec![
            Span::styled(
                "    kumir3-interpreter ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("check <файл>", Style::default().fg(Color::Cyan)),
            Span::raw("   Проверить синтаксис"),
        ]),
        Line::from(vec![
            Span::styled(
                "    kumir3-interpreter ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled("--help", Style::default().fg(Color::Cyan)),
            Span::raw("           Помощь"),
        ]),
    ]);
    frame.render_widget(usage, chunks[3]);

    // Подсказка
    let hint = Paragraph::new(Line::from(Span::styled(
        "  Нажмите Enter или Q для выхода",
        Style::default().fg(Color::DarkGray),
    )));
    frame.render_widget(hint, chunks[4]);
}

// =============================================================================
//                              FILE RUNNER TUI
// =============================================================================

struct FileRunnerApp {
    output: Vec<OutputLine>,
    scroll: usize,
    finished: bool,
    error: Option<String>,
}

fn run_file_tui(path: &PathBuf, debug: bool, show_time: bool) -> Result<(), String> {
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
            if !matches!(value, kumir3_corelib::shared::types::Value::Null) {
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

        if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter if app.finished => break,
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.scroll = app.scroll.saturating_sub(1)
                        }
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

// =============================================================================
//                              CHECK SYNTAX TUI
// =============================================================================

fn run_check_tui(path: &PathBuf) -> Result<(), String> {
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

        if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => break,
                        _ => {}
                    }
                }
            }
        }
    }
    Ok(())
}

// =============================================================================
//                              AST VIEWER TUI
// =============================================================================

fn run_ast_tui(path: &PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Файл не найден: {}", path.display()));
    }

    let source = fs::read_to_string(path).map_err(|e| format!("Ошибка чтения: {}", e))?;

    let program = parse(&source).map_err(|e| format!("Ошибка парсинга: {}", e))?;

    let mut output: Vec<OutputLine> = Vec::new();

    output.push(OutputLine::Header("  AST Программы".to_string()));
    output.push(OutputLine::System("═".repeat(50)));
    output.push(OutputLine::System(String::new()));

    output.push(OutputLine::Header("  Импорты:".to_string()));
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
                .map(|p| format!("{}: {:?}", p.name, p.type_spec))
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

        if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                if key.kind == KeyEventKind::Press {
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
        }
    }

    restore_terminal()?;
    Ok(())
}

// =============================================================================
//                              REPL TUI
// =============================================================================

struct ReplApp {
    interpreter: Interpreter,
    input: String,
    cursor_pos: usize,
    output: Vec<OutputLine>,
    history: Vec<String>,
    history_idx: isize,
    saved_input: String,
    scroll: usize,
    debug_mode: bool,
    should_quit: bool,
    multiline: bool,
    multiline_buf: String,
}

impl ReplApp {
    fn new(debug: bool) -> Self {
        let mut interpreter = Interpreter::new();
        interpreter.set_debug_mode(debug);

        let mut app = Self {
            interpreter,
            input: String::new(),
            cursor_pos: 0,
            output: Vec::new(),
            history: Vec::new(),
            history_idx: -1,
            saved_input: String::new(),
            scroll: 0,
            debug_mode: debug,
            should_quit: false,
            multiline: false,
            multiline_buf: String::new(),
        };

        app.output.push(OutputLine::Header(
            "╔══════════════════════════════════════════════════════════════╗".to_string(),
        ));
        app.output.push(OutputLine::Header(
            "║           KUMIR 3 - Интерактивный режим (TUI)                ║".to_string(),
        ));
        app.output.push(OutputLine::Header(
            "╚══════════════════════════════════════════════════════════════╝".to_string(),
        ));
        app.output.push(OutputLine::System(String::new()));
        app.output.push(OutputLine::System(
            "  Введите код на языке Кумир. Команды: .помощь .выход".to_string(),
        ));
        app.output.push(OutputLine::System(
            "  Ctrl+D выход | Page Up/Down прокрутка".to_string(),
        ));
        app.output.push(OutputLine::System(String::new()));

        app
    }

    fn execute(&mut self) {
        let input = if self.multiline {
            self.multiline_buf.push_str(&self.input);
            self.multiline_buf.push('\n');

            let trimmed = self.input.trim();
            if trimmed == "кон" || trimmed == "все" || trimmed == "кц" {
                let opens = self.multiline_buf.matches("нач").count()
                    + self.multiline_buf.matches("нц").count()
                    + self.multiline_buf.matches("если").count();
                let closes = self.multiline_buf.matches("кон").count()
                    + self.multiline_buf.matches("кц").count()
                    + self.multiline_buf.matches("все").count();

                if closes >= opens {
                    self.multiline = false;
                    let code = std::mem::take(&mut self.multiline_buf);
                    self.input.clear();
                    self.cursor_pos = 0;
                    code
                } else {
                    self.output
                        .push(OutputLine::Input(format!("...   {}", self.input)));
                    self.input.clear();
                    self.cursor_pos = 0;
                    return;
                }
            } else {
                self.output
                    .push(OutputLine::Input(format!("...   {}", self.input)));
                self.input.clear();
                self.cursor_pos = 0;
                return;
            }
        } else {
            std::mem::take(&mut self.input)
        };

        self.cursor_pos = 0;

        if input.trim().is_empty() {
            return;
        }

        self.history.push(input.clone());
        self.history_idx = -1;

        self.output
            .push(OutputLine::Input(format!("кумир> {}", input)));

        let trimmed = input.trim();
        if trimmed.ends_with("нач")
            || trimmed.ends_with("то")
            || trimmed.ends_with("иначе")
            || trimmed.ends_with("нц")
            || trimmed.ends_with("класс")
        {
            self.multiline = true;
            self.multiline_buf = input;
            self.multiline_buf.push('\n');
            return;
        }

        match trimmed {
            ".exit" | ".quit" | ".q" | ".выход" => {
                self.should_quit = true;
                return;
            }
            ".help" | ".h" | ".помощь" => {
                self.show_help();
                return;
            }
            ".clear" | ".cls" | ".очистить" => {
                self.output.clear();
                return;
            }
            ".debug" | ".отладка" => {
                self.debug_mode = !self.debug_mode;
                self.interpreter.set_debug_mode(self.debug_mode);
                let msg = if self.debug_mode {
                    "включён"
                } else {
                    "выключен"
                };
                self.output
                    .push(OutputLine::System(format!("  Режим отладки: {}", msg)));
                return;
            }
            ".vars" | ".переменные" => {
                self.output.push(OutputLine::System(
                    "  Переменные: (пока не реализовано)".to_string(),
                ));
                return;
            }
            ":q!" | ":q" | ":wq" | ":x" => {
                self.output.push(OutputLine::Warning(
                    "  Мы не в Vim, к сожалению.".to_string(),
                ));
                self.output.push(OutputLine::Warning(
                    "  Но автор оценил ваше стремление к оптимизации).".to_string(),
                ));
                self.output.push(OutputLine::Warning(
                    "  Используйте .выход для выхода из консоли.".to_string(),
                ));
                return;
            }
            ".ai" | ".vibecode" | ".vibecoding" => {
                self.output.push(OutputLine::Error(
                    "  Какой ВАЙБ-КОДИНГ в REPL!?".to_string(),
                ));
                self.output.push(OutputLine::Error(
                    "  Пожалуйста, используйте нормальные команды и не позорьтесь.".to_string(),
                ));
                return;
            }
            ".whoareyou" => {
                self.output.push(OutputLine::System(
                    "  Я - интерактивная консоль Кумир 3.".to_string(),
                ));
                self.output.push(OutputLine::System(
                    "  Моя задача - помочь вам выполнять код на языке Кумир.".to_string(),
                ));
                self.output.push(OutputLine::System(
                    "  Пожалуйста, используйте меня по назначению.".to_string(),
                ));
                return;
            }
            _ if trimmed.starts_with('.') => {
                self.output.push(OutputLine::Error(format!(
                    "  Неизвестная команда: {}",
                    trimmed
                )));
                return;
            }
            _ => {}
        }

        self.interpreter.clear_output();

        match self.interpreter.run(&input) {
            Ok(value) => {
                let out = self.interpreter.get_output();
                if !out.is_empty() {
                    for line in out.lines() {
                        self.output.push(OutputLine::Normal(format!("  {}", line)));
                    }
                }

                if !matches!(value, kumir3_corelib::shared::types::Value::Null) {
                    self.output
                        .push(OutputLine::Success(format!("  => {:?}", value)));
                }
            }
            Err(e) => {
                self.output
                    .push(OutputLine::Error(format!("  Ошибка: {}", e)));
            }
        }

        self.scroll_to_bottom();
    }

    fn show_help(&mut self) {
        let help = vec![
            "",
            "  ╭─────────────────────────────────────────╮",
            "  │           Команды REPL                  │",
            "  ├─────────────────────────────────────────┤",
            "  │  .помощь     - эта справка              │",
            "  │  .выход      - выйти                    │",
            "  │  .очистить   - очистить экран           │",
            "  │  .отладка    - вкл/выкл отладку         │",
            "  │  .переменные - показать переменные      │",
            "  ├─────────────────────────────────────────┤",
            "  │           Горячие клавиши               │",
            "  ├─────────────────────────────────────────┤",
            "  │  Enter       - выполнить                │",
            "  │  Up/Down     - история команд           │",
            "  │  Ctrl+C      - прервать ввод            │",
            "  │  Ctrl+D      - выход                    │",
            "  │  Page Up/Dn  - прокрутка                │",
            "  ╰─────────────────────────────────────────╯",
            "",
            "  Пример:",
            "    цел x := 5",
            "    вывод x * 2",
            "",
        ];

        for line in help {
            self.output.push(OutputLine::System(line.to_string()));
        }
    }

    fn scroll_to_bottom(&mut self) {
        if self.output.len() > 5 {
            self.scroll = self.output.len().saturating_sub(5);
        }
    }

    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        match (code, mods) {
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => self.should_quit = true,
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                if self.multiline {
                    self.multiline = false;
                    self.multiline_buf.clear();
                    self.output
                        .push(OutputLine::Warning("  Ввод отменён".to_string()));
                }
                self.input.clear();
                self.cursor_pos = 0;
            }
            (KeyCode::Enter, _) => self.execute(),
            (KeyCode::Up, _) => {
                if !self.history.is_empty() {
                    if self.history_idx == -1 {
                        self.saved_input = self.input.clone();
                    }
                    if self.history_idx < self.history.len() as isize - 1 {
                        self.history_idx += 1;
                        let idx = self.history.len() - 1 - self.history_idx as usize;
                        self.input = self.history[idx].clone();
                        self.cursor_pos = self.input.len();
                    }
                }
            }
            (KeyCode::Down, _) => {
                if self.history_idx > -1 {
                    self.history_idx -= 1;
                    if self.history_idx == -1 {
                        self.input = self.saved_input.clone();
                    } else {
                        let idx = self.history.len() - 1 - self.history_idx as usize;
                        self.input = self.history[idx].clone();
                    }
                    self.cursor_pos = self.input.len();
                }
            }
            (KeyCode::PageUp, _) => self.scroll = self.scroll.saturating_sub(5),
            (KeyCode::PageDown, _) => {
                self.scroll = self
                    .scroll
                    .saturating_add(5)
                    .min(self.output.len().saturating_sub(1))
            }
            (KeyCode::Left, _) => self.cursor_pos = self.cursor_pos.saturating_sub(1),
            (KeyCode::Right, _) => {
                if self.cursor_pos < self.input.len() {
                    self.cursor_pos += 1
                }
            }
            (KeyCode::Home, _) => self.cursor_pos = 0,
            (KeyCode::End, _) => self.cursor_pos = self.input.len(),
            (KeyCode::Backspace, _) => {
                if self.cursor_pos > 0 {
                    self.input.remove(self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                }
            }
            (KeyCode::Delete, _) => {
                if self.cursor_pos < self.input.len() {
                    self.input.remove(self.cursor_pos);
                }
            }
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.input.insert(self.cursor_pos, c);
                self.cursor_pos += 1;
            }
            _ => {}
        }
    }
}

fn run_repl_tui(debug: bool) -> Result<(), String> {
    let mut terminal = init_terminal()?;
    let mut app = ReplApp::new(debug);

    loop {
        terminal
            .draw(|frame| draw_repl(frame, &app))
            .map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())? {
            if let Event::Key(key) = event::read().map_err(|e| e.to_string())? {
                if key.kind == KeyEventKind::Press {
                    app.handle_key(key.code, key.modifiers);
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    restore_terminal()?;
    Ok(())
}

fn draw_repl(frame: &mut Frame, app: &ReplApp) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(5), Constraint::Length(3)])
        .split(area);

    // Вывод
    let output_block = Block::default()
        .title(" Вывод ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = output_block.inner(chunks[0]);
    frame.render_widget(output_block, chunks[0]);

    let visible = inner.height as usize;
    let total = app.output.len();
    let start = app.scroll.min(total.saturating_sub(1));
    let end = (start + visible).min(total);

    let items: Vec<ListItem> = app
        .output
        .get(start..end)
        .unwrap_or(&[])
        .iter()
        .map(|line| ListItem::new(line.to_styled_line()))
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);

    if total > visible {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"));
        let mut state = ScrollbarState::new(total).position(app.scroll);
        frame.render_stateful_widget(
            scrollbar,
            chunks[0].inner(Margin {
                horizontal: 0,
                vertical: 1,
            }),
            &mut state,
        );
    }

    // Ввод
    let prompt = if app.multiline {
        "...   "
    } else {
        "кумир> "
    };
    let input_text = format!("{}{}", prompt, app.input);

    let title = if app.debug_mode {
        " Ввод [отладка] "
    } else {
        " Ввод "
    };
    let border_color = if app.multiline {
        Color::Yellow
    } else {
        Color::Green
    };

    let input_block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let input_para = Paragraph::new(input_text)
        .style(Style::default().fg(Color::White))
        .block(input_block);

    frame.render_widget(input_para, chunks[1]);

    // Курсор
    let cursor_x = chunks[1].x + 1 + prompt.chars().count() as u16 + app.cursor_pos as u16;
    let cursor_y = chunks[1].y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}
