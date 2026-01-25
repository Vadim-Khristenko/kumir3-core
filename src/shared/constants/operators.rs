//! Операторы языка Кумир
//!
//! Содержит таблицы операторов разной длины.

use std::collections::HashMap;
use once_cell::sync::Lazy;

use crate::shared::types::Token;

// ============================================================================
//                    ТРЁХСИМВОЛЬНЫЕ ОПЕРАТОРЫ
// ============================================================================

/// Трёхсимвольные операторы.
pub static OPERATORS_3: Lazy<HashMap<&'static str, Token>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("...", Token::Ellipsis);
    m
});

// ============================================================================
//                    ДВУХСИМВОЛЬНЫЕ ОПЕРАТОРЫ
// ============================================================================

/// Двухсимвольные операторы (проверяются первыми после трёхсимвольных).
pub static OPERATORS_2: Lazy<HashMap<&'static str, Token>> = Lazy::new(|| {
    let mut m = HashMap::new();
    
    // Сравнение
    m.insert("<>", Token::NotEqual);
    m.insert("<=", Token::LessEqual);
    m.insert(">=", Token::GreaterEqual);
    
    // Присваивание
    m.insert(":=", Token::Assign);
    m.insert("+=", Token::PlusAssign);
    m.insert("-=", Token::MinusAssign);
    m.insert("*=", Token::StarAssign);
    m.insert("/=", Token::SlashAssign);
    
    // Возведение в степень
    m.insert("**", Token::Power);
    
    // Kumir 3: специальные операторы
    m.insert("->", Token::Arrow);
    m.insert("=>", Token::FatArrow);
    m.insert("::", Token::DoubleColon);
    m.insert("|>", Token::Pipe);
    m.insert(">>", Token::Compose);
    m.insert("..", Token::DoubleDot);
    
    m
});

// ============================================================================
//                    ОДНОСИМВОЛЬНЫЕ ОПЕРАТОРЫ
// ============================================================================

/// Односимвольные операторы.
pub static OPERATORS_1: Lazy<HashMap<char, Token>> = Lazy::new(|| {
    let mut m = HashMap::new();
    
    // Арифметика
    m.insert('+', Token::Plus);
    m.insert('-', Token::Minus);
    m.insert('*', Token::Star);
    m.insert('/', Token::Slash);
    m.insert('%', Token::Percent);
    
    // Сравнение
    m.insert('=', Token::Equal);
    m.insert('<', Token::Less);
    m.insert('>', Token::Greater);
    
    // Разделители
    m.insert('(', Token::LParen);
    m.insert(')', Token::RParen);
    m.insert('[', Token::LBracket);
    m.insert(']', Token::RBracket);
    m.insert('{', Token::LBrace);
    m.insert('}', Token::RBrace);
    m.insert(',', Token::Comma);
    m.insert(':', Token::Colon);
    m.insert(';', Token::SemiColon);
    m.insert('.', Token::Dot);
    
    // Kumir 3: специальные символы
    m.insert('@', Token::At);
    m.insert('&', Token::Ampersand);
    m.insert('^', Token::Caret);
    m.insert('?', Token::Question);
    
    m
});
