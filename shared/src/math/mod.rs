// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Arithmetic core of Kumir 3.
//!
//! Module organization by responsibility:
//! * `error` — error type [`MathErr`]
//! * `promote` — predicates, conversions, and numeric type promotion
//! * `arith` — basic operators: `+ - * / div mod **`
//! * `strings` — string/aggregate helpers for these operators
//! * `funcs` — roots, rounding, trigonometry, absolute value
//!
//! All methods belong to [`MathOperators`]; inherent implementations are split
//! across files, but public paths remain unchanged.

// =============================================================================
//         SECTION: MODULES
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
