// ============================================================================
//                         ПАРСИНГ ПАТТЕРНОВ
// ============================================================================

use crate::types::{Token, Pattern, Value, Number};
use super::core::Parser;
use super::error::{ParseError, ParseResult};

impl Parser {
    /// Парсить паттерн для match.
    pub fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        // Wildcard: _
        if let Token::Identifier(name) = self.peek() {
            if name == "_" {
                self.advance();
                return Ok(Pattern::Wildcard);
            }
        }
        
        // Литералы
        match self.peek().clone() {
            Token::Integer(n) => {
                self.advance();
                // Проверка на диапазон: 1..10
                if self.match_token(&Token::DoubleDot) {
                    let end = self.parse_range_end()?;
                    return Ok(Pattern::Range {
                        start: Some(Box::new(crate::types::Expr::Literal(
                            Value::Number(Number::I64(n))
                        ))),
                        end,
                        inclusive: false,
                    });
                }
                return Ok(Pattern::Literal(Value::Number(Number::I64(n))));
            }
            Token::Float(n) => {
                self.advance();
                return Ok(Pattern::Literal(Value::Number(Number::F64(n))));
            }
            Token::String(s) => {
                self.advance();
                return Ok(Pattern::Literal(Value::String(s)));
            }
            Token::Char(c) => {
                self.advance();
                return Ok(Pattern::Literal(Value::Char(c)));
            }
            Token::True => {
                self.advance();
                return Ok(Pattern::Literal(Value::Boolean(true)));
            }
            Token::False => {
                self.advance();
                return Ok(Pattern::Literal(Value::Boolean(false)));
            }
            Token::None => {
                self.advance();
                return Ok(Pattern::Literal(Value::Null));
            }
            _ => {}
        }
        
        // Кортеж: (p1, p2, ...)
        if self.match_token(&Token::LParen) {
            let patterns = self.parse_pattern_list(&Token::RParen)?;
            self.expect(&Token::RParen, ")")?;
            return Ok(Pattern::Tuple(patterns));
        }
        
        // Массив: [p1, p2, ...rest]
        if self.match_token(&Token::LBracket) {
            return self.parse_array_pattern();
        }
        
        // Идентификатор или enum variant
        if let Token::Identifier(name) = self.peek().clone() {
            self.advance();
            
            // Enum variant: EnumName::Variant(bindings)
            if self.match_token(&Token::DoubleColon) {
                return self.parse_enum_pattern(name);
            }
            
            // Диапазон: start..end
            if self.match_token(&Token::DoubleDot) {
                let end = self.parse_range_end()?;
                return Ok(Pattern::Range {
                    start: Some(Box::new(crate::types::Expr::Variable(name))),
                    end,
                    inclusive: false,
                });
            }
            
            // Простая привязка к переменной
            return Ok(Pattern::Variable(name));
        }
        
        Err(ParseError::unexpected("паттерн", self.peek(), self.span()))
    }
    
    /// Парсить паттерн массива.
    fn parse_array_pattern(&mut self) -> ParseResult<Pattern> {
        let mut elements = Vec::new();
        let mut rest = None;
        
        if !self.check(&Token::RBracket) {
            loop {
                // ...rest
                if self.match_token(&Token::Ellipsis) {
                    if let Token::Identifier(name) = self.peek().clone() {
                        self.advance();
                        rest = Some(name);
                    }
                    break;
                }
                
                elements.push(self.parse_pattern()?);
                if !self.match_token(&Token::Comma) { break; }
            }
        }
        
        self.expect(&Token::RBracket, "]")?;
        Ok(Pattern::Array { elements, rest })
    }
    
    /// Парсить enum паттерн после ::
    fn parse_enum_pattern(&mut self, enum_name: String) -> ParseResult<Pattern> {
        let variant = self.expect_ident("вариант enum")?;
        
        let bindings = if self.match_token(&Token::LParen) {
            let mut bindings = Vec::new();
            if !self.check(&Token::RParen) {
                loop {
                    bindings.push(self.expect_ident("привязка")?);
                    if !self.match_token(&Token::Comma) { break; }
                }
            }
            self.expect(&Token::RParen, ")")?;
            bindings
        } else {
            Vec::new()
        };
        
        Ok(Pattern::EnumVariant { enum_name, variant, bindings })
    }
    
    /// Парсить список паттернов.
    fn parse_pattern_list(&mut self, end_token: &Token) -> ParseResult<Vec<Pattern>> {
        let mut patterns = Vec::new();
        
        if !self.check(end_token) {
            loop {
                patterns.push(self.parse_pattern()?);
                if !self.match_token(&Token::Comma) { break; }
            }
        }
        
        Ok(patterns)
    }
    
    /// Парсить конец диапазона.
    fn parse_range_end(&mut self) -> ParseResult<Option<Box<crate::types::Expr>>> {
        // Может быть пусто (открытый диапазон)
        match self.peek() {
            Token::Integer(n) => {
                let n = *n;
                self.advance();
                Ok(Some(Box::new(crate::types::Expr::Literal(
                    Value::Number(Number::I64(n))
                ))))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Some(Box::new(crate::types::Expr::Variable(name))))
            }
            _ => Ok(None),
        }
    }
    
    /// Парсить паттерн с альтернативами: 1 | 2 | 3
    pub fn parse_pattern_with_or(&mut self) -> ParseResult<Pattern> {
        let first = self.parse_pattern()?;
        
        // Проверяем на |
        if !matches!(self.peek(), Token::Or) {
            return Ok(first);
        }
        
        let mut alternatives = vec![first];
        
        while self.match_token(&Token::Or) {
            alternatives.push(self.parse_pattern()?);
        }
        
        Ok(Pattern::Or(alternatives))
    }
}
