//! Импорт библиотек и модулей.

use super::super::environment::Environment;
use super::super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::file_importer::FileImporter;
use super::Executor;
use std::sync::Arc;

impl Executor {
    // =========================================================================
    //                    ИМПОРТ БИБЛИОТЕК И МОДУЛЕЙ
    // =========================================================================

    /// Выполняет импорт библиотеки или файла.
    ///
    /// Поддерживает:
    /// - Библиотеки: `использовать время`, `использовать время@^2.0`
    /// - Файлы: `использовать "./модуль.kum"`, `использовать ../utils`
    /// - Алиасы: `использовать время как т`
    /// - Выборочный импорт: `использовать время { now_ms, sleep }`
    pub(crate) fn execute_import(
        path: &str,
        alias: Option<&str>,
        items: Option<&[String]>,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        // Проверяем, является ли это файлом или библиотекой
        let is_file = FileImporter::is_kum_file(path);

        if is_file {
            // Импорт .kum файла
            Self::execute_file_import(path, alias, items, env)
        } else {
            // Импорт библиотеки
            Self::execute_library_import(path, alias, items, env)
        }
    }

    /// Импортирует библиотеку.
    fn execute_library_import(
        path: &str,
        alias: Option<&str>,
        items: Option<&[String]>,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        // Парсим имя библиотеки и версию (например, "время@^2.0")
        let (lib_name, version_spec) = if let Some(at_pos) = path.find('@') {
            let name = &path[..at_pos];
            let version = &path[at_pos + 1..];
            (name, Some(version))
        } else {
            (path, None)
        };

        // Получаем менеджер библиотек
        let lib_manager = env.library_manager().ok_or_else(|| {
            RuntimeError::new(
                "Менеджер библиотек не инициализирован",
                RuntimeErrorKind::Other,
            )
        })?;

        // Импортируем библиотеку
        if let Some(version) = version_spec {
            lib_manager
                .write()
                .unwrap()
                .import_versioned(lib_name, version, alias)?;
        } else {
            lib_manager.write().unwrap().import(lib_name, alias)?;
        }

        // Если указан выборочный импорт - проверяем доступность функций
        if let Some(item_names) = items {
            let manager = lib_manager.read().unwrap();
            for item in item_names {
                if !manager.is_library_function(item) {
                    return Err(RuntimeError::new(
                        format!("Функция '{}' не найдена в библиотеке '{}'", item, lib_name),
                        RuntimeErrorKind::UndefinedAlgorithm,
                    ));
                }
            }
        }

        Ok(ControlFlow::None)
    }

    /// Импортирует .kum файл.
    fn execute_file_import(
        path: &str,
        alias: Option<&str>,
        items: Option<&[String]>,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        // Получаем импортер файлов
        let file_importer = env.file_importer().ok_or_else(|| {
            RuntimeError::new(
                "Импортер файлов не инициализирован",
                RuntimeErrorKind::Other,
            )
        })?;

        // Импортируем модуль
        let module = {
            let mut importer = file_importer.write().unwrap();
            importer.import(path, alias)?
        };

        // Регистрируем алгоритмы модуля в среде
        let module_prefix = alias.unwrap_or_else(|| module.name.as_str());

        if let Some(item_names) = items {
            // Выборочный импорт
            for item in item_names {
                if let Some(alg) = module.get_algorithm(item) {
                    // Регистрируем с префиксом модуля
                    let full_name = format!("{}.{}", module_prefix, item);
                    let mut prefixed_alg = alg.clone();
                    prefixed_alg.name = Arc::from(full_name.as_str());
                    env.define_algorithm(prefixed_alg);
                } else if let Some(class) = module.get_class(item) {
                    // Регистрируем класс
                    env.define_class(class.clone());
                } else {
                    return Err(RuntimeError::new(
                        format!("Элемент '{}' не найден в модуле '{}'", item, path),
                        RuntimeErrorKind::UndefinedAlgorithm,
                    ));
                }
            }
        } else {
            // Импортируем все публичные элементы
            for (name, alg) in module.public_algorithms() {
                let full_name = format!("{}.{}", module_prefix, name);
                let mut prefixed_alg = alg.clone();
                prefixed_alg.name = Arc::from(full_name.as_str());
                env.define_algorithm(prefixed_alg);
            }

            for (_, class) in module.public_classes() {
                env.define_class(class.clone());
            }
        }

        Ok(ControlFlow::None)
    }
}
