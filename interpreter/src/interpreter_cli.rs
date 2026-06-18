//! Kumir 3 CLI - простой консольный интерпретатор без TUI.
//!
//! Использование:
//! - kumir3-cli <файл.kum>   - выполнить файл
//! - kumir3-cli --help       - показать справку

use std::fs;
use std::time::Instant;

use clap::Parser;
mod cli;
mod interpreter;
pub use cli::{Cli, Commands};
use interpreter::Interpreter;

fn main() {
    let cli = Cli::parse();

    let file = cli.file();

    // Читаем файл
    let source = fs::read_to_string(file)
        .map_err(|e| {
            eprintln!("Ошибка чтения файла '{}': {}", file.display(), e);
            std::process::exit(1);
        })
        .unwrap();

    // Создаём интерпретатор
    let mut interpreter = Interpreter::new();

    // [KITE 5] Базовая директория для импорта — папка скрипта, чтобы
    // `использовать "соседний.kum"` работало независимо от текущего каталога.
    if let Some(dir) = file.parent()
        && !dir.as_os_str().is_empty()
    {
        interpreter.set_base_dir(dir);
    }

    if cli.debug {
        interpreter.set_debug_mode(true);
    }

    // Выполняем
    let start = Instant::now();
    match interpreter.run(&source) {
        Ok(_) => {
            // Выводим результат
            let output = interpreter.get_output();
            if !output.is_empty() {
                print!("{}", output);
            }

            if cli.time {
                eprintln!("\n[Время выполнения: {:?}]", start.elapsed());
            }
        }
        Err(e) => {
            eprintln!("Ошибка выполнения: {}", e);
            std::process::exit(1);
        }
    }
}
