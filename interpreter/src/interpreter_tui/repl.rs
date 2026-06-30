use crate::interpreter::Interpreter;
use crate::terminal::{init_terminal, restore_terminal};
use crate::ui::OutputLine;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use shared::types::Value;
use std::io::Stdout;
use std::time::Duration;

pub(crate) struct ReplApp {
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

                if !matches!(value, Value::Null) {
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

pub(crate) fn run_repl_tui(debug: bool) -> Result<(), String> {
    let mut terminal = init_terminal()?;
    let mut app = ReplApp::new(debug);

    loop {
        terminal
            .draw(|frame| draw_repl(frame, &app))
            .map_err(|e| e.to_string())?;

        if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())?
            && let Event::Key(key) = event::read().map_err(|e| e.to_string())?
            && key.kind == KeyEventKind::Press
        {
            app.handle_key(key.code, key.modifiers);
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
