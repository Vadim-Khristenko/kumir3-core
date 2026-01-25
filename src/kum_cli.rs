//! Kumir 3 CLI - простой консольный интерпретатор без TUI.
//! 
//! Использование:
//! - kumir3-cli <файл.kum>   - выполнить файл
//! - kumir3-cli --help       - показать справку

use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use clap::Parser;
use kumir3_corelib::interpreter::Interpreter;

#[derive(Parser)]
#[command(
    name = "kumir3-cli",
    author = "Vadim Khristenko <just@vai-prog.ru>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Kumir 3 CLI - Простой консольный интерпретатор",
)]
struct Cli {
    /// Файл программы .kum для выполнения
    #[arg(value_name = "ФАЙЛ")]
    file: PathBuf,

    /// Режим отладки
    #[arg(short, long)]
    debug: bool,

    /// Измерять время выполнения
    #[arg(short, long)]
    time: bool,
}

fn main() {
    let cli = Cli::parse();
    
    // Читаем файл
    let source = match fs::read_to_string(&cli.file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Ошибка чтения файла '{}': {}", cli.file.display(), e);
            std::process::exit(1);
        }
    };
    
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