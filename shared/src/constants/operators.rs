//! Операторы языка Кумир.
//!
//! Источник истины по *написанию* операторов — таблица `OPERATORS` в
//! `shared/build.rs`; здесь — сгенерированная единая `phf`-карта символ->Token
//! и набор первых символов.
//!
//! Источник истины по *приоритету* — [`crate::parser::precedence`]
//! (нормативная таблица парсера выражений). Предикаты ниже опираются на неё,
//! собственной таблицы приоритетов этот модуль не держит.

use crate::parser::precedence::binary_precedence;
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

/// Проверяет, является ли токен инфиксным бинарным оператором.
///
/// Выводится из единственной таблицы приоритетов
/// [`crate::parser::precedence::binary_precedence`] — своего списка здесь
/// намеренно нет, чтобы предикат и приоритет не могли разойтись.
/// Точка и `::` бинарными операторами не считаются: это постфиксные
/// конструкции (доступ к полю / путь), разбираемые отдельными правилами.
#[inline]
pub fn is_binary_operator(token: &Token) -> bool {
    binary_precedence(token).is_some()
}

/// Проверяет, является ли токен унарным (префиксным) оператором.
///
/// `Plus`/`Minus` намеренно и унарные, и бинарные (`-a` и `a - b`);
/// остальные (`не`, `&`, `^`) в таблице приоритетов отсутствуют.
pub fn is_unary_operator(token: &Token) -> bool {
    matches!(
        token,
        Token::Plus | Token::Minus | Token::Not | Token::Ampersand | Token::Caret
    )
}

/// Проверяет, является ли токен оператором присваивания.
///
/// Присваивание — не выражение-оператор Кумира: ни один из этих токенов
/// не должен иметь приоритета в [`binary_precedence`].
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

    /// Предикаты не должны противоречить единственной таблице приоритетов.
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
            assert!(is_binary_operator(&t), "{t:?} должен быть бинарным");
            assert!(binary_precedence(&t).is_some());
        }

        // Постфиксные и унарные конструкции бинарными не считаются.
        for t in [Token::Dot, Token::DoubleColon, Token::Not] {
            assert!(!is_binary_operator(&t), "{t:?} не бинарный оператор");
        }

        // Присваивание никогда не участвует в precedence-climbing.
        for t in [
            Token::Assign,
            Token::PlusAssign,
            Token::MinusAssign,
            Token::StarAssign,
            Token::SlashAssign,
        ] {
            assert!(is_assignment_operator(&t));
            assert!(!is_binary_operator(&t), "{t:?} не бинарный оператор");
        }

        // Унарные, не являющиеся бинарными.
        for t in [Token::Not, Token::Ampersand, Token::Caret] {
            assert!(is_unary_operator(&t));
            assert!(!is_binary_operator(&t));
        }
    }
}
