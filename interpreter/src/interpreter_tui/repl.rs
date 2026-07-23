//! Interactive REPL: session state and key handling.
//!
//! Display logic is in [`super::draw`]. The separation is not cosmetic: console
//! behavior (what counts as unclosed construct, what Tab does, whether variables
//! survive Enter) is tested here, while rendering requires a terminal and
//! cannot be tested. Keeping them separate lets us verify behavior without running
//! the display.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use shared::types::Value;
use std::time::Duration;

use crate::editor::InputLine;
use crate::interpreter::Interpreter;
use crate::terminal::{init_terminal, restore_terminal};
use crate::ui::OutputLine;

/// Whether the side panel is visible.
///
/// Variables and algorithms are shown together in it, so there is no need to
/// choose between them — we just show or hide the panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    Shown,
    /// Panel hidden — whole screen is for output.
    Hidden,
}

impl Panel {
    fn toggled(self) -> Self {
        match self {
            Self::Shown => Self::Hidden,
            Self::Hidden => Self::Shown,
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
    /// How many lines of output scrolled up from the end. 0 = at the end.
    scroll_back: usize,
    panel: Panel,
    show_help: bool,
    debug_mode: bool,
    should_quit: bool,
    /// Accumulated text of unclosed constructs (`алг`, `нц`, `если`).
    pending: String,
    /// How many constructs are still open.
    depth: usize,
    /// How many `алг` headers await their `нач`.
    awaiting_body: usize,
    /// Completion options for current word; empty if cycling is not active.
    completions: Vec<String>,
    completion_idx: usize,
}

impl ReplApp {
    fn new(debug: bool) -> Self {
        let mut interpreter = Interpreter::new();
        interpreter.set_debug_mode(debug);

        // Output starts empty: while it is empty, the display shows welcome
        // text with examples. Filling it with hints would clutter what the user
        // opened the console to do.
        Self {
            interpreter,
            input: InputLine::default(),
            output: Vec::new(),
            history: Vec::new(),
            history_idx: -1,
            saved_input: String::new(),
            scroll_back: 0,
            panel: Panel::Shown,
            show_help: false,
            debug_mode: debug,
            should_quit: false,
            pending: String::new(),
            depth: 0,
            awaiting_body: 0,
            completions: Vec::new(),
            completion_idx: 0,
        }
    }

    // ---------------------------------------------------------------------------
    //                            EXECUTION
    // ---------------------------------------------------------------------------

    /// Handles Enter key press.
    fn submit(&mut self) {
        let line = self.input.take();
        self.completions.clear();
        self.scroll_back = 0;

        if self.depth == 0 && line.trim().is_empty() {
            return;
        }

        self.history.push(line.clone());
        self.history_idx = -1;

        // Gutter symbol is drawn by the display — here we just add text.
        self.output.push(OutputLine::Input(line.clone()));

        if self.depth == 0 && line.trim_start().starts_with([',', '.', ':']) {
            self.run_command(line.trim());
            return;
        }

        // Unclosed constructs accumulate until `кон`/`кц`/`все`.
        self.track_depth(&line);
        self.pending.push_str(&line);
        self.pending.push('\n');

        if self.depth > 0 {
            return;
        }

        let code = std::mem::take(&mut self.pending);
        self.run_code(&code);
    }

    /// Recounts how many constructs are still open after this line.
    ///
    /// `алг` and `нач` are counted together, not separately: an algorithm
    /// header and its body are closed by a single `кон`, so `нач` following
    /// `алг` does not increase depth. Without this, `алг цел удвоить(цел x)`
    /// would execute immediately — before the user types `нач` — and the parser
    /// would complain about unexpected end of file.
    fn track_depth(&mut self, line: &str) {
        let algs = count_words(line, &["алг"]);
        let begins = count_words(line, &["нач"]);
        let opens = count_words(line, &["нц", "если", "выбор", "попытка"]);
        let closes = closes_in(line);

        self.depth += algs + opens;

        // `нач` that closes header expectation does not add to depth.
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
        // Interactive execution: text typed on one line should be visible on
        // the next line, so free statements execute in the shared scope, not
        // in a disposable frame.
        let outcome = self.interpreter.run_interactive(code);

        // Output is shown in both success and error cases: on error it indicates
        // how far execution got, making it easier to find the issue.
        let printed = self.interpreter.get_output();
        for line in printed.lines() {
            self.output.push(OutputLine::Normal(line.to_string()));
        }

        match outcome {
            Ok(value) => {
                if !matches!(value, Value::Null) {
                    self.output.push(OutputLine::Success(value.to_string()));
                }
            }
            Err(err) => self.output.push(OutputLine::Error(err.to_string())),
        }
    }

    fn run_command(&mut self, command: &str) {
        let (name, arg) = match command.split_once(char::is_whitespace) {
            Some((n, a)) => (n, a.trim()),
            None => (command, ""),
        };

        let mut say = |line: &str, kind: fn(String) -> OutputLine| {
            self.output.push(kind(line.to_string()));
        };

        match name {
            ".выход" | ".exit" | ".quit" | ".q" => self.should_quit = true,
            ".помощь" | ".help" | ".h" | ".?" => self.show_help = true,
            ".очистить" | ".clear" | ".cls" => {
                self.output.clear();
                self.scroll_back = 0;
            }
            ".переменные" | ".vars" | ".алгоритмы" | ".algs" | ".панель" => {
                self.panel = Panel::Shown
            }
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

            // Easter eggs: the console would be duller without them.
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
                "Укажите файл: .загрузить путь/к/программе.kum".to_string(),
            ));
            return;
        }
        match std::fs::read_to_string(path) {
            Ok(source) => {
                self.output
                    .push(OutputLine::Success(format!("Загружен {path}")));
                self.run_code(&source);
            }
            Err(err) => self
                .output
                .push(OutputLine::Error(format!("{path}: {err}"))),
        }
    }

