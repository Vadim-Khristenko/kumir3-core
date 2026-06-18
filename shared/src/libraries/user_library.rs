// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Система пользовательских библиотек
//!
//! Позволяет пользователям создавать свои библиотеки на языке Kumir 3.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::parser::parse;
use crate::types::{
    Algorithm, Program,
    config::KumirConfig,
    library::{LibFunctionDef, LibParamDef, LibraryDef},
};

/// Загрузчик пользовательских библиотек из .kum файлов.
pub struct UserLibraryLoader {
    /// Кэш загруженных библиотек
    cache: HashMap<PathBuf, LibraryDef>,
}

impl UserLibraryLoader {
    /// Создаёт новый загрузчик.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Загружает библиотеку из .kum файла.
    pub fn load_from_file(&mut self, path: &Path) -> Result<LibraryDef, String> {
        // Проверяем кэш
        if let Some(lib) = self.cache.get(path) {
            return Ok(lib.clone());
        }

        // Читаем файл
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("Не удалось прочитать файл '{}': {}", path.display(), e))?;

        // Парсим
        let program =
            parse(&source).map_err(|e| format!("Ошибка парсинга '{}': {:?}", path.display(), e))?;

        // Конвертируем в библиотеку
        let lib = self.program_to_library(program, path, None)?;

        // Кэшируем
        self.cache.insert(path.to_path_buf(), lib.clone());

        Ok(lib)
    }

    /// Загружает библиотеку из директории с kumir.toml.
    pub fn load_from_directory(&mut self, dir: &Path) -> Result<LibraryDef, String> {
        let config_path = dir.join("kumir.toml");

        if !config_path.exists() {
            return Err(format!("Файл kumir.toml не найден в '{}'", dir.display()));
        }

        // Загружаем конфигурацию
        let config = KumirConfig::load(&config_path)
            .map_err(|e| format!("Ошибка загрузки kumir.toml: {:?}", e))?;

        // Определяем главный файл библиотеки
        let main_file = dir.join(&config.build.main_file);

        if !main_file.exists() {
            return Err(format!("Главный файл '{}' не найден", main_file.display()));
        }

        // Читаем и парсим главный файл
        let source = std::fs::read_to_string(&main_file)
            .map_err(|e| format!("Не удалось прочитать файл '{}': {}", main_file.display(), e))?;

        let program = parse(&source)
            .map_err(|e| format!("Ошибка парсинга '{}': {:?}", main_file.display(), e))?;

        // Конвертируем в библиотеку с метаданными из конфига
        let lib = self.program_to_library(program, &main_file, Some(&config))?;

        // Кэшируем
        self.cache.insert(dir.to_path_buf(), lib.clone());

        Ok(lib)
    }

    /// Конвертирует программу в определение библиотеки.
    fn program_to_library(
        &self,
        program: Program,
        path: &Path,
        config: Option<&KumirConfig>,
    ) -> Result<LibraryDef, String> {
        let lib_name = if let Some(cfg) = config {
            cfg.metadata.name.clone()
        } else {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unnamed")
                .to_string()
        };

        let mut functions = Vec::new();

        // Конвертируем все алгоритмы в функции библиотеки
        for alg in &program.algorithms {
            let func = self.algorithm_to_function(alg)?;
            functions.push(func);
        }

        // Создаём определение библиотеки
        let version = if let Some(cfg) = config {
            crate::types::library::LibVersion::from_version(&cfg.metadata.version)
        } else {
            crate::types::library::LibVersion::new(1, 0, 0)
        };

        let description = if let Some(cfg) = config {
            cfg.metadata
                .description
                .as_ref()
                .map(|s| Arc::from(s.as_str()))
        } else {
            Some(Arc::from(
                format!("Пользовательская библиотека из {}", path.display()).as_str(),
            ))
        };

        let author = if let Some(cfg) = config {
            if !cfg.metadata.authors.is_empty() {
                Arc::from(cfg.metadata.authors[0].as_str())
            } else {
                Arc::from("Пользователь")
            }
        } else {
            Arc::from("Пользователь")
        };

        let dependencies = if let Some(cfg) = config {
            cfg.dependencies.values().cloned().collect()
        } else {
            Vec::new()
        };

        Ok(LibraryDef {
            id: Arc::from(lib_name.as_str()),
            name: Arc::from(lib_name.as_str()),
            aliases: Vec::new(),
            description,
            version,
            author,
            dependencies,
            functions,
            types: Vec::new(),
            classes: Vec::new(),
            constants: Vec::new(),
            kumir_version: config.and_then(|c| c.metadata.kumir_version.clone()),
            stable: true,
        })
    }

    /// Конвертирует алгоритм в функцию библиотеки.
    fn algorithm_to_function(&self, alg: &Algorithm) -> Result<LibFunctionDef, String> {
        let mut params = Vec::new();

        // Конвертируем параметры
        for param in &alg.params {
            let type_kind = param
                .type_kind
                .clone()
                .unwrap_or(crate::types::TypeKind::Any);
            params.push(LibParamDef {
                name: Arc::from(param.name.as_ref()),
                type_kind,
                mode: param.mode.clone(),
                default: None,
                description: None,
                optional: false,
            });
        }

        Ok(LibFunctionDef {
            name: Arc::from(alg.name.as_ref()),
            aliases: Vec::new(),
            description: alg.doc.as_ref().map(|s| Arc::from(s.as_ref())),
            params,
            returns: alg.return_type.clone(),
            handler: None, // Пользовательские функции не имеют нативного обработчика
            is_procedure: alg.return_type.is_none(),
            is_async: false,
            is_pure: false,
            since: None,
            deprecated: None,
            example: None,
        })
    }

    /// Очищает кэш.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for UserLibraryLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Создаёт пример пользовательской библиотеки.
