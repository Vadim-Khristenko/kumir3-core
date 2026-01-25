//! Функции работы с путями и директориями

use std::path::Path;

use crate::shared::types::library::{LibFunctionDef, LibParamDef};
use crate::shared::types::type_spec::TypeSpec;
use crate::shared::types::Value;

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

/// текущая_директория() -> лит
pub fn cwd_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущая_директория")
        .with_aliases(&["cwd", "getcwd", "pwd", "рабочая_директория"])
        .with_description("Возвращает текущую рабочую директорию")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            std::env::current_dir()
                .map(|p| Value::String(p.to_string_lossy().to_string()))
                .map_err(|e| format!("Не удалось получить текущую директорию: {}", e))
        })
}

/// сменить_директорию(путь)
pub fn chdir_fn() -> LibFunctionDef {
    LibFunctionDef::new("сменить_директорию")
        .with_aliases(&["chdir", "cd", "set_cwd"])
        .with_description("Изменяет текущую рабочую директорию")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            std::env::set_current_dir(&path)
                .map_err(|e| format!("Не удалось сменить директорию: {}", e))?;
            Ok(Value::Null)
        })
}

/// домашняя_директория() -> лит
pub fn home_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("домашняя_директория")
        .with_aliases(&["home_dir", "homedir", "home"])
        .with_description("Возвращает домашнюю директорию пользователя")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            // Используем переменные окружения для кроссплатформенности
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

/// временная_директория() -> лит
pub fn temp_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("временная_директория")
        .with_aliases(&["temp_dir", "tmpdir", "tmp"])
        .with_description("Возвращает директорию для временных файлов")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            Ok(Value::String(
                std::env::temp_dir().to_string_lossy().to_string(),
            ))
        })
}

/// путь_существует(путь) -> лог
pub fn path_exists_fn() -> LibFunctionDef {
    LibFunctionDef::new("путь_существует")
        .with_aliases(&["path_exists", "exists"])
        .with_description("Проверяет существование пути (файл или директория)")
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

/// это_директория(путь) -> лог
pub fn is_dir_fn() -> LibFunctionDef {
    LibFunctionDef::new("это_директория")
        .with_aliases(&["is_dir", "isdir"])
        .with_description("Проверяет, является ли путь директорией")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::Bool)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            Ok(Value::Boolean(Path::new(&path).is_dir()))
        })
}

/// абсолютный_путь(путь) -> лит
pub fn abs_path_fn() -> LibFunctionDef {
    LibFunctionDef::new("абсолютный_путь")
        .with_aliases(&["abs_path", "abspath", "realpath"])
        .with_description("Возвращает абсолютный путь")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            std::fs::canonicalize(&path)
                .map(|p| Value::String(p.to_string_lossy().to_string()))
                .map_err(|e| format!("Не удалось получить абсолютный путь: {}", e))
        })
}

/// имя_файла(путь) -> лит
pub fn basename_fn() -> LibFunctionDef {
    LibFunctionDef::new("имя_файла")
        .with_aliases(&["basename", "filename"])
        .with_description("Возвращает имя файла из пути")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let name = Path::new(&path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(name))
        })
}

/// директория_файла(путь) -> лит
pub fn dirname_fn() -> LibFunctionDef {
    LibFunctionDef::new("директория_файла")
        .with_aliases(&["dirname", "parent_dir"])
        .with_description("Возвращает директорию из пути")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let dir = Path::new(&path)
                .parent()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(dir))
        })
}

/// расширение_файла(путь) -> лит
pub fn extension_fn() -> LibFunctionDef {
    LibFunctionDef::new("расширение_файла")
        .with_aliases(&["extension", "ext", "file_ext"])
        .with_description("Возвращает расширение файла")
        .with_param(LibParamDef::value("путь", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let path = expect_string(args, 0, "путь")?;
            let ext = Path::new(&path)
                .extension()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(ext))
        })
}

/// соединить_пути(путь1, путь2) -> лит
pub fn join_path_fn() -> LibFunctionDef {
    LibFunctionDef::new("соединить_пути")
        .with_aliases(&["join_path", "path_join"])
        .with_description("Соединяет два пути")
        .with_param(LibParamDef::value("путь1", TypeSpec::String))
        .with_param(LibParamDef::value("путь2", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let p1 = expect_string(args, 0, "путь1")?;
            let p2 = expect_string(args, 1, "путь2")?;
            let joined = Path::new(&p1).join(&p2);
            Ok(Value::String(joined.to_string_lossy().to_string()))
        })
}
