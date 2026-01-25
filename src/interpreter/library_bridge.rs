//! Мост между интерпретатором и системой библиотек Kumir 3
//!
//! Этот модуль интегрирует:
//! - Реестр библиотек из `shared/libraries`
//! - Нативные обработчики функций библиотек
//! - Динамическую загрузку и импорт библиотек

use std::collections::HashMap;
use std::sync::Arc;

use crate::shared::libraries::registry::{
    find_library, find_library_matching, is_known_library, all_libraries,
    activate_project_environment, register_library,
};
use crate::shared::types::library::{LibraryDef, LibFunctionDef, NativeFn};
use crate::shared::types::version::VersionSpec;
use crate::shared::types::Value;

use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::environment::Environment;

// ============================================================================
//                    МЕНЕДЖЕР БИБЛИОТЕК
// ============================================================================

/// Менеджер загруженных библиотек для интерпретатора.
#[derive(Default)]
pub struct LibraryManager {
    /// Загруженные библиотеки (имя -> определение)
    loaded: HashMap<String, LibraryDef>,
    /// Алиасы библиотек (алиас -> реальное имя)
    aliases: HashMap<String, String>,
    /// Функции библиотек (полное имя -> нативный обработчик)
    functions: HashMap<String, NativeFn>,
    /// Функции с алиасами (короткое имя -> полное имя)
    function_aliases: HashMap<String, String>,
}

impl LibraryManager {
    /// Создаёт новый менеджер.
    pub fn new() -> Self {
        Self::default()
    }

    /// Импортирует библиотеку по имени.
    pub fn import(&mut self, name: &str, alias: Option<&str>) -> RuntimeResult<()> {
        // Проверяем, не загружена ли уже
        if self.loaded.contains_key(name) {
            return Ok(());
        }

        // Пробуем найти в реестре
        let lib = find_library(name).ok_or_else(|| {
            RuntimeError::new(
                format!("Библиотека '{}' не найдена", name),
                RuntimeErrorKind::Other,
            )
        })?;

        self.load_library(lib, alias)
    }

    /// Импортирует библиотеку с версией.
    pub fn import_versioned(
        &mut self,
        name: &str,
        version_spec: &str,
        alias: Option<&str>,
    ) -> RuntimeResult<()> {
        let spec: VersionSpec = version_spec.parse().map_err(|e| {
            RuntimeError::new(
                format!("Некорректная спецификация версии: {:?}", e),
                RuntimeErrorKind::Other,
            )
        })?;

        let lib = find_library_matching(name, &spec).ok_or_else(|| {
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
                self.functions.insert(full_name.clone(), Arc::clone(handler));

                // Регистрируем алиасы функций
                self.function_aliases
                    .insert(func.name.to_string(), full_name.clone());
                for &fn_alias in func.aliases {
                    self.function_aliases
                        .insert(fn_alias.to_string(), full_name.clone());
                }
            }
        }

        // Регистрируем алиасы библиотеки
        for &lib_alias in lib.aliases {
            self.aliases.insert(lib_alias.to_string(), lib_name.clone());
        }

        // Дополнительный алиас от пользователя
        if let Some(user_alias) = alias {
            self.aliases.insert(user_alias.to_string(), lib_name.clone());
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
        if let Some(full_name) = self.function_aliases.get(name) {
            if let Some(handler) = self.functions.get(full_name) {
                return handler(args)
                    .map(Some)
                    .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other));
            }
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
                if func.name == name || func.aliases.contains(&name) {
                    return Some(func);
                }
            }
        }
        None
    }

    /// Получает константу из библиотеки.
    pub fn get_constant(&self, lib_name: &str, const_name: &str) -> Option<Value> {
        let real_name = self.aliases.get(lib_name).map(|s| s.as_str()).unwrap_or(lib_name);
        
        if let Some(lib) = self.loaded.get(real_name) {
            for constant in &lib.constants {
                if constant.name == const_name || constant.aliases.contains(&const_name) {
                    return Some(constant.value.clone());
                }
            }
        }
        None
    }

    /// Получает список загруженных библиотек.
    pub fn loaded_libraries(&self) -> Vec<&str> {
        self.loaded.keys().map(|s| s.as_str()).collect()
    }

    /// Проверяет, загружена ли библиотека.
    pub fn is_loaded(&self, name: &str) -> bool {
        self.loaded.contains_key(name)
            || self.aliases.get(name).map(|n| self.loaded.contains_key(n)).unwrap_or(false)
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
        let real_lib_name = self.aliases.get(lib_name)
            .map(|s| s.as_str())
            .unwrap_or(lib_name);
        
        // Проверяем, загружена ли библиотека
        let lib = self.loaded.get(real_lib_name).ok_or_else(|| {
            RuntimeError::new(
                format!("Библиотека '{}' не загружена. Используйте: использовать {}", lib_name, lib_name),
                RuntimeErrorKind::Other,
            )
        })?;
        
        // Ищем функцию в библиотеке
        for func in &lib.functions {
            if func.name == func_name || func.aliases.contains(&func_name) {
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
            format!("Функция '{}' не найдена в библиотеке '{}'", func_name, lib_name),
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

    // Проверяем по русскому алиасу
    for lib in all_libraries() {
        if lib.aliases.contains(&lib_name) {
            return Some(lib.name.to_string());
        }
    }

    None
}

/// Активирует окружение проекта.
pub fn activate_environment(project_root: &str) -> RuntimeResult<()> {
    activate_project_environment(project_root).map_err(|e| {
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
            assert!(manager.is_library_function("время_мс") || manager.is_library_function("now_ms"));
        }
    }
}
