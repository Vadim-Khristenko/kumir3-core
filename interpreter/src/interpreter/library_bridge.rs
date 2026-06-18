//! Мост между интерпретатором и системой библиотек Kumir 3
//!
//! Этот модуль интегрирует:
//! - Реестр библиотек из `shared/libraries`
//! - Нативные обработчики функций библиотек
//! - Динамическую загрузку и импорт библиотек

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use shared::libraries::registry::{find_library, is_known_library};
use shared::libraries::user_library::UserLibraryLoader;
use shared::types::Value;
use shared::types::library::{LibFunctionDef, LibraryDef, NativeFn};

use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

// ============================================================================
//                    МЕНЕДЖЕР БИБЛИОТЕК
// ============================================================================

/// Менеджер загруженных библиотек для интерпретатора.
pub struct LibraryManager {
    /// Загруженные библиотеки (имя -> определение)
    loaded: HashMap<String, LibraryDef>,
    /// Алиасы библиотек (алиас -> реальное имя)
    aliases: HashMap<String, String>,
    /// Функции библиотек (полное имя -> нативный обработчик)
    functions: HashMap<String, NativeFn>,
    /// Функции с алиасами (короткое имя -> полное имя)
    function_aliases: HashMap<String, String>,
    /// Загрузчик пользовательских библиотек
    user_loader: UserLibraryLoader,
}

impl LibraryManager {
    /// Создаёт новый менеджер.
    pub fn new() -> Self {
        Self {
            loaded: HashMap::new(),
            aliases: HashMap::new(),
            functions: HashMap::new(),
            function_aliases: HashMap::new(),
            user_loader: UserLibraryLoader::new(),
        }
    }

    /// Импортирует библиотеку по имени или пути к файлу.
    pub fn import(&mut self, name: &str, alias: Option<&str>) -> RuntimeResult<()> {
        // Проверяем, не загружена ли уже
        if self.loaded.contains_key(name) {
            return Ok(());
        }

        // Если это путь к файлу, загружаем как пользовательскую библиотеку
        if name.ends_with(".kum") || name.contains('/') || name.contains('\\') {
            return self.import_user_library(name, alias);
        }

        // Пробуем найти в реестре встроенных библиотек
        let lib = find_library(name).ok_or_else(|| {
            RuntimeError::new(
                format!("Библиотека '{}' не найдена", name),
                RuntimeErrorKind::Other,
            )
        })?;

        self.load_library(lib, alias)
    }

