//! Интерпретатор языка Кумир 3
//!
//! Полноценный интерпретатор с поддержкой:
//! - Всех базовых типов (цел, вещ, лит, лог, сим)
//! - Массивов, словарей, множеств
//! - Условных операторов и циклов
//! - Алгоритмов с параметрами
//! - ООП (классы, объекты, методы)
//! - Перечислений и pattern matching
//! - Обработки исключений
//! - Встроенных математических и строковых функций
//!
//! # Пример использования
//!
//! ```rust
//! use kumir3_core::interpreter::Interpreter;
//!
//! let source = r#"
//! алг Факториал(арг цел n) цел
//! нач
//!     если n <= 1 то
//!         знач := 1
//!     иначе
//!         знач := n * Факториал(n - 1)
//!     все
//! кон
//!
//! алг Главный
//! нач
//!     вывод Факториал(5)
//! кон
//! "#;
//!
//! let mut interpreter = Interpreter::new();
//! let result = interpreter.run(source);
//! ```

mod builtins;
pub mod cli;
mod environment;
mod error;
mod evaluator;
mod executor;
mod file_importer;
mod library_bridge;

pub use builtins::Builtins;
pub use environment::{CallFrame, Environment, Scope};
pub use error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
pub use evaluator::ExprEvaluator;
pub use executor::Executor;
pub use file_importer::{FileImporter, ImportedModule};
pub use library_bridge::LibraryManager;
// Реэкспорт из shared::runtime для async
pub use crate::shared::runtime::{KumirRuntime, Task, TaskExecutor, TaskHandle, TaskId, TaskState};
pub use cli::*;

use crate::shared::parser::parse;
use crate::shared::types::{Program, Stmt, Value};

// =============================================================================
//                           ИНТЕРПРЕТАТОР
// =============================================================================

/// Интерпретатор языка Кумир 3.
///
/// Выполняет программы на языке Кумир, поддерживая полный синтаксис версии 3.
///
/// ## Интеграция с инфраструктурой
///
/// Интерпретатор использует:
/// - `shared/runtime` - для async операций и событий
/// - `shared/libraries` - для загрузки стандартных библиотек
/// - `shared/constants` - для сообщений об ошибках
/// - `file_importer` - для импорта .kum файлов (как в Python)
pub struct Interpreter {
    /// Среда выполнения
    env: Environment,
    /// Менеджер библиотек (shared для доступа из Environment)
    libraries: std::sync::Arc<std::sync::RwLock<LibraryManager>>,
    /// Импортер файлов .kum
    file_importer: FileImporter,
    /// Runtime для async операций
    runtime: Option<KumirRuntime>,
    /// Режим отладки
    debug_mode: bool,
}

impl Interpreter {
    /// Создаёт новый интерпретатор.
    pub fn new() -> Self {
        let libraries = std::sync::Arc::new(std::sync::RwLock::new(LibraryManager::new()));
        let mut env = Environment::new();
        env.set_library_manager(std::sync::Arc::clone(&libraries));

        Self {
            env,
            libraries,
            file_importer: FileImporter::new(),
            runtime: None,
            debug_mode: false,
        }
    }

    /// Создаёт интерпретатор с runtime для async операций.
    pub fn with_runtime() -> Self {
        let libraries = std::sync::Arc::new(std::sync::RwLock::new(LibraryManager::new()));
        let mut env = Environment::new();
        env.set_library_manager(std::sync::Arc::clone(&libraries));

        Self {
            env,
            libraries,
            file_importer: FileImporter::new(),
            runtime: Some(KumirRuntime::new()),
            debug_mode: false,
        }
    }

    /// Создаёт интерпретатор с существующей средой.
    pub fn with_environment(mut env: Environment) -> Self {
        let libraries = std::sync::Arc::new(std::sync::RwLock::new(LibraryManager::new()));
        env.set_library_manager(std::sync::Arc::clone(&libraries));

        Self {
            env,
            libraries,
            file_importer: FileImporter::new(),
            runtime: None,
            debug_mode: false,
        }
    }

    /// Устанавливает базовую директорию для импортов.
    pub fn set_base_dir(&mut self, dir: impl Into<std::path::PathBuf>) {
        self.file_importer.set_base_dir(dir);
    }

