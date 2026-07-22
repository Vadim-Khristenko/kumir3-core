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
///
/// Ошибка ядра НЕСЁТ СВОЙ ВИД: она передаётся вызывающему как значение
/// (а не как строка), чтобы интерпретатор мог сопоставить каждую разновидность
/// со своим `RuntimeErrorKind` (KITE-0014 § 3.2) — `DivisionByZero`,
/// `Overflow`, `TypeMismatch` и т. д.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Текст — обычная русская фраза без служебных маркеров: вид ошибки
    /// передаётся отдельно (самим значением [`MathErr`]), а не префиксом
    /// в сообщении.
    ///
    /// # Returns
    /// * `String` - Localized error message describing the mathematical error.
    pub fn msg(&self) -> String {
        match self {
            MathErr::DivisionByZero =>
                "Деление на ноль не определено".to_string(),
            MathErr::NegativeSqrt =>
                "Квадратный корень из отрицательного числа не определён".to_string(),
            MathErr::NegativeRoot =>
                "Корень чётной степени из отрицательного числа не определён".to_string(),
            MathErr::NotRealOneSqrt =>
                "Квадратный корень из -1 — мнимая единица; комплексные числа не поддерживаются".to_string(),
            MathErr::NegativePowNonInteger =>
                "Отрицательное основание допускается только с целой степенью".to_string(),
            MathErr::Overflow =>
                "Переполнение числа".to_string(),
            MathErr::FloatOverflow =>
                "Переполнение числа (вещественный тип)".to_string(),
            MathErr::DomainError(m) =>
                format!("Нарушение области определения: {}", m),
            MathErr::TypeMismatch(m) =>
                format!("Несовместимые типы операндов: {}", m),
        }
    }
}

impl std::fmt::Display for MathErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.msg())
    }
}

impl std::error::Error for MathErr {}

/// [STABLE] Warning message for automatic type widening on overflow.
///
/// Used internally when integer operations overflow and are automatically
/// promoted to a wider type to prevent data loss.
pub(super) fn warn_auto_widen() -> &'static str {
    "[MathWarn] Переполнение, выполнено автоматическое расширение типа"
}