    /// Импортирует пользовательскую библиотеку из файла или директории.
    pub fn import_user_library(&mut self, path: &str, alias: Option<&str>) -> RuntimeResult<()> {
        let path_obj = Path::new(path);

        // Проверяем наличие kumir.toml для определения типа библиотеки
        let config_path = path_obj.join("kumir.toml");

        let lib = if config_path.exists() {
            // Загружаем из директории с kumir.toml
            self.user_loader
                .load_from_directory(path_obj)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other))?
        } else {
            // Загружаем из .kum файла
            self.user_loader
                .load_from_file(path_obj)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other))?
        };

        self.load_library(lib, alias)
    }

    /// Импортирует библиотеку с версией.
    pub fn import_versioned(
        &mut self,
        name: &str,
        version_spec: &str,
        alias: Option<&str>,
    ) -> RuntimeResult<()> {
        // Проверяем, не загружена ли уже
        if self.loaded.contains_key(name) {
            return Ok(());
        }

        let lib = shared::libraries::registry::find_library_with_version(name, version_spec)
            .ok_or_else(|| {
                RuntimeError::new(
                    format!("Библиотека '{}' версии {} не найдена", name, version_spec),
                    RuntimeErrorKind::Other,
                )
            })?;

        self.load_library(lib, alias)
    }

    /// Загружает определение библиотеки.
    fn load_library(&mut self, lib: LibraryDef, alias: Option<&str>) -> RuntimeResult<()> {
        let lib_name = lib.name.to_string();

        // Регистрируем функции библиотеки
        for func in &lib.functions {
            if let Some(handler) = &func.handler {
                // Полное имя: библиотека::функция
                let full_name = format!("{}::{}", lib_name, func.name);
                self.functions
                    .insert(full_name.clone(), Arc::clone(handler));

                // Регистрируем алиасы функций
                self.function_aliases
                    .insert(func.name.to_string(), full_name.clone());
                for fn_alias in &func.aliases {
                    self.function_aliases
                        .insert(fn_alias.to_string(), full_name.clone());
                }
            }
        }

        // Регистрируем алиасы библиотеки
        for lib_alias in &lib.aliases {
            self.aliases.insert(lib_alias.to_string(), lib_name.clone());
        }

        // Дополнительный алиас от пользователя
        if let Some(user_alias) = alias {
            self.aliases
                .insert(user_alias.to_string(), lib_name.clone());
        }

        self.loaded.insert(lib_name, lib);
        Ok(())
    }

    /// Вызывает функцию библиотеки.
    pub fn call_function(&self, name: &str, args: &[Value]) -> RuntimeResult<Option<Value>> {
        // Сначала ищем по полному имени
        if let Some(handler) = self.functions.get(name) {
            return handler(args)
                .map(Some)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other));
        }

        // Затем по алиасу
        if let Some(full_name) = self.function_aliases.get(name)
            && let Some(handler) = self.functions.get(full_name)
        {
            return handler(args)
                .map(Some)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other));
        }

        // Не найдена
        Ok(None)
    }

    /// Проверяет, является ли имя функцией библиотеки.
    pub fn is_library_function(&self, name: &str) -> bool {
        self.functions.contains_key(name) || self.function_aliases.contains_key(name)
    }

    /// Получает определение функции.
    pub fn get_function_def(&self, name: &str) -> Option<&LibFunctionDef> {
        // Находим библиотеку и функцию
        for lib in self.loaded.values() {
            for func in &lib.functions {
                if func.name.as_ref() == name || func.aliases.iter().any(|a| a.as_ref() == name) {
                    return Some(func);
                }
            }
        }
        None
    }

    /// Получает константу из библиотеки.
    pub fn get_constant(&self, lib_name: &str, const_name: &str) -> Option<Value> {
        let real_name = self
            .aliases
            .get(lib_name)
            .map(|s| s.as_str())
            .unwrap_or(lib_name);

        if let Some(lib) = self.loaded.get(real_name) {
            for constant in &lib.constants {
                if constant.name.as_ref() == const_name
                    || constant.aliases.iter().any(|a| a.as_ref() == const_name)
                {
                    return Some(constant.value.clone());
                }
            }
        }
        None
    }

    /// Получает список всех загруженных библиотек.
    pub fn loaded_libraries(&self) -> Vec<&str> {
        self.loaded.keys().map(|s| s.as_str()).collect()
    }

    /// Получает информацию о загруженной библиотеке.
    pub fn get_library_info(&self, name: &str) -> Option<&LibraryDef> {
        let real_name = self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);
        self.loaded.get(real_name)
    }

    /// Получает список всех доступных библиотек (не обязательно загруженных).
    pub fn list_available_libraries() -> Vec<String> {
        shared::libraries::registry::list_available()
    }

    /// Получает список всех доступных версий библиотеки.
    pub fn get_available_versions(name: &str) -> Vec<shared::types::version::Version> {
        shared::libraries::registry::get_library_versions(name)
    }

    /// Выгружает библиотеку.
    pub fn unload(&mut self, name: &str) -> RuntimeResult<()> {
        let real_name = self
            .aliases
            .get(name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| name.to_string());

        if !self.loaded.contains_key(&real_name) {
            return Err(RuntimeError::new(
                format!("Библиотека '{}' не загружена", name),
                RuntimeErrorKind::Other,
            ));
        }

        // Удаляем функции библиотеки
        let lib = self.loaded.get(&real_name).unwrap();
        for func in &lib.functions {
            let full_name = format!("{}::{}", real_name, func.name);
            self.functions.remove(&full_name);
            self.function_aliases.remove(func.name.as_ref());
            for alias in &func.aliases {
                self.function_aliases.remove(alias.as_ref());
            }
        }

        // Удаляем алиасы библиотеки
        self.aliases.retain(|_, v| v != &real_name);

        // Удаляем саму библиотеку
        self.loaded.remove(&real_name);

        Ok(())
    }

    /// Проверяет, загружена ли библиотека.
    pub fn is_loaded(&self, name: &str) -> bool {
        self.loaded.contains_key(name)
            || self
                .aliases
                .get(name)
                .map(|n| self.loaded.contains_key(n))
                .unwrap_or(false)
    }

    /// Вызывает функцию библиотеки по квалифицированному имени (Библиотека.функция).
    ///
    /// # Пример
    /// ```
    /// // Сеть.http_получить("https://example.com")
    /// manager.call_qualified_function("Сеть", "http_получить", &args)
    /// ```
    pub fn call_qualified_function(
        &self,
        lib_name: &str,
        func_name: &str,
        args: &[Value],
    ) -> RuntimeResult<Option<Value>> {
        // Резолвим алиас библиотеки
        let real_lib_name = self
            .aliases
            .get(lib_name)
            .map(|s| s.as_str())
            .unwrap_or(lib_name);

        // Проверяем, загружена ли библиотека
        let lib = self.loaded.get(real_lib_name).ok_or_else(|| {
            RuntimeError::new(
                format!(
                    "Библиотека '{}' не загружена. Используйте: использовать {}",
                    lib_name, lib_name
                ),
                RuntimeErrorKind::Other,
            )
        })?;

        // Ищем функцию в библиотеке
        for func in &lib.functions {
            if func.name.as_ref() == func_name
                || func.aliases.iter().any(|a| a.as_ref() == func_name)
            {
                if let Some(handler) = &func.handler {
                    return handler(args)
                        .map(Some)
                        .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other));
                } else {
                    return Err(RuntimeError::new(
                        format!("Функция '{}.{}' не имеет реализации", lib_name, func_name),
                        RuntimeErrorKind::Other,
                    ));
                }
            }
        }

        // Функция не найдена
        Err(RuntimeError::new(
            format!(
                "Функция '{}' не найдена в библиотеке '{}'",
                func_name, lib_name
            ),
            RuntimeErrorKind::UndefinedAlgorithm,
        ))
    }

    /// Получает реальное имя библиотеки по алиасу.
    pub fn resolve_library_name<'a>(&'a self, name: &'a str) -> Option<&'a str> {
        if self.loaded.contains_key(name) {
            Some(name)
        } else {
            self.aliases.get(name).map(|s| s.as_str())
        }
    }
}

