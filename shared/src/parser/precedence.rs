// ============================================================================
//                    ПРИОРИТЕТ ОПЕРАТОРОВ
// ============================================================================

use crate::types::Token;

/// Приоритет бинарного оператора (больше = выше).
///
/// **Единственный источник истины о приоритете операторов языка Кумир 3.**
/// Эту таблицу использует парсер выражений (`parser::expr`), она нормативна
/// для грамматики (см. KITE-0012). Любые предикаты вида «является ли токен
/// бинарным оператором» обязаны выводиться отсюда
/// (см. `constants::operators::is_binary_operator`), а не дублировать список.
///
/// `None` означает «токен не является инфиксным бинарным оператором».
/// Постфиксные/путевые конструкции (`.`, `::`) и унарные операторы (`не`)
/// в таблицу намеренно не входят: они разбираются отдельными правилами
/// парсера и не участвуют в алгоритме precedence-climbing.
#[inline]
pub fn binary_precedence(token: &Token) -> Option<u8> {
    Some(match token {
        // Логические и null-coalescing (низший приоритет)
        Token::Or | Token::QuestionQuestion => 1,
        Token::And => 2,

        // Сравнение
        Token::Equal | Token::NotEqual => 3,
        Token::Less | Token::Greater | Token::LessEqual | Token::GreaterEqual => 4,

        // Диапазон
        Token::DoubleDot | Token::DoubleDotEq => 5,

        // Аддитивные
        Token::Plus | Token::Minus => 6,

        // Мультипликативные
        Token::Star | Token::Slash | Token::IntDiv | Token::Percent => 7,

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
    matches!(token, Token::Power | Token::QuestionQuestion)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prec(t: Token) -> u8 {
        binary_precedence(&t).expect("токен должен быть бинарным оператором")
    }

    /// Страж: порядок ярусов приоритета. Если кто-то заведёт вторую таблицу
    /// или переставит уровни — тест упадёт.
    #[test]
    fn test_precedence_tiers_ordering() {
        // логические < сравнение < диапазон < аддитивные < мультипликативные < степень
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

    /// Не-инфиксные токены не должны попадать в таблицу приоритетов:
    /// `.`/`::` — постфиксные, `не` — унарный, `:=` — присваивание.
    #[test]
    fn test_non_binary_tokens_have_no_precedence() {
        for t in [Token::Dot, Token::DoubleColon, Token::Not, Token::Assign] {
            assert_eq!(binary_precedence(&t), None, "{t:?} не бинарный оператор");
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