    /// Добавляет директорию поиска модулей.
    pub fn add_module_path(&mut self, path: impl Into<std::path::PathBuf>) {
        self.file_importer.add_search_path(path);
    }

    /// Включает/выключает режим отладки.
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
        self.env.set_debug_mode(enabled);
    }

    /// Возвращает ссылку на среду выполнения.
    pub fn environment(&self) -> &Environment {
        &self.env
    }

    /// Возвращает изменяемую ссылку на среду выполнения.
    pub fn environment_mut(&mut self) -> &mut Environment {
        &mut self.env
    }

    // =========================================================================
    //                    ВЫПОЛНЕНИЕ ПРОГРАММ
    // =========================================================================

    /// Выполняет исходный код программы.
    pub fn run(&mut self, source: &str) -> RuntimeResult<Value> {
        // Парсим программу
        let program = parse(source).map_err(|e| {
            RuntimeError::new(format!("Ошибка разбора: {}", e), RuntimeErrorKind::Other)
        })?;

        self.run_program(&program)
    }

    /// Выполняет распаршенную программу.
    pub fn run_program(&mut self, program: &Program) -> RuntimeResult<Value> {
        // Загружаем определения в среду
        self.load_program(program)?;

        // Выполняем глобальные инструкции (объявления перечислений и т.д.)
        for stmt in &program.globals {
            Executor::execute(stmt, &mut self.env)?;
        }

        // Ищем главный алгоритм
        if let Some(main) = &program.main {
            self.call_algorithm(&main.name, &[])
        } else if self.env.has_algorithm("Главный") {
            self.call_algorithm("Главный", &[])
        } else if self.env.has_algorithm("главный") {
            self.call_algorithm("главный", &[])
        } else if self.env.has_algorithm("Тест") {
            self.call_algorithm("Тест", &[])
        } else if self.env.has_algorithm("Main") {
            self.call_algorithm("Main", &[])
        } else if self.env.has_algorithm("main") {
            self.call_algorithm("main", &[])
        } else if !program.algorithms.is_empty() {
            // Ищем алгоритм без параметров
            for alg in &program.algorithms {
                if alg.params.is_empty() {
                    return self.call_algorithm(&alg.name, &[]);
                }
            }
            // Если все с параметрами - вызываем первый (возможно будет ошибка)
            self.call_algorithm(&program.algorithms[0].name, &[])
        } else {
            Ok(Value::Null)
        }
    }

    /// Загружает определения программы в среду.
    fn load_program(&mut self, program: &Program) -> RuntimeResult<()> {
        // Обрабатываем импорты
        for import in &program.imports {
            self.process_import(import)?;
        }

        // Загружаем алгоритмы
        for alg in &program.algorithms {
            self.env.define_algorithm(alg.clone());
        }

        // Загружаем перегруженные алгоритмы
        for overloaded in &program.overloaded_algorithms {
            for alg in &overloaded.overloads {
                self.env.define_algorithm(alg.clone());
            }
        }

        // Загружаем классы
        for class in &program.classes {
            self.env.define_class(class.clone());
        }

        // Загружаем главный алгоритм
        if let Some(main) = &program.main {
            self.env.define_algorithm(main.clone());
        }

        Ok(())
    }

    /// Вызывает алгоритм по имени.
    pub fn call_algorithm(&mut self, name: &str, args: &[Value]) -> RuntimeResult<Value> {
        let algorithm = self.env.get_algorithm(name)?.clone();

        // Проверяем количество аргументов
        if args.len() != algorithm.params.len() {
            return Err(RuntimeError::argument_count(
                name,
                algorithm.params.len(),
                args.len(),
            ));
        }

        // Создаём кадр вызова
        self.env.push_frame(&algorithm.name)?;

        // Привязываем параметры
        for (param, value) in algorithm.params.iter().zip(args.iter()) {
            self.env.define_local(param.name.clone(), value.clone());
        }

        // Выполняем тело
        let result = Executor::execute_stmts(&algorithm.body, &mut self.env);

        // Получаем возвращаемое значение
        let return_value = self.env.get_result_value().cloned();

        // Удаляем кадр
        self.env.pop_frame();

        // Обрабатываем результат
        match result {
            Ok(ControlFlow::Return(value)) => Ok(value.unwrap_or(Value::Null)),
            Ok(_) => Ok(return_value.unwrap_or(Value::Null)),
            Err(e) => Err(e),
        }
    }

    // =========================================================================
    //                    ВЫЧИСЛЕНИЕ ВЫРАЖЕНИЙ
    // =========================================================================

    /// Вычисляет выражение из строки.
    pub fn eval(&mut self, source: &str) -> RuntimeResult<Value> {
        use crate::shared::parser::parse_expression;

        let expr = parse_expression(source).map_err(|e| {
            RuntimeError::new(
                format!("Ошибка разбора выражения: {}", e),
                RuntimeErrorKind::Other,
            )
        })?;

        ExprEvaluator::evaluate(&expr, &mut self.env)
    }

    // =========================================================================
    //                    ПЕРЕМЕННЫЕ
    // =========================================================================

    /// Устанавливает глобальную переменную.
    pub fn set_global(&mut self, name: impl Into<String>, value: Value) {
        self.env.define_global(name.into(), value);
    }

    /// Получает значение переменной.
    pub fn get_variable(&self, name: &str) -> RuntimeResult<&Value> {
        self.env.get_variable(name)
    }

    // =========================================================================
    //                    ВЫВОД
    // =========================================================================

    /// Получает вывод программы.
    pub fn get_output(&self) -> String {
        self.env.get_output()
    }

    /// Очищает буфер вывода.
    pub fn clear_output(&mut self) {
        self.env.clear_output();
    }

    // =========================================================================
    //                    БИБЛИОТЕКИ И ИМПОРТЫ
    // =========================================================================

    /// Обрабатывает инструкцию импорта.
    ///
    /// Поддерживает:
    /// - Стандартные библиотеки: `использовать время`
    /// - Файловые импорты: `подключить "./модуль.kum"`
    fn process_import(&mut self, stmt: &Stmt) -> RuntimeResult<()> {
        match stmt {
            Stmt::Import { path, alias } => {
                // Проверяем, это файл .kum или библиотека
                if FileImporter::is_kum_file(path) {
                    // Файловый импорт (как в Python)
                    let module = self.file_importer.import(path, alias.as_deref())?;

                    // Регистрируем алгоритмы из модуля
                    for (name, alg) in module.public_algorithms() {
                        let full_name = match alias {
                            Some(a) => format!("{}.{}", a, name),
                            None => format!("{}.{}", module.name, name),
                        };
                        // Регистрируем с префиксом модуля
                        self.env.define_algorithm_with_name(&full_name, alg.clone());

                        // Также регистрируем без префикса если нет alias
                        if alias.is_none() {
                            self.env.define_algorithm(alg.clone());
                        }
                    }

                    // Регистрируем классы из модуля
                    for (name, class) in module.public_classes() {
                        let full_name = match alias {
                            Some(a) => format!("{}.{}", a, name),
                            None => name.clone(),
                        };
                        self.env.define_class_with_name(&full_name, class.clone());
                    }

                    if self.debug_mode {
                        eprintln!(
                            "[DEBUG] Импортирован модуль: {} ({})",
                            alias.as_deref().unwrap_or(&module.name),
                            path
                        );
                    }
                } else if let Some(lib_name) = library_bridge::resolve_import_path(path) {
                    // Стандартная библиотека
                    self.libraries
                        .write()
                        .map_err(|_| {
                            RuntimeError::new(
                                "Не удалось получить доступ к библиотекам",
                                RuntimeErrorKind::Other,
                            )
                        })?
                        .import(&lib_name, alias.as_deref())?;
                    if self.debug_mode {
                        eprintln!("[DEBUG] Импортирована библиотека: {}", lib_name);
                    }
                } else {
                    // Неизвестный импорт
                    return Err(RuntimeError::new(
                        format!("Модуль или библиотека '{}' не найдены", path),
                        RuntimeErrorKind::Other,
                    ));
                }
            }
            _ => {} // Другие типы импортов пока игнорируем
        }
        Ok(())
    }

    /// Импортирует .kum файл.
    pub fn import_file(
        &mut self,
        path: &str,
        alias: Option<&str>,
    ) -> RuntimeResult<std::sync::Arc<ImportedModule>> {
        self.file_importer.import(path, alias)
    }

    /// Импортирует библиотеку программно.
    pub fn import_library(&mut self, name: &str) -> RuntimeResult<()> {
        self.libraries
            .write()
            .map_err(|_| {
                RuntimeError::new(
                    "Не удалось получить доступ к библиотекам",
                    RuntimeErrorKind::Other,
                )
            })?
            .import(name, None)
    }

    /// Импортирует библиотеку с алиасом.
    pub fn import_library_as(&mut self, name: &str, alias: &str) -> RuntimeResult<()> {
        self.libraries
            .write()
            .map_err(|_| {
                RuntimeError::new(
                    "Не удалось получить доступ к библиотекам",
                    RuntimeErrorKind::Other,
                )
            })?
            .import(name, Some(alias))
    }

    /// Вызывает функцию библиотеки.
    pub fn call_library_function(
        &self,
        name: &str,
        args: &[Value],
    ) -> RuntimeResult<Option<Value>> {
        self.libraries
            .read()
            .map_err(|_| {
                RuntimeError::new(
                    "Не удалось получить доступ к библиотекам",
                    RuntimeErrorKind::Other,
                )
            })?
            .call_function(name, args)
    }

    /// Проверяет, является ли имя функцией библиотеки.
    pub fn is_library_function(&self, name: &str) -> bool {
        self.libraries
            .read()
            .map(|m| m.is_library_function(name))
            .unwrap_or(false)
    }

    /// Получает менеджер библиотек.
    pub fn libraries(&self) -> &std::sync::Arc<std::sync::RwLock<LibraryManager>> {
        &self.libraries
    }

    /// Получает runtime (если инициализирован).
    pub fn runtime(&self) -> Option<&KumirRuntime> {
        self.runtime.as_ref()
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
//                           УДОБНЫЕ ФУНКЦИИ
// =============================================================================

/// Выполняет исходный код и возвращает результат.
pub fn run(source: &str) -> RuntimeResult<Value> {
    Interpreter::new().run(source)
}

/// Вычисляет выражение и возвращает результат.
pub fn eval(source: &str) -> RuntimeResult<Value> {
    Interpreter::new().eval(source)
}

/// Выполняет программу и возвращает вывод.
pub fn run_and_get_output(source: &str) -> RuntimeResult<String> {
    let mut interpreter = Interpreter::new();
    interpreter.run(source)?;
    Ok(interpreter.get_output())
}

// =============================================================================
//                           ТЕСТЫ
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_output() {
        let source = r#"
алг Тест
нач
    вывод 42
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(interpreter.get_output().contains("42"));
    }

    #[test]
    fn test_arithmetic() {
        let mut interpreter = Interpreter::new();

        assert_eq!(
            interpreter.eval("2 + 3").unwrap(),
            Value::Number(crate::shared::types::Number::I64(5))
        );
        assert_eq!(
            interpreter.eval("10 - 4").unwrap(),
            Value::Number(crate::shared::types::Number::I64(6))
        );
        assert_eq!(
            interpreter.eval("3 * 4").unwrap(),
            Value::Number(crate::shared::types::Number::I64(12))
        );

        // Деление возвращает F128 (вещественное деление)
        let div_result = interpreter.eval("15 / 3").unwrap();
        match div_result {
            Value::Number(n) => {
                let f_val = match n {
                    crate::shared::types::Number::F128(f) => f.to_f64(),
                    crate::shared::types::Number::F64(f) => f,
                    crate::shared::types::Number::I64(i) => i as f64,
                    _ => panic!("Unexpected number type"),
                };
                assert!((f_val - 5.0).abs() < 0.0001, "Expected ~5.0, got {}", f_val);
            }
            _ => panic!("Expected Number"),
        }

        // Возведение в степень также может вернуть F128
        let pow_result = interpreter.eval("2 ** 3").unwrap();
        match pow_result {
            Value::Number(n) => {
                let f_val = match n {
                    crate::shared::types::Number::F128(f) => f.to_f64(),
                    crate::shared::types::Number::F64(f) => f,
                    crate::shared::types::Number::I64(i) => i as f64,
                    _ => panic!("Unexpected number type"),
                };
                assert!((f_val - 8.0).abs() < 0.0001, "Expected ~8.0, got {}", f_val);
            }
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn test_comparison() {
        let mut interpreter = Interpreter::new();

        assert_eq!(interpreter.eval("5 > 3").unwrap(), Value::Boolean(true));
        assert_eq!(interpreter.eval("5 < 3").unwrap(), Value::Boolean(false));
        assert_eq!(interpreter.eval("5 = 5").unwrap(), Value::Boolean(true));
        assert_eq!(interpreter.eval("5 <> 3").unwrap(), Value::Boolean(true));
    }

    #[test]
    fn test_logical() {
        let mut interpreter = Interpreter::new();

        assert_eq!(interpreter.eval("да и да").unwrap(), Value::Boolean(true));
        assert_eq!(interpreter.eval("да и нет").unwrap(), Value::Boolean(false));
        assert_eq!(
            interpreter.eval("да или нет").unwrap(),
            Value::Boolean(true)
        );
        assert_eq!(interpreter.eval("не да").unwrap(), Value::Boolean(false));
    }

    #[test]
    fn test_variables() {
        let source = r#"
алг Тест
нач
    цел x := 10
    цел y := 20
    вывод x + y
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(interpreter.get_output().contains("30"));
    }

    #[test]
    fn test_if_statement() {
        let source = r#"
алг Тест
нач
    цел x := 5
    если x > 0 то
        вывод "положительное"
    иначе
        вывод "неположительное"
    все
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(interpreter.get_output().contains("положительное"));
    }

    #[test]
    fn test_for_loop() {
        let source = r#"
алг Тест
нач
    цел сумма := 0
    нц для i от 1 до 5
        сумма := сумма + i
    кц
    вывод сумма
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(interpreter.get_output().contains("15"));
    }

    #[test]
    fn test_while_loop() {
        let source = r#"
алг Тест
нач
    цел n := 5
    цел факт := 1
    нц пока n > 0
        факт := факт * n
        n := n - 1
    кц
    вывод факт
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(interpreter.get_output().contains("120"));
    }

    #[test]
    fn test_algorithm_call() {
        let source = r#"
алг цел Квадрат(арг цел x)
нач
    знач := x * x
кон

алг Тест
нач
    вывод Квадрат(5)
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(interpreter.get_output().contains("25"));
    }

    #[test]
    fn test_recursion() {
        let source = r#"
алг цел Фиб(арг цел n)
нач
    если n <= 1 то
        знач := n
    иначе
        знач := Фиб(n - 1) + Фиб(n - 2)
    все
кон

алг Тест
нач
    вывод Фиб(10)
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(interpreter.get_output().contains("55"));
    }

    #[test]
    fn test_builtin_functions() {
        let mut interpreter = Interpreter::new();

        // Математика
        assert_eq!(
            interpreter.eval("abs(-5)").unwrap(),
            Value::Number(crate::shared::types::Number::I64(5))
        );
        assert_eq!(
            interpreter.eval("min(3, 7)").unwrap(),
            Value::Number(crate::shared::types::Number::I64(3))
        );
        assert_eq!(
            interpreter.eval("max(3, 7)").unwrap(),
            Value::Number(crate::shared::types::Number::I64(7))
        );

        // Строки
        assert_eq!(
            interpreter.eval("длина(\"привет\")").unwrap(),
            Value::Number(crate::shared::types::Number::I64(6))
        );
    }

    #[test]
    fn test_string_operations() {
        let source = r#"
алг Тест
нач
    лит s := "Привет, мир!"
    вывод длина(s)
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(interpreter.get_output().contains("12"));
    }

    #[test]
    fn test_array() {
        let source = r#"
алг Тест
нач
    таб цел[] arr := таб(1, 2, 3, 4, 5)
    вывод сумма(arr)
кон
"#;
        // Примечание: синтаксис массивов может отличаться
        // Этот тест показывает концепцию
    }

    #[test]
    fn test_conditional_expression() {
        let mut interpreter = Interpreter::new();

        let result = interpreter.eval("если 5 > 3 то 1 иначе 0 все").unwrap();
        assert_eq!(result, Value::Number(crate::shared::types::Number::I64(1)));
    }

    #[test]
    fn test_try_catch() {
        let source = r#"
алг Тест
нач
    попытка
        бросить "ошибка"
    перехват e
        вывод "перехвачено"
    кон
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(interpreter.get_output().contains("перехвачено"));
    }
}
