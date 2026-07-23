//! Import handling for libraries and .kum modules.

use super::Interpreter;
use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::file_importer::{FileImporter, ImportedModule};
use super::library_bridge;
use shared::types::Stmt;

impl Interpreter {
    // =============================================================================
    //                       IMPORT HANDLING
    // =============================================================================

    /// Processes an import statement.
    ///
    /// Supports:
    /// - Standard libraries: `использовать время`
    /// - File imports: `подключить "./module.kum"`
    /// - Project libraries: directories with `kumir.toml`
    pub(crate) fn process_import(&mut self, stmt: &Stmt) -> RuntimeResult<()> {
        if let Stmt::Import { path, alias, .. } = stmt {
            let path_obj = std::path::Path::new(path);

            if let Some(main_file) = self.resolve_dir_library_main(path) {
                // [KITE 5] Project library (directory with kumir.toml): import its main file
                // as a module so algorithms become callable.
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
                // File import (like Python)
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
                // User library (directory or file path)
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
                // Standard library
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
                return Err(RuntimeError::new(
                    format!("Модуль или библиотека '{}' не найдены", path),
                    RuntimeErrorKind::Other,
                ));
            }
        }
        Ok(())
    }

    /// [KITE 5] Registers public algorithms and classes from an imported module into
    /// the environment (with module/alias prefix; without prefix if no alias).
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

    /// [KITE 5] If `path` points to a project directory with `kumir.toml`, returns
    /// the path to its main `.kum` file. The directory is searched as-is and relative
    /// to the base directory (script directory).
    fn resolve_dir_library_main(&self, path: &str) -> Option<std::path::PathBuf> {
        use shared::types::KumirConfig;
        let base = self.file_importer.read().ok()?.base_dir().to_path_buf();
        let candidates = [std::path::PathBuf::from(path), base.join(path)];
        for dir in candidates {
            if dir.is_dir() {
                let toml = dir.join("kumir.toml");
                if toml.exists() {
                    // 1) Main file from config.
                    if let Ok(cfg) = KumirConfig::load(&toml) {
                        let main = cfg.main_file_path();
                        if main.exists() {
                            return Some(main);
                        }
                    }
                    // 2) Fallback conventional locations.
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

    /// Imports a .kum file.
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

    /// Imports a library programmatically.
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

    /// Imports a library with an alias.
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
}
