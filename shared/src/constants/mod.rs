//! Константы языка Кумир
//!
//! Этот модуль организует все константы языка:
//! - Ключевые слова (keywords)
//! - Операторы (operators)
//! - Математические константы (math)
//! - Встроенные функции (builtins)
//! - Сообщения об ошибках (errors)
//! - Утилиты для работы с идентификаторами (ident)

pub mod keywords;
pub mod operators;
pub mod math;
pub mod builtins;
pub mod errors;
pub mod ident;

// Реэкспорт для удобства
pub use keywords::*;
pub use operators::*;
pub use math::*;
pub use builtins::*;
pub use ident::*;

// errors реэкспортируется как модуль, а не содержимое
// Использование: constants::errors::UNEXPECTED_CHAR
// или: use crate::constants::errors::errors;
