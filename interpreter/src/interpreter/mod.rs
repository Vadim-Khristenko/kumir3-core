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
mod environment;
mod error;
mod evaluator;
mod executor;
mod file_importer;
mod library_bridge;
mod ops;

pub use environment::Environment;
pub use error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
pub use evaluator::ExprEvaluator;
pub use executor::Executor;
pub use file_importer::{FileImporter, ImportedModule};
pub use library_bridge::LibraryManager;

// Реэкспорт из shared::runtime для async
pub use shared::runtime::KumirRuntime;

use shared::parser::parse;
use shared::types::{Program, Stmt, Value};

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
    /// Импортер файлов .kum (shared для доступа из Environment)
    file_importer: std::sync::Arc<std::sync::RwLock<FileImporter>>,
    /// Runtime для async операций
    runtime: Option<KumirRuntime>,
    /// Режим отладки
    debug_mode: bool,
}

impl Interpreter {
    /// Создаёт новый интерпретатор.
    pub fn new() -> Self {
        let libraries = std::sync::Arc::new(std::sync::RwLock::new(LibraryManager::new()));
        let file_importer = std::sync::Arc::new(std::sync::RwLock::new(FileImporter::new()));
        let mut env = Environment::new();
        env.set_library_manager(std::sync::Arc::clone(&libraries));
        env.set_file_importer(std::sync::Arc::clone(&file_importer));

        Self {
            env,
            libraries,
            file_importer,
            runtime: None,
            debug_mode: false,
        }
    }

    /// Создаёт интерпретатор с runtime для async операций.
    pub fn with_runtime() -> Self {
        let libraries = std::sync::Arc::new(std::sync::RwLock::new(LibraryManager::new()));
        let file_importer = std::sync::Arc::new(std::sync::RwLock::new(FileImporter::new()));
        let mut env = Environment::new();
        env.set_library_manager(std::sync::Arc::clone(&libraries));
        env.set_file_importer(std::sync::Arc::clone(&file_importer));

        Self {
            env,
            libraries,
            file_importer,
            runtime: Some(KumirRuntime::new()),
            debug_mode: false,
        }
    }

    /// Создаёт интерпретатор с существующей средой.
    pub fn with_environment(mut env: Environment) -> Self {
        let libraries = std::sync::Arc::new(std::sync::RwLock::new(LibraryManager::new()));
        let file_importer = std::sync::Arc::new(std::sync::RwLock::new(FileImporter::new()));
        env.set_library_manager(std::sync::Arc::clone(&libraries));
        env.set_file_importer(std::sync::Arc::clone(&file_importer));

        Self {
            env,
            libraries,
            file_importer,
            runtime: None,
            debug_mode: false,
        }
    }

    /// Устанавливает базовую директорию для импортов.
    pub fn set_base_dir(&mut self, dir: impl Into<std::path::PathBuf>) {
        if let Ok(mut importer) = self.file_importer.write() {
            importer.set_base_dir(dir);
        }
    }

