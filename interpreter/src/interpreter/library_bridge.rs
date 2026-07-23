//! Bridge between the interpreter and the Kumir 3 library system.
//!
//! Integrates:
//! - Library registry from `shared/libraries`
//! - Native function handlers
//! - Dynamic loading and imports

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use shared::libraries::registry::{find_library, is_known_library};
use shared::libraries::user_library::UserLibraryLoader;
use shared::types::Value;
use shared::types::library::{LibFunctionDef, LibraryDef, NativeFn};

use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

// =============================================================================
//                        LIBRARY MANAGER
// =============================================================================

/// Manages loaded libraries for the interpreter.
pub struct LibraryManager {
    /// Loaded libraries (name -> definition).
    loaded: HashMap<String, LibraryDef>,
    /// Library aliases (alias -> real name).
    aliases: HashMap<String, String>,
    /// Library functions (full name -> native handler).
    functions: HashMap<String, NativeFn>,
    /// Function aliases (short name -> full name).
    function_aliases: HashMap<String, String>,
    /// User library loader.
    user_loader: UserLibraryLoader,
}

impl LibraryManager {
    /// Creates a new manager.
    pub fn new() -> Self {
        Self {
            loaded: HashMap::new(),
            aliases: HashMap::new(),
            functions: HashMap::new(),
            function_aliases: HashMap::new(),
            user_loader: UserLibraryLoader::new(),
        }
    }

    /// Imports a library by name or file path.
    pub fn import(&mut self, name: &str, alias: Option<&str>) -> RuntimeResult<()> {
        if self.loaded.contains_key(name) {
            return Ok(());
        }

        // If it's a file path, load as user library
        if name.ends_with(".kum") || name.contains('/') || name.contains('\\') {
            return self.import_user_library(name, alias);
        }

        let lib = find_library(name).ok_or_else(|| {
            RuntimeError::new(
                format!("Библиотека '{}' не найдена", name),
                RuntimeErrorKind::Other,
            )
        })?;

        self.load_library(lib, alias)
    }

