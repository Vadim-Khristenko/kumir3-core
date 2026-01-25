// ============================================================================
//                         ЯДРО ПАРСЕРА
// ============================================================================

use crate::shared::types::Token;
use crate::shared::lexer::{SpannedToken, Span, tokenize};
use super::error::{ParseError, ParseResult};

/// Парсер языка Кумир.
pub struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    /// Создаёт парсер из исходного кода.
    pub fn new(source: &str) -> ParseResult<Self> {
        Ok(Self { tokens: tokenize(source)?, pos: 0 })
    }
    
    /// Создаёт парсер из готовых токенов.
    pub fn from_tokens(tokens: Vec<SpannedToken>) -> Self {
        Self { tokens, pos: 0 }
    }
    
    // =========================================================================
    //                    НАВИГАЦИЯ ПО ТОКЕНАМ
    // =========================================================================
    
    /// Текущий токен.
    #[inline]
    pub fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }
    
    /// Токен на n позиций вперёд.
    #[inline]
    pub fn peek_n(&self, n: usize) -> &Token {
        &self.tokens[(self.pos + n).min(self.tokens.len() - 1)].token
    }
    
    /// Span текущего токена.
    #[inline]
    pub fn span(&self) -> Span {
        self.tokens[self.pos].span
    }
    
    /// Конец файла?
    #[inline]
    pub fn is_eof(&self) -> bool {
        matches!(self.peek(), Token::EOF)
    }
    
    /// Продвинуться и вернуть предыдущий токен.
    #[inline]
    pub fn advance(&mut self) -> &SpannedToken {
        if !self.is_eof() { self.pos += 1; }
        &self.tokens[self.pos - 1]
    }
    
    /// Проверить тип текущего токена (по дискриминанту).
    #[inline]
    pub fn check(&self, token: &Token) -> bool {
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token)
    }
    
    /// Проверить и продвинуться если совпадает.
    #[inline]
    pub fn match_token(&mut self, token: &Token) -> bool {
        if self.check(token) { self.advance(); true } else { false }
    }
    
    /// Требовать конкретный токен.
    #[inline]
    pub fn expect(&mut self, token: &Token, msg: &str) -> ParseResult<&SpannedToken> {
        if self.check(token) {
            Ok(self.advance())
        } else {
            Err(ParseError::unexpected(msg, self.peek(), self.span()))
        }
    }
    
    /// Пропустить переводы строк и комментарии.
    #[inline]
    pub fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline | Token::Comment(_)) {
            self.advance();
        }
    }
    
    /// Требовать конец строки или EOF.
    pub fn expect_eol(&mut self) -> ParseResult<()> {
        if matches!(self.peek(), Token::Newline | Token::EOF | Token::Comment(_)) {
            if !self.is_eof() { self.advance(); }
            Ok(())
        } else {
            Err(ParseError::expected_newline(self.span()))
        }
    }
    
    /// Проверить, является ли текущий токен одним из списка.
    #[inline]
    pub fn check_any(&self, tokens: &[Token]) -> bool {
        tokens.iter().any(|t| self.check(t))
    }
    
    /// Получить идентификатор или вернуть ошибку.
    pub fn expect_ident(&mut self, msg: &str) -> ParseResult<String> {
        if let Token::Identifier(name) = self.peek().clone() {
            self.advance();
            Ok(name)
        } else {
            Err(ParseError::unexpected(msg, self.peek(), self.span()))
        }
    }
    
    /// Проверить, есть ли указанный токен где-то впереди (без изменения позиции).
    pub fn has_token_ahead(&self, token: &Token) -> bool {
        for i in self.pos..self.tokens.len() {
            if std::mem::discriminant(&self.tokens[i].token) == std::mem::discriminant(token) {
                return true;
            }
        }
        false
    }
}
