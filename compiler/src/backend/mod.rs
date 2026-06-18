// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Бэкенды компиляции для Kumir 3
//!
//! Каждый бэкенд преобразует IR в конечный формат:
//! - InterpreterBackend: выполняет IR напрямую (для отладки)
//! - RustBackend: генерирует Rust код и компилирует через rustc
//! - WasmBackend: генерирует WebAssembly (TODO)

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