// ============================================================================
//                    УТИЛИТЫ
// ============================================================================

/// Преобразует путь импорта в имя библиотеки.
///
/// Поддерживает:
/// - "time" -> "time"
/// - "время" -> "time"
/// - "net/http" -> "net"
/// - "файлы" -> "files"
pub fn resolve_import_path(path: &str) -> Option<String> {
    // Убираем .kum расширение если есть
    let clean_path = path.trim_end_matches(".kum").trim_matches('"');

    // Если это путь с /, берём первую часть
    let lib_name = if clean_path.contains('/') {
        clean_path.split('/').next()?
    } else {
        clean_path
    };

    // Проверяем, есть ли такая библиотека
    if is_known_library(lib_name) {
        return Some(lib_name.to_string());
    }

    // Проверяем через find_library
    if let Some(lib) = find_library(lib_name) {
        return Some(lib.name.to_string());
    }

    None
}

/// Активирует окружение проекта.
pub fn activate_environment(project_root: &str) -> RuntimeResult<()> {
    shared::libraries::registry::activate_project(project_root).map_err(|e| {
        RuntimeError::new(
            format!("Не удалось активировать окружение: {}", e),
            RuntimeErrorKind::Other,
        )
    })
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_manager_creation() {
        let manager = LibraryManager::new();
        assert!(manager.loaded_libraries().is_empty());
    }

    #[test]
    fn test_import_time_library() {
        let mut manager = LibraryManager::new();
        // Библиотека time должна быть зарегистрирована
        if is_known_library("time") {
            assert!(manager.import("time", None).is_ok());
            assert!(manager.is_loaded("time"));
        }
    }

    #[test]
    fn test_resolve_import_path() {
        // Эти тесты зависят от зарегистрированных библиотек
        if is_known_library("time") {
            assert_eq!(resolve_import_path("time"), Some("time".to_string()));
        }
    }

    #[test]
    fn test_function_lookup() {
        let mut manager = LibraryManager::new();
        if is_known_library("time") && manager.import("time", None).is_ok() {
            // Проверяем, что функции доступны
            assert!(
                manager.is_library_function("время_мс") || manager.is_library_function("now_ms")
            );
        }
    }
}
