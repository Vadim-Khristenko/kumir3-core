//! Константы языка Кумир
//!
//! Этот модуль организует все константы языка:
//! - Ключевые слова (keywords)
//! - Операторы (operators)
//! - Математические константы (math)
//! - Встроенные функции (builtins)
//! - Утилиты для работы с идентификаторами (ident)

pub mod builtins;
pub mod ident;
pub mod keywords;
pub mod math;
pub mod operators;

// Реэкспорт для удобства
pub use builtins::*;
pub use ident::*;
pub use keywords::*;
pub use math::*;
pub use operators::*;
