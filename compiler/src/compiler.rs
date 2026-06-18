// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Главный модуль компилятора Kumir 3
//!
//! Координирует процесс компиляции:
//! 1. Лексический анализ (shared::lexer)
//! 2. Синтаксический анализ (shared::parser)
//! 3. Преобразование AST → IR
//! 4. Оптимизация IR
//! 5. Генерация кода через выбранный backend

use std::path::Path;

use shared::codegen::ir::IrModule;
use shared::lexer::tokenize;
use shared::parser::parse;
use shared::types::Program;

use crate::ast_to_ir::AstToIr;
use crate::backend::{Backend, RustBackend};
use crate::optimizer::IrOptimizer;

// =============================================================================
//                           КОМПИЛЯТОР
// =============================================================================

/// Компилятор Kumir 3.
pub struct Compiler {
    /// Режим отладки
    pub(crate) debug: bool,

    /// Уровень оптимизации (0-3)
    pub(crate) opt_level: u8,

    /// Последний скомпилированный IR модуль
    last_ir: Option<IrModule>,

    /// Последний сгенерированный Rust код
    last_rust: Option<String>,
}

impl Compiler {
    /// Создаёт новый компилятор.
    pub fn new() -> Self {
        Self {
            debug: false,
            opt_level: 0,
            last_ir: None,
            last_rust: None,
        }
    }

    /// Устанавливает режим отладки.
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    /// Устанавливает уровень оптимизации.
    pub fn set_opt_level(&mut self, level: u8) {
        self.opt_level = level.min(3);
    }

    // =========================================================================
    //                    ПРОВЕРКА СИНТАКСИСА
    // =========================================================================

    /// Проверяет синтаксис программы без компиляции.
    pub fn check(&self, source: &str) -> Result<(), String> {
        // Лексический анализ
        let tokens = tokenize(source).map_err(|e| format!("Ошибка лексера: {:?}", e))?;

        if self.debug {
            eprintln!("[DEBUG] Токенов: {}", tokens.len());
        }

        // Синтаксический анализ
        let _program = parse(source).map_err(|e| format!("Ошибка парсера: {:?}", e))?;

        if self.debug {
            eprintln!("[DEBUG] AST построен успешно");
        }

        Ok(())
    }

    // =========================================================================
    //                    КОМПИЛЯЦИЯ В РАЗНЫЕ ФОРМАТЫ
    // =========================================================================

    /// Компилирует в нативный исполняемый файл.
    pub fn compile_to_exe(&mut self, source: &str, output: &Path) -> Result<(), String> {
        // Парсим исходный код
        let program = self.parse(source)?;

        // Преобразуем в IR
        let ir_module = self.ast_to_ir(&program)?;
        self.last_ir = Some(ir_module.clone());

        // Оптимизируем IR
        let optimized = self.optimize_ir(ir_module)?;

        // Генерируем Rust код
        let rust_backend = RustBackend::new();
        let rust_code = rust_backend.generate(&optimized)?;
        self.last_rust = Some(rust_code.clone());

        // Компилируем Rust код в исполняемый файл
        rust_backend.compile_to_exe(&rust_code, output)?;

        Ok(())
    }

    /// Компилирует в WebAssembly модуль.
    pub fn compile_to_wasm(&mut self, source: &str, _output: &Path) -> Result<(), String> {
        let program = self.parse(source)?;
        let ir_module = self.ast_to_ir(&program)?;
        self.last_ir = Some(ir_module.clone());

        let _optimized = self.optimize_ir(ir_module)?;

        // TODO: WASM backend
        Err("WASM backend пока не реализован".to_string())
    }

    /// Компилирует в IR (промежуточное представление).
    pub fn compile_to_ir(&mut self, source: &str, output: &Path) -> Result<(), String> {
        let program = self.parse(source)?;
        let ir_module = self.ast_to_ir(&program)?;

        // Сохраняем IR в файл
        let ir_text = format!("{:#?}", ir_module);
        std::fs::write(output, ir_text).map_err(|e| format!("Не удалось записать IR: {}", e))?;

        self.last_ir = Some(ir_module);
        Ok(())
    }

    /// Компилирует в Rust исходный код.
    pub fn compile_to_rust(&mut self, source: &str, output: &Path) -> Result<(), String> {
        let program = self.parse(source)?;
        let ir_module = self.ast_to_ir(&program)?;
        self.last_ir = Some(ir_module.clone());

        let optimized = self.optimize_ir(ir_module)?;

        let rust_backend = RustBackend::new();
        let rust_code = rust_backend.generate(&optimized)?;

        std::fs::write(output, &rust_code)
            .map_err(|e| format!("Не удалось записать Rust код: {}", e))?;

        self.last_rust = Some(rust_code);
        Ok(())
    }

    // =========================================================================
    //                    ВСПОМОГАТЕЛЬНЫЕ МЕТОДЫ
    // =========================================================================

    /// Парсит исходный код в AST.
    fn parse(&self, source: &str) -> Result<Program, String> {
        if self.debug {
            eprintln!("[DEBUG] Парсинг исходного кода...");
        }

        let program = parse(source).map_err(|e| format!("Ошибка парсера: {:?}", e))?;

        if self.debug {
            eprintln!("[DEBUG] Алгоритмов: {}", program.algorithms.len());
            eprintln!("[DEBUG] Классов: {}", program.classes.len());
        }

        Ok(program)
    }

    /// Преобразует AST в IR.
    fn ast_to_ir(&self, program: &Program) -> Result<IrModule, String> {
        if self.debug {
            eprintln!("[DEBUG] Преобразование AST → IR...");
        }

        let mut converter = AstToIr::new();
        let ir_module = converter.convert(program)?;

        if self.debug {
            eprintln!("[DEBUG] IR функций: {}", ir_module.functions.len());
        }

        Ok(ir_module)
    }

    /// Оптимизирует IR модуль.
    fn optimize_ir(&self, module: IrModule) -> Result<IrModule, String> {
        if self.opt_level == 0 {
            return Ok(module);
        }

        if self.debug {
            eprintln!("[DEBUG] Оптимизация IR (уровень {})...", self.opt_level);
        }

        let optimizer = IrOptimizer::new(self.opt_level).with_debug(self.debug);
        let optimized = optimizer.optimize(module);

        Ok(optimized)
    }

    /// Сохраняет IR в файл.
    pub fn emit_ir(&self, path: &Path) -> Result<(), String> {
        let ir = self.last_ir.as_ref().ok_or("IR не был сгенерирован")?;

        let ir_text = format!("{:#?}", ir);
        std::fs::write(path, ir_text).map_err(|e| format!("Не удалось записать IR: {}", e))?;

        Ok(())
    }

    /// Сохраняет Rust код в файл.
    pub fn emit_rust(&self, path: &Path) -> Result<(), String> {
        let rust = self
            .last_rust
            .as_ref()
            .ok_or("Rust код не был сгенерирован")?;

        std::fs::write(path, rust).map_err(|e| format!("Не удалось записать Rust код: {}", e))?;

        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
