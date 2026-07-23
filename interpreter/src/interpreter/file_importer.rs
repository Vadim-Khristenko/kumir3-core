//! Module import system for .kum files.
//!
//! Supports:
//! - Relative paths: `"./neighbor.kum"`
//! - Library search: `"module"`
//! - Aliases: `использовать ./module как m`

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use shared::parser::parse;
use shared::types::{Algorithm, ClassDef, Program, Stmt, Value};

// =============================================================================
//                    IMPORTED MODULE (.kum FILE)
// =============================================================================

/// An imported module from a .kum file.
#[derive(Debug, Clone)]
pub struct ImportedModule {
    /// Module name (filename without extension or alias).
    pub name: String,

    /// Path to the file.
    pub path: PathBuf,

    /// Exported algorithms.
    pub algorithms: HashMap<String, Algorithm>,

    /// Exported classes.
    pub classes: HashMap<String, ClassDef>,

    /// Exported variables (global).
    pub globals: HashMap<String, Value>,

    /// Explicit export list (if specified).
    pub exports: Option<Vec<String>>,
}

impl ImportedModule {
    /// Checks if a name is exported.
    pub fn is_exported(&self, name: &str) -> bool {
        match &self.exports {
            Some(exports) => exports.iter().any(|e| e == name),
            None => true, // If no explicit export list, everything is public.
        }
    }

    /// Gets an algorithm by name (with export check).
    pub fn get_algorithm(&self, name: &str) -> Option<&Algorithm> {
        if self.is_exported(name) {
            self.algorithms.get(name)
        } else {
            None
        }
    }

    /// Gets a class by name.
    pub fn get_class(&self, name: &str) -> Option<&ClassDef> {
        if self.is_exported(name) {
            self.classes.get(name)
        } else {
            None
        }
    }

    /// Gets all public algorithms.
    pub fn public_algorithms(&self) -> impl Iterator<Item = (&String, &Algorithm)> {
        self.algorithms
            .iter()
            .filter(|(name, _)| self.is_exported(name))
    }

    /// Gets all public classes.
    pub fn public_classes(&self) -> impl Iterator<Item = (&String, &ClassDef)> {
        self.classes
            .iter()
            .filter(|(name, _)| self.is_exported(name))
    }
}

// =============================================================================
//                      FILE IMPORT MANAGER
// =============================================================================

/// Manages imports of .kum files with caching and cycle detection.
#[derive(Debug)]
pub struct FileImporter {
    /// Cache of loaded modules (path -> module).
    loaded: HashMap<PathBuf, Arc<ImportedModule>>,

    /// Module aliases (alias -> path).
    aliases: HashMap<String, PathBuf>,

    /// Base directory for relative paths.
    base_dir: PathBuf,

    /// Module search directories.
    search_paths: Vec<PathBuf>,

    /// Import stack for cycle detection.
    import_stack: Vec<PathBuf>,
}

