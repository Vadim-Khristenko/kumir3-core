//! Integrated library loader with virtual environment support.
//!
//! Combines library loading and environment management functionality.
//!
//! # Storage structure
//!
//! ```text
//! ~/.kumir/
//! ├── registry/                    # Global library registry
//! │   ├── sockets-1.0.0/
//! │   │   ├── manifest.toml        # Library metadata
//! │   │   ├── lib.kum              # Source code (Kumir library)
//! │   │   └── native/              # Native modules (optional)
//! │   │       └── sockets.dll
//! │   └── http-2.1.0/
//! │       └── ...
//! ├── cache/                       # Downloaded packages cache
//! │   └── downloads/
//! └── config.toml                  # Global configuration
//!
//! project/
//! ├── kumir.toml                   # Project configuration
//! ├── kumir.lock                   # Lock file with resolved versions
//! ├── libs/                        # Project-local libraries
//! │   └── mylib/
//! │       └── lib.kum
//! └── main.kum
//! ```
//!
//! # Loading priority
//!
//! 1. Built-in libraries (compiled-in)
//! 2. Project-local libraries (./libs/)
//! 3. Lock file (kumir.lock) - exact versions
//! 4. Global registry (~/.kumir/registry/)
//! 5. Remote repositories (future)
//!
//! # Module organization
//!
//! - [`error`] — loader error type and result alias;
//! - [`manifest`] — library manifest (`manifest.toml`) and parsing;
//! - [`discovery`] — library discovery on disk and file reading;
//! - [`resolve`] — version resolution and loading with dependencies;
//! - [`activate`] — project environment activation and lock file.

mod activate;
mod discovery;
mod error;
mod manifest;
mod resolve;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use once_cell::sync::Lazy;

use super::environment::{EnvironmentManager, LibrarySource, VersionedLibrary};
use super::library::LibraryDef;
use super::version::{Version, VersionSpec};

pub use error::{LoaderError, LoaderResult};
pub use manifest::{LibraryManifest, ManifestDependency};

// =============================================================================
//                         LOADED LIBRARY
// =============================================================================

/// Information about a loaded library.
#[derive(Debug, Clone)]
pub struct LoadedLibrary {
    /// Library definition
    pub def: LibraryDef,
    /// Version
    pub version: Version,
    /// Loading source
    pub source: LibrarySource,
    /// Path to library (if any)
    pub path: Option<PathBuf>,
    /// Source code (for Kumir libraries)
    pub source_code: Option<String>,
    /// Manifest (if any)
    pub manifest: Option<LibraryManifest>,
}

impl LoadedLibrary {
    /// Converts to VersionedLibrary for environment.
    pub fn into_versioned(self) -> VersionedLibrary {
        VersionedLibrary {
            def: self.def,
            version: self.version,
            source: self.source,
            path: self.path,
            checksum: None,
        }
    }
}

// =============================================================================
//                    INTEGRATED LOADER
// =============================================================================

/// Global loader.
pub static LOADER: Lazy<RwLock<IntegratedLoader>> =
    Lazy::new(|| RwLock::new(IntegratedLoader::new()));

/// Integrated loader with virtual environment support.
pub struct IntegratedLoader {
    /// Environment manager
    pub env_manager: EnvironmentManager,
    /// Built-in libraries
    builtins: HashMap<String, LibraryDef>,
    /// Loading stack (for cycle detection)
    loading_stack: Vec<String>,
    /// Loaded libraries cache (path -> library)
    file_cache: HashMap<PathBuf, LoadedLibrary>,
}

impl IntegratedLoader {
    /// Creates a new loader.
    pub fn new() -> Self {
        Self {
            env_manager: EnvironmentManager::new(),
            builtins: HashMap::new(),
            loading_stack: Vec::new(),
            file_cache: HashMap::new(),
        }
    }

    // =========================================================================
    //                         BUILTIN REGISTRATION
    // =========================================================================

    /// Register a built-in library.
    pub fn register_builtin(&mut self, def: LibraryDef) {
        // Store by all names
        let name = def.name.to_string();
        self.builtins.insert(def.id.to_string(), def.clone());
        self.builtins.insert(name, def.clone());
        for alias in &def.aliases {
            self.builtins.insert(alias.to_string(), def.clone());
        }

        // Register in global environment
        self.env_manager.global_mut().register_builtin(def);
    }

