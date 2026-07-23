//! Full-featured Kumir 3 language interpreter.
//!
//! Supports all core features: basic types, arrays, control flow, algorithms,
//! object-oriented programming, exception handling, and built-in functions.

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
//                          INTERPRETER
// =============================================================================

/// Kumir 3 language interpreter.
///
/// Executes Kumir programs with full language support. Methods are organized
/// across submodules within this module for logical grouping:
/// - `construct`: constructors and `Default`
/// - `config`: configuration (import paths, debug, strict mode)
/// - `state`: environment, variables, output, warnings, libraries
/// - `run`: program execution, algorithm calls, expression evaluation
/// - `import`, `oop`: imports and OOP validation
pub struct Interpreter {
    /// Execution environment.
    env: Environment,
    /// Library manager (shared for access from Environment).
    libraries: std::sync::Arc<std::sync::RwLock<LibraryManager>>,
    /// File importer for .kum modules (shared for access from Environment).
    file_importer: std::sync::Arc<std::sync::RwLock<FileImporter>>,
    /// Runtime for async operations.
    runtime: Option<KumirRuntime>,
    /// Debug mode flag.
    debug_mode: bool,
}

// =============================================================================
//                            TESTS
// =============================================================================

#[cfg(test)]
mod tests;

#[cfg(test)]
mod typeops_characterization;
