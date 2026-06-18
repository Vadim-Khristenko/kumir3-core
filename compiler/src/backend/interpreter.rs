// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Интерпретатор IR (для отладки)
//!
//! Выполняет IR напрямую без компиляции в нативный код.
//! Полезен для быстрого тестирования и отладки компилятора.

use super::Backend;
use shared::codegen::ir::IrModule;
use std::path::Path;

/// Бэкенд-интерпретатор IR.
pub struct InterpreterBackend {
    debug: bool,
}

impl InterpreterBackend {
    /// Создаёт новый интерпретатор IR.
    pub fn new() -> Self {
        Self { debug: false }
    }

    /// Включает режим отладки.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Выполняет IR модуль.
    pub fn execute(&self, module: &IrModule) -> Result<(), String> {
        if self.debug {
            eprintln!("[IR Interpreter] Выполнение модуля...");
            eprintln!("[IR Interpreter] Функций: {}", module.functions.len());
        }

        // TODO: реализация интерпретатора IR
        // - Создать стек вызовов
        // - Найти функцию main/Главный
        // - Выполнить инструкции
        // - Обработать вызовы функций
        // - Управление памятью (переменные)

        Err("IR интерпретатор пока не реализован".to_string())
    }
}

impl Backend for InterpreterBackend {
    fn generate(&self, module: &IrModule) -> Result<String, String> {
        // Интерпретатор не генерирует код, возвращаем текстовое представление IR
        Ok(format!("{:#?}", module))
    }

    fn compile(&self, _code: &str, _output: &Path) -> Result<(), String> {
        // Интерпретатор не компилирует в файл
        Err("Интерпретатор IR не создаёт исполняемые файлы".to_string())
    }
}

impl Default for InterpreterBackend {
    fn default() -> Self {
        Self::new()
    }
}
