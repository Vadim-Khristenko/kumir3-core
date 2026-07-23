//! Simple CLI interpreter for Kumir 3 without TUI.

use std::fs;
use std::io;
use std::time::Instant;

use clap::Parser;
mod cli;
mod interpreter;
pub use cli::{Cli, Commands};
use interpreter::Interpreter;

fn main() {
    let cli = Cli::parse();

    let file = cli.file();

    let source = fs::read_to_string(file)
        .map_err(|e| {
            eprintln!("Ошибка чтения файла '{}': {}", file.display(), e);
            std::process::exit(1);
        })
        .unwrap();

    let mut interpreter = Interpreter::new();

    // [KITE 5] Base directory for imports is the script's directory, so that
    // `использовать "neighbor.kum"` works regardless of the current working directory.
    if let Some(dir) = file.parent()
        && !dir.as_os_str().is_empty()
    {
        interpreter.set_base_dir(dir);
    }

    if cli.debug {
        interpreter.set_debug_mode(true);
    }

    if cli.strict {
        interpreter.set_strict(true);
    }

    let start = Instant::now();
    match interpreter.run(&source) {
        Ok(_) => {
            let output = interpreter.get_output();
            if !output.is_empty() {
                print!("{}", output);
            }

            // [W0] Warnings (e.g., undeclared variables) are printed to stderr
            // to keep them separate from program output.
            for warning in interpreter.warnings() {
                eprintln!("Предупреждение: {}", warning);
            }

            if cli.time {
                eprintln!("\n[Время выполнения: {:?}]", start.elapsed());
            }
        }
        Err(e) => {
            // [KITE 7] Output produced before the error has already happened from the program's
            // perspective, so it is printed even on failure. Without it, students don't see how
            // far execution got, and the test corpus runner cannot check partial output
            // (KITE 18 § 3.7, item 4).
            let output = interpreter.get_output();
            if !output.is_empty() {
                print!("{}", output);
                let _ = io::Write::flush(&mut io::stdout());
            }
            eprintln!("Ошибка выполнения: {}", e);
            if cli.error_kind {
                eprintln!("[вид ошибки] {:?}", e.kind);
            }
            std::process::exit(1);
        }
    }
}