    // ---------------------------------------------------------------------------
    //                          AUTOCOMPLETION
    // ---------------------------------------------------------------------------

    /// Completes the word under the cursor; pressing Tab again cycles through options.
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

        // Complete everything nameable: language keywords, builtins, and
        // user-defined names from this session.
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
        // No point cycling through a single option — don't store it.
        self.completions = if found.len() > 1 { found } else { Vec::new() };
    }

    // ---------------------------------------------------------------------------
    //                              KEY HANDLING
    // ---------------------------------------------------------------------------

    fn handle_key(&mut self, code: KeyCode, mods: KeyModifiers) {
        // Help closes on any key: it's a reference, not a mode.
        if self.show_help {
            self.show_help = false;
            return;
        }

        let ctrl = mods.contains(KeyModifiers::CONTROL);

        // Any action except Tab stops completion cycling.
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
                        .push(OutputLine::Warning("Ввод отменён".to_string()));
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
            (KeyCode::F(2), _) => self.panel = self.panel.toggled(),
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

/// How many constructs this line opens.
fn opens_in(line: &str) -> usize {
    count_words(line, &["нач", "нц", "если", "выбор", "попытка"])
}

/// How many constructs this line closes.
fn closes_in(line: &str) -> usize {
    count_words(line, &["кон", "кц", "все", "кв"])
}

/// Counts word occurrences as whole words, not substrings.
///
/// Otherwise `конец := 1` would close a construct, and `начало := 1` would open
/// one: both contain keywords inside variable names. Comments are stripped:
/// `| кон` also closes nothing.
fn count_words(line: &str, words: &[&str]) -> usize {
    let code = line.split('|').next().unwrap_or("");
    code.split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|token| words.contains(&token.to_lowercase().as_str()))
        .count()
}

// =============================================================================
//                              DISPLAY
// =============================================================================

