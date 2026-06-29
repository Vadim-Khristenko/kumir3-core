//! Операции с директориями

use std::sync::Arc;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::value::{TypeKind, Value};

/// список_файлов(путь) → [лит]
pub fn list_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("список_файлов")
        .with_aliases(vec![
            Arc::from("list_dir"),
            Arc::from("readdir"),
            Arc::from("содержимое_директории"),
        ])
        .with_description("Возвращает список имён файлов и подпапок в директории")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Array(Box::new(TypeKind::String)))
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let mut entries = Vec::new();
            let dir = std::fs::read_dir(path.as_str())
                .map_err(|e| format!("Ошибка чтения директории '{}': {}", path, e))?;
            for entry in dir {
                let entry = entry.map_err(|e| format!("Ошибка чтения элемента: {}", e))?;
                let name = entry.file_name().to_string_lossy().into_owned();
                entries.push(Value::String(name));
            }
            Ok(Value::Array(entries))
        })
}

/// только_файлы(путь) → [лит]
pub fn list_files_fn() -> LibFunctionDef {
    LibFunctionDef::new("только_файлы")
        .with_aliases(vec![Arc::from("list_files"), Arc::from("файлы")])
        .with_description("Возвращает список только файлов (без подпапок) в директории")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Array(Box::new(TypeKind::String)))
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let mut entries = Vec::new();
            let dir = std::fs::read_dir(path.as_str())
                .map_err(|e| format!("Ошибка чтения директории '{}': {}", path, e))?;
            for entry in dir {
                let entry = entry.map_err(|e| format!("Ошибка чтения элемента: {}", e))?;
                if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    entries.push(Value::String(name));
                }
            }
            Ok(Value::Array(entries))
        })
}

/// только_директории(путь) → [лит]
pub fn list_dirs_fn() -> LibFunctionDef {
    LibFunctionDef::new("только_директории")
        .with_aliases(vec![Arc::from("list_dirs"), Arc::from("директории")])
        .with_description("Возвращает список только поддиректорий")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Array(Box::new(TypeKind::String)))
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let mut entries = Vec::new();
            let dir = std::fs::read_dir(path.as_str())
                .map_err(|e| format!("Ошибка чтения директории '{}': {}", path, e))?;
            for entry in dir {
                let entry = entry.map_err(|e| format!("Ошибка чтения элемента: {}", e))?;
                if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    entries.push(Value::String(name));
                }
            }
            Ok(Value::Array(entries))
        })
}

/// создать_директорию(путь)
pub fn make_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("создать_директорию")
        .with_aliases(vec![Arc::from("mkdir"), Arc::from("make_dir")])
        .with_description("Создаёт директорию")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            std::fs::create_dir(path.as_str())
                .map_err(|e| format!("Ошибка создания директории '{}': {}", path, e))?;
            Ok(Value::Null)
        })
}

/// создать_все_директории(путь)
pub fn make_dirs_fn() -> LibFunctionDef {
    LibFunctionDef::new("создать_все_директории")
        .with_aliases(vec![
            Arc::from("mkdirs"),
            Arc::from("make_dirs"),
            Arc::from("mkdir_p"),
        ])
        .with_description("Создаёт директорию и все промежуточные директории рекурсивно")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            std::fs::create_dir_all(path.as_str())
                .map_err(|e| format!("Ошибка создания директорий '{}': {}", path, e))?;
            Ok(Value::Null)
        })
}

/// обход(путь) → [лит]
pub fn walk_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("обход")
        .with_aliases(vec![
            Arc::from("walk"),
            Arc::from("walk_dir"),
            Arc::from("рекурсивный_обход"),
        ])
        .with_description("Рекурсивно обходит директорию и возвращает все пути")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Array(Box::new(TypeKind::String)))
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;

            fn walk_recursive(
                dir: &std::path::Path,
                results: &mut Vec<Value>,
            ) -> Result<(), String> {
                let entries = std::fs::read_dir(dir)
                    .map_err(|e| format!("Ошибка чтения '{}': {}", dir.display(), e))?;
                for entry in entries {
                    let entry = entry.map_err(|e| format!("Ошибка: {}", e))?;
                    let p = entry.path();
                    results.push(Value::String(p.to_string_lossy().into_owned()));
                    if p.is_dir() {
                        walk_recursive(&p, results)?;
                    }
                }
                Ok(())
            }

            let mut results = Vec::new();
            walk_recursive(std::path::Path::new(path.as_str()), &mut results)?;
            Ok(Value::Array(results))
        })
}

/// найти_по_расширению(путь, расширение) → [лит]
pub fn glob_ext_fn() -> LibFunctionDef {
    LibFunctionDef::new("найти_по_расширению")
        .with_aliases(vec![Arc::from("glob_ext"), Arc::from("find_by_ext")])
        .with_description("Рекурсивно ищет все файлы с указанным расширением")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .with_param(LibParamDef::value("расширение", TypeKind::String))
        .returns(TypeKind::Array(Box::new(TypeKind::String)))
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let ext = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'расширение'".to_string())?;
            let ext_str = ext;
            let ext_clean = ext_str.trim_start_matches('.');

            fn find_ext(
                dir: &std::path::Path,
                ext: &str,
                results: &mut Vec<Value>,
            ) -> Result<(), String> {
                let entries = std::fs::read_dir(dir)
                    .map_err(|e| format!("Ошибка чтения '{}': {}", dir.display(), e))?;
                for entry in entries {
                    let entry = entry.map_err(|e| format!("Ошибка: {}", e))?;
                    let p = entry.path();
                    if p.is_dir() {
                        find_ext(&p, ext, results)?;
                    } else if let Some(file_ext) = p.extension()
                        && file_ext.to_string_lossy().eq_ignore_ascii_case(ext)
                    {
                        results.push(Value::String(p.to_string_lossy().into_owned()));
                    }
                }
                Ok(())
            }

            let mut results = Vec::new();
            find_ext(std::path::Path::new(path.as_str()), ext_clean, &mut results)?;
            Ok(Value::Array(results))
        })
}
