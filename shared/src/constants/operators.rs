//! Kumir language operators.
//!
//! Single source of truth for operator *spelling*: `OPERATORS` table in
//! `shared/build.rs`. This module provides a generated symbol-to-Token map
//! and the set of operator-starting characters.
//!
//! Single source of truth for operator *precedence*: [`crate::parser::precedence`].
//! Predicates here use only that table; this module does not maintain a separate
//! precedence table to prevent drift.

use crate::parser::precedence::binary_precedence;
use crate::types::Token;

include!(concat!(env!("OUT_DIR"), "/operators_gen.rs"));

/// Looks up an operator token by symbol (1–3 characters), or None.
#[inline]
pub fn operator_token(s: &str) -> Option<Token> {
    OPERATOR_INDEX.get(s).cloned()
}

/// Checks if a character can start an operator.
#[inline]
pub fn is_operator_char(c: char) -> bool {
    OPERATOR_FIRST_CHARS.contains(&c)
}

/// Checks if a token is an infix binary operator.
///
/// Derived from [`crate::parser::precedence::binary_precedence`] — the only
/// source of truth. We maintain no separate list to prevent divergence.
/// Dot and `::` are not binary operators: they are postfix constructs
/// (field access, path separator) handled by separate parser rules.
#[inline]
pub fn is_binary_operator(token: &Token) -> bool {
    binary_precedence(token).is_some()
}

/// Checks if a token is a unary (prefix) operator.
///
/// `Plus` and `Minus` are intentionally both unary and binary (`-a` vs `a - b`);
/// the others (`не`, `&`, `^`) have no precedence entry.
pub fn is_unary_operator(token: &Token) -> bool {
    matches!(
        token,
        Token::Plus | Token::Minus | Token::Not | Token::Ampersand | Token::Caret
    )
}

/// Checks if a token is an assignment operator.
///
/// Assignment is not an expression-operator in Kumir: none of these tokens
/// should have a precedence in [`binary_precedence`].
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

#[cfg(test)]
mod operator_predicate_tests {
    use super::*;

    /// Predicates must agree with the single source of truth: the precedence table.
    #[test]
    fn test_predicates_agree_with_precedence_table() {
        // Всё, что имеет приоритет, — бинарный оператор, и наоборот.
        for t in [
            Token::Or,
            Token::QuestionQuestion,
            Token::And,
            Token::Equal,
            Token::NotEqual,
            Token::Less,
            Token::Greater,
            Token::LessEqual,
            Token::GreaterEqual,
            Token::DoubleDot,
            Token::DoubleDotEq,
            Token::Plus,
            Token::Minus,
            Token::Star,
            Token::Slash,
            Token::IntDiv,
            Token::Percent,
            Token::Power,
            Token::Pipe,
            Token::Compose,
        ] {
            assert!(is_binary_operator(&t), "{t:?} must be binary");
            assert!(binary_precedence(&t).is_some());
        }

        // Postfix and unary constructs are not binary operators.
        for t in [Token::Dot, Token::DoubleColon, Token::Not] {
            assert!(!is_binary_operator(&t), "{t:?} is not binary");
        }

        // Assignment never participates in precedence-climbing.
        for t in [
            Token::Assign,
            Token::PlusAssign,
            Token::MinusAssign,
            Token::StarAssign,
            Token::SlashAssign,
        ] {
            assert!(is_assignment_operator(&t));
            assert!(!is_binary_operator(&t), "{t:?} is not binary");
        }

        // Unary operators that are not binary.
        for t in [Token::Not, Token::Ampersand, Token::Caret] {
            assert!(is_unary_operator(&t));
            assert!(!is_binary_operator(&t));
        }
    }
}
