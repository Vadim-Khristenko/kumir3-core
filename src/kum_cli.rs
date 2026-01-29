//! Kumir 3 CLI - простой консольный интерпретатор без TUI.
//!
//! Использование:
//! - kumir3-cli <файл.kum>   - выполнить файл
//! - kumir3-cli --help       - показать справку

use std::fs;
use std::time::Instant;

use clap::Parser;
use kumir3_corelib::interpreter::Interpreter;
pub use kumir3_corelib::interpreter::cli::{Cli, Commands};

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
