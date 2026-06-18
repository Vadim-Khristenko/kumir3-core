//! Библиотека системных вызовов для КуМир 3
//!
//! Предоставляет функции для взаимодействия с операционной системой:
//! - Выполнение команд и процессов
//! - Работа с переменными окружения
//! - Операции с путями и директориями
//! - Получение системной информации
//!
//! Без внешних зависимостей, только std.

mod constants;
mod env;
mod path;
mod process;
mod sysinfo;

use std::sync::Arc;

use crate::types::library::{LibVersion, LibraryDef};

pub use constants::*;
pub use env::*;
pub use path::*;
pub use process::*;
pub use sysinfo::*;

/// Создаёт библиотеку syscall
pub fn create_syscall_library() -> LibraryDef {
    let mut lib = LibraryDef::new("syscall", "Системные вызовы");
    lib.aliases = vec![
        Arc::from("syscall"),
        Arc::from("sys"),
        Arc::from("системные_вызовы"),
        Arc::from("os"),
    ];
    lib.description = Some(Arc::from(
        "Выполнение команд ОС, работа с окружением, путями и системной информацией",
    ));
    lib.author = Arc::from("Vadim Khristenko <just@vai-prog.ru>");
    lib.version = LibVersion::new(2, 0, 0);
    lib.stable = false;

    lib.functions = vec![
        // === Переменные окружения ===
        env_get_fn(),
        env_set_fn(),
        env_unset_fn(),
        env_all_fn(),
        env_exists_fn(),
        // === Выполнение команд ===
        run_fn(),
        system_fn(),
        popen_fn(),
        run_success_fn(),
        // === Пути и директории ===
        cwd_fn(),
        chdir_fn(),
        home_dir_fn(),
        temp_dir_fn(),
        path_exists_fn(),
        is_file_fn(),
        is_dir_fn(),
        abs_path_fn(),
        basename_fn(),
        dirname_fn(),
        extension_fn(),
        join_path_fn(),
        stem_fn(),
        // === Системная информация ===
        os_name_fn(),
        os_family_fn(),
        arch_fn(),
        hostname_fn(),
        username_fn(),
        path_sep_fn(),
        pathlist_sep_fn(),
        line_sep_fn(),
        cpu_count_fn(),
        argv_fn(),
        exe_path_fn(),
        system_info_fn(),
        pid_fn(),
    ];

    lib.constants = vec![
        const_windows(),
        const_linux(),
        const_macos(),
        const_unix(),
        const_path_sep(),
        const_line_sep(),
        const_pathlist_sep(),
        const_arch(),
    ];

    lib
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_creation() {
        let lib = create_syscall_library();
        assert_eq!(lib.id.as_ref(), "syscall");
        assert!(!lib.functions.is_empty());
        assert!(!lib.constants.is_empty());
    }

    #[test]
    fn test_os_name() {
        let f = os_name_fn();
        let result = f.call(&[]).unwrap();
        match result {
            crate::types::Value::String(s) => {
                assert!(["windows", "linux", "macos", "unknown"].contains(&s.as_str()));
            }
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_cwd() {
        let f = cwd_fn();
        let result = f.call(&[]).unwrap();
        match result {
            crate::types::Value::String(s) => assert!(!s.is_empty()),
            _ => panic!("Expected String"),
        }
    }
}
