//! Операции над файлами и директориями

use std::fs::{self, OpenOptions};

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::type_spec::TypeSpec;
use crate::types::{Number, Value};

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

/// копировать(откуда, куда) -> нат_64 (байты)
pub fn copy_fn() -> LibFunctionDef {
    LibFunctionDef::new("копировать")
        .with_aliases(&["copy", "cp"])
        .with_description("Копирует файл и возвращает число записанных байт")
        .with_param(LibParamDef::value("откуда", TypeSpec::String))
        .with_param(LibParamDef::value("куда", TypeSpec::String))
        .returns(TypeSpec::UInt64)
        .with_handler(|args| {
            let src = expect_string(args, 0, "откуда")?;
            let dst = expect_string(args, 1, "куда")?;
            let bytes = fs::copy(&src, &dst)
                .map_err(|e| format!("Не удалось копировать файл: {}", e))?;
            Ok(Value::Number(Number::U64(bytes)))
        })
}

/// переместить(откуда, куда)
pub fn move_fn() -> LibFunctionDef {
    LibFunctionDef::new("переместить")
        .with_aliases(&["move", "mv", "rename"])
        .with_description("Перемещает или переименовывает файл/директорию")
        .with_param(LibParamDef::value("откуда", TypeSpec::String))
        .with_param(LibParamDef::value("куда", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let src = expect_string(args, 0, "откуда")?;
            let dst = expect_string(args, 1, "куда")?;
            fs::rename(&src, &dst)
                .map_err(|e| format!("Не удалось переместить: {}", e))?;
            Ok(Value::Null)
        })
}

/// удалить_файл(путь)
pub fn remove_file_fn() -> LibFunctionDef {
    LibFunctionDef::new("удалить_файл")
        .with_aliases(&["remove_file", "rm"])
        .with_description("Удаляет файл")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            fs::remove_file(&path)
                .map_err(|e| format!("Не удалось удалить файл: {}", e))?;
            Ok(Value::Null)
        })
}

/// удалить_директорию(путь)
pub fn remove_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("удалить_директорию")
        .with_aliases(&["remove_dir", "rmdir"])
        .with_description("Удаляет пустую директорию")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            fs::remove_dir(&path)
                .map_err(|e| format!("Не удалось удалить директорию: {}", e))?;
            Ok(Value::Null)
        })
}

/// удалить_все(путь)
pub fn remove_dir_all_fn() -> LibFunctionDef {
    LibFunctionDef::new("удалить_все")
        .with_aliases(&["remove_all", "rm_rf", "rmtree"])
        .with_description("Рекурсивно удаляет директорию вместе с содержимым")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            fs::remove_dir_all(&path)
                .map_err(|e| format!("Не удалось удалить: {}", e))?;
            Ok(Value::Null)
        })
}

/// коснуться(путь)
pub fn touch_fn() -> LibFunctionDef {
    LibFunctionDef::new("коснуться")
        .with_aliases(&["touch", "create_empty"])
        .with_description("Создаёт файл, если нет, иначе обновляет время модификации")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let _ = OpenOptions::new()
                .create(true)
                .write(true)
                .open(&path)
                .map_err(|e| format!("Не удалось открыть файл: {}", e))?;
            Ok(Value::Null)
        })
}
