// ============================================================================
//                         ПАРСИНГ ТИПОВ
// ============================================================================

use crate::shared::types::{Token, TypeSpec};
use super::core::Parser;
use super::error::{ParseError, ParseResult};

impl Parser {
    /// Попытаться распарсить тип (None если не тип).
    /// НЕ распознаёт произвольные идентификаторы как типы.
    pub fn try_parse_type(&mut self) -> Option<TypeSpec> {
        let type_spec = match self.peek() {
            Token::IntType => TypeSpec::Int,
            Token::FloatType => TypeSpec::Float,
            Token::BoolType => TypeSpec::Bool,
            Token::CharType => TypeSpec::Char,
            Token::StringType => TypeSpec::String,
            Token::AutoType => TypeSpec::Auto,
            Token::NoneType => TypeSpec::None,
            
            Token::ArrayType => {
                self.advance();
                let elem = self.try_parse_type().unwrap_or(TypeSpec::Auto);
                return Some(TypeSpec::Array(Box::new(elem)));
            }
            
            Token::PointerType => {
                self.advance();
                let pointee = self.try_parse_type().unwrap_or(TypeSpec::Auto);
                return Some(TypeSpec::Pointer(Box::new(pointee)));
            }
            
            Token::OptionalType => {
                self.advance();
                // Необязательно<T> или Необязательно T
                if self.match_token(&Token::Less) {
                    let inner = self.try_parse_type()?;
                    if !self.match_token(&Token::Greater) { return None; }
                    return Some(TypeSpec::option(inner));
                }
                let inner = self.try_parse_type().unwrap_or(TypeSpec::Auto);
                return Some(TypeSpec::option(inner));
            }
            
            _ => return None,
        };
        
        self.advance();
        Some(type_spec)
    }
    
    /// Попытаться распарсить тип, включая пользовательские (классы).
    pub fn try_parse_type_with_custom(&mut self) -> Option<TypeSpec> {
        // Сначала пробуем базовые типы
        if let Some(t) = self.try_parse_type() {
            return Some(t);
        }
        
        // Пользовательский тип (класс или другой)
        if let Token::Identifier(name) = self.peek() {
            let name = name.clone();
            self.advance();
            
            // Проверка на дженерик: Тип<T>
            if self.match_token(&Token::Less) {
                let mut depth = 1;
                while depth > 0 && !self.is_eof() {
                    match self.peek() {
                        Token::Less => depth += 1,
                        Token::Greater => depth -= 1,
                        _ => {}
                    }
                    self.advance();
                }
            }
            return Some(TypeSpec::Object(name));
        }
        
        // Функциональный тип: (T1, T2) -> R
        if matches!(self.peek(), Token::LParen) {
            return self.try_parse_function_type();
        }
        
        None
    }
    
    /// Парсить тип (обязательно), включая пользовательские.
    pub fn parse_type(&mut self) -> ParseResult<TypeSpec> {
        self.try_parse_type_with_custom()
            .ok_or_else(|| ParseError::expected_type(self.span()))
    }
    
    /// Попытаться распарсить функциональный тип: (T1, T2) -> R
    fn try_parse_function_type(&mut self) -> Option<TypeSpec> {
        if !self.match_token(&Token::LParen) {
            return None;
        }
        
        let mut params = Vec::new();
        
        if !self.check(&Token::RParen) {
            loop {
                params.push(self.try_parse_type_with_custom()?);
                if !self.match_token(&Token::Comma) { break; }
            }
        }
        
        self.expect(&Token::RParen, ")").ok()?;
        
        let result = if self.match_token(&Token::Arrow) {
            Some(Box::new(self.try_parse_type_with_custom()?))
        } else {
            None
        };
        
        Some(TypeSpec::Function { params, result })
    }
    
    /// Проверить, начинается ли токен с базового типа (без идентификаторов).
    #[inline]
    pub fn is_type_start(&self) -> bool {
        matches!(self.peek(),
            Token::IntType | Token::FloatType | Token::BoolType |
            Token::CharType | Token::StringType | Token::ArrayType |
            Token::AutoType | Token::PointerType | Token::OptionalType |
            Token::NoneType
        )
    }
    
    /// Проверить, начинается ли токен с типа (включая идентификаторы).
    #[inline]
    pub fn is_type_start_with_custom(&self) -> bool {
        self.is_type_start() || matches!(self.peek(), Token::Identifier(_))
    }
}
