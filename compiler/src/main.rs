// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Компилятор языка Кумир 3
//!
//! Компилирует программы на языке Кумир в различные форматы:
//! - Нативные исполняемые файлы (через Rust backend)
//! - WebAssembly модули
//! - Интерпретируемый IR (для отладки)

use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand, ValueEnum};

use kumir3_compiler::Compiler;

// =============================================================================
//                           CLI АРГУМЕНТЫ
// =============================================================================

#[derive(Parser)]
#[command(name = "kumir3c")]
#[command(about = "Компилятор языка Кумир 3", long_about = None)]
#[command(version)]
struct Cli {
    /// Входной файл .kum
    #[arg(value_name = "FILE")]
    input: PathBuf,

    /// Выходной файл
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Тип выходного файла
    #[arg(short = 't', long, value_enum, default_value = "exe")]
    target: Target,

    /// Уровень оптимизации (0-3)
    #[arg(short = 'O', long, default_value = "0")]
    opt_level: u8,

    /// Режим отладки (добавляет отладочную информацию)
    #[arg(short, long)]
    debug: bool,

    /// Вывести IR (промежуточное представление)
    #[arg(long)]
    emit_ir: bool,

    /// Вывести сгенерированный Rust код
    #[arg(long)]
    emit_rust: bool,

    /// Только проверить синтаксис, не компилировать
    #[arg(long)]
    check: bool,

    /// Подкоманды
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Собрать проект из kumir.toml
    Build {
        /// Путь к директории проекта
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Release сборка (с оптимизациями)
        #[arg(short, long)]
        release: bool,
    },

    /// Запустить программу после компиляции
    Run {
        /// Входной файл .kum
        file: PathBuf,

        /// Аргументы для программы
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Показать информацию о компиляторе
    Info,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Target {
    /// Нативный исполняемый файл
    Exe,
    /// WebAssembly модуль
    Wasm,
    /// IR (промежуточное представление)
    Ir,
    /// Rust исходный код
    Rust,
}

// =============================================================================
//                           MAIN
// =============================================================================

fn main() {
    let cli = Cli::parse();

    // Обработка подкоманд
    if let Some(command) = cli.command {
        match command {
            Commands::Build { path, release } => {
                if let Err(e) = build_project(&path, release) {
                    eprintln!("Ошибка сборки: {}", e);
                    process::exit(1);
                }
            }
            Commands::Run { file, args } => {
                if let Err(e) = run_program(&file, &args) {
                    eprintln!("Ошибка выполнения: {}", e);
                    process::exit(1);
                }
            }
            Commands::Info => {
                print_info();
            }
        }
        return;
    }

    // Основная компиляция
    if let Err(e) = compile(&cli) {
        eprintln!("Ошибка компиляции: {}", e);
        process::exit(1);
    }
}

// =============================================================================
//                           ФУНКЦИИ
// =============================================================================

fn compile(cli: &Cli) -> Result<(), String> {
    // Создаём компилятор
    let mut compiler = Compiler::new();
    compiler.set_debug(cli.debug);
    compiler.set_opt_level(cli.opt_level);

    // Читаем исходный файл
    let source = std::fs::read_to_string(&cli.input)
        .map_err(|e| format!("Не удалось прочитать файл '{}': {}", cli.input.display(), e))?;

    // Только проверка синтаксиса
    if cli.check {
        compiler.check(&source)?;
        println!("✓ Синтаксис корректен");
        return Ok(());
    }

    // Определяем выходной файл
    let output = cli.output.clone().unwrap_or_else(|| {
        let mut out = cli.input.clone();
        out.set_extension(match cli.target {
            Target::Exe => {
                if cfg!(windows) {
                    "exe"
                } else {
                    ""
                }
            }
            Target::Wasm => "wasm",
            Target::Ir => "ir",
            Target::Rust => "rs",
        });
        out
    });

    // Компилируем
    println!("Компиляция {} → {}", cli.input.display(), output.display());

    match cli.target {
        Target::Exe => {
            compiler.compile_to_exe(&source, &output)?;
        }
        Target::Wasm => {
            compiler.compile_to_wasm(&source, &output)?;
        }
        Target::Ir => {
            compiler.compile_to_ir(&source, &output)?;
        }
        Target::Rust => {
            compiler.compile_to_rust(&source, &output)?;
        }
    }

    // Дополнительные выводы
    if cli.emit_ir {
        let ir_path = output.with_extension("ir");
        compiler.emit_ir(&ir_path)?;
        println!("IR сохранён в {}", ir_path.display());
    }

    if cli.emit_rust {
        let rust_path = output.with_extension("rs");
        compiler.emit_rust(&rust_path)?;
        println!("Rust код сохранён в {}", rust_path.display());
    }

    println!("✓ Компиляция завершена");
    Ok(())
}

fn build_project(path: &Path, release: bool) -> Result<(), String> {
    println!("Сборка проекта в {}", path.display());
    if release {
        println!("Режим: Release (с оптимизациями)");
    } else {
        println!("Режим: Debug");
    }
    // TODO: реализация сборки проекта
    Err("Сборка проектов пока не реализована".to_string())
}

fn run_program(file: &Path, args: &[String]) -> Result<(), String> {
    println!("Запуск программы {}", file.display());
    if !args.is_empty() {
        println!("Аргументы: {:?}", args);
    }
    // TODO: компиляция и запуск
    Err("Запуск программ пока не реализован".to_string())
}

fn print_info() {
    println!("Компилятор Kumir 3");
    println!("Версия: {}", env!("CARGO_PKG_VERSION"));
    println!("Автор: Vadim Khristenko <just@vai-prog.ru>");
    println!();
    println!("Поддерживаемые цели компиляции:");
    println!("  - exe:  Нативный исполняемый файл");
    println!("  - wasm: WebAssembly модуль");
    println!("  - ir:   Промежуточное представление");
    println!("  - rust: Rust исходный код");
    println!();
    println!("Используемые компоненты:");
    println!("  - Лексер:  shared::lexer");
    println!("  - Парсер:  shared::parser");
    println!("  - IR:      shared::codegen::ir");
}
