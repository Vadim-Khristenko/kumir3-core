//! Утилиты для работы с идентификаторами
//!
//! Содержит функции проверки символов для лексера.

// ============================================================================
//                    ПРОВЕРКА ИДЕНТИФИКАТОРОВ
// ============================================================================

/// Проверяет, является ли символ началом идентификатора.
#[inline]
pub fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

/// Проверяет, может ли символ быть частью идентификатора.
/// 
/// Поддерживает Unicode combining characters (диакритические знаки),
/// что позволяет использовать символы вида e\u{0301} (é) в идентификаторах.
#[inline]
pub fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || is_unicode_combining_mark(c)
}

/// Проверяет, является ли символ Unicode combining mark (диакритический знак).
/// 
/// Combining marks — это символы, которые присоединяются к предыдущему символу
/// для образования составных символов (например, e + ́ = é).
#[inline]
pub fn is_unicode_combining_mark(c: char) -> bool {
    // Unicode категории Mn (Mark, Nonspacing), Mc (Mark, Spacing Combining), Me (Mark, Enclosing)
    // Диапазоны основных combining marks:
    // U+0300..U+036F - Combining Diacritical Marks
    // U+0483..U+0489 - Combining Cyrillic
    // U+1DC0..U+1DFF - Combining Diacritical Marks Supplement
    // U+20D0..U+20FF - Combining Diacritical Marks for Symbols
    // U+FE20..U+FE2F - Combining Half Marks
    let cp = c as u32;
    matches!(cp,
        0x0300..=0x036F |  // Combining Diacritical Marks
        0x0483..=0x0489 |  // Combining Cyrillic
        0x0591..=0x05BD |  // Hebrew combining marks
        0x05BF          |
        0x05C1..=0x05C2 |
        0x05C4..=0x05C5 |
        0x05C7          |
        0x0610..=0x061A |  // Arabic combining marks
        0x064B..=0x065F |
        0x0670          |
        0x06D6..=0x06DC |
        0x06DF..=0x06E4 |
        0x06E7..=0x06E8 |
        0x06EA..=0x06ED |
        0x1DC0..=0x1DFF |  // Combining Diacritical Marks Supplement
        0x20D0..=0x20FF |  // Combining Diacritical Marks for Symbols
        0xFE20..=0xFE2F    // Combining Half Marks
    )
}

/// Проверяет, является ли символ пробельным (кроме новой строки).
#[inline]
pub fn is_whitespace(c: char) -> bool {
    c == ' ' || c == '\t' || c == '\r'
}

/// Проверяет, является ли символ началом числа.
#[inline]
pub fn is_digit_start(c: char) -> bool {
    c.is_ascii_digit()
}
