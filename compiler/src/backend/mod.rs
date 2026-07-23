// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Kumir 3 compilation backends.
//!
//! Each backend transforms IR to a final format:
//! - InterpreterBackend: executes IR directly (for debugging)
//! - RustBackend: generates Rust code and compiles via rustc
//! - WasmBackend: generates WebAssembly (TODO)

use shared::codegen::ir::IrModule;
use std::path::Path;

mod interpreter;
mod rust;

pub use interpreter::InterpreterBackend;
pub use rust::RustBackend;

// =============================================================================
//                           BACKEND TRAIT
// =============================================================================

/// Trait для бэкендов компиляции.
pub trait Backend {
    /// Генерирует код из IR модуля.
    fn generate(&self, module: &IrModule) -> Result<String, String>;

    /// Компилирует сгенерированный код в исполняемый файл.
    fn compile(&self, code: &str, output: &Path) -> Result<(), String>;
}
