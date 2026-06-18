//! Импорт .kum файлов
//!
//! Поддерживает:
//! - `подключить "./соседний_файл.kum"` — относительный путь
//! - `подключить "модуль"` — поиск в директории модулей
//! - `использовать ./модуль` — альтернативный синтаксис
//!
//! Пример:
//! ```kumir
//! | main.kum
//! подключить "./математика.kum" как мат
//!
//! алг Тест
//! нач
//!     вывод мат.квадрат(5)
//! кон
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use shared::parser::parse;
use shared::types::{Algorithm, ClassDef, Program, Stmt, Value};

// ============================================================================
//                         МОДУЛЬ (ИМПОРТИРОВАННЫЙ ФАЙЛ)
// ============================================================================

/// Импортированный модуль из .kum файла
#[derive(Debug, Clone)]
pub struct ImportedModule {
    /// Имя модуля (имя файла без расширения или alias)
    pub name: String,

    /// Путь к файлу
    pub path: PathBuf,

    /// Экспортированные алгоритмы
    pub algorithms: HashMap<String, Algorithm>,

    /// Экспортированные классы
    pub classes: HashMap<String, ClassDef>,

    /// Экспортированные переменные (глобальные)
    pub globals: HashMap<String, Value>,

    /// Список явно экспортированных имён (если указано)
    pub exports: Option<Vec<String>>,
}

impl ImportedModule {
    /// Проверяет, экспортирован ли элемент
    pub fn is_exported(&self, name: &str) -> bool {
        match &self.exports {
            Some(exports) => exports.iter().any(|e| e == name),
            None => true, // Если нет явного экспорта — всё публично
        }
    }

    /// Получает алгоритм по имени (с проверкой экспорта)
    pub fn get_algorithm(&self, name: &str) -> Option<&Algorithm> {
        if self.is_exported(name) {
            self.algorithms.get(name)
        } else {
            None
        }
    }

    /// Получает класс по имени
    pub fn get_class(&self, name: &str) -> Option<&ClassDef> {
        if self.is_exported(name) {
            self.classes.get(name)
        } else {
            None
        }
    }

    /// Получает все публичные алгоритмы
    pub fn public_algorithms(&self) -> impl Iterator<Item = (&String, &Algorithm)> {
        self.algorithms
            .iter()
            .filter(|(name, _)| self.is_exported(name))
    }

    /// Получает все публичные классы
    pub fn public_classes(&self) -> impl Iterator<Item = (&String, &ClassDef)> {
        self.classes
            .iter()
            .filter(|(name, _)| self.is_exported(name))
    }
}

// ============================================================================
//                         МЕНЕДЖЕР ИМПОРТОВ ФАЙЛОВ
// ============================================================================

/// Менеджер импорта .kum файлов
#[derive(Debug)]
pub struct FileImporter {
    /// Кеш загруженных модулей (путь -> модуль)
    loaded: HashMap<PathBuf, Arc<ImportedModule>>,

    /// Алиасы модулей (alias -> путь)
    aliases: HashMap<String, PathBuf>,

    /// Текущая рабочая директория (для относительных путей)
    base_dir: PathBuf,

    /// Директории поиска модулей
    search_paths: Vec<PathBuf>,

    /// Стек импортов (для обнаружения циклов)
    import_stack: Vec<PathBuf>,
}

impl FileImporter {
    /// Создаёт новый импортер
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

    /// Создаёт импортер с базовой директорией
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

    /// Устанавливает базовую директорию
    pub fn set_base_dir(&mut self, dir: impl Into<PathBuf>) {
        self.base_dir = dir.into();
        if !self.search_paths.contains(&self.base_dir) {
            self.search_paths.insert(0, self.base_dir.clone());
        }
    }

    /// Базовая директория для разрешения относительных путей.
    pub fn base_dir(&self) -> &std::path::Path {
        &self.base_dir
    }