    /// Imports a user library from a file or directory.
    pub fn import_user_library(&mut self, path: &str, alias: Option<&str>) -> RuntimeResult<()> {
        let path_obj = Path::new(path);
        let config_path = path_obj.join("kumir.toml");

        let lib = if config_path.exists() {
            // Load from directory with kumir.toml
            self.user_loader
                .load_from_directory(path_obj)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other))?
        } else {
            // Load from .kum file
            self.user_loader
                .load_from_file(path_obj)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other))?
        };

        self.load_library(lib, alias)
    }

    /// Imports a library with a version specifier.
    pub fn import_versioned(
        &mut self,
        name: &str,
        version_spec: &str,
        alias: Option<&str>,
    ) -> RuntimeResult<()> {
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

    /// Loads a library definition and registers its functions.
    fn load_library(&mut self, lib: LibraryDef, alias: Option<&str>) -> RuntimeResult<()> {
        let lib_name = lib.name.to_string();

        for func in &lib.functions {
            if let Some(handler) = &func.handler {
                let full_name = format!("{}::{}", lib_name, func.name);
                self.functions
                    .insert(full_name.clone(), Arc::clone(handler));

                self.function_aliases
                    .insert(func.name.to_string(), full_name.clone());
                for fn_alias in &func.aliases {
                    self.function_aliases
                        .insert(fn_alias.to_string(), full_name.clone());
                }
            }
        }

        for lib_alias in &lib.aliases {
            self.aliases.insert(lib_alias.to_string(), lib_name.clone());
        }

        if let Some(user_alias) = alias {
            self.aliases
                .insert(user_alias.to_string(), lib_name.clone());
        }

        self.loaded.insert(lib_name, lib);
        Ok(())
    }

    /// Calls a library function.
    ///
    /// Returns `Ok(None)` to indicate "function not found" — the caller continues search.
    pub fn call_function(&self, name: &str, args: &[Value]) -> RuntimeResult<Option<Value>> {
        let Some(full_name) = self.resolve_function_name(name) else {
            return Ok(None);
        };
        let Some(handler) = self.functions.get(&full_name) else {
            return Ok(None);
        };

        // Argument count is checked here, not in each handler. Otherwise, an extra
        // argument is silently ignored and a missing one yields "parameter not passed"
        // without naming the function or its expected count.
        if let Some(def) = self.get_function_def(name) {
            Self::check_arity(&full_name, def, args.len())?;
        }

        handler(args)
            .map(Some)
            .map_err(|message| Self::library_error(&full_name, message))
    }

    /// Resolves the full name `library::function` for a function name or alias.
    fn resolve_function_name(&self, name: &str) -> Option<String> {
        if self.functions.contains_key(name) {
            return Some(name.to_string());
        }
        self.function_aliases.get(name).cloned()
    }

    /// Checks the argument count against declared parameters.
    fn check_arity(full_name: &str, def: &LibFunctionDef, given: usize) -> RuntimeResult<()> {
        let required = def
            .params
            .iter()
            .filter(|p| !p.optional && p.default.is_none())
            .count();
        let total = def.params.len();

        if given < required || given > total {
            let expected = if given < required { required } else { total };
            return Err(RuntimeError::argument_count(full_name, expected, given));
        }
        Ok(())
    }

    /// Converts a handler message into a runtime error.
    ///
    /// Library handlers return a string; the error kind is inferred from its text.
    /// The error phrases are defined by the libraries themselves and listed here in full.
    /// Without this, any library error would arrive as `Other`, and neither exception
    /// handling nor the test corpus could distinguish a type error from an I/O error
    /// (KITE 14 § 3.2).
    fn library_error(full_name: &str, message: String) -> RuntimeError {
        let kind = if message.starts_with("Ожидается") || message.starts_with("Ожидался")
        {
            RuntimeErrorKind::TypeMismatch
        } else if message.starts_with("Не передан параметр") {
            RuntimeErrorKind::ArgumentCount
        } else {
            RuntimeErrorKind::Other
        };

        let message = if message.contains(full_name) {
            message
        } else {
            format!("{full_name}: {message}")
        };
        RuntimeError::new(message, kind)
    }

    /// Checks if a name is a library function.
    pub fn is_library_function(&self, name: &str) -> bool {
        self.functions.contains_key(name) || self.function_aliases.contains_key(name)
    }

    /// Gets the definition of a function.
    pub fn get_function_def(&self, name: &str) -> Option<&LibFunctionDef> {
        for lib in self.loaded.values() {
            for func in &lib.functions {
                if func.name.as_ref() == name || func.aliases.iter().any(|a| a.as_ref() == name) {
                    return Some(func);
                }
            }
        }
        None
    }

    /// Gets a constant from a library.
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

    /// Gets the list of all loaded libraries.
    pub fn loaded_libraries(&self) -> Vec<&str> {
        self.loaded.keys().map(|s| s.as_str()).collect()
    }

    /// Gets information about a loaded library.
    pub fn get_library_info(&self, name: &str) -> Option<&LibraryDef> {
        let real_name = self.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);
        self.loaded.get(real_name)
    }

    /// Gets the list of all available libraries (not necessarily loaded).
    pub fn list_available_libraries() -> Vec<String> {
        shared::libraries::registry::list_available()
    }

    /// Gets all available versions of a library.
    pub fn get_available_versions(name: &str) -> Vec<shared::types::version::Version> {
        shared::libraries::registry::get_library_versions(name)
    }

    /// Unloads a library.
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

        let lib = self.loaded.get(&real_name).unwrap();
        for func in &lib.functions {
            let full_name = format!("{}::{}", real_name, func.name);
            self.functions.remove(&full_name);
            self.function_aliases.remove(func.name.as_ref());
            for alias in &func.aliases {
                self.function_aliases.remove(alias.as_ref());
            }
        }

        self.aliases.retain(|_, v| v != &real_name);
        self.loaded.remove(&real_name);

        Ok(())
    }

    /// Checks if a library is loaded.
    pub fn is_loaded(&self, name: &str) -> bool {
        self.loaded.contains_key(name)
            || self
                .aliases
                .get(name)
                .map(|n| self.loaded.contains_key(n))
                .unwrap_or(false)
    }

    /// Calls a library function by qualified name (Library.function).
    pub fn call_qualified_function(
        &self,
        lib_name: &str,
        func_name: &str,
        args: &[Value],
    ) -> RuntimeResult<Option<Value>> {
        let real_lib_name = self
            .aliases
            .get(lib_name)
            .map(|s| s.as_str())
            .unwrap_or(lib_name);

        let lib = self.loaded.get(real_lib_name).ok_or_else(|| {
            RuntimeError::new(
                format!(
                    "Библиотека '{}' не загружена. Используйте: использовать {}",
                    lib_name, lib_name
                ),
                RuntimeErrorKind::Other,
            )
        })?;

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

        Err(RuntimeError::new(
            format!(
                "Функция '{}' не найдена в библиотеке '{}'",
                func_name, lib_name
            ),
            RuntimeErrorKind::UndefinedAlgorithm,
        ))
    }

    /// Resolves the real library name from an alias.
    pub fn resolve_library_name<'a>(&'a self, name: &'a str) -> Option<&'a str> {
        if self.loaded.contains_key(name) {
            Some(name)
        } else {
            self.aliases.get(name).map(|s| s.as_str())
        }
    }
}

// =============================================================================
//                          UTILITIES
// =============================================================================

/// Resolves an import path to a library name.
pub fn resolve_import_path(path: &str) -> Option<String> {
    let clean_path = path.trim_end_matches(".kum").trim_matches('"');

    let lib_name = if clean_path.contains('/') {
        clean_path.split('/').next()?
    } else {
        clean_path
    };

    if is_known_library(lib_name) {
        return Some(lib_name.to_string());
    }

    if let Some(lib) = find_library(lib_name) {
        return Some(lib.name.to_string());
    }

    None
}

/// Activates a project environment.
pub fn activate_environment(project_root: &str) -> RuntimeResult<()> {
    shared::libraries::registry::activate_project(project_root).map_err(|e| {
        RuntimeError::new(
            format!("Не удалось активировать окружение: {}", e),
            RuntimeErrorKind::Other,
        )
    })
}

// =============================================================================
//                            TESTS
// =============================================================================

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
        if is_known_library("time") {
            assert!(manager.import("time", None).is_ok());
            assert!(manager.is_loaded("time"));
        }
    }

    #[test]
    fn test_resolve_import_path() {
        if is_known_library("time") {
            assert_eq!(resolve_import_path("time"), Some("time".to_string()));
        }
    }

    #[test]
    fn test_function_lookup() {
        let mut manager = LibraryManager::new();
        if is_known_library("time") && manager.import("time", None).is_ok() {
            assert!(
                manager.is_library_function("время_мс") || manager.is_library_function("now_ms")
            );
        }
    }
}
