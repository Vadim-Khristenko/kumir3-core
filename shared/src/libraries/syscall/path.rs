//! Функции работы с путями и директориями

use std::path::Path;
use std::sync::Arc;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::{TypeKind, Value};

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

pub fn cwd_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущая_директория")
        .with_aliases(vec![
            Arc::from("cwd"),
            Arc::from("getcwd"),
            Arc::from("pwd"),
        ])
        .with_description("Возвращает текущую рабочую директорию")
        .returns(TypeKind::String)
        .with_handler(|_args| {
            std::env::current_dir()
                .map(|p| Value::String(p.to_string_lossy().to_string()))
                .map_err(|e| format!("Не удалось получить текущую директорию: {}", e))
        })
}

pub fn chdir_fn() -> LibFunctionDef {
    LibFunctionDef::new("сменить_директорию")
        .with_aliases(vec![
            Arc::from("chdir"),
            Arc::from("cd"),
            Arc::from("set_cwd"),
        ])
        .with_description("Изменяет текущую рабочую директорию")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .as_procedure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            std::env::set_current_dir(&path)
                .map_err(|e| format!("Не удалось сменить директорию: {}", e))?;
            Ok(Value::Null)
        })
}

pub fn home_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("домашняя_директория")
        .with_aliases(vec![
            Arc::from("home_dir"),
            Arc::from("homedir"),
            Arc::from("home"),
        ])
        .with_description("Возвращает домашнюю директорию пользователя")
        .returns(TypeKind::String)
        .with_handler(|_args| {
            let home = if cfg!(windows) {
                std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
            } else {
                std::env::var("HOME")
            };
            match home {
                Ok(h) => Ok(Value::String(h)),
                Err(_) => Ok(Value::Null),
            }
        })
}

pub fn temp_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("временная_директория")
        .with_aliases(vec![
            Arc::from("temp_dir"),
            Arc::from("tmpdir"),
            Arc::from("tmp"),
        ])
        .with_description("Возвращает директорию для временных файлов")
        .returns(TypeKind::String)
        .with_handler(|_args| {
            Ok(Value::String(
                std::env::temp_dir().to_string_lossy().to_string(),
            ))
        })
}

pub fn path_exists_fn() -> LibFunctionDef {
    LibFunctionDef::new("путь_существует")
        .with_aliases(vec![Arc::from("path_exists"), Arc::from("exists")])
        .with_description("Проверяет существование пути (файл или директория)")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Bool)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            Ok(Value::Boolean(Path::new(&path).exists()))
        })
}

pub fn is_file_fn() -> LibFunctionDef {
    LibFunctionDef::new("это_файл")
        .with_aliases(vec![Arc::from("is_file"), Arc::from("isfile")])
        .with_description("Проверяет, является ли путь файлом")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Bool)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            Ok(Value::Boolean(Path::new(&path).is_file()))
        })
}

pub fn is_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("это_директория")
        .with_aliases(vec![Arc::from("is_dir"), Arc::from("isdir")])
        .with_description("Проверяет, является ли путь директорией")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::Bool)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            Ok(Value::Boolean(Path::new(&path).is_dir()))
        })
}

pub fn abs_path_fn() -> LibFunctionDef {
    LibFunctionDef::new("абсолютный_путь")
        .with_aliases(vec![
            Arc::from("abs_path"),
            Arc::from("abspath"),
            Arc::from("realpath"),
        ])
        .with_description("Возвращает абсолютный путь")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            std::fs::canonicalize(&path)
                .map(|p| Value::String(p.to_string_lossy().to_string()))
                .map_err(|e| format!("Не удалось получить абсолютный путь: {}", e))
        })
}

pub fn basename_fn() -> LibFunctionDef {
    LibFunctionDef::new("имя_файла")
        .with_aliases(vec![Arc::from("basename"), Arc::from("filename")])
        .with_description("Возвращает имя файла из пути")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let name = Path::new(&path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(name))
        })
}

pub fn dirname_fn() -> LibFunctionDef {
    LibFunctionDef::new("директория_файла")
        .with_aliases(vec![Arc::from("dirname"), Arc::from("parent_dir")])
        .with_description("Возвращает директорию из пути")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let dir = Path::new(&path)
                .parent()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(dir))
        })
}

pub fn extension_fn() -> LibFunctionDef {
    LibFunctionDef::new("расширение_файла")
        .with_aliases(vec![
            Arc::from("extension"),
            Arc::from("ext"),
            Arc::from("file_ext"),
        ])
        .with_description("Возвращает расширение файла")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let ext = Path::new(&path)
                .extension()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(ext))
        })
}

pub fn join_path_fn() -> LibFunctionDef {
    LibFunctionDef::new("соединить_пути")
        .with_aliases(vec![Arc::from("join_path"), Arc::from("path_join")])
        .with_description("Соединяет два пути")
        .with_param(LibParamDef::value("путь1", TypeKind::String))
        .with_param(LibParamDef::value("путь2", TypeKind::String))
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|args| {
            let p1 = expect_string(args, 0, "путь1")?;
            let p2 = expect_string(args, 1, "путь2")?;
            let joined = Path::new(&p1).join(&p2);
            Ok(Value::String(joined.to_string_lossy().to_string()))
        })
}

/// без_расширения(путь) -> лит
pub fn stem_fn() -> LibFunctionDef {
    LibFunctionDef::new("без_расширения")
        .with_aliases(vec![Arc::from("stem"), Arc::from("file_stem")])
        .with_description("Возвращает имя файла без расширения")
        .with_param(LibParamDef::value("путь", TypeKind::String))
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let stem = Path::new(&path)
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(stem))
        })
}
