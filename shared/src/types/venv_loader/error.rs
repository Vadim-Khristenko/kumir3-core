//! Integrated loader errors.

use std::io;
use std::path::PathBuf;

use crate::types::version::{Version, VersionSpec};

// =============================================================================
//                         ERRORS
// =============================================================================

/// Loader errors.
#[derive(Debug, Clone)]
pub enum LoaderError {
    /// Library not found
    NotFound {
        name: String,
        searched_paths: Vec<PathBuf>,
    },
    /// Version not found
    VersionNotFound {
        name: String,
        requested: VersionSpec,
        available: Vec<Version>,
    },
    /// Incompatible version
    VersionMismatch {
        name: String,
        required: VersionSpec,
        found: Version,
    },
    /// File read error
    IoError(String),
    /// Library parsing error
    ParseError { path: PathBuf, message: String },
    /// Manifest parsing error
    ManifestError { path: PathBuf, message: String },
    /// Cyclic dependency
    CyclicDependency(Vec<String>),
    /// Version conflict
    VersionConflict {
        name: String,
        required_by: Vec<(String, VersionSpec)>,
    },
    /// Native module loading error
    NativeLoadError { path: PathBuf, message: String },
    /// Environment not initialized
    EnvironmentNotInitialized,
    /// Lock is busy
    LockError(String),
}

impl std::fmt::Display for LoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoaderError::NotFound {
                name,
                searched_paths,
            } => {
                write!(f, "Библиотека '{}' не найдена.\nПути поиска:\n", name)?;
                for path in searched_paths {
                    writeln!(f, "  - {}", path.display())?;
                }
                Ok(())
            }
            LoaderError::VersionNotFound {
                name,
                requested,
                available,
            } => {
                write!(
                    f,
                    "Версия {} библиотеки '{}' не найдена.\nДоступные версии: {}",
                    requested,
                    name,
                    available
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            LoaderError::VersionMismatch {
                name,
                required,
                found,
            } => {
                write!(
                    f,
                    "Несовместимая версия библиотеки '{}': требуется {}, найдена {}",
                    name, required, found
                )
            }
            LoaderError::IoError(e) => write!(f, "Ошибка чтения: {}", e),
            LoaderError::ParseError { path, message } => {
                write!(f, "Ошибка парсинга {}: {}", path.display(), message)
            }
            LoaderError::ManifestError { path, message } => {
                write!(f, "Ошибка манифеста {}: {}", path.display(), message)
            }
            LoaderError::CyclicDependency(chain) => {
                write!(f, "Циклическая зависимость: {}", chain.join(" → "))
            }
            LoaderError::VersionConflict { name, required_by } => {
                writeln!(f, "Конфликт версий библиотеки '{}':", name)?;
                for (dep, spec) in required_by {
                    writeln!(f, "  - {} требует {}", dep, spec)?;
                }
                Ok(())
            }
            LoaderError::NativeLoadError { path, message } => {
                write!(
                    f,
                    "Ошибка загрузки нативного модуля {}: {}",
                    path.display(),
                    message
                )
            }
            LoaderError::EnvironmentNotInitialized => {
                write!(f, "Окружение не инициализировано")
            }
            LoaderError::LockError(e) => write!(f, "Ошибка блокировки: {}", e),
        }
    }
}

impl std::error::Error for LoaderError {}

impl From<io::Error> for LoaderError {
    fn from(e: io::Error) -> Self {
        LoaderError::IoError(e.to_string())
    }
}

pub type LoaderResult<T> = Result<T, LoaderError>;
