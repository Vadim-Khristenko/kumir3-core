//! Интегрированный загрузчик библиотек с поддержкой виртуальных окружений
//!
//! Объединяет функциональность загрузки библиотек и управления окружениями.
//!
//! # Структура хранения
//!
//! ```text
//! ~/.kumir/
//! ├── registry/                    # Глобальный реестр библиотек
//! │   ├── sockets-1.0.0/
//! │   │   ├── manifest.toml        # Метаданные библиотеки
//! │   │   ├── lib.kum              # Исходный код (Kumir-библиотека)
//! │   │   └── native/              # Нативные модули (опционально)
//! │   │       └── sockets.dll
//! │   └── http-2.1.0/
//! │       └── ...
//! ├── cache/                       # Кэш скачанных пакетов
//! │   └── downloads/
//! └── config.toml                  # Глобальная конфигурация
//!
//! project/
//! ├── kumir.toml                   # Конфигурация проекта
//! ├── kumir.lock                   # Lock-файл с разрешёнными версиями
//! ├── libs/                        # Локальные библиотеки проекта
//! │   └── mylib/
//! │       └── lib.kum
//! └── main.kum
//! ```
//!
//! # Приоритет загрузки
//!
//! 1. Встроенные библиотеки (compiled-in)
//! 2. Локальные библиотеки проекта (./libs/)
//! 3. Lock-файл (kumir.lock) - точные версии
//! 4. Глобальный реестр (~/.kumir/registry/)
//! 5. Удалённые репозитории (в будущем)
//!
//! # Организация модуля
//!
//! - [`error`] — тип ошибки загрузчика и алиас результата;
//! - [`manifest`] — манифест библиотеки (`manifest.toml`) и его разбор;
//! - [`discovery`] — поиск библиотек на диске и чтение файлов;
//! - [`resolve`] — разрешение версий и загрузка с зависимостями;
//! - [`activate`] — активация окружения проекта и lock-файл.

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
//                         ЗАГРУЖЕННАЯ БИБЛИОТЕКА
// =============================================================================

/// Информация о загруженной библиотеке
#[derive(Debug, Clone)]
pub struct LoadedLibrary {
    /// Определение библиотеки
    pub def: LibraryDef,
    /// Версия
    pub version: Version,
    /// Источник загрузки
    pub source: LibrarySource,
    /// Путь к библиотеке (если есть)
    pub path: Option<PathBuf>,
    /// Исходный код (для Kumir-библиотек)
    pub source_code: Option<String>,
    /// Манифест (если есть)
    pub manifest: Option<LibraryManifest>,
}

impl LoadedLibrary {
    /// Конвертирует в VersionedLibrary для окружения
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
//                    ИНТЕГРИРОВАННЫЙ ЗАГРУЗЧИК
// =============================================================================

/// Глобальный загрузчик
pub static LOADER: Lazy<RwLock<IntegratedLoader>> =
    Lazy::new(|| RwLock::new(IntegratedLoader::new()));

/// Интегрированный загрузчик с поддержкой виртуальных окружений
pub struct IntegratedLoader {
    /// Менеджер окружений
    pub env_manager: EnvironmentManager,
    /// Встроенные библиотеки
    builtins: HashMap<String, LibraryDef>,
    /// Стек загрузки (для обнаружения циклов)
    loading_stack: Vec<String>,
    /// Кэш загруженных библиотек (путь -> библиотека)
    file_cache: HashMap<PathBuf, LoadedLibrary>,
}

impl IntegratedLoader {
    /// Создаёт новый загрузчик
    pub fn new() -> Self {
        Self {
            env_manager: EnvironmentManager::new(),
            builtins: HashMap::new(),
            loading_stack: Vec::new(),
            file_cache: HashMap::new(),
        }
    }

    // =========================================================================
    //                         РЕГИСТРАЦИЯ ВСТРОЕННЫХ
    // =========================================================================

    /// Регистрирует встроенную библиотеку
    pub fn register_builtin(&mut self, def: LibraryDef) {
        // Сохраняем по всем именам
        let name = def.name.to_string();
        self.builtins.insert(def.id.to_string(), def.clone());
        self.builtins.insert(name, def.clone());
        for alias in &def.aliases {
            self.builtins.insert(alias.to_string(), def.clone());
        }

        // Регистрируем в глобальном окружении
        self.env_manager.global_mut().register_builtin(def);
    }

    /// Проверяет, является ли библиотека встроенной
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