    /// Добавляет директорию поиска модулей.
    pub fn add_module_path(&mut self, path: impl Into<std::path::PathBuf>) {
        if let Ok(mut importer) = self.file_importer.write() {
            importer.add_search_path(path);
        }
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

        // [KITE 11] Проверки ООП: финал-переопределение, абстрактные методы.
        self.validate_classes()?;

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

    /// [KITE 11] Проверяет ООП-инварианты по всем загруженным классам:
    /// нельзя переопределять `финал`-методы; неабстрактный класс обязан
    /// реализовать все унаследованные абстрактные методы.
    fn validate_classes(&self) -> RuntimeResult<()> {
        use std::collections::{HashMap, HashSet};
        type ClassDef = shared::types::ClassDef;

        // Снимок всех классов (без удержания заимствования env во время проверок).
        let map: HashMap<String, ClassDef> = self
            .env
            .all_classes()
            .map(|(n, c)| (n.clone(), c.clone()))
            .collect();

        // Цепочка предков (имена) от родителя вверх.
        let ancestors = |start: &str| -> Vec<String> {
            let mut chain = Vec::new();
            let mut cur = map
                .get(start)
                .and_then(|c| c.parent.as_ref().map(|p| p.to_string()));
            while let Some(name) = cur {
                if chain.contains(&name) {
                    break;
                } // защита от циклов
                chain.push(name.clone());
                cur = map
                    .get(&name)
                    .and_then(|c| c.parent.as_ref().map(|p| p.to_string()));
            }
            chain
        };

        for class in map.values() {
            let chain = ancestors(&class.name);

            // 1) Запрет переопределения `финал`-методов предков.
            for m in &class.methods {
                for anc in &chain {
                    if let Some(am) = map.get(anc).and_then(|c| {
                        c.methods
                            .iter()
                            .find(|x| x.algorithm.name == m.algorithm.name)
                    }) && am.is_final
                    {
                        return Err(RuntimeError::new(
                            format!(
                                "Метод '{}' помечен 'финал' в классе '{}' и не может быть переопределён в '{}'",
                                m.algorithm.name, anc, class.name
                            ),
                            RuntimeErrorKind::Other,
                        ));
                    }
                }
            }

            // 2) Неабстрактный класс обязан реализовать абстрактные методы иерархии.
            if !class.is_abstract {
                // Имена абстрактных методов в классе и предках.
                let mut abstract_names: HashSet<String> = HashSet::new();
                for scope in std::iter::once(class.name.to_string()).chain(chain.iter().cloned()) {
                    if let Some(c) = map.get(&scope) {
                        for m in &c.methods {
                            if m.is_abstract {
                                abstract_names.insert(m.algorithm.name.to_string());
                            }
                        }
                    }
                }
                // Эффективная реализация (первая сверху вниз) должна быть конкретной.
                for name in &abstract_names {
                    let mut implemented = false;
                    for scope in
                        std::iter::once(class.name.to_string()).chain(chain.iter().cloned())
                    {
                        if let Some(m) = map.get(&scope).and_then(|c| {
                            c.methods
                                .iter()
                                .find(|x| x.algorithm.name.as_ref() == name.as_str())
                        }) {
                            implemented = !m.is_abstract;
                            break; // первый найденный определяет поведение
                        }
                    }
                    if !implemented {
                        return Err(RuntimeError::new(
                            format!(
                                "Класс '{}' должен реализовать абстрактный метод '{}'",
                                class.name, name
                            ),
                            RuntimeErrorKind::Other,
                        ));
                    }
                }
            }
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
        self.env.push_frame(algorithm.name.as_ref())?;

        // Привязываем параметры
        for (param, value) in algorithm.params.iter().zip(args.iter()) {
            self.env.define_local(param.name.to_string(), value.clone());
        }

        // Выполняем тело
        let result =
            Executor::execute_stmts(algorithm.body.as_deref().unwrap_or(&[]), &mut self.env);

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
        use shared::parser::parse_expression;

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
        if let Stmt::Import {
            path,
            alias,
            items: _,
            ..
        } = stmt
        {
            // Проверяем, это файл .kum, директория с библиотекой, или встроенная библиотека
            let path_obj = std::path::Path::new(path);

            if let Some(main_file) = self.resolve_dir_library_main(path) {
                // [KITE 5] Библиотека-проект (директория с kumir.toml):
                // импортируем её главный файл как модуль — алгоритмы становятся вызываемыми.
                let main_str = main_file.to_string_lossy().to_string();
                let module = {
                    let mut importer = self.file_importer.write().map_err(|_| {
                        RuntimeError::new(
                            "Не удалось получить доступ к импортеру",
                            RuntimeErrorKind::Other,
                        )
                    })?;
                    importer.import(&main_str, alias.as_deref())?
                };
                self.register_imported_module(module, alias);
                if self.debug_mode {
                    eprintln!(
                        "[DEBUG] Импортирована библиотека-проект: {} ({})",
                        path, main_str
                    );
                }
            } else if FileImporter::is_kum_file(path) {
                // Файловый импорт (как в Python)
                let module = {
                    let mut importer = self.file_importer.write().map_err(|_| {
                        RuntimeError::new(
                            "Не удалось получить доступ к импортеру",
                            RuntimeErrorKind::Other,
                        )
                    })?;
                    importer.import(path, alias.as_deref())?
                };
                self.register_imported_module(module, alias);
                if self.debug_mode {
                    eprintln!(
                        "[DEBUG] Импортирован модуль: {} ({})",
                        alias.as_deref().unwrap_or("?"),
                        path
                    );
                }
            } else if path_obj.is_dir() || path.contains('/') || path.contains('\\') {
                // Пользовательская библиотека (директория или путь к файлу)
                self.libraries
                    .write()
                    .map_err(|_| {
                        RuntimeError::new(
                            "Не удалось получить доступ к библиотекам",
                            RuntimeErrorKind::Other,
                        )
                    })?
                    .import(path, alias.as_deref())?;
                if self.debug_mode {
                    eprintln!(
                        "[DEBUG] Импортирована пользовательская библиотека: {}",
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
        Ok(())
    }

    /// [KITE 5] Регистрирует публичные алгоритмы и классы импортированного модуля
    /// в среде (с префиксом модуля/алиаса; без префикса — если алиаса нет).
    fn register_imported_module(
        &mut self,
        module: std::sync::Arc<ImportedModule>,
        alias: &Option<String>,
    ) {
        for (name, alg) in module.public_algorithms() {
            let full_name = match alias {
                Some(a) => format!("{}.{}", a, name),
                None => format!("{}.{}", module.name, name),
            };
            self.env.define_algorithm_with_name(&full_name, alg.clone());
            if alias.is_none() {
                self.env.define_algorithm(alg.clone());
            }
        }
        for (name, class) in module.public_classes() {
            let full_name = match alias {
                Some(a) => format!("{}.{}", a, name),
                None => name.clone(),
            };
            self.env.define_class_with_name(&full_name, class.clone());
        }
    }

    /// [KITE 5] Если `path` указывает на директорию-проект с `kumir.toml`,
    /// возвращает путь к её главному `.kum` файлу. Директория ищется как есть
    /// и относительно базовой директории (каталога скрипта).
    fn resolve_dir_library_main(&self, path: &str) -> Option<std::path::PathBuf> {
        use shared::types::KumirConfig;
        let base = self.file_importer.read().ok()?.base_dir().to_path_buf();
        let candidates = [std::path::PathBuf::from(path), base.join(path)];
        for dir in candidates {
            if dir.is_dir() {
                let toml = dir.join("kumir.toml");
                if toml.exists() {
                    // 1) Главный файл из конфига.
                    if let Ok(cfg) = KumirConfig::load(&toml) {
                        let main = cfg.main_file_path();
                        if main.exists() {
                            return Some(main);
                        }
                    }
                    // 2) Запасные общепринятые расположения главного файла.
                    for rel in ["src/lib.kum", "lib.kum", "src/main.kum", "main.kum"] {
                        let cand = dir.join(rel);
                        if cand.exists() {
                            return Some(cand);
                        }
                    }
                }
            }
        }
        None
    }

    /// Импортирует .kum файл.
    pub fn import_file(
        &mut self,
        path: &str,
        alias: Option<&str>,
    ) -> RuntimeResult<std::sync::Arc<ImportedModule>> {
        let mut importer = self.file_importer.write().map_err(|_| {
            RuntimeError::new(
                "Не удалось получить доступ к импортеру",
                RuntimeErrorKind::Other,
            )
        })?;
        importer.import(path, alias)
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
mod typeops_characterization;

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
            Value::Number(shared::types::Number::I64(5))
        );
        assert_eq!(
            interpreter.eval("10 - 4").unwrap(),
            Value::Number(shared::types::Number::I64(6))
        );
        assert_eq!(
            interpreter.eval("3 * 4").unwrap(),
            Value::Number(shared::types::Number::I64(12))
        );

        // Деление возвращает F128 (вещественное деление)
        let div_result = interpreter.eval("15 / 3").unwrap();
        match div_result {
            Value::Number(n) => {
                let f_val = match n {
                    shared::types::Number::F128(f) => f.to_f64(),
                    shared::types::Number::F64(f) => f,
                    shared::types::Number::I64(i) => i as f64,
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
                    shared::types::Number::F128(f) => f.to_f64(),
                    shared::types::Number::F64(f) => f,
                    shared::types::Number::I64(i) => i as f64,
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
    fn test_inheritance_method_dispatch() {
        // [KITE 11] Унаследованный метод должен находиться по иерархии (Кот → Животное).
        let source = r#"
класс Животное
алг лит звук()
нач
    знач := "животное"
кон
кон

класс Кот расширяет Животное
конструктор()
нач
кон
кон

алг Тест
нач
    к := новый Кот()
    вывод к.звук()
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(interpreter.get_output().contains("животное"));
    }

    #[test]
    fn test_method_override_polymorphism() {
        // [KITE 11] Переопределённый метод подкласса должен выигрывать у родителя
        // — это работает только при корректной идентичности объекта (type_id).
        let source = r#"
класс Животное
алг лит звук()
нач
    знач := "животное"
кон
кон

класс Собака расширяет Животное
конструктор()
нач
кон
алг лит звук()
нач
    знач := "гав"
кон
кон

алг Тест
нач
    с := новый Собака()
    вывод с.звук()
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        let out = interpreter.get_output();
        assert!(
            out.contains("гав"),
            "ожидали переопределённый метод, вывод: {}",
            out
        );
    }

    #[test]
    fn test_super_method_call() {
        // [KITE 11] предок.метод() вызывает реализацию родителя, обходя переопределение.
        let source = r#"
класс Животное
алг лит звук()
нач
    знач := "животное"
кон
кон

класс Собака расширяет Животное
конструктор()
нач
кон
алг лит звук()
нач
    знач := предок.звук() + "-гав"
кон
кон

алг Тест
нач
    с := новый Собака()
    вывод с.звук()
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        let out = interpreter.get_output();
        assert!(out.contains("животное-гав"), "вывод: {}", out);
    }

    #[test]
    fn test_abstract_cannot_instantiate() {
        // [KITE 11] Создание экземпляра абстрактного класса запрещено.
        let source = r#"
абстрактный класс Фигура
конструктор()
нач
кон
кон

алг Тест
нач
    ф := новый Фигура()
кон
"#;
        let mut interpreter = Interpreter::new();
        assert!(
            interpreter.run(source).is_err(),
            "абстрактный класс не должен создаваться"
        );
    }

    #[test]
    fn test_final_method_cannot_be_overridden() {
        // [KITE 11] Переопределение `финал`-метода запрещено.
        let source = r#"
класс Основа
финал алг лит метка()
нач
    знач := "основа"
кон
кон

класс Потомок расширяет Основа
алг лит метка()
нач
    знач := "потомок"
кон
кон

алг Тест
нач
кон
"#;
        let mut interpreter = Interpreter::new();
        let err = interpreter.run(source).unwrap_err();
        assert!(
            err.message.contains("переопредел"),
            "сообщение: {}",
            err.message
        );
    }

    #[test]
    fn test_abstract_method_must_be_implemented() {
        // [KITE 11] Неабстрактный класс обязан реализовать абстрактный метод предка.
        let source = r#"
абстрактный класс Фигура
абстрактный алг вещ площадь()
кон

класс Круг расширяет Фигура
конструктор()
нач
кон
кон

алг Тест
нач
кон
"#;
        let mut interpreter = Interpreter::new();
        let err = interpreter.run(source).unwrap_err();
        assert!(
            err.message.contains("реализовать"),
            "сообщение: {}",
            err.message
        );
    }

    #[test]
    fn test_impl_method_dispatch() {
        // [KITE 11, шаг 4] Метод из impl-блока (`реализация Тип`) должен диспетчеризоваться.
        let source = r#"
класс Точка
конструктор()
нач
кон
кон

реализация Точка
алг лит показать()
нач
    знач := "точка!"
кон
кон

алг Тест
нач
    т := новый Точка()
    вывод т.показать()
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        let out = interpreter.get_output();
        assert!(out.contains("точка!"), "вывод: {}", out);
    }

    #[test]
    fn test_private_field_access_denied() {
        // [KITE 11, шаг 5] Доступ к закрытому полю извне класса запрещён.
        let source = r#"
класс Счёт
закрытый:
цел баланс
открытый:
конструктор()
нач
кон
кон

алг Тест
нач
    с := новый Счёт()
    вывод с.баланс
кон
"#;
        let mut interpreter = Interpreter::new();
        let err = interpreter.run(source).unwrap_err();
        assert!(err.message.contains("закрыт"), "сообщение: {}", err.message);
    }

    #[test]
    fn test_public_field_access_allowed() {
        // [KITE 11, шаг 5] Открытое поле доступно извне.
        let source = r#"
класс Точка2
открытый:
цел x
конструктор()
нач
кон
кон

алг Тест
нач
    т := новый Точка2()
    вывод т.x
кон
"#;
        let mut interpreter = Interpreter::new();
        assert!(
            interpreter.run(source).is_ok(),
            "открытое поле должно быть доступно"
        );
    }

    #[test]
    fn test_kumir_toml_library_dir() {
        // [KITE 5] Библиотека-проект (директория с kumir.toml): её функции вызываемы.
        use std::fs;
        let dir = std::env::temp_dir().join(format!("kumir_lib_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("kumir.toml"),
            "[package]\nname = \"л\"\nmain = \"src/lib.kum\"\n",
        )
        .unwrap();
        fs::write(
            dir.join("src").join("lib.kum"),
            "алг цел дв(цел x)\nнач\n  знач := x * 2\nкон\n",
        )
        .unwrap();

        let parent = dir.parent().unwrap().to_path_buf();
        let libname = dir.file_name().unwrap().to_string_lossy().to_string();

        let mut interp = Interpreter::new();
        interp.set_base_dir(&parent);
        let src = format!(
            "использовать \"{}\"\nалг Тест\nнач\n    вывод дв(21)\nкон\n",
            libname
        );
        interp.run(&src).unwrap();
        assert!(
            interp.get_output().contains("42"),
            "вывод: {}",
            interp.get_output()
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_bytes_type() {
        // [KITE 2] Тип байты: создание из строки, длина, обратное преобразование.
        let source = r#"
алг Тест
нач
    б := байты("AB")
    вывод длина(б)
    вывод строка_из_байт(б)
кон
"#;
        let mut interp = Interpreter::new();
        interp.run(source).unwrap();
        let out = interp.get_output();
        assert!(out.contains("2"), "длина байтов: {}", out);
        assert!(out.contains("AB"), "обратное преобразование: {}", out);
    }

    #[test]
    fn test_range_value_display() {
        // [KITE 2] Диапазон как значение печатается как 1..10 / 1..=10.
        let mut interpreter = Interpreter::new();
        interpreter
            .run("алг Тест\nнач\n    д := 1..10\n    вывод д\nкон\n")
            .unwrap();
        assert!(
            interpreter.get_output().contains("1..10"),
            "вывод: {}",
            interpreter.get_output()
        );

        let mut i2 = Interpreter::new();
        i2.run("алг Тест\nнач\n    д := 1..=10\n    вывод д\nкон\n")
            .unwrap();
        assert!(
            i2.get_output().contains("1..=10"),
            "вывод: {}",
            i2.get_output()
        );
    }

    #[test]
    fn test_range_loop_iteration() {
        // [KITE 2/4] Итерация по диапазону в `нц для … в …`.
        let source = r#"
алг Тест
нач
    цел сумма
    сумма := 0
    нц для к в 1..=5
        сумма := сумма + к
    кц
    вывод сумма
кон
"#;
        let mut interpreter = Interpreter::new();
        interpreter.run(source).unwrap();
        assert!(
            interpreter.get_output().contains("15"),
            "вывод: {}",
            interpreter.get_output()
        );
    }

    #[test]
    fn test_builtin_functions() {
        let mut interpreter = Interpreter::new();

        // Математика
        assert_eq!(
            interpreter.eval("abs(-5)").unwrap(),
            Value::Number(shared::types::Number::I64(5))
        );
        assert_eq!(
            interpreter.eval("min(3, 7)").unwrap(),
            Value::Number(shared::types::Number::I64(3))
        );
        assert_eq!(
            interpreter.eval("max(3, 7)").unwrap(),
            Value::Number(shared::types::Number::I64(7))
        );

        // Строки
        assert_eq!(
            interpreter.eval("длина(\"привет\")").unwrap(),
            Value::Number(shared::types::Number::I64(6))
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
        let _source = r#"
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
        assert_eq!(result, Value::Number(shared::types::Number::I64(1)));
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