pub(crate) fn run_repl_tui(debug: bool) -> Result<(), String> {
    let mut terminal = init_terminal()?;
    let mut app = ReplApp::new(debug);

    let result = (|| -> Result<(), String> {
        loop {
            terminal
                .draw(|frame| crate::draw::draw(frame, &app))
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

    // Restore terminal state even on render error: otherwise control returns
    // to console without echo and without cursor.
    restore_terminal()?;
    result
}

// =============================================================================
//                      DISPLAY ACCESS
// =============================================================================
// The display lives in a separate module and only reads state, so fields are
// private. We expose only the values that the display actually draws.

impl ReplApp {
    pub(crate) fn panel(&self) -> Panel {
        self.panel
    }

    pub(crate) fn depth(&self) -> usize {
        self.depth
    }

    pub(crate) fn debug(&self) -> bool {
        self.debug_mode
    }

    pub(crate) fn help_visible(&self) -> bool {
        self.show_help
    }

    pub(crate) fn scroll_back(&self) -> usize {
        self.scroll_back
    }

    pub(crate) fn output(&self) -> &[OutputLine] {
        &self.output
    }

    pub(crate) fn input_text(&self) -> String {
        self.input.text()
    }

    pub(crate) fn input_cursor(&self) -> usize {
        self.input.cursor()
    }

    pub(crate) fn interpreter_env(&self) -> &crate::interpreter::Environment {
        self.interpreter.environment()
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
        // "начало" and "конец" are ordinary names, not keywords.
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
        assert_eq!(app.depth, 1, "нач opened — waiting for кон");
        assert!(!app.pending.is_empty());

        app.input.set("кон");
        app.submit();
        assert_eq!(app.depth, 0);
        assert!(app.pending.is_empty(), "accumulated text went to execution");
    }

    /// Variables survive pressing Enter.
    ///
    /// Normal execution wraps free statements in an algorithm and pops its frame,
    /// so text typed on one line would not be visible on the next, and the state
    /// panel would remain empty.
    #[test]
    fn peremennye_zhivut_mezhdu_strokami() {
        let mut app = ReplApp::new(false);
        app.input.set("цел счётчик := 5");
        app.submit();

        let state = app.interpreter.environment().globals_snapshot();
        assert!(
            state.iter().any(|(name, _, _)| name == "счётчик"),
            "variable should be in state, got {state:?}"
        );

        app.input.set("вывод счётчик * 2");
        app.submit();
        assert!(
            app.output
                .iter()
                .any(|l| matches!(l, OutputLine::Normal(s) if s.trim() == "10")),
            "next line should see the variable"
        );
    }

    /// Algorithm header waits for its `нач`, does not execute immediately.
    #[test]
    fn zagolovok_algoritma_zhdet_tela() {
        let mut app = ReplApp::new(false);
        app.input.set("алг цел удвоить(цел x)");
        app.submit();
        assert_eq!(app.depth, 1, "after header waiting for нач");
        assert!(
            !app.output.iter().any(|l| matches!(l, OutputLine::Error(_))),
            "header alone is not an error"
        );

        for line in ["нач", "  знач := x * 2", "кон"] {
            app.input.set(line);
            app.submit();
        }
        assert_eq!(app.depth, 0);

        let algorithms = app.interpreter.environment().algorithm_names();
        assert!(
            algorithms.iter().any(|n| n == "удвоить"),
            "algorithm should be defined, got {algorithms:?}"
        );

        app.input.set("вывод удвоить(21)");
        app.submit();
        assert!(
            app.output
                .iter()
                .any(|l| matches!(l, OutputLine::Normal(s) if s.trim() == "42")),
            "defined algorithm should be callable"
        );
    }

    /// Declaring an algorithm is a declaration, not a call.
    ///
    /// Normal execution, finding no entry point, calls the first algorithm:
    /// declaring `алг цел удвоить(цел x)` used to immediately give "expected 1
    /// argument, got 0" — and again after clearing the screen, because clearing
    /// does not reset state.
    #[test]
    fn obyavlenie_algoritma_ne_vyzyvaet_ego() {
        let mut app = ReplApp::new(false);
        for line in ["алг цел удвоить(цел x)", "нач", "  знач := x * 2", "кон"]
        {
            app.input.set(line);
            app.submit();
        }

        let errors: Vec<&OutputLine> = app
            .output
            .iter()
            .filter(|l| matches!(l, OutputLine::Error(_)))
            .collect();
        assert!(
            errors.is_empty(),
            "declaration should not give errors, got {}",
            errors.len()
        );
    }

    /// Clear removes output, but not declared items.
    #[test]
    fn ochistka_ne_trogaet_sostoyanie_a_sbros_trogaet() {
        let mut app = ReplApp::new(false);
        app.input.set("цел счётчик := 5");
        app.submit();

        app.input.set(".очистить");
        app.submit();
        assert!(
            app.interpreter
                .environment()
                .globals_snapshot()
                .iter()
                .any(|(name, _, _)| name == "счётчик"),
            "clearing screen should not forget variables"
        );

        app.input.set(".сброс");
        app.submit();
        assert!(
            app.interpreter.environment().globals_snapshot().is_empty(),
            "reset should forget everything"
        );
    }

    /// `нач` without a header opens a construct by itself.
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
        assert!(app.pending.is_empty(), "accumulated text should disappear");
    }

    /// Typing Cyrillic in interactive mode used to crash the interpreter.
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
            "error should be visible"
        );
        assert!(!app.should_quit, "session continues");
    }

    #[test]
    fn neizvestnaya_komanda_ne_ischezaet_molcha() {
        let mut app = ReplApp::new(false);
        app.input.set(".такой_команды_нет");
        app.submit();
        assert!(app.output.iter().any(|l| matches!(l, OutputLine::Error(_))));
    }

    #[test]
    fn panel_ubiraetsya_i_vozvrashchaetsya() {
        let mut app = ReplApp::new(false);
        assert_eq!(app.panel, Panel::Shown);
        app.handle_key(KeyCode::F(2), KeyModifiers::NONE);
        assert_eq!(app.panel, Panel::Hidden);
        app.handle_key(KeyCode::F(2), KeyModifiers::NONE);
        assert_eq!(app.panel, Panel::Shown);
    }

    #[test]
    fn spravka_zakryvaetsya_lyuboj_klavishej() {
        let mut app = ReplApp::new(false);
        app.handle_key(KeyCode::F(1), KeyModifiers::NONE);
        assert!(app.show_help);
        app.handle_key(KeyCode::Char('я'), KeyModifiers::NONE);
        assert!(!app.show_help);
        assert!(app.input.is_empty(), "closing key should not be printed");
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
