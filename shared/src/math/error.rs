// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Ошибки математических операций.

// =============================================================================
//         TYPES
// =============================================================================

/// [STABLE] Enumeration of mathematical operation errors.
///
/// Provides detailed error messages for various math operation failures,
/// including division by zero, domain errors, and type mismatches.
#[derive(Debug, Clone)]
pub enum MathErr {
    DivisionByZero,
    NegativeSqrt,
    NegativeRoot,
    NotRealOneSqrt,
    NegativePowNonInteger,
    Overflow,
    FloatOverflow,
    DomainError(&'static str),
    TypeMismatch(&'static str),
}

impl MathErr {
    /// [STABLE] Returns a human-readable error message for the error variant.
    ///
    /// # Returns
    /// * `String` - Localized error message describing the mathematical error.
    pub fn msg(&self) -> String {
        match self {
            MathErr::DivisionByZero =>
                "[MathErr] Деление на ноль не определено".to_string(),
            MathErr::NegativeSqrt =>
                "[MathErr] Квадратный корень из отрицательного числа не определён".to_string(),
            MathErr::NegativeRoot =>
                "[MathErr] Корень чётной степени из отрицательного числа не определён".to_string(),
            MathErr::NotRealOneSqrt =>
                "[MathErr] Мнимая еденица! К сожалению пока что мы не поддерживаем такие 'Жёские вычисления', а так-то результат i".to_string(),
            MathErr::NegativePowNonInteger =>
                "[MathErr] Отрицательное основание допускается только с целой степенью".to_string(),
            MathErr::Overflow =>
                "[MathErr] Переполнение числа".to_string(),
            MathErr::FloatOverflow =>
                "[MathErr] Переполнение числа (вещественный тип)".to_string(),
            MathErr::DomainError(m) =>
                format!("[MathErr] Нарушение области определения: {}", m),
            MathErr::TypeMismatch(m) =>
                format!("[MathErr] Несовместимые типы операндов: {}", m),
        }
    }
}

/// [STABLE] Warning message for automatic type widening on overflow.
///
/// Used internally when integer operations overflow and are automatically
/// promoted to a wider type to prevent data loss.
pub(super) fn warn_auto_widen() -> &'static str {
    "[MathWarn] Переполнение, выполнено автоматическое расширение типа"
}
