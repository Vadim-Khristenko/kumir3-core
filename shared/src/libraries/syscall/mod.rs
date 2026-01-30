//! Библиотека системных вызовов для КуМир 3
//!
//! Предоставляет функции для взаимодействия с операционной системой:
//! - Выполнение команд и процессов
//! - Работа с переменными окружения
//! - Операции с путями и директориями
//! - Получение системной информации
//!
//! Без внешних зависимостей, только std.
//!
//! ## Пример использования
//! ```kumir
//! использовать syscall
//!
//! вывод "ОС:", имя_ос()
//! вывод "Пользователь:", имя_пользователя()
//! вывод "Домашняя директория:", домашняя_директория()
//!
//! | Выполнение команды
//! результат := выполнить("echo Hello")
//! вывод "Вывод:", результат["stdout"]
//! ```

mod constants;
mod env;
mod path;
mod process;
mod sysinfo;

use crate::types::library::{LibraryDef, LibVersion};

// Реэкспорт внутренних модулей
pub use constants::*;
pub use env::*;
pub use path::*;
pub use process::*;
pub use sysinfo::*;

/// Создаёт библиотеку syscall
pub fn create_syscall_library() -> LibraryDef {
    let mut lib = LibraryDef::new("syscall", "Системные вызовы");
    lib.aliases = &["syscall", "sys", "системные_вызовы", "os"];
    lib.description = "Выполнение команд ОС, работа с окружением, путями и системной информацией";
    lib.author = "Vadim Khristenko <just@vai-prog.ru>";
    lib.version = LibVersion::new(2, 0, 0);
    lib.stable = false;

    // Регистрируем все функции
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
    ];

    // Регистрируем константы
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
