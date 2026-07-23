//! Интерактивный режим: состояние сессии и обработка клавиш.
//!
//! Как это выглядит на экране — в [`super::draw`]. Разделение не косметическое:
//! поведение консоли (что считается незакрытой конструкцией, что делает Tab,
//! переживают ли переменные Enter) проверяется тестами, а отрисовка требует
//! терминала и тестами не покрывается. Держать их врозь — значит иметь
//! возможность проверить первое, не запуская второе.

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use shared::types::Value;
use std::time::Duration;

use crate::editor::InputLine;
use crate::interpreter::Interpreter;
use crate::terminal::{init_terminal, restore_terminal};
use crate::ui::OutputLine;

/// Видна ли боковая колонка.
///
/// Переменные и алгоритмы показываются в ней одновременно, поэтому выбирать
/// между ними не нужно — остаётся только «показать или убрать».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Panel {
    Shown,
    /// Панель убрана — весь экран под вывод.
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

        // Вывод остаётся пустым: пока в нём ничего нет, отрисовка показывает
        // приветствие с примерами. Заполнять его подсказками — значит сразу
        // засорять то, ради чего консоль и открыли.
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

        // Знак на левом поле ставит отрисовка — здесь только сам текст.
        self.output.push(OutputLine::Input(line.clone()));

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

    // Терминал возвращается в исходное состояние даже при ошибке отрисовки:
    // иначе управление вернётся в консоль без эха и без курсора.
    restore_terminal()?;
    result
}

// ===========================================================================
//                      ДОСТУП ДЛЯ ОТРИСОВКИ
// ===========================================================================
// Отрисовка живёт в отдельном модуле и состояние только читает, поэтому поля
// остаются закрытыми, а наружу выставлены именно те величины, которые рисуются.

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

    /// Объявление алгоритма — это объявление, а не запуск.
    ///
    /// Обычный запуск, не найдя точки входа, зовёт первый попавшийся
    /// алгоритм: объявив `алг цел удвоить(цел x)`, человек тут же получал
    /// «ожидался 1 аргумент, получено 0» — и получал снова после очистки
    /// экрана, потому что очистка состояние не трогает.
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
            "объявление не должно давать ошибок, а получили {}",
            errors.len()
        );
    }

    /// Очистка убирает вывод, но не объявленное.
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
            "очистка экрана не должна забывать переменные"
        );

        app.input.set(".сброс");
        app.submit();
        assert!(
            app.interpreter.environment().globals_snapshot().is_empty(),
            "сброс должен забыть всё"
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