    /// Check if a library is built-in.
    pub fn is_builtin(&self, name: &str) -> bool {
        self.builtins.contains_key(name)
    }

    /// Очищает кэш файлов
    pub fn clear_cache(&mut self) {
        self.file_cache.clear();
    }
}

impl Default for IntegratedLoader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
//                         ПУБЛИЧНОЕ API
// =============================================================================

/// Получает глобальный загрузчик
pub fn loader() -> &'static RwLock<IntegratedLoader> {
    &LOADER
}

/// Регистрирует встроенную библиотеку
pub fn register_builtin(def: LibraryDef) {
    if let Ok(mut loader) = LOADER.write() {
        loader.register_builtin(def);
    }
}

/// Загружает библиотеку
pub fn load_library(name: &str) -> LoaderResult<LoadedLibrary> {
    LOADER
        .write()
        .map_err(|_| LoaderError::LockError("Не удалось получить блокировку".to_string()))?
        .load(name)
}

/// Загружает библиотеку с проверкой версии
pub fn load_library_versioned(name: &str, spec: &VersionSpec) -> LoaderResult<LoadedLibrary> {
    LOADER
        .write()
        .map_err(|_| LoaderError::LockError("Не удалось получить блокировку".to_string()))?
        .load_with_spec(name, spec)
}

/// Загружает библиотеку со всеми зависимостями
pub fn load_library_with_deps(name: &str) -> LoaderResult<Vec<LoadedLibrary>> {
    LOADER
        .write()
        .map_err(|_| LoaderError::LockError("Не удалось получить блокировку".to_string()))?
        .load_with_dependencies(name)
}

/// Активирует проект
pub fn activate_project(path: impl AsRef<Path>) -> LoaderResult<()> {
    LOADER
        .write()
        .map_err(|_| LoaderError::LockError("Не удалось получить блокировку".to_string()))?
        .activate_project(path)
}

/// Деактивирует проект
pub fn deactivate_project() {
    if let Ok(mut loader) = LOADER.write() {
        loader.deactivate_project();
    }
}

/// Возвращает список доступных библиотек
pub fn list_available() -> Vec<String> {
    LOADER
        .read()
        .map(|l| l.available_libraries())
        .unwrap_or_default()
}

// =============================================================================
//                         ТЕСТЫ
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_parse() {
        let toml = r#"
id = "sockets"
name = "Сокеты"
description = "Библиотека для работы с сетью"
version = "1.2.3"
author = "Kumir Team"
aliases = ["сокеты", "net"]

[dependencies]
http = "^2.0"

[dependencies.json]
version = ">=1.0"
optional = true
"#;

        let manifest = LibraryManifest::parse(toml).unwrap();

        assert_eq!(manifest.id, "sockets");
        assert_eq!(manifest.name, "Сокеты");
        assert_eq!(manifest.version, Version::new(1, 2, 3));
        assert_eq!(manifest.aliases, vec!["сокеты", "net"]);
        assert_eq!(manifest.dependencies.len(), 2);

        let http_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "http")
            .unwrap();
        assert!(!http_dep.optional);

        let json_dep = manifest
            .dependencies
            .iter()
            .find(|d| d.name == "json")
            .unwrap();
        assert!(json_dep.optional);
    }

    #[test]
    fn test_loader_creation() {
        let loader = IntegratedLoader::new();
        assert!(loader.builtins.is_empty());
    }

    #[test]
    fn test_builtin_registration() {
        let mut loader = IntegratedLoader::new();

        let def = LibraryDef::new("test", "Тест");
        loader.register_builtin(def);

        assert!(loader.is_builtin("test"));
        assert!(loader.is_builtin("Тест"));
    }

    #[test]
    fn test_load_builtin() {
        let mut loader = IntegratedLoader::new();

        let def = LibraryDef::new("mylib", "МояБиблиотека");
        loader.register_builtin(def);

        let result = loader.load("mylib");
        assert!(result.is_ok());

        let lib = result.unwrap();
        assert!(matches!(lib.source, LibrarySource::Builtin));
    }

    #[test]
    fn test_load_not_found() {
        let mut loader = IntegratedLoader::new();
        let result = loader.load("nonexistent");
        assert!(matches!(result, Err(LoaderError::NotFound { .. })));
    }
}
