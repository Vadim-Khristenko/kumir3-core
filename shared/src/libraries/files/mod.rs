//! Библиотека работы с файлами и директориями для КуМир 3
//!
//! Возможности:
//! - Чтение/запись текста и байтов
//! - Копирование, перемещение, удаление
//! - Создание директорий
//! - Получение метаданных и размеров
//! - Перечисление содержимого
//!
//! Без внешних зависимостей, только std.

mod io;
mod ops;
mod dir;
mod meta;

use crate::types::library::{LibraryDef, LibVersion};

// Реэкспорт
pub use io::*;
pub use ops::*;
pub use dir::*;
pub use meta::*;

/// Создаёт библиотеку файловой системы
pub fn create_files_library() -> LibraryDef {
    let mut lib = LibraryDef::new("files", "Файлы и директории");
    lib.aliases = &["files", "fs", "файлы", "фс"];
    lib.description = "Чтение/запись файлов, копирование, директории, метаданные";
    lib.author = "GitHub Copilot";
    lib.version = LibVersion::new(1, 0, 0);
    lib.stable = false;

    lib.functions = vec![
        // IO
        read_text_fn(),
        read_lines_fn(),
        write_text_fn(),
        append_text_fn(),
        read_bytes_fn(),
        write_bytes_fn(),
        
        // Операции
        copy_fn(),
        move_fn(),
        remove_file_fn(),
        remove_dir_fn(),
        remove_dir_all_fn(),
        touch_fn(),
        
        // Директории
        list_dir_fn(),
        list_files_fn(),
        make_dir_fn(),
        make_dirs_fn(),
        
        // Метаданные
        exists_fn(),
        is_file_fn(),
        is_dir_fn(),
        size_fn(),
        stat_fn(),
    ];

    lib
}
