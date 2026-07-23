//! Интерактивный режим.
//!
//! Устройство экрана: заголовок, рабочая область, строка ввода, строка клавиш.
//! Рабочая область делится на вывод и боковую панель — она показывает
//! переменные и алгоритмы, определённые за сеанс. Панель и есть главное
//! отличие от обычной консоли: состояние программы видно всё время, а не
//! только в тот момент, когда его напечатали.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Wrap,
};
use shared::types::Value;
use std::time::Duration;

use crate::editor::InputLine;
use crate::interpreter::Interpreter;
use crate::syntax;
use crate::terminal::{init_terminal, restore_terminal};
use crate::theme;
use crate::ui::OutputLine;

/// Что показывает боковая панель.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Panel {
    /// Переменные и их значения.
    State,
    /// Определённые алгоритмы.
    Algorithms,
    /// Панель скрыта — весь экран под вывод.
    Hidden,
}

impl Panel {
    fn next(self) -> Self {
        match self {
            Self::State => Self::Algorithms,
            Self::Algorithms => Self::Hidden,
            Self::Hidden => Self::State,
        }
    }

    fn title(self) -> &'static str {
        match self {
            Self::State => " Состояние ",
            Self::Algorithms => " Алгоритмы ",
            Self::Hidden => "",
        }
    }
}

pub(crate) struct ReplApp {
    interpreter: Interpreter,
    input: InputLine,
    output: Vec<OutputLine>,
    history: Vec<String>,
    history_idx: isize,
    saved_input: String,
    /// Сколько строк вывода прокручено вверх от конца. 0 — виден конец.
    scroll_back: usize,
    panel: Panel,
    show_help: bool,
    debug_mode: bool,
    should_quit: bool,
    /// Накопленный текст незакрытой конструкции (`алг`, `нц`, `если`).
    pending: String,
    /// Сколько конструкций ещё не закрыто.
    depth: usize,
    /// Сколько заголовков `алг` ждут своего `нач`.
    awaiting_body: usize,
    /// Варианты дополнения текущего слова; пусто — перебор не идёт.
    completions: Vec<String>,
    completion_idx: usize,
}

impl ReplApp {
    fn new(debug: bool) -> Self {
        let mut interpreter = Interpreter::new();
        interpreter.set_debug_mode(debug);

        let mut app = Self {
            interpreter,
            input: InputLine::default(),
            output: Vec::new(),
            history: Vec::new(),
            history_idx: -1,
            saved_input: String::new(),
            scroll_back: 0,
            panel: Panel::State,
            show_help: false,
            debug_mode: debug,
            should_quit: false,
            pending: String::new(),
            depth: 0,
            awaiting_body: 0,
            completions: Vec::new(),
            completion_idx: 0,
        };

        app.output.push(OutputLine::System(
            "Введите программу на Кумире. F1 — помощь, Tab — дополнить имя.".to_string(),
        ));
        app.output.push(OutputLine::System(String::new()));
        app
    }

    // -----------------------------------------------------------------------
    //                            ВЫПОЛНЕНИЕ
    // -----------------------------------------------------------------------

    /// Обрабатывает нажатие Enter.
    fn submit(&mut self) {
        let line = self.input.take();
        self.completions.clear();
        self.scroll_back = 0;

        if self.depth == 0 && line.trim().is_empty() {
            return;
        }

        self.history.push(line.clone());
        self.history_idx = -1;

        let prompt = if self.depth == 0 {
            "кумир> "
        } else {
            "  ...  "
        };
        self.output
            .push(OutputLine::Input(format!("{prompt}{line}")));

        if self.depth == 0 && line.trim_start().starts_with([',', '.', ':']) {
            self.run_command(line.trim());
            return;
        }

        // Незакрытая конструкция копится до своего `кон`/`кц`/`все`.
        self.track_depth(&line);
        self.pending.push_str(&line);
        self.pending.push('\n');

        if self.depth > 0 {
            return;
        }

        let code = std::mem::take(&mut self.pending);
        self.run_code(&code);
    }

