//! Kumir 3 Operators
//!
//! Operator tables for lexical analysis. Operators are checked by length
//! (3-char first, then 2-char, then 1-char) to ensure longest match.

use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::types::Token;

// =============================================================================
//         SECTION: THREE-CHARACTER OPERATORS
// =============================================================================

/// Three-character operators (checked first).
pub static OPERATORS_3: Lazy<HashMap<&'static str, Token>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("...", Token::Ellipsis); // variadic / spread
    m.insert("..=", Token::DoubleDotEq); // inclusive range
    m.insert("<<=", Token::Assign); // left shift assign (future)
    m.insert(">>=", Token::Assign); // right shift assign (future)
    m
});

// =============================================================================
//         SECTION: TWO-CHARACTER OPERATORS
// =============================================================================

/// Two-character operators (checked after 3-char).
pub static OPERATORS_2: Lazy<HashMap<&'static str, Token>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // Comparison
    m.insert("<>", Token::NotEqual); // inequality (Kumir style)
    m.insert("!=", Token::NotEqual); // inequality (C style)
    m.insert("<=", Token::LessEqual); // less or equal
    m.insert(">=", Token::GreaterEqual); // greater or equal
    m.insert("==", Token::Equal); // strict equality

    // Assignment
    m.insert(":=", Token::Assign); // assignment (Kumir style)
    m.insert("+=", Token::PlusAssign); // add-assign
    m.insert("-=", Token::MinusAssign); // subtract-assign
    m.insert("*=", Token::StarAssign); // multiply-assign
    m.insert("/=", Token::SlashAssign); // divide-assign
    m.insert("%=", Token::Assign); // modulo-assign (maps to assign for now)

    // Exponentiation
    m.insert("**", Token::Power); // power

    // Kumir 3 special operators
    m.insert("->", Token::Arrow); // arrow (lambdas, return types)
    m.insert("=>", Token::FatArrow); // fat arrow (match arms)
    m.insert("::", Token::DoubleColon); // module/namespace access
    m.insert("|>", Token::Pipe); // pipe operator
    m.insert(">>", Token::Compose); // function composition
    m.insert("..", Token::DoubleDot); // range operator

    // Logic (alternative)
    m.insert("&&", Token::And); // logical AND (C style)
    m.insert("||", Token::Or); // logical OR (C style)

    m
});

// =============================================================================
//         SECTION: SINGLE-CHARACTER OPERATORS
// =============================================================================

/// Single-character operators.
pub static OPERATORS_1: Lazy<HashMap<char, Token>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // Arithmetic
    m.insert('+', Token::Plus); // addition
    m.insert('-', Token::Minus); // subtraction
    m.insert('*', Token::Star); // multiplication
    m.insert('/', Token::Slash); // division
    m.insert('%', Token::Percent); // modulo

    // Comparison
    m.insert('=', Token::Equal); // equality
    m.insert('<', Token::Less); // less than
    m.insert('>', Token::Greater); // greater than

    // Delimiters
    m.insert('(', Token::LParen); // left paren
    m.insert(')', Token::RParen); // right paren
    m.insert('[', Token::LBracket); // left bracket
    m.insert(']', Token::RBracket); // right bracket
    m.insert('{', Token::LBrace); // left brace
    m.insert('}', Token::RBrace); // right brace
    m.insert(',', Token::Comma); // comma
    m.insert(':', Token::Colon); // colon
    m.insert(';', Token::SemiColon); // semicolon
    m.insert('.', Token::Dot); // dot

    // Kumir 3 special
    m.insert('@', Token::At); // decorator/annotation
    m.insert('&', Token::Ampersand); // reference
    m.insert('^', Token::Caret); // dereference
    m.insert('?', Token::Question); // optional/early return
    m.insert('!', Token::Not); // logical not (alternative)
    m.insert('~', Token::Not); // bitwise not (maps to Not)

    m
});

// =============================================================================
//         SECTION: HELPER FUNCTIONS
// =============================================================================

/// Checks if a character is a potential operator start.
#[inline]
pub fn is_operator_char(c: char) -> bool {
    OPERATORS_1.contains_key(&c) || matches!(c, '|' | '!' | '~')
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
