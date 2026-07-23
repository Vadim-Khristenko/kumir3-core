//! Constructors for the interpreter with optional async runtime.

use super::{Environment, FileImporter, Interpreter, KumirRuntime, LibraryManager};

impl Interpreter {
    /// Creates a new interpreter.
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

    /// Creates an interpreter with a runtime for async operations.
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

    /// Creates an interpreter with an existing environment.
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
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}
