//! Library manifest (`manifest.toml`) and parsing.

use std::fs;
use std::path::{Path, PathBuf};

use toml::Value;

use crate::types::version::{Version, VersionSpec};

use super::error::{LoaderError, LoaderResult};

// =============================================================================
//                         LIBRARY MANIFEST
// =============================================================================

/// Library manifest (manifest.toml).
#[derive(Debug, Clone)]
pub struct LibraryManifest {
    /// Library identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Aliases (alternative names)
    pub aliases: Vec<String>,
    /// Description
    pub description: String,
    /// Version
    pub version: Version,
    /// Author
    pub author: String,
    /// Minimum Kumir version
    pub kumir_version: String,
    /// Dependencies
    pub dependencies: Vec<ManifestDependency>,
    /// Entry point (main file)
    pub entry_point: String,
    /// Native modules
    pub native_modules: Vec<String>,
    /// Is this a stable library
    pub stable: bool,
}

/// Manifest dependency.
#[derive(Debug, Clone)]
pub struct ManifestDependency {
    /// Library name
    pub name: String,
    /// Version specification
    pub version: VersionSpec,
    /// Optional dependency
    pub optional: bool,
}

impl LibraryManifest {
    /// Parse manifest from TOML string.
    pub fn parse(content: &str) -> LoaderResult<Self> {
        let value: Value =
            content
                .parse::<Value>()
                .map_err(|e: toml::de::Error| LoaderError::ParseError {
                    path: PathBuf::new(),
                    message: e.to_string(),
                })?;

        let table = value.as_table().ok_or_else(|| LoaderError::ParseError {
            path: PathBuf::new(),
            message: "manifest.toml должен быть TOML-объектом".to_string(),
        })?;

        let mut manifest = LibraryManifest {
            id: String::new(),
            name: String::new(),
            aliases: Vec::new(),
            description: String::new(),
            version: Version::new(0, 1, 0),
            author: String::new(),
            kumir_version: "3.0".to_string(),
            dependencies: Vec::new(),
            entry_point: "lib.kum".to_string(),
            native_modules: Vec::new(),
            stable: true,
        };

        let get_str = |t: &toml::value::Table, key: &str| -> Option<String> {
            t.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
        };

        manifest.name = get_str(table, "name").unwrap_or_default();
        manifest.id = get_str(table, "id").unwrap_or_else(|| manifest.name.clone());

        if manifest.id.is_empty() && manifest.name.is_empty() {
            return Err(LoaderError::ParseError {
                path: PathBuf::new(),
                message: "Не указан id или name в manifest.toml".to_string(),
            });
        }

        if let Some(ver) = get_str(table, "version") {
            manifest.version = ver.parse().map_err(|e| LoaderError::ParseError {
                path: PathBuf::new(),
                message: format!("Неверный формат version: {}", e),
            })?;
        } else {
            return Err(LoaderError::ParseError {
                path: PathBuf::new(),
                message: "Отсутствует обязательное поле version".to_string(),
            });
        }

        manifest.description = get_str(table, "description").unwrap_or_default();
        manifest.author = get_str(table, "author").unwrap_or_default();
        manifest.kumir_version =
            get_str(table, "kumir_version").unwrap_or_else(|| "3.0".to_string());
        manifest.entry_point =
            get_str(table, "entry_point").unwrap_or_else(|| "lib.kum".to_string());
        manifest.stable = table
            .get("stable")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if let Some(arr) = table.get("aliases").and_then(|v| v.as_array()) {
            manifest.aliases = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        if let Some(arr) = table.get("native_modules").and_then(|v| v.as_array()) {
            manifest.native_modules = arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        }

        if let Some(deps) = table.get("dependencies").and_then(|v| v.as_table()) {
            for (dep_name, dep_val) in deps {
                match dep_val {
                    Value::String(req) => {
                        let version = req.parse().map_err(|e| LoaderError::ParseError {
                            path: PathBuf::new(),
                            message: format!("Неверная версия для зависимости {}: {}", dep_name, e),
                        })?;
                        manifest.dependencies.push(ManifestDependency {
                            name: dep_name.to_string(),
                            version,
                            optional: false,
                        });
                    }
                    Value::Table(dep_table) => {
                        let version_str = dep_table
                            .get("version")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| LoaderError::ParseError {
                                path: PathBuf::new(),
                                message: format!(
                                    "Зависимость {} должна содержать version",
                                    dep_name
                                ),
                            })?;

                        let version = version_str.parse().map_err(|e| LoaderError::ParseError {
                            path: PathBuf::new(),
                            message: format!("Неверная версия для зависимости {}: {}", dep_name, e),
                        })?;

                        let optional = dep_table
                            .get("optional")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);

                        manifest.dependencies.push(ManifestDependency {
                            name: dep_name.to_string(),
                            version,
                            optional,
                        });
                    }
                    _ => {
                        return Err(LoaderError::ParseError {
                            path: PathBuf::new(),
                            message: format!("Неверный формат зависимости {}", dep_name),
                        });
                    }
                }
            }
        }

        // Если name не указан, но есть id, используем id как name
        if manifest.name.is_empty() {
            manifest.name = manifest.id.clone();
        }

        Ok(manifest)
    }

    /// Загружает манифест из файла
    pub fn load(path: &Path) -> LoaderResult<Self> {
        let content = fs::read_to_string(path)?;
        Self::parse(&content).map_err(|e| LoaderError::ManifestError {
            path: path.to_path_buf(),
            message: e.to_string(),
        })
    }
}