    /// Пересчитывает число незакрытых конструкций после очередной строки.
    ///
    /// `алг` и `нач` считаются вместе, а не по отдельности: заголовок
    /// алгоритма и его тело закрываются одним `кон`, поэтому `нач`, идущий за
    /// `алг`, глубину не увеличивает. Без этого `алг цел удвоить(цел x)`
    /// уходил на выполнение сразу — до того, как человек напишет `нач`, — и
    /// разбор жаловался на неожиданный конец файла.
    fn track_depth(&mut self, line: &str) {
        let algs = count_words(line, &["алг"]);
        let begins = count_words(line, &["нач"]);
        let opens = count_words(line, &["нц", "если", "выбор", "попытка"]);
        let closes = closes_in(line);

        self.depth += algs + opens;

        // `нач`, закрывающий ожидание заголовка, своей глубины не добавляет.
        let covered = begins.min(self.awaiting_body + algs);
        self.awaiting_body = self.awaiting_body + algs - covered;
        self.depth += begins - covered;

        self.depth = self.depth.saturating_sub(closes);
        if self.depth == 0 {
            self.awaiting_body = 0;
        }
    }

    fn run_code(&mut self, code: &str) {
        self.interpreter.clear_output();
        // Интерактивный запуск: набранное строкой выше должно быть видно
        // строкой ниже, поэтому свободные инструкции выполняются в общей
        // области, а не в снимаемом кадре.
        let outcome = self.interpreter.run_interactive(code);

        // Вывод показывается в обоих случаях: при ошибке он говорит, докуда
        // дошло выполнение, и без него причину искать труднее.
        let printed = self.interpreter.get_output();
        for line in printed.lines() {
            self.output.push(OutputLine::Normal(format!("  {line}")));
        }

        match outcome {
            Ok(value) => {
                if !matches!(value, Value::Null) {
                    self.output
                        .push(OutputLine::Success(format!("  ⇒ {value}")));
                }
            }
            Err(err) => self.output.push(OutputLine::Error(format!("  ✗ {err}"))),
        }
    }

    fn run_command(&mut self, command: &str) {
        let (name, arg) = match command.split_once(char::is_whitespace) {
            Some((n, a)) => (n, a.trim()),
            None => (command, ""),
        };

        let mut say = |line: &str, kind: fn(String) -> OutputLine| {
            self.output.push(kind(format!("  {line}")));
        };

        match name {
            ".выход" | ".exit" | ".quit" | ".q" => self.should_quit = true,
            ".помощь" | ".help" | ".h" | ".?" => self.show_help = true,
            ".очистить" | ".clear" | ".cls" => {
                self.output.clear();
                self.scroll_back = 0;
            }
            ".переменные" | ".vars" => self.panel = Panel::State,
            ".алгоритмы" | ".algs" => self.panel = Panel::Algorithms,
            ".отладка" | ".debug" => {
                self.debug_mode = !self.debug_mode;
                self.interpreter.set_debug_mode(self.debug_mode);
                let state = if self.debug_mode {
                    "включён"
                } else {
                    "выключен"
                };
                say(&format!("Режим отладки: {state}"), OutputLine::System);
            }
            ".сброс" | ".reset" => {
                self.interpreter = Interpreter::new();
                self.interpreter.set_debug_mode(self.debug_mode);
                self.pending.clear();
                self.depth = 0;
                self.awaiting_body = 0;
                say("Состояние сброшено", OutputLine::Success);
            }
            ".загрузить" | ".load" => self.load_file(arg),

            // Пасхалки: без них консоль была бы скучнее.
            ":q!" | ":q" | ":wq" | ":x" => {
                say("Мы не в Vim, к сожалению.", OutputLine::Warning);
                say(
                    "Но автор оценил ваше стремление к оптимизации).",
                    OutputLine::Warning,
                );
                say("Для выхода — .выход или Ctrl+D.", OutputLine::Warning);
            }
            ".ai" | ".vibecode" | ".vibecoding" => {
                say("Какой ВАЙБ-КОДИНГ в REPL!?", OutputLine::Error);
                say(
                    "Пожалуйста, используйте нормальные команды и не позорьтесь.",
                    OutputLine::Error,
                );
            }
            ".whoareyou" => {
                say("Я — интерактивная консоль Кумир 3.", OutputLine::System);
                say(
                    "Моя задача — помочь вам выполнять код на языке Кумир.",
                    OutputLine::System,
                );
            }

            other => say(
                &format!("Неизвестная команда «{other}». Список — F1."),
                OutputLine::Error,
            ),
        }
    }

