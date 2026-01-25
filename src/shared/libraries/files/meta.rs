//! Метаданные файлов и проверка существования

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use std::path::Path;
use std::collections::BTreeMap;

use crate::shared::types::library::{LibFunctionDef, LibParamDef};
use crate::shared::types::type_spec::TypeSpec;
use crate::shared::types::{Number, Value};

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

fn system_time_to_ms(time: SystemTime) -> Option<i64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
}

/// существует(путь) -> лог
pub fn exists_fn() -> LibFunctionDef {
    LibFunctionDef::new("существует")
        .with_aliases(&["exists", "path_exists"])
        .with_description("Проверяет существование пути")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::Bool)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            Ok(Value::Boolean(Path::new(&path).exists()))
        })
}

/// это_файл(путь) -> лог
pub fn is_file_fn() -> LibFunctionDef {
    LibFunctionDef::new("это_файл")
        .with_aliases(&["is_file", "isfile"])
        .with_description("Проверяет, является ли путь файлом")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::Bool)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            Ok(Value::Boolean(Path::new(&path).is_file()))
        })
}

/// это_дир(путь) -> лог
pub fn is_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("это_дир")
        .with_aliases(&["is_dir", "isdir"])
        .with_description("Проверяет, является ли путь директорией")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::Bool)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            Ok(Value::Boolean(Path::new(&path).is_dir()))
        })
}

/// размер(путь) -> нат_64
pub fn size_fn() -> LibFunctionDef {
    LibFunctionDef::new("размер")
        .with_aliases(&["size", "file_size"])
        .with_description("Возвращает размер файла в байтах")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::UInt64)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let meta = fs::metadata(&path)
                .map_err(|e| format!("Не удалось получить метаданные: {}", e))?;
            Ok(Value::Number(Number::U64(meta.len())))
        })
}

/// стат(путь) -> словарь
pub fn stat_fn() -> LibFunctionDef {
    LibFunctionDef::new("стат")
        .with_aliases(&["stat", "metadata"])
        .with_description("Возвращает словарь с метаданными файла/директории")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::Map(Box::new(TypeSpec::String), Box::new(TypeSpec::Any)))
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let meta = fs::metadata(&path)
                .map_err(|e| format!("Не удалось получить метаданные: {}", e))?;
            let mut map = BTreeMap::new();
            map.insert(Value::String("is_file".into()), Value::Boolean(meta.is_file()));
            map.insert(Value::String("is_dir".into()), Value::Boolean(meta.is_dir()));
            map.insert(Value::String("readonly".into()), Value::Boolean(meta.permissions().readonly()));
            map.insert(Value::String("size".into()), Value::Number(Number::U64(meta.len())));
            if let Ok(modified) = meta.modified() {
                if let Some(ms) = system_time_to_ms(modified) {
                    map.insert(Value::String("modified_ms".into()), Value::Number(Number::I64(ms)));
                }
            }
            if let Ok(created) = meta.created() {
                if let Some(ms) = system_time_to_ms(created) {
                    map.insert(Value::String("created_ms".into()), Value::Number(Number::I64(ms)));
                }
            }
            Ok(Value::Map(map))
        })
}
