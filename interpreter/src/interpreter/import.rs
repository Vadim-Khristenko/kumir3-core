use super::Interpreter;
use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::file_importer::{FileImporter, ImportedModule};
use super::library_bridge;
use shared::types::Stmt;

impl Interpreter {
    // =========================================================================
    //                    БИБЛИОТЕКИ И ИМПОРТЫ
    // =========================================================================

    /// Обрабатывает инструкцию импорта.
    ///
    /// Поддерживает:
    /// - Стандартные библиотеки: `использовать время`
    /// - Файловые импорты: `подключить "./модуль.kum"`
    pub(crate) fn process_import(&mut self, stmt: &Stmt) -> RuntimeResult<()> {
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
}
