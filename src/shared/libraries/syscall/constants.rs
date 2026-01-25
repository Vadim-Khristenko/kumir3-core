//! Константы для библиотеки системных вызовов

use crate::shared::types::library::LibConstantDef;
use crate::shared::types::type_spec::TypeSpec;
use crate::shared::types::Value;

// ============================================================================
// КОНСТАНТЫ ОС
// ============================================================================

/// Константа: WINDOWS
pub fn const_windows() -> LibConstantDef {
    LibConstantDef {
        name: "WINDOWS",
        aliases: &["WIN", "ВИНДОВС"],
        const_type: TypeSpec::Bool,
        value: Value::Boolean(cfg!(windows)),
        description: "Истина, если текущая ОС - Windows",
    }
}

/// Константа: LINUX
pub fn const_linux() -> LibConstantDef {
    LibConstantDef {
        name: "LINUX",
        aliases: &["ЛИНУКС"],
        const_type: TypeSpec::Bool,
        value: Value::Boolean(cfg!(target_os = "linux")),
        description: "Истина, если текущая ОС - Linux",
    }
}

/// Константа: MACOS
pub fn const_macos() -> LibConstantDef {
    LibConstantDef {
        name: "MACOS",
        aliases: &["MAC", "DARWIN", "МАКОС"],
        const_type: TypeSpec::Bool,
        value: Value::Boolean(cfg!(target_os = "macos")),
        description: "Истина, если текущая ОС - macOS",
    }
}

/// Константа: UNIX
pub fn const_unix() -> LibConstantDef {
    LibConstantDef {
        name: "UNIX",
        aliases: &["ЮНИКС"],
        const_type: TypeSpec::Bool,
        value: Value::Boolean(cfg!(unix)),
        description: "Истина, если текущая ОС относится к семейству Unix",
    }
}

/// Константа: РАЗДЕЛИТЕЛЬ_ПУТИ
pub fn const_path_sep() -> LibConstantDef {
    LibConstantDef {
        name: "РАЗДЕЛИТЕЛЬ_ПУТИ",
        aliases: &["PATH_SEP", "SEP"],
        const_type: TypeSpec::String,
        value: Value::String(std::path::MAIN_SEPARATOR.to_string()),
        description: "Системный разделитель пути (/ или \\)",
    }
}

/// Константа: РАЗДЕЛИТЕЛЬ_СТРОК
pub fn const_line_sep() -> LibConstantDef {
    let sep = if cfg!(windows) { "\r\n" } else { "\n" };
    LibConstantDef {
        name: "РАЗДЕЛИТЕЛЬ_СТРОК",
        aliases: &["LINE_SEP", "LINESEP", "NL"],
        const_type: TypeSpec::String,
        value: Value::String(sep.to_string()),
        description: "Системный разделитель строк (\\n или \\r\\n)",
    }
}

/// Константа: РАЗДЕЛИТЕЛЬ_ПУТЕЙ
pub fn const_pathlist_sep() -> LibConstantDef {
    let sep = if cfg!(windows) { ";" } else { ":" };
    LibConstantDef {
        name: "РАЗДЕЛИТЕЛЬ_ПУТЕЙ",
        aliases: &["PATHLIST_SEP"],
        const_type: TypeSpec::String,
        value: Value::String(sep.to_string()),
        description: "Разделитель в списке путей (; на Windows, : на Unix)",
    }
}

/// Константа: АРХИТЕКТУРА
pub fn const_arch() -> LibConstantDef {
    LibConstantDef {
        name: "АРХИТЕКТУРА",
        aliases: &["ARCH"],
        const_type: TypeSpec::String,
        value: Value::String(std::env::consts::ARCH.to_string()),
        description: "Архитектура процессора (x86_64, aarch64 и т.д.)",
    }
}
