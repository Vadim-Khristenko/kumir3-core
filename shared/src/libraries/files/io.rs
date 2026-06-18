//! Функции чтения/записи файлов

use std::sync::Arc;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::{TypeKind, Value};

/// чтение_текста(путь) → лит
pub fn read_text_fn() -> LibFunctionDef {
    LibFunctionDef::new("чтение_текста")
        .with_aliases(vec![Arc::from("read_text"), Arc::from("прочитать_файл")])
        .with_description("Читает текстовый файл целиком и возвращает строку")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let content = std::fs::read_to_string(path.as_str())
                .map_err(|e| format!("Ошибка чтения файла '{}': {}", path, e))?;
            Ok(Value::String(content))
        })
}

/// чтение_строк(путь) → [лит]
pub fn read_lines_fn() -> LibFunctionDef {
    LibFunctionDef::new("чтение_строк")
        .with_aliases(vec![Arc::from("read_lines"), Arc::from("прочитать_строки")])
        .with_description("Читает файл и возвращает массив строк")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Array(Box::new(TypeKind::String)))
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let content = std::fs::read_to_string(path.as_str())
                .map_err(|e| format!("Ошибка чтения файла '{}': {}", path, e))?;
            let lines: Vec<Value> = content
                .lines()
                .map(|l| Value::String(l.to_string()))
                .collect();
            Ok(Value::Array(lines))
        })
}

/// запись_текста(путь, содержимое)
pub fn write_text_fn() -> LibFunctionDef {
    LibFunctionDef::new("запись_текста")
        .with_aliases(vec![Arc::from("write_text"), Arc::from("записать_файл")])
        .with_description("Записывает строку в файл (перезаписывает)")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .with_param(LibParamDef::value("содержимое", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let content = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'содержимое'".to_string())?;
            std::fs::write(path.as_str(), content.as_bytes())
                .map_err(|e| format!("Ошибка записи файла '{}': {}", path, e))?;
            Ok(Value::Null)
        })
}

/// дозапись_текста(путь, содержимое)
pub fn append_text_fn() -> LibFunctionDef {
    use std::io::Write;
    LibFunctionDef::new("дозапись_текста")
        .with_aliases(vec![Arc::from("append_text"), Arc::from("дописать_файл")])
        .with_description("Дозаписывает строку в конец файла")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .with_param(LibParamDef::value("содержимое", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let content = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'содержимое'".to_string())?;
            let mut file = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(path.as_str())
                .map_err(|e| format!("Ошибка открытия '{}': {}", path, e))?;
            file.write_all(content.as_bytes())
                .map_err(|e| format!("Ошибка дозаписи в '{}': {}", path, e))?;
            Ok(Value::Null)
        })
}

/// чтение_байтов(путь) → [цел]
pub fn read_bytes_fn() -> LibFunctionDef {
    LibFunctionDef::new("чтение_байтов")
        .with_aliases(vec![Arc::from("read_bytes"), Arc::from("прочитать_байты")])
        .with_description("Читает файл как массив байтов")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Array(Box::new(TypeKind::Int64)))
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let bytes = std::fs::read(path.as_str())
                .map_err(|e| format!("Ошибка чтения файла '{}': {}", path, e))?;
            let values: Vec<Value> = bytes
                .iter()
                .map(|&b| Value::Number(crate::types::Number::I64(b as i64)))
                .collect();
            Ok(Value::Array(values))
        })
}

/// запись_байтов(путь, байты)
pub fn write_bytes_fn() -> LibFunctionDef {
    LibFunctionDef::new("запись_байтов")
        .with_aliases(vec![Arc::from("write_bytes"), Arc::from("записать_байты")])
        .with_description("Записывает массив байтов в файл")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .with_param(LibParamDef::value(
            "байты",
            TypeKind::Array(Box::new(TypeKind::Int64)),
        ))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let arr = args
                .get(1)
                .and_then(|v| v.as_array())
                .ok_or_else(|| "Ожидается массив байтов".to_string())?;
            let bytes: Vec<u8> = arr
                .iter()
                .map(|v| {
                    v.as_number()
                        .and_then(|n| n.to_i64())
                        .map(|n| n as u8)
                        .unwrap_or(0)
                })
                .collect();
            std::fs::write(path.as_str(), &bytes)
                .map_err(|e| format!("Ошибка записи файла '{}': {}", path, e))?;
            Ok(Value::Null)
        })
}

/// запись_строк(путь, строки)
pub fn write_lines_fn() -> LibFunctionDef {
    LibFunctionDef::new("запись_строк")
        .with_aliases(vec![Arc::from("write_lines"), Arc::from("записать_строки")])
        .with_description("Записывает массив строк в файл (каждая строка на новой строке)")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .with_param(LibParamDef::value(
            "строки",
            TypeKind::Array(Box::new(TypeKind::String)),
        ))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let arr = args
                .get(1)
                .and_then(|v| v.as_array())
                .ok_or_else(|| "Ожидается массив строк".to_string())?;
            let lines: Vec<String> = arr
                .iter()
                .map(|v| v.as_string().map(|s| s.to_string()).unwrap_or_default())
                .collect();
            let content = lines.join("\n");
            std::fs::write(path.as_str(), content.as_bytes())
                .map_err(|e| format!("Ошибка записи файла '{}': {}", path, e))?;
            Ok(Value::Null)
        })
}
