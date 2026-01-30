// ============================================================================
//                    ПРИОРИТЕТ ОПЕРАТОРОВ
// ============================================================================

use crate::types::Token;

/// Приоритет бинарного оператора (больше = выше).
#[inline]
pub fn binary_precedence(token: &Token) -> Option<u8> {
    Some(match token {
        // Логические (низший приоритет)
        Token::Or => 1,
        Token::And => 2,
        
        // Сравнение
        Token::Equal | Token::NotEqual => 3,
        Token::Less | Token::Greater | Token::LessEqual | Token::GreaterEqual => 4,
        
        // Диапазон
        Token::DoubleDot => 5,
        
        // Аддитивные
        Token::Plus | Token::Minus => 6,
        
        // Мультипликативные
        Token::Star | Token::Slash | Token::Percent => 7,
        
        // Степень (правоассоциативный)
        Token::Power => 8,
        
        // Pipe (функциональная композиция)
        Token::Pipe => 9,
        
        // Compose (композиция функций)
        Token::Compose => 10,
        
        _ => return None,
    })
}

/// Проверяет, является ли оператор правоассоциативным.
#[inline]
pub fn is_right_associative(token: &Token) -> bool {
    matches!(token, Token::Power)
}
