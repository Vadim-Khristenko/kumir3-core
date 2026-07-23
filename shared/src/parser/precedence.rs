//! Binary operator precedence table and associativity rules.
//!
//! **Single source of truth for Kumir 3 operator precedence.** This table
//! governs the expression parser (`parser::expr`) and is normative for the
//! grammar (see KITE-0012). Precedence predicates must derive from this
//! table (see `constants::operators::is_binary_operator`), not duplicate it.

use crate::types::Token;

/// Binary operator precedence level (higher value = tighter binding).
///
/// `None` means the token is not an infix binary operator.
/// Postfix/path operators (`.`, `::`) and unary operators (`не`)
/// are intentionally excluded from this table: they are parsed by
/// separate parser rules and do not participate in precedence-climbing.
#[inline]
pub fn binary_precedence(token: &Token) -> Option<u8> {
    Some(match token {
        // Logical and null-coalescing (lowest precedence)
        Token::Or | Token::QuestionQuestion => 1,
        Token::And => 2,

        // Comparison
        Token::Equal | Token::NotEqual => 3,
        Token::Less | Token::Greater | Token::LessEqual | Token::GreaterEqual => 4,

        // Range
        Token::DoubleDot | Token::DoubleDotEq => 5,

        // Addition
        Token::Plus | Token::Minus => 6,

        // Multiplication
        Token::Star | Token::Slash | Token::IntDiv | Token::Percent => 7,

        // Power (right-associative)
        Token::Power => 8,

        // Pipe (function composition)
        Token::Pipe => 9,

        // Compose (function composition)
        Token::Compose => 10,

        _ => return None,
    })
}

/// Returns `true` if the operator is right-associative.
#[inline]
pub fn is_right_associative(token: &Token) -> bool {
    matches!(token, Token::Power | Token::QuestionQuestion)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prec(t: Token) -> u8 {
        binary_precedence(&t).expect("token must be a binary operator")
    }

    /// Guard: precedence tier ordering. If someone creates a second table
    /// or reorders levels, this test will fail.
    #[test]
    fn test_precedence_tiers_ordering() {
        // logical < comparison < range < additive < multiplicative < power
        assert!(prec(Token::Or) < prec(Token::And));
        assert!(prec(Token::And) < prec(Token::Equal));
        assert!(prec(Token::Equal) < prec(Token::Less));
        assert!(prec(Token::Less) < prec(Token::DoubleDot));
        assert!(prec(Token::DoubleDot) < prec(Token::Plus));
        assert!(prec(Token::Plus) < prec(Token::Star));
        assert!(prec(Token::Star) < prec(Token::Power));
        assert!(prec(Token::Power) < prec(Token::Pipe));
        assert!(prec(Token::Pipe) < prec(Token::Compose));
    }

    #[test]
    fn test_precedence_tiers_are_flat() {
        assert_eq!(prec(Token::Or), prec(Token::QuestionQuestion));
        assert_eq!(prec(Token::Equal), prec(Token::NotEqual));
        assert_eq!(prec(Token::Less), prec(Token::Greater));
        assert_eq!(prec(Token::Less), prec(Token::LessEqual));
        assert_eq!(prec(Token::Less), prec(Token::GreaterEqual));
        assert_eq!(prec(Token::DoubleDot), prec(Token::DoubleDotEq));
        assert_eq!(prec(Token::Plus), prec(Token::Minus));
        assert_eq!(prec(Token::Star), prec(Token::Slash));
        assert_eq!(prec(Token::Star), prec(Token::IntDiv));
        assert_eq!(prec(Token::Star), prec(Token::Percent));
    }

    /// Non-infix tokens must not appear in the precedence table:
    /// `.`/`::` are postfix, `не` is unary, `:=` is assignment.
    #[test]
    fn test_non_binary_tokens_have_no_precedence() {
        for t in [Token::Dot, Token::DoubleColon, Token::Not, Token::Assign] {
            assert_eq!(
                binary_precedence(&t),
                None,
                "{t:?} is not a binary operator"
            );
        }
    }

    #[test]
    fn test_right_associative_operators() {
        assert!(is_right_associative(&Token::Power));
        assert!(is_right_associative(&Token::QuestionQuestion));
        assert!(!is_right_associative(&Token::Plus));
        assert!(!is_right_associative(&Token::Star));
    }
}
