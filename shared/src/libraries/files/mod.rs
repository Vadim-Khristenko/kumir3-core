//! Библиотека файловых операций для КуМир 3
//!
//! Предоставляет функции для работы с файловой системой:
//! - Чтение и запись текстовых/бинарных файлов
//! - Копирование, перемещение, удаление
//! - Работа с директориями
//! - Метаданные файлов
//!
//! Без внешних зависимостей, только std.

mod dir;
mod io;
mod meta;
mod ops;

use std::sync::Arc;

use crate::types::library::{LibVersion, LibraryDef};

pub use dir::*;
pub use io::*;
pub use meta::*;
pub use ops::*;

/// Создаёт библиотеку files
pub fn create_files_library() -> LibraryDef {
    let mut lib = LibraryDef::new("files", "Файлы");
    lib.aliases = vec![
        Arc::from("files"),
        Arc::from("файлы"),
        Arc::from("fs"),
        Arc::from("файловая_система"),
    ];
    lib.description = Some(Arc::from(
        "Чтение, запись, копирование файлов и работа с директориями",
    ));
    lib.author = Arc::from("Vadim Khristenko <just@vai-prog.ru>");
    lib.version = LibVersion::new(2, 0, 0);
    lib.stable = false;

    lib.functions = vec![
        // === Чтение/запись ===
        read_text_fn(),
        read_lines_fn(),
        write_text_fn(),
        append_text_fn(),
        read_bytes_fn(),
        write_bytes_fn(),
        write_lines_fn(),
        // === Файловые операции ===
        copy_file_fn(),
        move_file_fn(),
        remove_file_fn(),
        remove_dir_fn(),
        remove_dir_all_fn(),
        touch_fn(),
        symlink_fn(),
        // === Директории ===
        list_dir_fn(),
        list_files_fn(),
        list_dirs_fn(),
        make_dir_fn(),
        make_dirs_fn(),
        walk_dir_fn(),
        glob_ext_fn(),
        // === Метаданные ===
        exists_fn(),
        is_file_fn(),
        is_dir_fn(),
        is_symlink_fn(),
        file_size_fn(),
        stat_fn(),
        canonical_path_fn(),
        set_readonly_fn(),
    ];

    lib
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_library_creation() {
        let lib = create_files_library();
        assert_eq!(lib.id.as_ref(), "files");
        assert!(!lib.functions.is_empty());
        assert_eq!(lib.functions.len(), 29);
    }

    #[test]
    fn test_read_write_text() {
        let dir = std::env::temp_dir();
        let path = dir.join("kumir_test_rw.txt");
        let path_str = path.to_string_lossy().to_string();

        // Запись
        let wf = write_text_fn();
        wf.call(&[
            crate::types::Value::String(path_str.clone()),
            crate::types::Value::String("Привет мир".to_string()),
        ])
        .unwrap();

        // Чтение
        let rf = read_text_fn();
        let result = rf
            .call(&[crate::types::Value::String(path_str.clone())])
            .unwrap();

        match result {
            crate::types::Value::String(s) => assert_eq!(s.as_str(), "Привет мир"),
            _ => panic!("Expected String"),
        }

        // Дозапись
        let af = append_text_fn();
        af.call(&[
            crate::types::Value::String(path_str.clone()),
            crate::types::Value::String("!".to_string()),
        ])
        .unwrap();

        let result2 = rf
            .call(&[crate::types::Value::String(path_str.clone())])
            .unwrap();

        match result2 {
            crate::types::Value::String(s) => assert_eq!(s.as_str(), "Привет мир!"),
            _ => panic!("Expected String"),
        }

        // Очистка
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_exists_and_meta() {
        let dir = std::env::temp_dir();
        let path = dir.join("kumir_test_meta.txt");
        let path_str = path.to_string_lossy().to_string();

        // Создаём файл
        {
            let mut f = std::fs::File::create(&path).unwrap();
            f.write_all(b"test").unwrap();
        }

        let ef = exists_fn();
        let result = ef
            .call(&[crate::types::Value::String(path_str.clone())])
            .unwrap();
        assert_eq!(result, crate::types::Value::Boolean(true));

        let is_f = is_file_fn();
        let result = is_f
            .call(&[crate::types::Value::String(path_str.clone())])
            .unwrap();
        assert_eq!(result, crate::types::Value::Boolean(true));

        let sf = file_size_fn();
        let result = sf
            .call(&[crate::types::Value::String(path_str.clone())])
            .unwrap();
        match result {
            crate::types::Value::Number(n) => assert_eq!(n.to_i64(), Some(4)),
            _ => panic!("Expected Number"),
        }

        // Очистка
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_mkdir_and_list() {
        let dir = std::env::temp_dir().join("kumir_test_dir");
        let dir_str = dir.to_string_lossy().to_string();
        let _ = std::fs::remove_dir_all(&dir);

        // Создаём
        let mf = make_dir_fn();
        mf.call(&[crate::types::Value::String(dir_str.clone())])
            .unwrap();

        // Проверяем
        let idf = is_dir_fn();
        let result = idf
            .call(&[crate::types::Value::String(dir_str.clone())])
            .unwrap();
        assert_eq!(result, crate::types::Value::Boolean(true));

        // Создаём файл внутри
        std::fs::write(dir.join("a.txt"), "A").unwrap();

        let lf = list_dir_fn();
        let result = lf
            .call(&[crate::types::Value::String(dir_str.clone())])
            .unwrap();
        match result {
            crate::types::Value::Array(arr) => assert_eq!(arr.len(), 1),
            _ => panic!("Expected Array"),
        }

        // Очистка
        let _ = std::fs::remove_dir_all(&dir);
    }
}