pub fn create_example_library() -> String {
    r#"алг цел факториал(цел n)
| Вычисляет факториал числа
нач
    если n <= 1 то
        знач := 1
    иначе
        знач := n * факториал(n - 1)
    все
кон

алг цел фибоначчи(цел n)
| Вычисляет n-е число Фибоначчи
нач
    если n <= 1 то
        знач := n
    иначе
        знач := фибоначчи(n - 1) + фибоначчи(n - 2)
    все
кон

алг лог простое(цел n)
| Проверяет, является ли число простым
нач
    если n <= 1 то
        знач := нет
    иначе
        цел i
        i := 2
        нц пока i * i <= n
            если n % i = 0 то
                знач := нет
                выход
            все
            i := i + 1
        кц
        знач := да
    все
кон

алг цел НОД(цел a, цел b)
| Наибольший общий делитель (алгоритм Евклида)
нач
    нц пока b <> 0
        цел temp
        temp := b
        b := a % b
        a := temp
    кц
    знач := a
кон

алг цел НОК(цел a, цел b)
| Наименьшее общее кратное
нач
    знач := (a * b) / НОД(a, b)
кон
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_library_parses() {
        let source = create_example_library();
        let result = parse(&source);
        assert!(result.is_ok(), "Пример библиотеки должен парситься");
    }

    #[test]
    fn test_user_library_loader() {
        let mut loader = UserLibraryLoader::new();

        // Создаём временный файл с примером
        let temp_dir = std::env::temp_dir();
        let lib_path = temp_dir.join("test_user_lib.kum");

        let source = r#"
алг цел удвоить(цел x)
нач
    знач := x * 2
кон

алг цел утроить(цел x)
нач
    знач := x * 3
кон
"#;

        std::fs::write(&lib_path, source).unwrap();

        let result = loader.load_from_file(&lib_path);
        assert!(
            result.is_ok(),
            "Должна загрузиться пользовательская библиотека"
        );

        let lib = result.unwrap();
        assert_eq!(lib.functions.len(), 2);
        assert_eq!(lib.functions[0].name.as_ref(), "удвоить");
        assert_eq!(lib.functions[1].name.as_ref(), "утроить");

        // Очистка
        let _ = std::fs::remove_file(&lib_path);
    }
}
