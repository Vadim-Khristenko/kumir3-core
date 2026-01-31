// ============================================================================
//                         ПАРСИНГ ВЫРАЖЕНИЙ
// ============================================================================

use crate::types::{Token, Expr, Value, Number};
use super::core::Parser;
use super::error::{ParseError, ParseResult};
use super::precedence::{binary_precedence, is_right_associative};

impl Parser {
    /// Парсить выражение.
    pub fn parse_expr(&mut self) -> ParseResult<Expr> {
        self.parse_binary_expr(0)
    }
    
    /// Парсить бинарное выражение с приоритетом (Pratt parsing).
    fn parse_binary_expr(&mut self, min_prec: u8) -> ParseResult<Expr> {
        let mut left = self.parse_unary_expr()?;
        
        loop {
            let op = self.peek().clone();
            
            let prec = match binary_precedence(&op) {
                Some(p) if p >= min_prec => p,
                _ => break,
            };
            
            self.advance();
            
            let next_min = if is_right_associative(&op) { prec } else { prec + 1 };
            let right = self.parse_binary_expr(next_min)?;
            
            left = match op {
                Token::Pipe => Expr::Pipe(Box::new(left), Box::new(right)),
                _ => Expr::BinaryOp(Box::new(left), op, Box::new(right)),
            };
        }
        
        Ok(left)
    }
    
    /// Парсить унарное выражение.
    fn parse_unary_expr(&mut self) -> ParseResult<Expr> {
        match self.peek().clone() {
            Token::Minus | Token::Not => {
                let op = self.advance().token.clone();
                let expr = self.parse_unary_expr()?;
                Ok(Expr::UnaryOp(op, Box::new(expr)))
            }
            Token::Ampersand => {
                self.advance();
                Ok(Expr::Ref(Box::new(self.parse_unary_expr()?)))
            }
            Token::Caret => {
                self.advance();
                Ok(Expr::Deref(Box::new(self.parse_unary_expr()?)))
            }
            Token::New => {
                self.advance();
                // новый Класс(args) или новый тип(значение)
                if let Token::Identifier(class_name) = self.peek().clone() {
                    self.advance();
                    if self.match_token(&Token::LParen) {
                        let args = self.parse_args()?;
                        self.expect(&Token::RParen, ")")?;
                        return Ok(Expr::NewInstance { class_name, args });
                    }
                    // Просто новый указатель на переменную
                    return Ok(Expr::New(Box::new(Expr::Variable(class_name))));
                }
                Ok(Expr::New(Box::new(self.parse_unary_expr()?)))
            }
            _ => self.parse_postfix_expr(),
        }
    }
    
    /// Парсить постфиксное выражение (вызовы, индексы, поля).
    fn parse_postfix_expr(&mut self) -> ParseResult<Expr> {
        let mut expr = self.parse_primary_expr()?;
        
        loop {
            match self.peek() {
                // Вызов: f(args)
                Token::LParen => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(&Token::RParen, ")")?;
                    
                    expr = match expr {
                        Expr::Variable(name) => Expr::Call(name, args),
                        Expr::ModuleAccess(module, func) => {
                            // Модуль::функция(args) -> Call
                            Expr::Call(format!("{}::{}", module, func), args)
                        }
                        Expr::FieldAccess(obj, method) => {
                            Expr::MethodCall { object: obj, method, args }
                        }
                        _ => return Err(ParseError::new("Невозможно вызвать", self.span())),
                    };
                }
                
                // Индекс: arr[i, j]
                Token::LBracket => {
                    self.advance();
                    let mut indices = Vec::new();
                    loop {
                        indices.push(self.parse_expr()?);
                        if !self.match_token(&Token::Comma) { break; }
                    }
                    self.expect(&Token::RBracket, "]")?;
                    
                    expr = match expr {
                        Expr::Variable(name) => Expr::ArrayAccess(name, indices),
                        _ => return Err(ParseError::new("Невозможно индексировать", self.span())),
                    };
                }
                
                // Доступ к модулю: Модуль::член
                Token::DoubleColon => {
                    self.advance();
                    let member = self.expect_ident("идентификатор")?;
                    
                    expr = match expr {
                        Expr::Variable(module) => Expr::ModuleAccess(module, member),
                        Expr::ModuleAccess(m, sub) => {
                            Expr::ModuleAccess(format!("{}::{}", m, sub), member)
                        }
                        _ => return Err(ParseError::new("Невозможно обратиться к модулю", self.span())),
                    };
                }
                
                // Доступ к полю: obj.field
                Token::Dot => {
                    self.advance();
                    let field = self.expect_ident("имя поля")?;
                    expr = Expr::FieldAccess(Box::new(expr), field);
                }
                
                _ => break,
            }
        }
        
