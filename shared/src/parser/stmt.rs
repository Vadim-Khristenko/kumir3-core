// ============================================================================
//                         ПАРСИНГ ИНСТРУКЦИЙ
// ============================================================================

use crate::types::{Token, Stmt, Expr, MatchArm};
use super::core::Parser;
use super::error::{ParseError, ParseResult};

impl Parser {
    /// Парсить инструкции до стоп-токенов.
    pub fn parse_stmts_until(&mut self, stop: &[Token]) -> ParseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        
        while !self.is_eof() {
            // Проверить стоп-токены
            if stop.iter().any(|t| self.check(t)) {
                break;
            }
            
            // Пропустить пустые строки
            if matches!(self.peek(), Token::Newline | Token::Comment(_)) {
                self.advance();
                continue;
            }
            
            stmts.push(self.parse_stmt()?);
        }
        
        Ok(stmts)
    }
    
    /// Парсить одну инструкцию.
    pub fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        match self.peek().clone() {
            // Объявление переменных
            Token::IntType | Token::FloatType | Token::BoolType |
            Token::CharType | Token::StringType | Token::ArrayType |
            Token::PointerType => self.parse_var_decl(),
            
            // Авто-объявление
            Token::AutoType => self.parse_auto_decl(),
            
            // Условие
            Token::If => self.parse_if(),
            
            // Циклы
            Token::Loop => self.parse_loop(),
            Token::For => self.parse_for(),
            Token::While => self.parse_while(),
            
            // Ввод/вывод
            Token::Input => self.parse_input(),
            Token::Output => self.parse_output(),
            
            // Управление
            Token::Assert => self.parse_assert(),
            Token::Halt => { self.advance(); self.expect_eol()?; Ok(Stmt::Return) }
            Token::Throw => self.parse_throw(),
            
            // Возврат значения: вернуть выражение
            Token::Return => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_eol()?;
                Ok(Stmt::ReturnValue(expr))
            }
            
            // Присваивание результата: знач := выражение
            Token::ResultValue => {
                self.advance();
                self.expect(&Token::Assign, ":=")?;
                let expr = self.parse_expr()?;
                self.expect_eol()?;
                Ok(Stmt::ResultAssign(expr))
            }
            
            // Match
            Token::Match => self.parse_match(),
            
            // Try-Catch
            Token::Try => self.parse_try_catch(),
            
            // Rust-вставка
            Token::RustBlockStart | Token::Rust => self.parse_rust_block(),
            
            // Delete
            Token::Delete => self.parse_delete(),
            
            // Await
            Token::Await => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect_eol()?;
                Ok(Stmt::Await(expr))
            }
            
            // Идентификатор - присваивание или вызов
            Token::Identifier(_) => self.parse_assignment_or_call(),
            
            // This/Self - присваивание полю
            Token::This | Token::Self_ => self.parse_field_assignment(),
            
            _ => Err(ParseError::unexpected("инструкция", self.peek(), self.span())),
        }
    }
    
    /// Парсить объявление переменной (одной или нескольких через запятую).
    /// Примеры:
    ///   цел a
    ///   цел a, b, c
    ///   цел x := 42   (только для одной переменной)
    fn parse_var_decl(&mut self) -> ParseResult<Stmt> {
        let type_spec = self.parse_type()?;
        
        // Собираем имена переменных через запятую
        let mut names = Vec::new();
        names.push(self.expect_ident("имя переменной")?);
        
        while self.match_token(&Token::Comma) {
            names.push(self.expect_ident("имя переменной")?);
        }
        
        // Инициализация только для одной переменной
        let init = if names.len() == 1 && self.match_token(&Token::Assign) {
            Some(self.parse_expr()?)
        } else { None };
        
        self.expect_eol()?;
        Ok(Stmt::VarDecl { type_spec, names, init })
    }
    
    /// Парсить авто-объявление.
    fn parse_auto_decl(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::AutoType, "авто")?;
        let name = self.expect_ident("имя переменной")?;
        self.expect(&Token::Assign, ":=")?;
        let init = self.parse_expr()?;
        self.expect_eol()?;
        Ok(Stmt::AutoVarDecl { name, init })
    }
    
    /// Парсить условный оператор.
    fn parse_if(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::If, "если")?;
        let condition = self.parse_expr()?;
        self.expect(&Token::Then, "то")?;
        self.skip_newlines();
        
        let then_branch = self.parse_stmts_until(&[Token::Else, Token::Fi])?;
        
        let else_branch = if self.match_token(&Token::Else) {
            self.skip_newlines();
            Some(self.parse_stmts_until(&[Token::Fi])?)
        } else { None };
        
        self.expect(&Token::Fi, "все")?;
        self.skip_newlines();
        
        Ok(Stmt::If { condition, then_branch, else_branch })
    }
    
    /// Парсить цикл нц...кц
    fn parse_loop(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Loop, "нц")?;
        
        // нц пока
        if self.match_token(&Token::While) {
            let condition = self.parse_expr()?;
            self.skip_newlines();
            let body = self.parse_stmts_until(&[Token::EndLoop])?;
            self.expect(&Token::EndLoop, "кц")?;
            self.skip_newlines();
            return Ok(Stmt::LoopWhile { condition, body });
        }
        
        // нц для
        if self.match_token(&Token::For) {
            return self.parse_for_body();
        }
        
        // Бесконечный цикл
        self.skip_newlines();
        let body = self.parse_stmts_until(&[Token::EndLoop])?;
        self.expect(&Token::EndLoop, "кц")?;
        
        // кц при условие (do-while)
        if self.match_token(&Token::Case) {
            let condition = self.parse_expr()?;
            self.skip_newlines();
            return Ok(Stmt::LoopDoWhile { body, condition });
        }
        
        self.skip_newlines();
        Ok(Stmt::LoopInfinite { body })
    }
    
    /// Парсить for.
    fn parse_for(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::For, "для")?;
        self.parse_for_body()
    }
    
    /// Тело цикла for.
    fn parse_for_body(&mut self) -> ParseResult<Stmt> {
        let variable = self.expect_ident("переменная цикла")?;
        self.expect(&Token::From, "от")?;
        let from = self.parse_expr()?;
        self.expect(&Token::To, "до")?;
        let to = self.parse_expr()?;
        
        let step = if self.match_token(&Token::Step) {
            Some(self.parse_expr()?)
        } else { None };
        
        self.skip_newlines();
        let body = self.parse_stmts_until(&[Token::EndLoop])?;
        self.expect(&Token::EndLoop, "кц")?;
        self.skip_newlines();
        
        Ok(Stmt::LoopFor { variable, from, to, step, body })
    }
    
    /// Парсить while.
    fn parse_while(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::While, "пока")?;
        let condition = self.parse_expr()?;
        self.skip_newlines();
        let body = self.parse_stmts_until(&[Token::EndLoop])?;
        self.expect(&Token::EndLoop, "кц")?;
        self.skip_newlines();
        Ok(Stmt::LoopWhile { condition, body })
    }
    
    /// Парсить ввод.
    fn parse_input(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Input, "ввод")?;
        let mut vars = Vec::new();
        
        loop {
            vars.push(self.expect_ident("переменная")?);
            if !self.match_token(&Token::Comma) { break; }
        }
        
        self.expect_eol()?;
        Ok(Stmt::Input(vars))
    }
    
    /// Парсить вывод.
    fn parse_output(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Output, "вывод")?;
        
        // Пустой вывод
        if matches!(self.peek(), Token::Newline | Token::EOF | Token::Comment(_)) {
            self.expect_eol()?;
            return Ok(Stmt::Output(Vec::new()));
        }
        
        let mut exprs = Vec::new();
        loop {
            exprs.push(self.parse_expr()?);
            if !self.match_token(&Token::Comma) { break; }
        }
        
        self.expect_eol()?;
        Ok(Stmt::Output(exprs))
    }
    
    /// Парсить утверждение.
    fn parse_assert(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Assert, "утв")?;
        let expr = self.parse_expr()?;
        self.expect_eol()?;
        Ok(Stmt::Assert(expr))
    }
    
    /// Парсить throw.
    fn parse_throw(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Throw, "бросить")?;
        let expr = self.parse_expr()?;
        self.expect_eol()?;
        Ok(Stmt::Throw(expr))
    }
    
    /// Парсить match.
    fn parse_match(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Match, "совпадение")?;
        let expr = self.parse_expr()?;
        self.skip_newlines();
        
        let mut arms = Vec::new();
        
        while self.match_token(&Token::Case) {
            let pattern = self.parse_pattern_with_or()?;
            
            let guard = if self.match_token(&Token::If) {
                Some(self.parse_expr()?)
            } else { None };
            
            self.expect(&Token::FatArrow, "=>")?;
            self.skip_newlines();
            
            let body = self.parse_stmts_until(&[Token::Case, Token::Fi])?;
            arms.push(MatchArm { pattern, guard, body });
        }
        
        self.expect(&Token::Fi, "все")?;
        self.skip_newlines();
        
        Ok(Stmt::Match { expr, arms })
    }
    
    /// Парсить try-catch.
    fn parse_try_catch(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Try, "попытка")?;
        self.skip_newlines();
        
        let try_block = self.parse_stmts_until(&[Token::Catch])?;
        self.expect(&Token::Catch, "перехват")?;
        
        let catch_var = if let Token::Identifier(_) = self.peek() {
            Some(self.expect_ident("переменная")?)
        } else { None };
        
        self.skip_newlines();
        let catch_block = self.parse_stmts_until(&[Token::Finally, Token::End])?;
        
        let finally_block = if self.match_token(&Token::Finally) {
            self.skip_newlines();
            Some(self.parse_stmts_until(&[Token::End])?)
        } else { None };
        
        self.expect(&Token::End, "кон")?;
        self.skip_newlines();
        
        Ok(Stmt::TryCatch { try_block, catch_var, catch_block, finally_block })
    }
    
    /// Парсить Rust-блок.
    fn parse_rust_block(&mut self) -> ParseResult<Stmt> {
        // Два варианта синтаксиса:
        // 1. РастВставкаНЦ ... РастВставкаКЦ
        // 2. ржавчина нач ... кон
        
        let code = if self.match_token(&Token::RustBlockStart) {
            let code = if let Token::RustCode(c) = self.peek().clone() {
                self.advance();
                c
            } else { String::new() };
            self.expect(&Token::RustBlockEnd, "РастВставкаКЦ")?;
            code
        } else if self.match_token(&Token::Rust) {
            self.expect(&Token::Begin, "нач")?;
            let code = if let Token::RustCode(c) = self.peek().clone() {
                self.advance();
                c
            } else { String::new() };
            self.expect(&Token::End, "кон")?;
            code
        } else {
            return Err(ParseError::unexpected("rust блок", self.peek(), self.span()));
        };
        
        // Извлекаем имена переменных из кода (ищем {имя} паттерны)
        let captured_vars = Self::extract_captured_vars(&code);
        
        self.skip_newlines();
        Ok(Stmt::RustBlock { code, captured_vars })
    }
    
    /// Извлекает имена переменных из Rust-кода.
    /// Ищет паттерны {имя_переменной} в строках.
    fn extract_captured_vars(code: &str) -> Vec<String> {
        let mut vars = Vec::new();
        let mut chars = code.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '{' {
                // Собираем имя переменной до }
                let mut name = String::new();
                for ch in chars.by_ref() {
                    if ch == '}' {
                        break;
                    }
                    if ch == ':' {
                        // Форматирование {x:?} - останавливаемся на :
                        break;
                    }
                    name.push(ch);
                }
                
                // Проверяем что это валидный идентификатор (не число, не пусто)
                let name = name.trim();
                if !name.is_empty() 
                   && !name.starts_with(|c: char| c.is_ascii_digit())
                   && name.chars().all(|c| c.is_alphanumeric() || c == '_' || c > '\x7F')
                {
                    if !vars.contains(&name.to_string()) {
                        vars.push(name.to_string());
                    }
                }
            }
        }
        
        vars
    }
    
    /// Парсить delete.
    fn parse_delete(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Delete, "удалить")?;
        let name = self.expect_ident("имя переменной")?;
        self.expect_eol()?;
        Ok(Stmt::PointerDelete { name })
    }
    
    /// Парсить присваивание или вызов.
    fn parse_assignment_or_call(&mut self) -> ParseResult<Stmt> {
        let name = self.expect_ident("идентификатор")?;
        
        // Доступ к полю через точку: obj.field := ...
        if self.match_token(&Token::Dot) {
            let field = self.expect_ident("имя поля")?;
            
            if self.match_token(&Token::Assign) {
                let value = self.parse_expr()?;
                self.expect_eol()?;
                return Ok(Stmt::FieldAssignment {
                    object: Expr::Variable(name),
                    field,
                    value,
                });
            }
            
            // Вызов метода как инструкция
            if self.match_token(&Token::LParen) {
                let args = self.parse_args()?;
                self.expect(&Token::RParen, ")")?;
                self.expect_eol()?;
                return Ok(Stmt::ExprStmt(Expr::MethodCall {
                    object: Box::new(Expr::Variable(name)),
                    method: field,
                    args,
                }));
            }
        }
        
        // Массив: arr[i] := ...
        if self.match_token(&Token::LBracket) {
            let mut indices = Vec::new();
            loop {
                indices.push(self.parse_expr()?);
                if !self.match_token(&Token::Comma) { break; }
            }
            self.expect(&Token::RBracket, "]")?;
            self.expect(&Token::Assign, ":=")?;
            let value = self.parse_expr()?;
            self.expect_eol()?;
            return Ok(Stmt::ArrayAssignment(name, indices, value));
        }
        
        // Составное присваивание: x += 1
        if let Some(op) = self.try_compound_assign() {
            let right = self.parse_expr()?;
            self.expect_eol()?;
            let binary = Expr::BinaryOp(
                Box::new(Expr::Variable(name.clone())),
                op,
                Box::new(right)
            );
            return Ok(Stmt::Assignment(name, binary));
        }
        
        // Простое присваивание
        if self.match_token(&Token::Assign) {
            let value = self.parse_expr()?;
            self.expect_eol()?;
            return Ok(Stmt::Assignment(name, value));
        }
        
        // Вызов процедуры
        let args = if self.match_token(&Token::LParen) {
            let args = self.parse_args()?;
            self.expect(&Token::RParen, ")")?;
            args
        } else { Vec::new() };
        
        self.expect_eol()?;
        Ok(Stmt::ExprStmt(Expr::Call(name, args)))
    }
    
    /// Парсить присваивание полю this/self.
    fn parse_field_assignment(&mut self) -> ParseResult<Stmt> {
        self.advance(); // this/self
        self.expect(&Token::Dot, ".")?;
        let field = self.expect_ident("имя поля")?;
        self.expect(&Token::Assign, ":=")?;
        let value = self.parse_expr()?;
        self.expect_eol()?;
        
        Ok(Stmt::FieldAssignment {
            object: Expr::SelfRef,
            field,
            value,
        })
    }
    
    /// Попытаться распарсить составное присваивание.
    fn try_compound_assign(&mut self) -> Option<Token> {
        match self.peek() {
            Token::PlusAssign => { self.advance(); Some(Token::Plus) }
            Token::MinusAssign => { self.advance(); Some(Token::Minus) }
            Token::StarAssign => { self.advance(); Some(Token::Star) }
            Token::SlashAssign => { self.advance(); Some(Token::Slash) }
            _ => None,
        }
    }
}
