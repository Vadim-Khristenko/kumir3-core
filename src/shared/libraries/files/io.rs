//! Чтение и запись файлов

use std::fs::{self, OpenOptions};
use std::io::Write;

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

fn bytes_to_value(bytes: Vec<u8>) -> Value {
    let arr = bytes
        .into_iter()
        .map(|b| Value::Number(Number::U8(b)))
        .collect();
    Value::Array(arr)
}

fn expect_bytes_array(args: &[Value], idx: usize, name: &str) -> Result<Vec<u8>, String> {
    let v = args
        .get(idx)
        .ok_or_else(|| format!("Не передан параметр: {}", name))?;
    match v {
        Value::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::Number(n) => {
                        let i = n.to_i64().ok_or_else(|| format!("Ожидается байт в {}", name))?;
                        if (0..=255).contains(&i) {
                            out.push(i as u8);
                        } else {
                            return Err(format!("Байт вне диапазона 0..255 в {}", name));
                        }
                    }
                    _ => return Err(format!("Ожидается массив байтов в {}", name)),
                }
            }
            Ok(out)
        }
        _ => Err(format!("Ожидается массив байтов для {}", name)),
    }
}

/// прочитать_текст(путь) -> лит
pub fn read_text_fn() -> LibFunctionDef {
    LibFunctionDef::new("прочитать_текст")
        .with_aliases(&["read_text", "read_file", "read"])
        .with_description("Читает файл как UTF-8 строку")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            fs::read_to_string(&path)
                .map(Value::String)
                .map_err(|e| format!("Не удалось прочитать файл: {}", e))
        })
}

/// прочитать_строки(путь) -> массив лит
pub fn read_lines_fn() -> LibFunctionDef {
    LibFunctionDef::new("прочитать_строки")
        .with_aliases(&["read_lines", "readlines"])
        .with_description("Читает файл и возвращает массив строк без разделителей строк")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::Array(Box::new(TypeSpec::String)))
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let text = fs::read_to_string(&path)
                .map_err(|e| format!("Не удалось прочитать файл: {}", e))?;
            let lines: Vec<Value> = text
                .lines()
                .map(|s| Value::String(s.to_string()))
                .collect();
            Ok(Value::Array(lines))
        })
}

/// записать_текст(путь, данные)
pub fn write_text_fn() -> LibFunctionDef {
    LibFunctionDef::new("записать_текст")
        .with_aliases(&["write_text", "write_file", "write"])
        .with_description("Записывает строку в файл (перезаписывает)")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .with_param(LibParamDef::value("данные", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let data = expect_string(args, 1, "данные")?;
            fs::write(&path, data).map_err(|e| format!("Не удалось записать файл: {}", e))?;
            Ok(Value::Null)
        })
}

/// дописать_текст(путь, данные)
pub fn append_text_fn() -> LibFunctionDef {
    LibFunctionDef::new("дописать_текст")
        .with_aliases(&["append_text", "append"])
        .with_description("Дописивает строку в конец файла")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .with_param(LibParamDef::value("данные", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let data = expect_string(args, 1, "данные")?;
            let mut f = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|e| format!("Не удалось открыть файл для дописи: {}", e))?;
            f.write_all(data.as_bytes())
                .map_err(|e| format!("Не удалось записать данные: {}", e))?;
            Ok(Value::Null)
        })
}

/// прочитать_байты(путь) -> массив нат_8
pub fn read_bytes_fn() -> LibFunctionDef {
    LibFunctionDef::new("прочитать_байты")
        .with_aliases(&["read_bytes", "readbin"])
        .with_description("Читает файл как массив байтов")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::Array(Box::new(TypeSpec::UInt8)))
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let data = fs::read(&path)
                .map_err(|e| format!("Не удалось прочитать файл: {}", e))?;
            Ok(bytes_to_value(data))
        })
}

/// записать_байты(путь, байты)
pub fn write_bytes_fn() -> LibFunctionDef {
    LibFunctionDef::new("записать_байты")
        .with_aliases(&["write_bytes", "writebin"])
        .with_description("Записывает массив байтов в файл")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .with_param(LibParamDef::value("байты", TypeSpec::Array(Box::new(TypeSpec::UInt8))))
        .as_procedure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let data = expect_bytes_array(args, 1, "байты")?;
            fs::write(&path, data).map_err(|e| format!("Не удалось записать файл: {}", e))?;
            Ok(Value::Null)
        })
}
