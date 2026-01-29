use std::path::PathBuf;

use clap::{Parser, Subcommand};

// =============================================================================
//                              CLI ARGUMENTS
// =============================================================================

#[derive(Parser)]
#[command(
    name = "kumir3",
    author = "Vadim Khristenko <just@vai-prog.ru>",
    version = env!("CARGO_PKG_VERSION"),
    about = "Kumir 3 Interpreter - Интерпретатор языка Кумир",
    long_about = None,
    styles = get_styles(),
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Файл программы .kum для выполнения
    #[arg(value_name = "ФАЙЛ")]
    pub file: Option<PathBuf>,

    /// Режим отладки
    #[arg(short, long)]
    pub debug: bool,

    /// Измерять время выполнения
    #[arg(short, long)]
    pub time: bool,

    /// Аргументы для программы
    #[arg(last = true)]
    pub args: Vec<String>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Запустить файл .kum
    #[command(name = "run", visible_alias = "запуск")]
    Run {
        file: PathBuf,
        #[arg(short, long)]
        debug: bool,
        #[arg(short, long)]
        time: bool,
    },

    /// Интерактивный режим (REPL)
    #[command(name = "repl", visible_alias = "консоль")]
    Repl {
        #[arg(short, long)]
        debug: bool,
    },

    /// Проверить синтаксис файла
    #[command(name = "check", visible_alias = "проверка")]
    Check { file: PathBuf },

    /// Показать AST программы
    #[command(name = "ast", visible_alias = "дерево")]
    Ast { file: PathBuf },

    /// Информация о версии
    #[command(name = "info", visible_alias = "инфо")]
    Info,
}

fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .header(clap::builder::styling::AnsiColor::Cyan.on_default().bold())
        .usage(clap::builder::styling::AnsiColor::Cyan.on_default().bold())
        .literal(clap::builder::styling::AnsiColor::Green.on_default())
        .placeholder(clap::builder::styling::AnsiColor::Yellow.on_default())
}

impl Cli {
    pub fn file(&self) -> &PathBuf {
        self.file.as_ref().unwrap_or_else(|| {
            eprintln!("не указан файл");
            std::process::exit(1);
        })
    }
}