    fn load_file(&mut self, path: &str) {
        if path.is_empty() {
            self.output.push(OutputLine::Error(
                "  ✗ Укажите файл: .загрузить путь/к/программе.kum".to_string(),
            ));
            return;
        }
        match std::fs::read_to_string(path) {
            Ok(source) => {
                self.output
                    .push(OutputLine::Success(format!("  Загружен {path}")));
                self.run_code(&source);
            }
            Err(err) => self
                .output
                .push(OutputLine::Error(format!("  ✗ {path}: {err}"))),
        }
    }

    // -----------------------------------------------------------------------
    //                          АВТОДОПОЛНЕНИЕ
    // -----------------------------------------------------------------------

    /// Дополняет слово под курсором; повторный Tab перебирает варианты.
    fn complete(&mut self) {
        if !self.completions.is_empty() {
            self.completion_idx = (self.completion_idx + 1) % self.completions.len();
            let pick = self.completions[self.completion_idx].clone();
            self.input.replace_word_at_cursor(&pick);
            return;
        }

        let prefix = self.input.word_at_cursor();
        if prefix.is_empty() {
            return;
        }
        let lower = prefix.to_lowercase();

        // Дополняется всё, что можно назвать: слова языка, встроенные функции
        // и то, что определил сам пользователь за сеанс.
        let env = self.interpreter.environment();
        let mut found: Vec<String> = shared::constants::keywords::all_keywords()
            .iter()
            .map(|s| (*s).to_string())
            .chain(
                shared::constants::builtins::get_all_builtin_names()
                    .iter()
                    .map(|s| (*s).to_string()),
            )
            .chain(env.algorithm_names())
            .chain(env.globals_snapshot().into_iter().map(|(name, _, _)| name))
            .filter(|name| name.to_lowercase().starts_with(&lower))
            .collect();
        found.sort();
        found.dedup();

        let Some(first) = found.first().cloned() else {
            return;
        };
        self.completion_idx = 0;
        self.input.replace_word_at_cursor(&first);
        // Единственный вариант перебирать незачем — список не запоминаем.
        self.completions = if found.len() > 1 { found } else { Vec::new() };
    }

    // -----------------------------------------------------------------------
    //                              КЛАВИШИ
    // -----------------------------------------------------------------------

    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        // Помощь закрывается любой клавишей: это справка, а не режим.
        if self.show_help {
            self.show_help = false;
            return;
        }

        let ctrl = mods.contains(KeyModifiers::CONTROL);

        // Любое действие, кроме Tab, прекращает перебор дополнений.
        if code != KeyCode::Tab {
            self.completions.clear();
        }

        match (code, ctrl) {
            (KeyCode::Char('d'), true) => self.should_quit = true,
            (KeyCode::Char('c'), true) => {
                if self.depth > 0 {
                    self.depth = 0;
                    self.awaiting_body = 0;
                    self.pending.clear();
                    self.output
                        .push(OutputLine::Warning("  Ввод отменён".to_string()));
                }
                self.input.clear();
            }
            (KeyCode::Char('l'), true) => {
                self.output.clear();
                self.scroll_back = 0;
            }
            (KeyCode::Char('u'), true) => self.input.delete_to_start(),
            (KeyCode::Char('k'), true) => self.input.delete_to_end(),
            (KeyCode::Char('w'), true) => self.input.delete_word_left(),
            (KeyCode::Left, true) => self.input.word_left(),
            (KeyCode::Right, true) => self.input.word_right(),

            (KeyCode::F(1), _) => self.show_help = true,
            (KeyCode::F(2), _) => self.panel = self.panel.next(),
            (KeyCode::Esc, _) => self.input.clear(),
            (KeyCode::Tab, _) => self.complete(),
            (KeyCode::Enter, _) => self.submit(),

            (KeyCode::Up, _) => self.history_back(),
            (KeyCode::Down, _) => self.history_forward(),
            (KeyCode::Left, _) => self.input.left(),
            (KeyCode::Right, _) => self.input.right(),
            (KeyCode::Home, _) => self.input.home(),
            (KeyCode::End, _) => self.input.end(),
            (KeyCode::Backspace, _) => self.input.backspace(),
            (KeyCode::Delete, _) => self.input.delete(),

            (KeyCode::PageUp, _) => self.scroll_back = self.scroll_back.saturating_add(10),
            (KeyCode::PageDown, _) => self.scroll_back = self.scroll_back.saturating_sub(10),

            (KeyCode::Char(ch), false) => self.input.insert(ch),
            _ => {}
        }
    }

    fn history_back(&mut self) {
        if self.history.is_empty() {
            return;
        }
        if self.history_idx == -1 {
            self.saved_input = self.input.text();
        }
        if self.history_idx < self.history.len() as isize - 1 {
            self.history_idx += 1;
            let idx = self.history.len() - 1 - self.history_idx as usize;
            let entry = self.history[idx].clone();
            self.input.set(&entry);
        }
    }

    fn history_forward(&mut self) {
        if self.history_idx <= -1 {
            return;
        }
        self.history_idx -= 1;
        let entry = if self.history_idx == -1 {
            self.saved_input.clone()
        } else {
            let idx = self.history.len() - 1 - self.history_idx as usize;
            self.history[idx].clone()
        };
        self.input.set(&entry);
    }
}

