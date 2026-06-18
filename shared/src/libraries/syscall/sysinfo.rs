//! Функции получения информации о системе

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::types::library::LibFunctionDef;
use crate::types::{Number, TypeKind, Value};

pub fn os_name_fn() -> LibFunctionDef {
    LibFunctionDef::new("имя_ос")
        .with_aliases(vec![
            Arc::from("os_name"),
            Arc::from("platform"),
            Arc::from("система"),
        ])
        .with_description("Возвращает имя ОС (windows, linux, macos)")
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|_args| {
            let name = if cfg!(windows) {
                "windows"
            } else if cfg!(target_os = "macos") {
                "macos"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else {
                "unknown"
            };
            Ok(Value::String(name.to_string()))
        })
}

pub fn os_family_fn() -> LibFunctionDef {
    LibFunctionDef::new("семейство_ос")
        .with_aliases(vec![Arc::from("os_family")])
        .with_description("Возвращает семейство ОС (unix, windows)")
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|_args| {
            let family = if cfg!(unix) {
                "unix"
            } else if cfg!(windows) {
                "windows"
            } else {
                "unknown"
            };
            Ok(Value::String(family.to_string()))
        })
}

pub fn arch_fn() -> LibFunctionDef {
    LibFunctionDef::new("архитектура")
        .with_aliases(vec![
            Arc::from("arch"),
            Arc::from("architecture"),
            Arc::from("cpu_arch"),
        ])
        .with_description("Возвращает архитектуру процессора")
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|_args| Ok(Value::String(std::env::consts::ARCH.to_string())))
}

pub fn hostname_fn() -> LibFunctionDef {
    LibFunctionDef::new("имя_хоста")
        .with_aliases(vec![Arc::from("hostname"), Arc::from("host")])
        .with_description("Возвращает имя компьютера")
        .returns(TypeKind::String)
        .with_handler(|_args| {
            let hostname = if cfg!(windows) {
                std::env::var("COMPUTERNAME").unwrap_or_default()
            } else {
                std::env::var("HOSTNAME")
                    .or_else(|_| std::env::var("HOST"))
                    .unwrap_or_default()
            };
            Ok(Value::String(hostname))
        })
}

pub fn username_fn() -> LibFunctionDef {
    LibFunctionDef::new("имя_пользователя")
        .with_aliases(vec![
            Arc::from("username"),
            Arc::from("user"),
            Arc::from("whoami"),
        ])
        .with_description("Возвращает имя текущего пользователя")
        .returns(TypeKind::String)
        .with_handler(|_args| {
            let user = if cfg!(windows) {
                std::env::var("USERNAME").unwrap_or_default()
            } else {
                std::env::var("USER")
                    .or_else(|_| std::env::var("LOGNAME"))
                    .unwrap_or_default()
            };
            Ok(Value::String(user))
        })
}

pub fn path_sep_fn() -> LibFunctionDef {
    LibFunctionDef::new("разделитель_пути")
        .with_aliases(vec![Arc::from("path_sep"), Arc::from("pathsep")])
        .with_description("Возвращает системный разделитель пути")
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|_args| Ok(Value::String(std::path::MAIN_SEPARATOR.to_string())))
}

pub fn pathlist_sep_fn() -> LibFunctionDef {
    LibFunctionDef::new("разделитель_путей")
        .with_aliases(vec![Arc::from("pathlist_sep"), Arc::from("path_delimiter")])
        .with_description("Разделитель в PATH (; или :)")
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|_args| {
            let sep = if cfg!(windows) { ";" } else { ":" };
            Ok(Value::String(sep.to_string()))
        })
}

pub fn line_sep_fn() -> LibFunctionDef {
    LibFunctionDef::new("разделитель_строк")
        .with_aliases(vec![
            Arc::from("line_sep"),
            Arc::from("linesep"),
            Arc::from("newline"),
        ])
        .with_description("Возвращает системный разделитель строк")
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|_args| {
            let sep = if cfg!(windows) { "\r\n" } else { "\n" };
            Ok(Value::String(sep.to_string()))
        })
}

