//! Операции с файлами: копирование, перемещение, удаление

use std::sync::Arc;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::value::{TypeKind, Value};

/// копировать_файл(источник, назначение)
pub fn copy_file_fn() -> LibFunctionDef {
    LibFunctionDef::new("копировать_файл")
        .with_aliases(vec![Arc::from("copy_file"), Arc::from("copy")])
        .with_description("Копирует файл из источника в назначение")
        .with_param(LibParamDef::value("источник", TypeKind::String))
        .with_param(LibParamDef::value("назначение", TypeKind::String))
        .returns(TypeKind::Int64)
        .with_handler(|args| {
            let src = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'источник'".to_string())?;
            let dst = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'назначение'".to_string())?;
            let bytes = std::fs::copy(src.as_str(), dst.as_str())
                .map_err(|e| format!("Ошибка копирования '{}' → '{}': {}", src, dst, e))?;
            Ok(Value::Number(crate::types::Number::I64(bytes as i64)))
        })
}

/// переместить_файл(источник, назначение)
pub fn move_file_fn() -> LibFunctionDef {
    LibFunctionDef::new("переместить_файл")
        .with_aliases(vec![
            Arc::from("move_file"),
            Arc::from("rename"),
            Arc::from("переименовать"),
        ])
        .with_description("Перемещает (переименовывает) файл или директорию")
        .with_param(LibParamDef::value("источник", TypeKind::String))
        .with_param(LibParamDef::value("назначение", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let src = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'источник'".to_string())?;
            let dst = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'назначение'".to_string())?;
            std::fs::rename(src.as_str(), dst.as_str())
                .map_err(|e| format!("Ошибка перемещения '{}' → '{}': {}", src, dst, e))?;
            Ok(Value::Null)
        })
}

/// удалить_файл(путь)
pub fn remove_file_fn() -> LibFunctionDef {
    LibFunctionDef::new("удалить_файл")
        .with_aliases(vec![Arc::from("remove_file"), Arc::from("delete_file")])
        .with_description("Удаляет файл")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            std::fs::remove_file(path.as_str())
                .map_err(|e| format!("Ошибка удаления файла '{}': {}", path, e))?;
            Ok(Value::Null)
        })
}

/// удалить_директорию(путь)
pub fn remove_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("удалить_директорию")
        .with_aliases(vec![Arc::from("remove_dir"), Arc::from("rmdir")])
        .with_description("Удаляет пустую директорию")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            std::fs::remove_dir(path.as_str())
                .map_err(|e| format!("Ошибка удаления директории '{}': {}", path, e))?;
            Ok(Value::Null)
        })
}

/// удалить_директорию_рекурсивно(путь)
pub fn remove_dir_all_fn() -> LibFunctionDef {
    LibFunctionDef::new("удалить_директорию_рекурсивно")
        .with_aliases(vec![Arc::from("remove_dir_all"), Arc::from("rm_rf")])
        .with_description("Удаляет директорию со всем содержимым рекурсивно")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            std::fs::remove_dir_all(path.as_str())
                .map_err(|e| format!("Ошибка рекурсивного удаления '{}': {}", path, e))?;
            Ok(Value::Null)
        })
}

/// создать_файл(путь)
pub fn touch_fn() -> LibFunctionDef {
    LibFunctionDef::new("создать_файл")
        .with_aliases(vec![Arc::from("touch"), Arc::from("create_file")])
        .with_description("Создаёт пустой файл (или обновляет время изменения)")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let p = std::path::Path::new(path.as_str());
            if p.exists() {
                // Обновляем metadata, если файл существует
                let _ = std::fs::OpenOptions::new().write(true).open(p);
            } else {
                std::fs::File::create(p)
                    .map_err(|e| format!("Ошибка создания файла '{}': {}", path, e))?;
            }
            Ok(Value::Null)
        })
}

/// символическая_ссылка(цель, ссылка)
pub fn symlink_fn() -> LibFunctionDef {
    LibFunctionDef::new("символическая_ссылка")
        .with_aliases(vec![Arc::from("symlink"), Arc::from("create_symlink")])
        .with_description("Создаёт символическую ссылку")
        .with_param(LibParamDef::value("цель", TypeKind::String))
        .with_param(LibParamDef::value("ссылка", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let target = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'цель'".to_string())?;
            let link = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'ссылка'".to_string())?;
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(target.as_str(), link.as_str())
                    .map_err(|e| format!("Ошибка создания символической ссылки: {}", e))?;
            }
            #[cfg(windows)]
            {
                let target_path = std::path::Path::new(target.as_str());
                if target_path.is_dir() {
                    std::os::windows::fs::symlink_dir(target.as_str(), link.as_str())
                        .map_err(|e| format!("Ошибка создания символической ссылки: {}", e))?;
                } else {
                    std::os::windows::fs::symlink_file(target.as_str(), link.as_str())
                        .map_err(|e| format!("Ошибка создания символической ссылки: {}", e))?;
                }
            }
            Ok(Value::Null)
        })
}
