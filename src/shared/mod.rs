pub mod f128;
pub mod types;       // Система типов (types/)
pub mod constants;   // Константы языка (constants/)
pub mod libraries;   // Определения библиотек (libraries/)
pub mod runtime;     // Runtime прослойка (async, callbacks, events)
pub mod math;
pub mod strings;
pub mod iostream;
pub mod lexer;
pub mod parser;
pub mod codegen;     // Кодогенерация (IR, Rust-блоки, бэкенды компилятора)

// Для обратной совместимости: повторно экспортируем все типы из types
pub use types::*;