/// Сколько конструкций открывает строка.
fn opens_in(line: &str) -> usize {
    count_words(line, &["нач", "нц", "если", "выбор", "попытка"])
}

/// Сколько конструкций закрывает строка.
fn closes_in(line: &str) -> usize {
    count_words(line, &["кон", "кц", "все", "кв"])
}

/// Считает вхождения слов как отдельных слов, а не подстрок.
///
/// Иначе `конец := 1` закрыл бы конструкцию, а `начало := 1` — открыло: обе
/// строки содержат ключевое слово внутри имени переменной. Комментарий
/// отбрасывается: `| кон` тоже ничего не закрывает.
fn count_words(line: &str, words: &[&str]) -> usize {
    let code = line.split('|').next().unwrap_or("");
    code.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|token| words.contains(&token.to_lowercase().as_str()))
        .count()
}

// ===========================================================================
//                              ОТРИСОВКА
// ===========================================================================

pub(crate) fn run_repl_tui(debug: bool) -> Result<(), String> {
    let mut terminal = init_terminal()?;
    let mut app = ReplApp::new(debug);

    let result = (|| -> Result<(), String> {
        loop {
            terminal
                .draw(|frame| draw(frame, &app))
                .map_err(|e| e.to_string())?;

            if event::poll(Duration::from_millis(50)).map_err(|e| e.to_string())?
                && let Event::Key(key) = event::read().map_err(|e| e.to_string())?
                && key.kind == KeyEventKind::Press
            {
                app.handle_key(key.code, key.modifiers);
            }

            if app.should_quit {
                return Ok(());
            }
        }
    })();

    // Терминал возвращается в исходное состояние даже при ошибке отрисовки:
    // иначе управление вернётся в консоль без эха и без курсора.
    restore_terminal()?;
    result
}

fn draw(frame: &mut Frame, app: &ReplApp) {
    let rows = Layout::vertical([
        Constraint::Length(1), // заголовок
        Constraint::Min(3),    // вывод и панель
        Constraint::Length(3), // ввод
        Constraint::Length(1), // клавиши
    ])
    .split(frame.area());

    draw_header(frame, rows[0], app);

    let body = if app.panel == Panel::Hidden {
        Layout::horizontal([Constraint::Percentage(100)]).split(rows[1])
    } else {
        Layout::horizontal([Constraint::Min(30), Constraint::Length(34)]).split(rows[1])
    };

    draw_output(frame, body[0], app);
    if app.panel != Panel::Hidden {
        draw_panel(frame, body[1], app);
    }

    draw_input(frame, rows[2], app);
    draw_keys(frame, rows[3], app);

    if app.show_help {
        draw_help(frame);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &ReplApp) {
    let mut spans = vec![
        Span::styled(
            " Кумир 3 ",
            Style::default().fg(Color::Black).bg(theme::ACCENT),
        ),
        Span::styled("  интерактивный режим", Style::default().fg(theme::MUTED)),
    ];
    if app.debug_mode {
        spans.push(Span::styled(
            "  ·  отладка",
            Style::default().fg(theme::WARN),
        ));
    }
    if app.depth > 0 {
        spans.push(Span::styled(
            format!("  ·  не закрыто конструкций: {}", app.depth),
            Style::default().fg(theme::WARN),
        ));
    }
    frame.render_widget(Line::from(spans), area);
}

fn draw_output(frame: &mut Frame, area: Rect, app: &ReplApp) {
    let block = Block::default()
        .title(Span::styled(" Вывод ", theme::title(true)))
        .borders(Borders::ALL)
        .border_style(theme::border(true));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let visible = inner.height as usize;
    let total = app.output.len();
    // Показывается хвост: прокрутка отсчитывается от конца, поэтому новые
    // строки видны сразу и «догонять» их не нужно.
    let end = total.saturating_sub(app.scroll_back);
    let start = end.saturating_sub(visible);

    let items: Vec<ListItem> = app.output[start..end]
        .iter()
        .map(|line| ListItem::new(line.to_styled_line()))
        .collect();
    frame.render_widget(List::new(items), inner);

    if total > visible {
        let mut state = ScrollbarState::new(total).position(start);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area.inner(Margin {
                horizontal: 0,
                vertical: 1,
            }),
            &mut state,
        );
    }
}

