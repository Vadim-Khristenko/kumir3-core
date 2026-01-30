//! Работа с директориями

use std::fs;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::type_spec::TypeSpec;
use crate::types::Value;

fn expect_string(args: &[Value], idx: usize, name: &str) -> Result<String, String> {
    let v = args
        .get(idx)
        .ok_or_else(|| format!("Не передан параметр: {}", name))?;
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(format!("Ожидается строка для параметра {}", name)),
    }
}

fn list_entries(path: &str, files_only: bool) -> Result<Value, String> {
    let entries = fs::read_dir(path)
        .map_err(|e| format!("Не удалось прочитать директорию: {}", e))?;
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Ошибка чтения элемента: {}", e))?;
        let meta = entry.metadata().map_err(|e| format!("Не удалось получить метаданные: {}", e))?;
        if files_only && !meta.is_file() {
            continue;
        }
        out.push(Value::String(entry.path().to_string_lossy().to_string()));
    }
    Ok(Value::Array(out))
}

/// содержимое(дир) -> массив путей
pub fn list_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("содержимое")
        .with_aliases(&["list_dir", "ls", "dir"])
        .with_description("Возвращает список всех элементов директории")
        .with_param(LibParamDef::value("директория", TypeSpec::String))
        .returns(TypeSpec::Array(Box::new(TypeSpec::String)))
        .with_handler(|args| {
            let dir = expect_string(args, 0, "директория")?;
            list_entries(&dir, false)
        })
}

/// файлы(дир) -> массив путей
pub fn list_files_fn() -> LibFunctionDef {
    LibFunctionDef::new("файлы")
        .with_aliases(&["list_files", "ls_files"])
        .with_description("Возвращает список файлов в директории")
        .with_param(LibParamDef::value("директория", TypeSpec::String))
        .returns(TypeSpec::Array(Box::new(TypeSpec::String)))
        .with_handler(|args| {
            let dir = expect_string(args, 0, "директория")?;
            list_entries(&dir, true)
        })
}

/// создать_директорию(путь)
pub fn make_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("создать_директорию")
        .with_aliases(&["mkdir", "make_dir"])
        .with_description("Создаёт одну директорию")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let dir = expect_string(args, 0, "путь")?;
            fs::create_dir(&dir).map_err(|e| format!("Не удалось создать директорию: {}", e))?;
            Ok(Value::Null)
        })
}

/// создать_дерево(путь)
pub fn make_dirs_fn() -> LibFunctionDef {
    LibFunctionDef::new("создать_дерево")
        .with_aliases(&["mkdirs", "make_dirs", "makedirs"])
        .with_description("Создаёт директорию и недостающие родительские")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let dir = expect_string(args, 0, "путь")?;
            fs::create_dir_all(&dir).map_err(|e| format!("Не удалось создать директории: {}", e))?;
            Ok(Value::Null)
        })
}
