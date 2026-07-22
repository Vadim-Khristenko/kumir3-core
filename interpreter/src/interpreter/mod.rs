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
mod config;
mod construct;
mod environment;
mod error;
mod evaluator;
mod executor;
mod file_importer;
mod import;
mod library_bridge;
mod oop;
mod ops;
mod run;
mod state;

pub use environment::Environment;
pub use error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
pub use evaluator::ExprEvaluator;
pub use executor::Executor;
pub use file_importer::{FileImporter, ImportedModule};
pub use library_bridge::LibraryManager;
pub use run::{eval, run, run_and_get_output};

// Реэкспорт из shared::runtime для async
pub use shared::runtime::KumirRuntime;

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
///
/// ## Организация реализации
///
/// Методы `Interpreter` разнесены по подмодулям одного и того же модуля:
/// - `construct` - конструкторы и `Default`;
/// - `config` - настройки (пути импорта, отладка, строгий режим);
/// - `state` - среда, переменные, вывод, предупреждения, библиотеки;
/// - `run` - запуск программ, вызов алгоритмов, вычисление выражений
///   и свободные функции `run`/`eval`/`run_and_get_output`;
/// - `import`, `oop` - импорты и проверки ООП.
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

// =============================================================================
//                           ТЕСТЫ
// =============================================================================

#[cfg(test)]
mod tests;

#[cfg(test)]
mod typeops_characterization;