    /// Добавляет директорию поиска
    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        let p = path.into();
        if !self.search_paths.contains(&p) {
            self.search_paths.push(p);
        }
    }

    /// Импортирует файл по пути
    pub fn import(
        &mut self,
        path: &str,
        alias: Option<&str>,
    ) -> RuntimeResult<Arc<ImportedModule>> {
        let resolved = self.resolve_path(path)?;

        // Проверяем циклический импорт
        if self.import_stack.contains(&resolved) {
            return Err(RuntimeError::new(
                format!("Циклический импорт: {}", resolved.display()),
                RuntimeErrorKind::Other,
            ));
        }

        // Проверяем кеш
        if let Some(module) = self.loaded.get(&resolved) {
            // Если есть alias — регистрируем
            if let Some(alias) = alias {
                self.aliases.insert(alias.to_string(), resolved.clone());
            }
            return Ok(module.clone());
        }

        // Загружаем файл
        self.import_stack.push(resolved.clone());
        let module = self.load_module(&resolved, alias)?;
        self.import_stack.pop();

        // Кешируем
        let module = Arc::new(module);
        self.loaded.insert(resolved.clone(), module.clone());

        // Регистрируем alias
        if let Some(alias) = alias {
            self.aliases.insert(alias.to_string(), resolved);
        }

        Ok(module)
    }

    /// Разрешает путь импорта
    fn resolve_path(&self, path: &str) -> RuntimeResult<PathBuf> {
        // Относительный путь: ./module или ../module
        if path.starts_with("./") || path.starts_with("../") {
            let resolved = self.base_dir.join(path);
            return self.ensure_kum_extension(resolved);
        }

        // Абсолютный путь
        if Path::new(path).is_absolute() {
            return self.ensure_kum_extension(PathBuf::from(path));
        }

        // Поиск в директориях
        for search_path in &self.search_paths {
            let candidate = search_path.join(path);
            if let Ok(resolved) = self.ensure_kum_extension(candidate.clone())
                && resolved.exists()
            {
                return Ok(resolved);
            }

            // Попробуем как директорию с index.kum
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

    /// Добавляет расширение .kum если нужно
    fn ensure_kum_extension(&self, path: PathBuf) -> RuntimeResult<PathBuf> {
        if path.extension().is_some() {
            return Ok(path);
        }

        // Пробуем с .kum
        let with_ext = path.with_extension("kum");
        if with_ext.exists() {
            return Ok(with_ext);
        }

        // Пробуем без расширения (если файл существует)
        if path.exists() {
            return Ok(path);
        }

        // Возвращаем с .kum (для сообщения об ошибке)
        Ok(with_ext)
    }

    /// Загружает модуль из файла
    fn load_module(&self, path: &PathBuf, alias: Option<&str>) -> RuntimeResult<ImportedModule> {
        // Читаем файл
        let source = fs::read_to_string(path).map_err(|e| {
            RuntimeError::new(
                format!("Не удалось прочитать файл '{}': {}", path.display(), e),
                RuntimeErrorKind::Other,
            )
        })?;

        // Парсим
        let program = parse(&source).map_err(|e| {
            RuntimeError::new(
                format!("Ошибка парсинга '{}': {:?}", path.display(), e),
                RuntimeErrorKind::Other,
            )
        })?;

        // Определяем имя модуля
        let name = alias.map(String::from).unwrap_or_else(|| {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("модуль")
                .to_string()
        });

        // Собираем алгоритмы
        let mut algorithms = HashMap::new();
        for alg in &program.algorithms {
            algorithms.insert(alg.name.to_string(), alg.clone());
        }

        // Добавляем overloaded алгоритмы
        for ov in &program.overloaded_algorithms {
            // Берём первую реализацию как основную
            if let Some(first) = ov.overloads.first() {
                algorithms.insert(ov.name.to_string(), first.clone());
            }
        }

        // Собираем классы
        let mut classes = HashMap::new();
        for class in &program.classes {
            classes.insert(class.name.to_string(), class.clone());
        }

        // Собираем экспорты
        let exports = self.extract_exports(&program);

        Ok(ImportedModule {
            name,
            path: path.clone(),
            algorithms,
            classes,
            globals: HashMap::new(), // TODO: выполнить глобальные инициализации
            exports,
        })
    }

    /// Извлекает список экспортов из программы
    fn extract_exports(&self, program: &Program) -> Option<Vec<String>> {
        // Ищем Stmt::Export в глобальных инструкциях
        for stmt in &program.globals {
            if let Stmt::Export { names } = stmt {
                return Some(names.clone());
            }
        }
        None
    }

    /// Получает загруженный модуль по alias
    pub fn get_module(&self, alias: &str) -> Option<Arc<ImportedModule>> {
        self.aliases
            .get(alias)
            .and_then(|path| self.loaded.get(path))
            .cloned()
    }

    /// Получает алгоритм из модуля
    pub fn get_algorithm(&self, module_alias: &str, alg_name: &str) -> Option<Algorithm> {
        self.get_module(module_alias)
            .and_then(|m| m.get_algorithm(alg_name).cloned())
    }

    /// Получает класс из модуля
    pub fn get_class(&self, module_alias: &str, class_name: &str) -> Option<ClassDef> {
        self.get_module(module_alias)
            .and_then(|m| m.get_class(class_name).cloned())
    }

    /// Проверяет, загружен ли модуль
    pub fn is_loaded(&self, alias: &str) -> bool {
        self.aliases.contains_key(alias)
    }

    /// Возвращает список всех загруженных модулей
    pub fn loaded_modules(&self) -> impl Iterator<Item = &Arc<ImportedModule>> {
        self.loaded.values()
    }

    /// Проверяет, является ли путь файлом .kum
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

// ============================================================================
//                         ТЕСТЫ
// ============================================================================

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
        assert!(!FileImporter::is_kum_file("time")); // библиотека
        assert!(!FileImporter::is_kum_file("files")); // библиотека
    }

    #[test]
    fn test_module_export_check() {
        // Тестируем логику экспорта без файловой системы
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
        // Если нет явного экспорта - всё публично
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