fn draw_panel(frame: &mut Frame, area: Rect, app: &ReplApp) {
    let block = Block::default()
        .title(Span::styled(app.panel.title(), theme::title(false)))
        .borders(Borders::ALL)
        .border_style(theme::border(false));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let env = app.interpreter.environment();
    let empty = |text: &str| {
        vec![Line::from(Span::styled(
            format!(" {text}"),
            Style::default().fg(theme::MUTED),
        ))]
    };

    let lines: Vec<Line> = match app.panel {
        Panel::State => {
            let items = env.globals_snapshot();
            if items.is_empty() {
                empty("пока ничего не определено")
            } else {
                items
                    .iter()
                    .map(|(name, value, is_const)| {
                        let name_color = if *is_const { theme::TYPE } else { Color::White };
                        Line::from(vec![
                            Span::styled(format!("{name} "), Style::default().fg(name_color)),
                            Span::styled(short_value(value), Style::default().fg(theme::OK)),
                        ])
                    })
                    .collect()
            }
        }
        Panel::Algorithms => {
            let names = env.algorithm_names();
            if names.is_empty() {
                empty("алгоритмы не определены")
            } else {
                names
                    .iter()
                    .map(|n| {
                        Line::from(Span::styled(n.clone(), Style::default().fg(theme::BUILTIN)))
                    })
                    .collect()
            }
        }
        Panel::Hidden => Vec::new(),
    };

    frame.render_widget(Paragraph::new(lines), inner);
}

/// Короткая запись значения для панели: длинное обрезается.
fn short_value(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 18 {
        let head: String = text.chars().take(17).collect();
        format!("= {head}…")
    } else {
        format!("= {text}")
    }
}

