//! Функции получения информации о системе

use std::collections::BTreeMap;

use crate::types::library::LibFunctionDef;
use crate::types::type_spec::TypeSpec;
use crate::types::{Number, Value};

/// имя_ос() -> лит
pub fn os_name_fn() -> LibFunctionDef {
    LibFunctionDef::new("имя_ос")
        .with_aliases(&["os_name", "platform", "система"])
        .with_description("Возвращает имя операционной системы (windows, linux, macos)")
        .returns(TypeSpec::String)
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

/// семейство_ос() -> лит
pub fn os_family_fn() -> LibFunctionDef {
    LibFunctionDef::new("семейство_ос")
        .with_aliases(&["os_family"])
        .with_description("Возвращает семейство ОС (unix, windows)")
        .returns(TypeSpec::String)
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

/// архитектура() -> лит
pub fn arch_fn() -> LibFunctionDef {
    LibFunctionDef::new("архитектура")
        .with_aliases(&["arch", "architecture", "cpu_arch"])
        .with_description("Возвращает архитектуру процессора (x86_64, aarch64 и т.д.)")
        .returns(TypeSpec::String)
        .with_handler(|_args| Ok(Value::String(std::env::consts::ARCH.to_string())))
}

/// имя_хоста() -> лит
pub fn hostname_fn() -> LibFunctionDef {
    LibFunctionDef::new("имя_хоста")
        .with_aliases(&["hostname", "host"])
        .with_description("Возвращает имя компьютера")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            // Используем переменные окружения для кроссплатформенности
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

/// имя_пользователя() -> лит
pub fn username_fn() -> LibFunctionDef {
    LibFunctionDef::new("имя_пользователя")
        .with_aliases(&["username", "user", "getuser", "whoami"])
        .with_description("Возвращает имя текущего пользователя")
        .returns(TypeSpec::String)
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

/// разделитель_пути() -> лит
pub fn path_sep_fn() -> LibFunctionDef {
    LibFunctionDef::new("разделитель_пути")
        .with_aliases(&["path_sep", "pathsep"])
        .with_description("Возвращает системный разделитель пути (/ или \\)")
        .returns(TypeSpec::String)
        .with_handler(|_args| Ok(Value::String(std::path::MAIN_SEPARATOR.to_string())))
}

/// разделитель_путей() -> лит
pub fn pathlist_sep_fn() -> LibFunctionDef {
    LibFunctionDef::new("разделитель_путей")
        .with_aliases(&["pathlist_sep", "path_delimiter"])
        .with_description("Возвращает разделитель в списке путей (; на Windows, : на Unix)")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            let sep = if cfg!(windows) { ";" } else { ":" };
            Ok(Value::String(sep.to_string()))
        })
}

/// разделитель_строк() -> лит
pub fn line_sep_fn() -> LibFunctionDef {
    LibFunctionDef::new("разделитель_строк")
        .with_aliases(&["line_sep", "linesep", "newline"])
        .with_description("Возвращает системный разделитель строк (\\n или \\r\\n)")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            let sep = if cfg!(windows) { "\r\n" } else { "\n" };
            Ok(Value::String(sep.to_string()))
        })
}

/// количество_ядер() -> цел
pub fn cpu_count_fn() -> LibFunctionDef {
    LibFunctionDef::new("количество_ядер")
        .with_aliases(&["cpu_count", "num_cpus", "cores"])
        .with_description("Возвращает количество логических ядер процессора")
        .returns(TypeSpec::UInt64)
        .with_handler(|_args| {
            // Используем thread::available_parallelism как безопасный способ
            let count = std::thread::available_parallelism()
                .map(|n| n.get() as u64)
                .unwrap_or(1);
            Ok(Value::Number(Number::U64(count)))
        })
}

/// аргументы_командной_строки() -> массив лит
pub fn argv_fn() -> LibFunctionDef {
    LibFunctionDef::new("аргументы_командной_строки")
        .with_aliases(&["argv", "args", "command_args"])
        .with_description("Возвращает аргументы командной строки")
        .returns(TypeSpec::Array(Box::new(TypeSpec::String)))
        .with_handler(|_args| {
            let args: Vec<Value> = std::env::args().map(Value::String).collect();
            Ok(Value::Array(args))
        })
}

/// путь_к_исполняемому() -> лит
pub fn exe_path_fn() -> LibFunctionDef {
    LibFunctionDef::new("путь_к_исполняемому")
        .with_aliases(&["exe_path", "executable"])
        .with_description("Возвращает путь к текущему исполняемому файлу")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            std::env::current_exe()
                .map(|p| Value::String(p.to_string_lossy().to_string()))
                .map_err(|e| format!("Не удалось получить путь к исполняемому файлу: {}", e))
        })
}

/// информация_о_системе() -> словарь
pub fn system_info_fn() -> LibFunctionDef {
    LibFunctionDef::new("информация_о_системе")
        .with_aliases(&["system_info", "sysinfo"])
        .with_description("Возвращает словарь с информацией о системе")
        .returns(TypeSpec::Map(Box::new(TypeSpec::String), Box::new(TypeSpec::Any)))
        .with_handler(|_args| {
            let mut map = BTreeMap::new();
            
            // ОС
            let os_name = if cfg!(windows) {
                "windows"
            } else if cfg!(target_os = "macos") {
                "macos"
            } else if cfg!(target_os = "linux") {
                "linux"
            } else {
                "unknown"
            };
            map.insert(Value::String("os".into()), Value::String(os_name.to_string()));
            
            // Семейство
            let family = if cfg!(unix) { "unix" } else if cfg!(windows) { "windows" } else { "unknown" };
            map.insert(Value::String("family".into()), Value::String(family.to_string()));
            
            // Архитектура
            map.insert(Value::String("arch".into()), Value::String(std::env::consts::ARCH.to_string()));
            
            // Количество ядер
            let cores = std::thread::available_parallelism()
                .map(|n| n.get() as i64)
                .unwrap_or(1);
            map.insert(Value::String("cores".into()), Value::Number(Number::I64(cores)));
            
            // Пользователь
            let user = if cfg!(windows) {
                std::env::var("USERNAME").unwrap_or_default()
            } else {
                std::env::var("USER").unwrap_or_default()
            };
            map.insert(Value::String("user".into()), Value::String(user));
            
            // Имя хоста
            let hostname = if cfg!(windows) {
                std::env::var("COMPUTERNAME").unwrap_or_default()
            } else {
                std::env::var("HOSTNAME").unwrap_or_default()
            };
            map.insert(Value::String("hostname".into()), Value::String(hostname));
            
            Ok(Value::Map(map))
        })
}
