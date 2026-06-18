//! Метаданные файлов: существование, размер, тип, права

use std::sync::Arc;

use crate::types::Number;
use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::value::{TypeKind, Value};

/// существует(путь) → лог
pub fn exists_fn() -> LibFunctionDef {
    LibFunctionDef::new("существует")
        .with_aliases(vec![Arc::from("exists"), Arc::from("файл_существует")])
        .with_description("Проверяет, существует ли файл или директория")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Bool)
        .as_pure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            Ok(Value::Boolean(std::path::Path::new(path.as_str()).exists()))
        })
}

/// это_файл(путь) → лог
pub fn is_file_fn() -> LibFunctionDef {
    LibFunctionDef::new("это_файл")
        .with_aliases(vec![Arc::from("is_file")])
        .with_description("Проверяет, является ли путь файлом")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Bool)
        .as_pure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            Ok(Value::Boolean(
                std::path::Path::new(path.as_str()).is_file(),
            ))
        })
}

/// это_директория(путь) → лог
pub fn is_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("это_директория")
        .with_aliases(vec![Arc::from("is_dir"), Arc::from("is_directory")])
        .with_description("Проверяет, является ли путь директорией")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Bool)
        .as_pure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            Ok(Value::Boolean(std::path::Path::new(path.as_str()).is_dir()))
        })
}

/// это_ссылка(путь) → лог
pub fn is_symlink_fn() -> LibFunctionDef {
    LibFunctionDef::new("это_ссылка")
        .with_aliases(vec![Arc::from("is_symlink")])
        .with_description("Проверяет, является ли путь символической ссылкой")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Bool)
        .as_pure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            Ok(Value::Boolean(
                std::path::Path::new(path.as_str()).is_symlink(),
            ))
        })
}

/// размер_файла(путь) → цел
pub fn file_size_fn() -> LibFunctionDef {
    LibFunctionDef::new("размер_файла")
        .with_aliases(vec![Arc::from("file_size"), Arc::from("size")])
        .with_description("Возвращает размер файла в байтах")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Int64)
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let meta = std::fs::metadata(path.as_str())
                .map_err(|e| format!("Ошибка получения метаданных '{}': {}", path, e))?;
            Ok(Value::Number(Number::I64(meta.len() as i64)))
        })
}

/// информация_о_файле(путь) → Мап
pub fn stat_fn() -> LibFunctionDef {
    LibFunctionDef::new("информация_о_файле")
        .with_aliases(vec![Arc::from("stat"), Arc::from("file_info")])
        .with_description("Возвращает словарь с полной информацией о файле")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Map(
            Box::new(TypeKind::String),
            Box::new(TypeKind::String),
        ))
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let meta = std::fs::metadata(path.as_str())
                .map_err(|e| format!("Ошибка получения метаданных '{}': {}", path, e))?;

            let mut entries = std::collections::BTreeMap::new();

            // Размер
            entries.insert(
                Value::String("размер".to_string()),
                Value::String(meta.len().to_string()),
            );
            entries.insert(
                Value::String("size".to_string()),
                Value::String(meta.len().to_string()),
            );

            // Тип
            let file_type = if meta.is_file() {
                "file"
            } else if meta.is_dir() {
                "directory"
            } else if meta.is_symlink() {
                "symlink"
            } else {
                "other"
            };
            entries.insert(
                Value::String("тип".to_string()),
                Value::String(file_type.to_string()),
            );
            entries.insert(
                Value::String("type".to_string()),
                Value::String(file_type.to_string()),
            );

            // Только для чтения
            let readonly = meta.permissions().readonly();
            entries.insert(
                Value::String("только_чтение".to_string()),
                Value::String(if readonly { "да" } else { "нет" }.to_string()),
            );
            entries.insert(
                Value::String("readonly".to_string()),
                Value::String(if readonly { "true" } else { "false" }.to_string()),
            );

            // Время модификации
            if let Ok(modified) = meta.modified()
                && let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH)
            {
                let secs = dur.as_secs().to_string();
                entries.insert(
                    Value::String("время_изменения".to_string()),
                    Value::String(secs.clone()),
                );
                entries.insert(Value::String("modified".to_string()), Value::String(secs));
            }

            // Время создания (если поддерживается ОС)
            if let Ok(created) = meta.created()
                && let Ok(dur) = created.duration_since(std::time::UNIX_EPOCH)
            {
                let secs = dur.as_secs().to_string();
                entries.insert(
                    Value::String("время_создания".to_string()),
                    Value::String(secs.clone()),
                );
                entries.insert(Value::String("created".to_string()), Value::String(secs));
            }

            Ok(Value::Map(entries))
        })
}

/// канонический_путь(путь) → лит
pub fn canonical_path_fn() -> LibFunctionDef {
    LibFunctionDef::new("канонический_путь")
        .with_aliases(vec![Arc::from("canonical_path"), Arc::from("realpath")])
        .with_description("Возвращает каноничный абсолютный путь (разрешает символические ссылки)")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let canonical = std::fs::canonicalize(path.as_str())
                .map_err(|e| format!("Ошибка получения канонического пути '{}': {}", path, e))?;
            Ok(Value::String(canonical.to_string_lossy().into_owned()))
        })
}

/// установить_только_чтение(путь, только_чтение)
pub fn set_readonly_fn() -> LibFunctionDef {
    LibFunctionDef::new("установить_только_чтение")
        .with_aliases(vec![Arc::from("set_readonly")])
        .with_description("Устанавливает или снимает атрибут «только для чтения»")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .with_param(LibParamDef::value("только_чтение", TypeKind::Bool))
        .as_procedure()
        .with_handler(|args| {
            let path = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'путь'".to_string())?;
            let readonly = args
                .get(1)
                .and_then(|v| v.as_bool())
                .ok_or_else(|| "Ожидается логический аргумент 'только_чтение'".to_string())?;
            let meta = std::fs::metadata(path.as_str())
                .map_err(|e| format!("Ошибка получения метаданных: {}", e))?;
            let mut perms = meta.permissions();
            perms.set_readonly(readonly);
            std::fs::set_permissions(path.as_str(), perms)
                .map_err(|e| format!("Ошибка установки прав: {}", e))?;
            Ok(Value::Null)
        })
}