fn draw_input(frame: &mut Frame, area: Rect, app: &ReplApp) {
    let continued = app.depth > 0;
    let prompt = if continued { "  ...  " } else { "кумир> " };
    let color = if continued { theme::WARN } else { theme::OK };

    let block = Block::default()
        .title(Span::styled(" Ввод ", Style::default().fg(color)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(color));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut spans = vec![Span::styled(prompt, Style::default().fg(theme::MUTED))];
    spans.extend(syntax::highlight(&app.input.text()));
    frame.render_widget(Paragraph::new(Line::from(spans)), inner);

    frame.set_cursor_position((
        inner.x + prompt.chars().count() as u16 + app.input.cursor() as u16,
        inner.y,
    ));
}

fn draw_keys(frame: &mut Frame, area: Rect, app: &ReplApp) {
    let panel_label = match app.panel {
        Panel::State => "алгоритмы",
        Panel::Algorithms => "скрыть",
        Panel::Hidden => "состояние",
    };
    let keys: [(&str, &str); 6] = [
        ("F1", "помощь"),
        ("F2", panel_label),
        ("Tab", "дополнить"),
        ("^L", "очистить"),
        ("^C", "прервать"),
        ("^D", "выход"),
    ];

    let mut spans = Vec::new();
    for (key, label) in keys {
        spans.push(Span::styled(format!(" {key} "), theme::key_hint()));
        spans.push(Span::styled(format!("{label}  "), theme::key_label()));
    }
    frame.render_widget(Line::from(spans), area);
}

fn draw_help(frame: &mut Frame) {
    let area = centered(66, 22, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(Span::styled(" Помощь ", theme::title(true)))
        .borders(Borders::ALL)
        .border_style(theme::border(true));

    let section = |t: &str| Line::from(Span::styled(t.to_string(), theme::title(true)));
    let row = |k: &str, v: &str| {
        Line::from(vec![
            Span::styled(format!("  {k:<15}"), theme::key_hint()),
            Span::raw(v.to_string()),
        ])
    };

    let text = vec![
        section(" Клавиши"),
        row("F1", "эта справка (закрыть — любая клавиша)"),
        row("F2", "переключить боковую панель"),
        row("Tab", "дополнить имя; ещё раз — следующий вариант"),
        row("↑ ↓", "история ввода"),
        row("Ctrl+← →", "перемещение по словам"),
        row("Ctrl+W U K", "удалить слово / до начала / до конца"),
        row("PgUp PgDn", "прокрутка вывода"),
        row("Ctrl+C", "прервать незакрытую конструкцию"),
        row("Ctrl+D", "выход"),
        Line::raw(""),
        section(" Команды"),
        row(".переменные", "показать состояние"),
        row(".алгоритмы", "показать алгоритмы"),
        row(".загрузить ф", "выполнить файл .kum"),
        row(".отладка", "включить или выключить отладку"),
        row(".очистить", "очистить вывод"),
        row(".сброс", "забыть всё определённое"),
        row(".выход", "выход"),
    ];

    frame.render_widget(
        Paragraph::new(text).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

/// Прямоугольник заданного размера по центру экрана.
fn centered(width: u16, height: u16, area: Rect) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schet_konstrukcij_ne_lovit_slova_vnutri_imen() {
        assert_eq!(opens_in("нач"), 1);
        assert_eq!(opens_in("нц для i от 1 до 3"), 1);
        assert_eq!(closes_in("кон"), 1);
        // «начало» и «конец» — обычные имена, а не ключевые слова.
        assert_eq!(opens_in("начало := 1"), 0);
        assert_eq!(closes_in("конец := 1"), 0);
    }

    #[test]
    fn kommentarij_ne_zakryvaet_konstrukciyu() {
        assert_eq!(closes_in("x := 1 | кон"), 0);
        assert_eq!(opens_in("x := 1 | нач"), 0);
    }

    #[test]
    fn mnogostrochnaya_konstrukciya_kopitsya_do_zakrytiya() {
        let mut app = ReplApp::new(false);
        for line in ["алг главный", "нач", "  вывод 7"] {
            app.input.set(line);
            app.submit();
        }
        assert_eq!(app.depth, 1, "открыт `нач` — ждём `кон`");
        assert!(!app.pending.is_empty());

        app.input.set("кон");
        app.submit();
        assert_eq!(app.depth, 0);
        assert!(app.pending.is_empty(), "накопленное ушло на выполнение");
    }

    /// Переменные должны переживать нажатие Enter.
    ///
    /// Обычный запуск оборачивает свободные инструкции в алгоритм и снимает
    /// его кадр, поэтому набранное строкой выше следующей строке было не
    /// видно, а панель состояния оставалась пустой.
    #[test]
    fn peremennye_zhivut_mezhdu_strokami() {
        let mut app = ReplApp::new(false);
        app.input.set("цел счётчик := 5");
        app.submit();

        let state = app.interpreter.environment().globals_snapshot();
        assert!(
            state.iter().any(|(name, _, _)| name == "счётчик"),
            "переменная должна попасть в состояние, а получили {state:?}"
        );

        app.input.set("вывод счётчик * 2");
        app.submit();
        assert!(
            app.output
                .iter()
                .any(|l| matches!(l, OutputLine::Normal(s) if s.trim() == "10")),
            "следующая строка должна видеть переменную"
        );
    }

    /// Заголовок алгоритма ждёт своего `нач`, а не уходит на выполнение сразу.
    #[test]
    fn zagolovok_algoritma_zhdet_tela() {
        let mut app = ReplApp::new(false);
        app.input.set("алг цел удвоить(цел x)");
        app.submit();
        assert_eq!(app.depth, 1, "после заголовка ждём `нач`");
        assert!(
            !app.output.iter().any(|l| matches!(l, OutputLine::Error(_))),
            "заголовок сам по себе ошибкой не является"
        );

        for line in ["нач", "  знач := x * 2", "кон"] {
            app.input.set(line);
            app.submit();
        }
        assert_eq!(app.depth, 0);

        let algorithms = app.interpreter.environment().algorithm_names();
        assert!(
            algorithms.iter().any(|n| n == "удвоить"),
            "алгоритм должен определиться, а получили {algorithms:?}"
        );

        app.input.set("вывод удвоить(21)");
        app.submit();
        assert!(
            app.output
                .iter()
                .any(|l| matches!(l, OutputLine::Normal(s) if s.trim() == "42")),
            "определённый алгоритм должен вызываться"
        );
    }

    /// `нач` без заголовка открывает конструкцию сам.
    #[test]
    fn goloe_nach_otkryvaet_konstrukciyu() {
        let mut app = ReplApp::new(false);
        app.input.set("нач");
        app.submit();
        assert_eq!(app.depth, 1);
        app.input.set("кон");
        app.submit();
        assert_eq!(app.depth, 0);
    }

    #[test]
    fn prervannyj_vvod_ne_ostavlyaet_hvosta() {
        let mut app = ReplApp::new(false);
        app.input.set("нач");
        app.submit();
        assert_eq!(app.depth, 1);

        app.handle_key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(app.depth, 0);
        assert!(app.pending.is_empty(), "накопленный текст должен исчезнуть");
    }

    /// Набор кириллицы в интерактивном режиме раньше ронял интерпретатор.
    #[test]
    fn nabor_kirillicy_dohodit_do_stroki_vvoda() {
        let mut app = ReplApp::new(false);
        for ch in "вывод 2 + 3".chars() {
            app.handle_key(KeyCode::Char(ch), KeyModifiers::NONE);
        }
        assert_eq!(app.input.text(), "вывод 2 + 3");
    }

    #[test]
    fn oshibka_pokazyvaetsya_no_ne_ubivaet_sessiyu() {
        let mut app = ReplApp::new(false);
        app.input.set("вывод неизвестная_переменная_ххх");
        app.submit();
        assert!(
            app.output.iter().any(|l| matches!(l, OutputLine::Error(_))),
            "ошибка должна быть видна"
        );
        assert!(!app.should_quit, "сессия продолжается");
    }

    #[test]
    fn neizvestnaya_komanda_ne_ischezaet_molcha() {
        let mut app = ReplApp::new(false);
        app.input.set(".такой_команды_нет");
        app.submit();
        assert!(app.output.iter().any(|l| matches!(l, OutputLine::Error(_))));
    }

    #[test]
    fn panel_perebiraetsya_po_krugu() {
        let mut app = ReplApp::new(false);
        assert_eq!(app.panel, Panel::State);
        app.handle_key(KeyCode::F(2), KeyModifiers::NONE);
        assert_eq!(app.panel, Panel::Algorithms);
        app.handle_key(KeyCode::F(2), KeyModifiers::NONE);
        assert_eq!(app.panel, Panel::Hidden);
        app.handle_key(KeyCode::F(2), KeyModifiers::NONE);
        assert_eq!(app.panel, Panel::State);
    }

    #[test]
    fn spravka_zakryvaetsya_lyuboj_klavishej() {
        let mut app = ReplApp::new(false);
        app.handle_key(KeyCode::F(1), KeyModifiers::NONE);
        assert!(app.show_help);
        app.handle_key(KeyCode::Char('я'), KeyModifiers::NONE);
        assert!(!app.show_help);
        assert!(app.input.is_empty(), "клавиша закрытия не печатается");
    }

    #[test]
    fn istoriya_hodit_v_obe_storony() {
        let mut app = ReplApp::new(false);
        for line in ["вывод 1", "вывод 2"] {
            app.input.set(line);
            app.submit();
        }
        app.handle_key(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.input.text(), "вывод 2");
        app.handle_key(KeyCode::Up, KeyModifiers::NONE);
        assert_eq!(app.input.text(), "вывод 1");
        app.handle_key(KeyCode::Down, KeyModifiers::NONE);
        assert_eq!(app.input.text(), "вывод 2");
    }
}
