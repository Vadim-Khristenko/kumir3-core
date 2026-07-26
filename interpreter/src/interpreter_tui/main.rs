//! Kumir 3 Interpreter - TUI
//!
//! Интерпретатор языка Кумир с полным TUI интерфейсом:
//! - Выполнение .kum файлов
//! - Интерактивный режим (REPL)
//! - Отладка программ
//! - Проверка синтаксиса
//! - Просмотр AST

use clap::Parser;

#[path = "../cli.rs"]
mod cli;
#[path = "../interpreter/mod.rs"]
mod interpreter;

mod ast;
mod check;
mod draw;
mod editor;
mod file_runner;
mod info;
mod repl;
mod syntax;
mod terminal;
mod theme;
mod ui;

use cli::{Cli, Commands};

// =============================================================================
//                              MAIN
// =============================================================================

/// Stack reserved for the thread that runs programs.
///
/// Same reason as in `interpreter_cli`: a КуМир call costs tens of kilobytes of
/// native stack, so on the default main-thread stack recursion died at about
/// twenty frames. Here it matters even more — a stack overflow takes the whole
/// console down with it, terminal state and all.
const STACK_SIZE: usize = 512 * 1024 * 1024;

fn main() {
    let worker = std::thread::Builder::new()
        .name("исполнение".to_string())
        .stack_size(STACK_SIZE)
        .spawn(run)
        .expect("поток исполнения создаётся");

    if worker.join().is_err() {
        std::process::exit(101);
    }
}

fn run() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Run { file, debug, time }) => {
            crate::file_runner::run_file_tui(&file, debug, time)
        }
        Some(Commands::Repl { debug }) => crate::repl::run_repl_tui(debug),
        Some(Commands::Check { file }) => crate::check::run_check_tui(&file),
        Some(Commands::Ast { file }) => crate::ast::run_ast_tui(&file),
        Some(Commands::Info) => crate::info::run_info_tui(),
        None => {
            if let Some(file) = cli.file {
                crate::file_runner::run_file_tui(&file, cli.debug, cli.time)
            } else {
                crate::repl::run_repl_tui(cli.debug)
            }
        }
    };

    if let Err(e) = result {
        eprintln!("Ошибка: {}", e);
        std::process::exit(1);
    }
}
