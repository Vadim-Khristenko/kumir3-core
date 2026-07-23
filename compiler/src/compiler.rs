// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Main Kumir 3 compiler module.
//!
//! Coordinates the compilation process:
//! 1. Lexical analysis (shared::lexer)
//! 2. Syntax analysis (shared::parser)
//! 3. AST → IR transformation
//! 4. IR optimization
//! 5. Code generation via chosen backend

use std::path::Path;

use shared::codegen::ir::IrModule;
use shared::lexer::tokenize;
use shared::parser::parse;
use shared::types::Program;

use crate::ast_to_ir::AstToIr;
use crate::backend::{Backend, RustBackend};
use crate::optimizer::IrOptimizer;

// =============================================================================
//                           COMPILER
// =============================================================================

/// Kumir 3 compiler.
pub struct Compiler {
    /// Debug mode
    pub(crate) debug: bool,

    /// Optimization level (0-3)
    pub(crate) opt_level: u8,

    /// Last compiled IR module
    last_ir: Option<IrModule>,

    /// Last generated Rust code
    last_rust: Option<String>,
}

impl Compiler {
    /// Creates a new compiler.
    pub fn new() -> Self {
        Self {
            debug: false,
            opt_level: 0,
            last_ir: None,
            last_rust: None,
        }
    }

    /// Sets debug mode.
    pub fn set_debug(&mut self, debug: bool) {
        self.debug = debug;
    }

    /// Sets optimization level.
    pub fn set_opt_level(&mut self, level: u8) {
        self.opt_level = level.min(3);
    }

    // =========================================================================
    //                    SYNTAX CHECK
    // =========================================================================

    /// Checks syntax without compilation.
    pub fn check(&self, source: &str) -> Result<(), String> {
        // Lexical analysis
        let tokens = tokenize(source).map_err(|e| format!("Ошибка лексера: {:?}", e))?;

        if self.debug {
            eprintln!("[DEBUG] Токенов: {}", tokens.len());
        }

        // Syntax analysis
        let _program = parse(source).map_err(|e| format!("Ошибка парсера: {:?}", e))?;

        if self.debug {
            eprintln!("[DEBUG] AST построен успешно");
        }

        Ok(())
    }

    // =========================================================================
    //                    COMPILATION TO VARIOUS FORMATS
    // =========================================================================

    /// Compiles to native executable.
    pub fn compile_to_exe(&mut self, source: &str, output: &Path) -> Result<(), String> {
        // Parse source code
        let program = self.parse(source)?;

        // Transform to IR
        let ir_module = self.ast_to_ir(&program)?;
        self.last_ir = Some(ir_module.clone());

        // Optimize IR
        let optimized = self.optimize_ir(ir_module)?;

        // Generate Rust code
        let rust_backend = RustBackend::new();
        let rust_code = rust_backend.generate(&optimized)?;
        self.last_rust = Some(rust_code.clone());

        // Compile Rust code to executable
        rust_backend.compile_to_exe(&rust_code, output)?;

        Ok(())
    }

    /// Compiles to WebAssembly module.
    pub fn compile_to_wasm(&mut self, source: &str, _output: &Path) -> Result<(), String> {
        let program = self.parse(source)?;
        let ir_module = self.ast_to_ir(&program)?;
        self.last_ir = Some(ir_module.clone());

        let _optimized = self.optimize_ir(ir_module)?;

        // TODO: WASM backend
        Err("WASM backend пока не реализован".to_string())
    }

    /// Compiles to IR (intermediate representation).
    pub fn compile_to_ir(&mut self, source: &str, output: &Path) -> Result<(), String> {
        let program = self.parse(source)?;
        let ir_module = self.ast_to_ir(&program)?;

        // Write IR to file
        let ir_text = format!("{:#?}", ir_module);
        std::fs::write(output, ir_text).map_err(|e| format!("Не удалось записать IR: {}", e))?;

        self.last_ir = Some(ir_module);
        Ok(())
    }

    /// Compiles to Rust source code.
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
