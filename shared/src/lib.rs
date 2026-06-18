// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

pub mod codegen;
pub mod constants; // Константы языка (constants/)
pub mod f128;
pub mod iostream;
pub mod lexer;
pub mod libraries; // Определения библиотек (libraries/)
pub mod math;
pub mod parser;
pub mod runtime; // Runtime прослойка (async, callbacks, events)
pub mod strings;
pub mod types; // Система типов (types/)
pub mod typesys; // Движок системы типов (правила, вывод, операции) — KITE 10
/// Кодогенерация (IR, Rust-блоки, бэкенды компилятора)
// Для обратной совместимости: повторно экспортируем все типы из types
pub use types::*;