impl FileImporter {
    /// Creates a new importer.
    pub fn new() -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Self {
            loaded: HashMap::new(),
            aliases: HashMap::new(),
            base_dir: cwd.clone(),
            search_paths: vec![cwd],
            import_stack: Vec::new(),
        }
    }

    /// Creates an importer with a base directory.
    pub fn with_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        let base = base_dir.into();
        Self {
            loaded: HashMap::new(),
            aliases: HashMap::new(),
            search_paths: vec![base.clone()],
            base_dir: base,
            import_stack: Vec::new(),
        }
    }

    /// Sets the base directory.
    pub fn set_base_dir(&mut self, dir: impl Into<PathBuf>) {
        self.base_dir = dir.into();
        if !self.search_paths.contains(&self.base_dir) {
            self.search_paths.insert(0, self.base_dir.clone());
        }
    }

    /// Base directory for resolving relative paths.
    pub fn base_dir(&self) -> &std::path::Path {
        &self.base_dir
    }

    /// Adds a module search directory.
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        let p = path.into();
        if !self.search_paths.contains(&p) {
            self.search_paths.push(p);
        }
    }

    /// Imports a module from a path.
    pub fn import(
        &mut self,
        path: &str,
        alias: Option<&str>,
    ) -> RuntimeResult<Arc<ImportedModule>> {
        let resolved = self.resolve_path(path)?;

        // Check for circular imports
        if self.import_stack.contains(&resolved) {
            return Err(RuntimeError::new(
                format!("Циклический импорт: {}", resolved.display()),
                RuntimeErrorKind::Other,
            ));
        }

        // Check cache
        if let Some(module) = self.loaded.get(&resolved) {
            // Register alias if provided
            if let Some(alias) = alias {
                self.aliases.insert(alias.to_string(), resolved.clone());
            }
            return Ok(module.clone());
        }

        // Load module
        self.import_stack.push(resolved.clone());
        let module = self.load_module(&resolved, alias)?;
        self.import_stack.pop();

        // Cache
        let module = Arc::new(module);
        self.loaded.insert(resolved.clone(), module.clone());

        // Register alias
        if let Some(alias) = alias {
            self.aliases.insert(alias.to_string(), resolved);
        }

        Ok(module)
    }

    /// Resolves an import path.
    fn resolve_path(&self, path: &str) -> RuntimeResult<PathBuf> {
        // Relative path: ./module or ../module
        if path.starts_with("./") || path.starts_with("../") {
            let resolved = self.base_dir.join(path);
            return self.ensure_kum_extension(resolved);
        }

        // Absolute path
        if Path::new(path).is_absolute() {
            return self.ensure_kum_extension(PathBuf::from(path));
        }

        // Search in directories
        for search_path in &self.search_paths {
            let candidate = search_path.join(path);
            if let Ok(resolved) = self.ensure_kum_extension(candidate.clone())
                && resolved.exists()
            {
                return Ok(resolved);
            }

            // Try as directory with index.kum
            let index = candidate.join("index.kum");
            if index.exists() {
                return Ok(index);
            }
        }

        Err(RuntimeError::new(
            format!("Модуль '{}' не найден", path),
            RuntimeErrorKind::Other,
        ))
    }

    /// Adds .kum extension if needed.
    fn ensure_kum_extension(&self, path: PathBuf) -> RuntimeResult<PathBuf> {
        if path.extension().is_some() {
            return Ok(path);
        }

        // Try with .kum
        let with_ext = path.with_extension("kum");
        if with_ext.exists() {
            return Ok(with_ext);
        }

        // Try without extension (if file exists)
        if path.exists() {
            return Ok(path);
        }

        // Return with .kum (for error messages)
        Ok(with_ext)
    }

    /// Loads a module from a file.
    fn load_module(&self, path: &PathBuf, alias: Option<&str>) -> RuntimeResult<ImportedModule> {
        let source = fs::read_to_string(path).map_err(|e| {
            RuntimeError::new(
                format!("Не удалось прочитать файл '{}': {}", path.display(), e),
                RuntimeErrorKind::Other,
            )
        })?;

        let program = parse(&source).map_err(|e| {
            RuntimeError::new(
                format!("Ошибка парсинга '{}': {:?}", path.display(), e),
                RuntimeErrorKind::Other,
            )
        })?;

        let name = alias.map(String::from).unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("модуль")
                .to_string()
        });

        let mut algorithms = HashMap::new();
        for alg in &program.algorithms {
            algorithms.insert(alg.name.to_string(), alg.clone());
        }

        // Add overloaded algorithms; take the first overload as canonical
        for ov in &program.overloaded_algorithms {
            if let Some(first) = ov.overloads.first() {
                algorithms.insert(ov.name.to_string(), first.clone());
            }
        }

        let mut classes = HashMap::new();
        for class in &program.classes {
            classes.insert(class.name.to_string(), class.clone());
        }

        let exports = self.extract_exports(&program);

        Ok(ImportedModule {
            name,
            path: path.clone(),
            algorithms,
            classes,
            globals: HashMap::new(),
            exports,
        })
    }

    /// Extracts the export list from a program.
    fn extract_exports(&self, program: &Program) -> Option<Vec<String>> {
        for stmt in &program.globals {
            if let Stmt::Export { names } = stmt {
                return Some(names.clone());
            }
        }
        None
    }

    /// Gets a loaded module by alias.
    pub fn get_module(&self, alias: &str) -> Option<Arc<ImportedModule>> {
        self.aliases
            .get(alias)
            .and_then(|path| self.loaded.get(path))
            .cloned()
    }

    /// Gets an algorithm from a module.
    pub fn get_algorithm(&self, module_alias: &str, alg_name: &str) -> Option<Algorithm> {
        self.get_module(module_alias)
            .and_then(|m| m.get_algorithm(alg_name).cloned())
    }

    /// Gets a class from a module.
    pub fn get_class(&self, module_alias: &str, class_name: &str) -> Option<ClassDef> {
        self.get_module(module_alias)
            .and_then(|m| m.get_class(class_name).cloned())
    }

    /// Checks if a module is loaded.
    pub fn is_loaded(&self, alias: &str) -> bool {
        self.aliases.contains_key(alias)
    }

    /// Returns all loaded modules.
    pub fn loaded_modules(&self) -> impl Iterator<Item = &Arc<ImportedModule>> {
        self.loaded.values()
    }

    /// Checks if a path is a .kum file or file-like reference.
    pub fn is_kum_file(path: &str) -> bool {
        path.ends_with(".kum")
            || path.starts_with("./")
            || path.starts_with("../")
            || path.contains('/')
            || path.contains('\\')
    }
}

impl Default for FileImporter {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
//                            TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_importer_creation() {
        let importer = FileImporter::new();
        assert!(importer.loaded.is_empty());
    }

    #[test]
    fn test_is_kum_file() {
        assert!(FileImporter::is_kum_file("./module.kum"));
        assert!(FileImporter::is_kum_file("../parent/module.kum"));
        assert!(FileImporter::is_kum_file("path/to/module"));
        assert!(!FileImporter::is_kum_file("time"));
        assert!(!FileImporter::is_kum_file("files"));
    }

    #[test]
    fn test_module_export_check() {
        let module = ImportedModule {
            name: "тест".to_string(),
            path: PathBuf::from("test.kum"),
            algorithms: HashMap::new(),
            classes: HashMap::new(),
            globals: HashMap::new(),
            exports: Some(vec!["публичная".to_string()]),
        };

        assert!(module.is_exported("публичная"));
        assert!(!module.is_exported("приватная"));
    }

    #[test]
    fn test_module_no_exports_all_public() {
        // Without an explicit export list, everything is public.
        let module = ImportedModule {
            name: "тест".to_string(),
            path: PathBuf::from("test.kum"),
            algorithms: HashMap::new(),
            classes: HashMap::new(),
            globals: HashMap::new(),
            exports: None,
        };

        assert!(module.is_exported("любое_имя"));
    }
}