        Ok(expr)
    }
    
    /// Парсить первичное выражение.
    fn parse_primary_expr(&mut self) -> ParseResult<Expr> {
        match self.peek().clone() {
            // Числа
            Token::Integer(n) => { self.advance(); Ok(Expr::Literal(Value::Number(Number::I64(n)))) }
            Token::Float(n) => { self.advance(); Ok(Expr::Literal(Value::Number(Number::F64(n)))) }
            
            // Строка и символ
            Token::String(s) => { self.advance(); Ok(Expr::Literal(Value::String(s))) }
            Token::Char(c) => { self.advance(); Ok(Expr::Literal(Value::Char(c))) }
            
            // Булевы
            Token::True => { self.advance(); Ok(Expr::Literal(Value::Boolean(true))) }
            Token::False => { self.advance(); Ok(Expr::Literal(Value::Boolean(false))) }
            
            // None
            Token::None => { self.advance(); Ok(Expr::None) }
            
            // NotImplemented
            Token::NotImplemented => {
                self.advance();
                // Опционально: сообщение в скобках
                let msg = if self.match_token(&Token::LParen) {
                    let m = if let Token::String(s) = self.peek().clone() {
                        self.advance();
                        Some(s)
                    } else { None };
                    self.expect(&Token::RParen, ")")?;
                    m
                } else { None };
                Ok(Expr::NotImplemented(msg))
            }
            
            // this / self / это
            Token::This | Token::Self_ => {
                self.advance();
                Ok(Expr::SelfRef)
            }
            
            // super / родитель
            Token::Super => {
                self.advance();
                Ok(Expr::SuperRef)
            }
            
            // Идентификатор
            Token::Identifier(name) => {
                self.advance();
                Ok(Expr::Variable(name))
            }
            
            // Скобки или кортеж
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                
                // Кортеж?
                if self.match_token(&Token::Comma) {
                    let mut elements = vec![expr];
                    loop {
                        elements.push(self.parse_expr()?);
                        if !self.match_token(&Token::Comma) { break; }
                    }
                    self.expect(&Token::RParen, ")")?;
                    // Преобразуем в кортеж
                    return Ok(Expr::Literal(Value::Tuple(
                        elements.into_iter().filter_map(expr_to_value).collect()
                    )));
                }
                
                self.expect(&Token::RParen, ")")?;
                Ok(expr)
            }
            
            // Массив литерал
            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                
                if !self.check(&Token::RBracket) {
                    loop {
                        elements.push(self.parse_expr()?);
                        if !self.match_token(&Token::Comma) { break; }
                    }
                }
                
                self.expect(&Token::RBracket, "]")?;
                Ok(Expr::Literal(Value::Array(
                    elements.into_iter().filter_map(expr_to_value).collect()
                )))
            }
            
            // Лямбда
            Token::Lambda => {
                self.advance();
                self.parse_lambda()
            }
            
            // Условное выражение
            Token::If => {
                self.advance();
                let condition = self.parse_expr()?;
                self.expect(&Token::Then, "то")?;
                let then_expr = self.parse_expr()?;
                self.expect(&Token::Else, "иначе")?;
                let else_expr = self.parse_expr()?;
                self.expect(&Token::Fi, "все")?;
                
                Ok(Expr::IfExpr {
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                })
            }
            
            _ => Err(ParseError::expected_expr(self.span())),
        }
    }
    
    /// Парсить лямбда-выражение.
    fn parse_lambda(&mut self) -> ParseResult<Expr> {
        let mut params = Vec::new();
        
        if self.match_token(&Token::LParen) {
            if !self.check(&Token::RParen) {
                loop {
                    params.push(self.expect_ident("параметр")?);
                    if !self.match_token(&Token::Comma) { break; }
                }
            }
            self.expect(&Token::RParen, ")")?;
        } else if let Token::Identifier(_) = self.peek() {
            params.push(self.expect_ident("параметр")?);
        }
        
        self.expect(&Token::Arrow, "->")?;
        let body = self.parse_expr()?;
        
        Ok(Expr::Lambda { params, body: Box::new(body) })
    }
    
    /// Парсить список аргументов.
    pub fn parse_args(&mut self) -> ParseResult<Vec<Expr>> {
        let mut args = Vec::new();
        
        if !self.check(&Token::RParen) {
            loop {
                args.push(self.parse_expr()?);
                if !self.match_token(&Token::Comma) { break; }
            }
        }
        
        Ok(args)
    }
    
    /// Парсить приведение типа: выражение как Тип
    pub fn parse_cast(&mut self, expr: Expr) -> ParseResult<Expr> {
        // "как" уже пропущен
        let target_type = self.parse_type()?;
        Ok(Expr::Cast { expr: Box::new(expr), target_type })
    }
}

/// Попытаться преобразовать Expr в Value (для литералов).
fn expr_to_value(expr: Expr) -> Option<Value> {
    match expr {
        Expr::Literal(v) => Some(v),
        _ => Some(Value::Undefined), // Заглушка
    }
}