pub fn cpu_count_fn() -> LibFunctionDef {
    LibFunctionDef::new("количество_ядер")
        .with_aliases(vec![
            Arc::from("cpu_count"),
            Arc::from("num_cpus"),
            Arc::from("cores"),
        ])
        .with_description("Возвращает количество логических ядер процессора")
        .returns(TypeKind::UInt64)
        .with_handler(|_args| {
            let count = std::thread::available_parallelism()
                .map(|n| n.get() as u64)
                .unwrap_or(1);
            Ok(Value::Number(Number::U64(count)))
        })
}

pub fn argv_fn() -> LibFunctionDef {
    LibFunctionDef::new("аргументы_командной_строки")
        .with_aliases(vec![
            Arc::from("argv"),
            Arc::from("args"),
            Arc::from("command_args"),
        ])
        .with_description("Возвращает аргументы командной строки")
        .returns(TypeKind::Array(Box::new(TypeKind::String)))
        .with_handler(|_args| {
            let args: Vec<Value> = std::env::args().map(Value::String).collect();
            Ok(Value::Array(args))
        })
}

pub fn exe_path_fn() -> LibFunctionDef {
    LibFunctionDef::new("путь_к_исполняемому")
        .with_aliases(vec![Arc::from("exe_path"), Arc::from("executable")])
        .with_description("Возвращает путь к текущему исполняемому файлу")
        .returns(TypeKind::String)
        .with_handler(|_args| {
            std::env::current_exe()
                .map(|p| Value::String(p.to_string_lossy().to_string()))
                .map_err(|e| format!("Не удалось получить путь: {}", e))
        })
}

pub fn system_info_fn() -> LibFunctionDef {
    LibFunctionDef::new("информация_о_системе")
        .with_aliases(vec![Arc::from("system_info"), Arc::from("sysinfo")])
        .with_description("Возвращает словарь с информацией о системе")
        .returns(TypeKind::Map(
            Box::new(TypeKind::String),
            Box::new(TypeKind::Any),
        ))
        .with_handler(|_args| {
            let mut map = BTreeMap::new();

            let os_name = if cfg!(windows) {
                "windows"
            } else if cfg!(target_os = "macos") {
                "macos"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else {
                "unknown"
            };
            map.insert(
                Value::String("os".into()),
                Value::String(os_name.to_string()),
            );

            let family = if cfg!(unix) {
                "unix"
            } else if cfg!(windows) {
                "windows"
            } else {
                "unknown"
            };
            map.insert(
                Value::String("family".into()),
                Value::String(family.to_string()),
            );

            map.insert(
                Value::String("arch".into()),
                Value::String(std::env::consts::ARCH.to_string()),
            );

            let cores = std::thread::available_parallelism()
                .map(|n| n.get() as i64)
                .unwrap_or(1);
            map.insert(
                Value::String("cores".into()),
                Value::Number(Number::I64(cores)),
            );

            let user = if cfg!(windows) {
                std::env::var("USERNAME").unwrap_or_default()
            } else {
                std::env::var("USER").unwrap_or_default()
            };
            map.insert(Value::String("user".into()), Value::String(user));

            let hostname = if cfg!(windows) {
                std::env::var("COMPUTERNAME").unwrap_or_default()
            } else {
                std::env::var("HOSTNAME").unwrap_or_default()
            };
            map.insert(Value::String("hostname".into()), Value::String(hostname));

            let pid = std::process::id();
            map.insert(
                Value::String("pid".into()),
                Value::Number(Number::U64(pid as u64)),
            );

            Ok(Value::Map(map))
        })
}

/// pid() -> нат_64
pub fn pid_fn() -> LibFunctionDef {
    LibFunctionDef::new("идентификатор_процесса")
        .with_aliases(vec![Arc::from("pid"), Arc::from("process_id")])
        .with_description("Возвращает ID текущего процесса")
        .returns(TypeKind::UInt64)
        .with_handler(|_args| Ok(Value::Number(Number::U64(std::process::id() as u64))))
}
