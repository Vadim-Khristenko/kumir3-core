// ============================================================================
//                         ОШИБКИ ПАРСЕРА
// ============================================================================

use crate::types::Token;
use crate::lexer::{Span, LexerError};
use crate::constants::errors::errors;
use std::fmt;

/// Ошибка синтаксического анализа.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl ParseError {
    /// Создаёт новую ошибку парсера.
    #[inline]
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self { message: message.into(), span }
    }
    
    /// Ошибка "неожиданный токен".
    #[inline]
    pub fn unexpected(expected: &str, found: &Token, span: Span) -> Self {
        Self::new(
            format!("{}: ожидалось {}, найдено {:?}", errors::UNEXPECTED_TOKEN, expected, found),
            span,
        )
    }
    
    /// Ошибка "ожидался тип".
    #[inline]
    pub fn expected_type(span: Span) -> Self {
        Self::new(errors::EXPECTED_TYPE, span)
    }
    
    /// Ошибка "ожидалось выражение".
    #[inline]
    pub fn expected_expr(span: Span) -> Self {
        Self::new(errors::EXPECTED_EXPRESSION, span)
    }
    
    /// Ошибка "ожидался конец строки".
    #[inline]
    pub fn expected_newline(span: Span) -> Self {
        Self::new("Ожидался конец строки", span)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} в {}", self.message, self.span)
    }
}

impl std::error::Error for ParseError {}

impl From<LexerError> for ParseError {
    fn from(err: LexerError) -> Self {
        ParseError::new(err.message, Span::new(err.position, err.position))
    }
}

/// Результат работы парсера.
pub type ParseResult<T> = Result<T, ParseError>;
