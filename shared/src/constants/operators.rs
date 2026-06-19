//! Операторы языка Кумир.
//!
//! Источник истины — таблица `OPERATORS` в `shared/build.rs`; здесь — сгенерированная
//! единая `phf`-карта символ->Token и набор первых символов. Логические предикаты
//! (precedence, binary/unary/assignment) остаются здесь.

use crate::types::Token;

include!(concat!(env!("OUT_DIR"), "/operators_gen.rs"));

/// Ищет токен оператора по символу (1–3 символа). None, если не оператор.
#[inline]
pub fn operator_token(s: &str) -> Option<Token> {
    OPERATOR_INDEX.get(s).cloned()
}

/// Проверяет, может ли символ начинать оператор.
#[inline]
pub fn is_operator_char(c: char) -> bool {
    OPERATOR_FIRST_CHARS.contains(&c)
}

/// Returns operator precedence (higher = binds tighter).
pub fn operator_precedence(token: &Token) -> u8 {
    match token {
        Token::Or => 1,
        Token::And => 2,
        Token::Equal | Token::NotEqual => 3,
        Token::Less | Token::Greater | Token::LessEqual | Token::GreaterEqual => 4,
        Token::DoubleDot | Token::DoubleDotEq => 5,
        Token::Pipe => 6,
        Token::Plus | Token::Minus => 7,
        Token::Star | Token::Slash | Token::Percent => 8,
        Token::Power => 9,
        Token::Compose => 10,
        Token::Dot | Token::DoubleColon => 11,
        Token::Not => 12,
        _ => 0,
    }
}

/// Checks if token is a binary operator.
pub fn is_binary_operator(token: &Token) -> bool {
    matches!(
        token,
        Token::Plus
            | Token::Minus
            | Token::Star
            | Token::Slash
            | Token::Percent
            | Token::Power
            | Token::Equal
            | Token::NotEqual
            | Token::Less
            | Token::Greater
            | Token::LessEqual
            | Token::GreaterEqual
            | Token::And
            | Token::Or
            | Token::Pipe
            | Token::Compose
            | Token::DoubleDot
            | Token::DoubleDotEq
            | Token::Dot
            | Token::DoubleColon
    )
}

/// Checks if token is a unary operator.
pub fn is_unary_operator(token: &Token) -> bool {
    matches!(
        token,
        Token::Plus | Token::Minus | Token::Not | Token::Ampersand | Token::Caret
    )
}

/// Checks if token is an assignment operator.
pub fn is_assignment_operator(token: &Token) -> bool {
    matches!(
        token,
        Token::Assign
            | Token::PlusAssign
            | Token::MinusAssign
            | Token::StarAssign
            | Token::SlashAssign
    )
}
