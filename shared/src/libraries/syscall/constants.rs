//! Константы для библиотеки системных вызовов

use std::sync::Arc;

use crate::types::library::LibConstantDef;
use crate::types::{TypeKind, Value};

pub fn const_windows() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("WINDOWS"),
        aliases: vec![Arc::from("WIN"), Arc::from("ВИНДОВС")],
        const_type: TypeKind::Bool,
        value: Value::Boolean(cfg!(windows)),
        description: Some(Arc::from("Истина, если текущая ОС — Windows")),
    }
}

pub fn const_linux() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("LINUX"),
        aliases: vec![Arc::from("ЛИНУКС")],
        const_type: TypeKind::Bool,
        value: Value::Boolean(cfg!(target_os = "linux")),
        description: Some(Arc::from("Истина, если текущая ОС — Linux")),
    }
}

pub fn const_macos() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("MACOS"),
        aliases: vec![Arc::from("MAC"), Arc::from("DARWIN"), Arc::from("МАКОС")],
        const_type: TypeKind::Bool,
        value: Value::Boolean(cfg!(target_os = "macos")),
        description: Some(Arc::from("Истина, если текущая ОС — macOS")),
    }
}

pub fn const_unix() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("UNIX"),
        aliases: vec![Arc::from("ЮНИКС")],
        const_type: TypeKind::Bool,
        value: Value::Boolean(cfg!(unix)),
        description: Some(Arc::from("Истина, если ОС из семейства Unix")),
    }
}

pub fn const_path_sep() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("РАЗДЕЛИТЕЛЬ_ПУТИ"),
        aliases: vec![Arc::from("PATH_SEP"), Arc::from("SEP")],
        const_type: TypeKind::String,
        value: Value::String(std::path::MAIN_SEPARATOR.to_string()),
        description: Some(Arc::from("Системный разделитель пути (/ или \\)")),
    }
}

pub fn const_line_sep() -> LibConstantDef {
    let sep = if cfg!(windows) { "\r\n" } else { "\n" };
    LibConstantDef {
        name: Arc::from("РАЗДЕЛИТЕЛЬ_СТРОК"),
        aliases: vec![Arc::from("LINE_SEP"), Arc::from("LINESEP"), Arc::from("NL")],
        const_type: TypeKind::String,
        value: Value::String(sep.to_string()),
        description: Some(Arc::from("Системный разделитель строк")),
    }
}

pub fn const_pathlist_sep() -> LibConstantDef {
    let sep = if cfg!(windows) { ";" } else { ":" };
    LibConstantDef {
        name: Arc::from("РАЗДЕЛИТЕЛЬ_ПУТЕЙ"),
        aliases: vec![Arc::from("PATHLIST_SEP")],
        const_type: TypeKind::String,
        value: Value::String(sep.to_string()),
        description: Some(Arc::from("Разделитель в PATH (; на Windows, : на Unix)")),
    }
}

pub fn const_arch() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("АРХИТЕКТУРА"),
        aliases: vec![Arc::from("ARCH")],
        const_type: TypeKind::String,
        value: Value::String(std::env::consts::ARCH.to_string()),
        description: Some(Arc::from("Архитектура процессора (x86_64, aarch64 и т.д.)")),
    }
}
