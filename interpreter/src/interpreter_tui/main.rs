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
mod file_runner;
mod info;
mod repl;
mod terminal;
mod ui;

use cli::{Cli, Commands};

// =============================================================================
//                              MAIN
// =============================================================================

fn main() {
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
