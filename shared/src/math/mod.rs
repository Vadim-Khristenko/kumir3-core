// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Арифметическое ядро Кумир 3.
//!
//! Модуль разбит по зонам ответственности:
//! * `error` — тип ошибки [`MathErr`];
//! * `promote` — предикаты, конвертации и продвижение числовых типов;
//! * `arith` — базовые операции `+ - * / div mod **`;
//! * `strings` — строковые/агрегатные помощники этих операций;
//! * `funcs` — корни, округление, тригонометрия, модуль.
//!
//! Все методы принадлежат единому типу [`MathOperators`]: inherent-`impl`
//! разнесён по файлам одного модуля, публичные пути при этом не меняются.

// =============================================================================
//         MODULES
// =============================================================================

mod arith;
mod error;
mod funcs;
mod promote;
mod strings;

// =============================================================================
//         RE-EXPORTS
// =============================================================================

pub use self::error::MathErr;

// =============================================================================
//         TYPES
// =============================================================================

/// [STABLE] Core mathematical operations provider.
///
/// Handles arithmetic, trigonometric, and rounding operations across all
/// supported number types, with automatic type promotion and overflow handling.
pub struct MathOperators;

/// [STABLE] Original floating-point type classification.
///
/// Used internally to track the precision level of floating-point numbers
/// for appropriate rounding and conversion operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum OrigKind {
    F32,
    F64,
    F128,
    Other,
}

#[cfg(test)]
mod tests;
