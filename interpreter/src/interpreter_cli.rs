//! Simple CLI interpreter for Kumir 3 without TUI.

use std::fs;
use std::io;
use std::time::Instant;

use clap::Parser;
mod cli;
mod interpreter;
pub use cli::{Cli, Commands};
use interpreter::Interpreter;

/// Stack reserved for the thread that runs the program.
///
/// The evaluator recurses through the expression tree, so a single КуМир call
/// costs tens of kilobytes of native stack. On the default 1 MiB main-thread
/// stack that ran out at about twenty nested calls — long before the
/// interpreter's own depth guard ([`Environment::set_max_call_depth`], 1000
/// frames) could report anything, and the process died without flushing the
/// output it had already produced.
///
/// Recursion is a third-lesson topic in КуМир, so the limit a student meets must
/// be the guard with its Russian message, not a stack overflow. The reservation
/// is virtual address space: pages are committed only as they are touched.
const STACK_SIZE: usize = 512 * 1024 * 1024;

fn main() {
    let worker = std::thread::Builder::new()
        .name("исполнение".to_string())
        .stack_size(STACK_SIZE)
        .spawn(run)
        .expect("поток исполнения создаётся");

    // A panic inside the program thread has already printed its own report;
    // exiting non-zero keeps that visible to the caller instead of pretending
    // the run succeeded.
    if worker.join().is_err() {
        std::process::exit(101);
    }
}

fn run() {
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
