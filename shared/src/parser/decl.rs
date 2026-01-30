// ============================================================================
//                    ПАРСИНГ ОБЪЯВЛЕНИЙ (алгоритмы, модули, enum)
// ============================================================================

use crate::types::{
    Token, Stmt, Program, Algorithm, Parameter, ParamMode, EnumVariant,
};
use super::core::Parser;
use super::error::{ParseError, ParseResult};

impl Parser {
    /// Парсить программу.
    pub fn parse_program(&mut self) -> ParseResult<Program> {
        let mut imports = Vec::new();
        let mut globals = Vec::new();
        let mut algorithms = Vec::new();
        let mut classes = Vec::new();
        let mut main = None;
        let mut warnings = Vec::new();
        
        self.skip_newlines();
        
        // Проверяем, содержит ли программа хотя бы один алгоритм
        let has_algorithm = self.has_token_ahead(&Token::Alg);
        
        // Если нет алгоритма — весь код "голый"
        if !has_algorithm {
            warnings.push(
                "Предупреждение: программа не содержит объявления алгоритма (алг).\n\
                 Код был автоматически обёрнут в анонимный алгоритм.\n\
                 Рекомендуется использовать структуру:\n\
                 \n\
                   алг\n\
                   нач\n\
                     <ваш код>\n\
                   кон\n".to_string()
            );
            
            // Парсим весь код как тело главного алгоритма
            let body = self.parse_bare_statements()?;
            
            main = Some(Algorithm {
                name: String::new(),
                params: Vec::new(),
                return_type: None,
                precondition: None,
                postcondition: None,
                body,
                is_async: false,
            });
            
            return Ok(Program {
                imports,
                globals,
                algorithms,
                overloaded_algorithms: Vec::new(),
                classes,
                interfaces: Vec::new(),
                main,
                warnings,
            });
        }
        
        while !self.is_eof() {
            match self.peek() {
                // Импорт: подключить или использовать
                Token::Import | Token::Use => imports.push(self.parse_import()?),
                
                Token::Alg => {
                    let alg = self.parse_algorithm()?;
                    if alg.name.is_empty() || alg.name == "главный" {
                        main = Some(alg);
                    } else {
                        algorithms.push(alg);
                    }
                }
                
                Token::IntType | Token::FloatType | Token::BoolType |
                Token::CharType | Token::StringType | Token::ArrayType |
                Token::AutoType | Token::PointerType => {
                    globals.push(self.parse_var_decl_global()?);
                }
                
                Token::EnumType => globals.push(self.parse_enum_decl()?),
                Token::Module => globals.push(self.parse_module_decl()?),
                Token::Class => classes.push(self.parse_class_decl()?),
                Token::Struct => globals.push(self.parse_struct_decl()?),
                Token::Interface => globals.push(self.parse_interface_decl()?),
                
                Token::Newline | Token::Comment(_) => { self.advance(); }
                
                _ => return Err(ParseError::unexpected(
                    "объявление алгоритма или переменной",
                    self.peek(), self.span()
                )),
            }
        }
        
        Ok(Program {
            imports,
            globals,
            algorithms,
            overloaded_algorithms: Vec::new(),
            classes,
            interfaces: Vec::new(),
            main,
            warnings,
        })
    }
    
    /// Парсит "голые" инструкции (код без алг/нач/кон).
    fn parse_bare_statements(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        
        while !self.is_eof() {
            self.skip_newlines();
            
            if self.is_eof() {
                break;
            }
            
            match self.peek() {
                // Пропускаем комментарии
                Token::Comment(_) | Token::Newline => {
                    self.advance();
                    continue;
                }
                
                // Парсим инструкцию
                _ => {
                    stmts.push(self.parse_stmt()?);
                }
            }
        }
        
        Ok(stmts)
    }
    
    /// Парсить импорт (подключить или использовать).
    fn parse_import(&mut self) -> ParseResult<Stmt> {
        // Принимаем оба варианта: подключить и использовать
        if !self.match_token(&Token::Import) && !self.match_token(&Token::Use) {
            return Err(ParseError::unexpected("подключить или использовать", self.peek(), self.span()));
        }
        
        let path = match self.peek().clone() {
            Token::String(s) => { self.advance(); s }
            Token::Identifier(name) => { self.advance(); name }
            _ => return Err(ParseError::unexpected("модуль или путь", self.peek(), self.span())),
        };
        
        // Опциональный alias: как Алиас
        let alias = if matches!(self.peek(), Token::Identifier(s) if s == "как") {
            self.advance();
            Some(self.expect_ident("алиас")?)
        } else { None };
        
        self.expect_eol()?;
        Ok(Stmt::Import { path, alias })
    }
    
    /// Парсить алгоритм.
    pub fn parse_algorithm(&mut self) -> ParseResult<Algorithm> {
        self.expect(&Token::Alg, "алг")?;
        
        // Модификаторы
        let is_async = self.match_token(&Token::Async);
        
        // Опциональный тип возврата (перед именем): алг цел Сумма(...)
        let mut return_type = self.try_parse_type();
        
        // Имя
        let name = if let Token::Identifier(_) = self.peek() {
            self.expect_ident("имя алгоритма")?
        } else { String::new() };
        
        // Параметры
        let params = if self.match_token(&Token::LParen) {
            let p = self.parse_params()?;
            self.expect(&Token::RParen, ")")?;
            p
        } else { Vec::new() };
        
        // Возвращаемый тип после параметров: алг Имя(...): Тип
        if self.match_token(&Token::Colon) {
            return_type = Some(self.parse_type()?);
        }
        
        self.skip_newlines();
        
        // Предусловие
        let precondition = if self.match_token(&Token::Given) {
            let e = self.parse_expr()?;
            self.skip_newlines();
            Some(e)
        } else { None };
        
        // Постусловие
        let postcondition = if self.match_token(&Token::Need) {
            let e = self.parse_expr()?;
            self.skip_newlines();
            Some(e)
        } else { None };
        
        // Тело
        self.expect(&Token::Begin, "нач")?;
        self.skip_newlines();
        
        let body = self.parse_stmts_until(&[Token::End])?;
        
        self.expect(&Token::End, "кон")?;
        self.skip_newlines();
        
        Ok(Algorithm { name, return_type, params, precondition, postcondition, body, is_async })
    }
    
    /// Парсить параметры.
    fn parse_params(&mut self) -> ParseResult<Vec<Parameter>> {
        let mut params = Vec::new();
        
        if self.check(&Token::RParen) {
            return Ok(params);
        }
        
        loop {
            params.push(self.parse_param()?);
            if !self.match_token(&Token::Comma) { break; }
        }
        
        Ok(params)
    }
    
    /// Парсить один параметр.
    /// Поддерживает два синтаксиса:
    /// 1. Классический Кумир: арг цел x
    /// 2. Современный: x: цел или запрос: HTTPЗапрос
    fn parse_param(&mut self) -> ParseResult<Parameter> {
        // Режим (опционально)
        let mode = if self.match_token(&Token::Arg) { ParamMode::Arg }
            else if self.match_token(&Token::Res) { ParamMode::Res }
            else if self.match_token(&Token::ArgRes) { ParamMode::ArgRes }
            else { ParamMode::Arg };
        
        // Пробуем оба синтаксиса
        // Если следующий токен - идентификатор, а за ним двоеточие — новый синтаксис
        if let Token::Identifier(_) = self.peek() {
            if matches!(self.peek_n(1), Token::Colon) {
                // Новый синтаксис: имя: Тип
                let name = self.expect_ident("имя параметра")?;
                self.expect(&Token::Colon, ":")?;
                let type_spec = self.parse_type()?;
                
                let default = if self.match_token(&Token::Assign) {
                    Some(self.parse_expr()?)
                } else { None };
                
                return Ok(Parameter { name, type_spec, mode, default });
            }
        }
        
        // Классический синтаксис: тип имя
        let type_spec = self.parse_type()?;
        let name = self.expect_ident("имя параметра")?;
        
        let default = if self.match_token(&Token::Assign) {
            Some(self.parse_expr()?)
        } else { None };
        
        Ok(Parameter { name, type_spec, mode, default })
    }
    
    /// Парсить глобальное объявление переменной (одной или нескольких).
    fn parse_var_decl_global(&mut self) -> ParseResult<Stmt> {
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
    
    /// Парсить enum.
    fn parse_enum_decl(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::EnumType, "перечисление")?;
        let name = self.expect_ident("имя перечисления")?;
        self.skip_newlines();
        
        let mut variants = Vec::new();
        
        while !self.check(&Token::End) && !self.is_eof() {
            if matches!(self.peek(), Token::Newline | Token::Comment(_)) {
                self.advance();
                continue;
            }
            
            if let Token::Identifier(_) = self.peek() {
                let variant_name = self.expect_ident("вариант")?;
                
                let data = if self.match_token(&Token::LParen) {
                    let t = self.parse_type()?;
                    self.expect(&Token::RParen, ")")?;
                    Some(t)
                } else { None };
                
                variants.push(EnumVariant { name: variant_name, data });
                self.skip_newlines();
            } else { break; }
        }
        
        self.expect(&Token::End, "кон")?;
        self.skip_newlines();
        
        Ok(Stmt::EnumDecl { name, variants })
    }
    
    /// Парсить модуль.
    fn parse_module_decl(&mut self) -> ParseResult<Stmt> {
        use crate::types::Algorithm;
        
        self.expect(&Token::Module, "модуль")?;
        let name = self.expect_ident("имя модуля")?;
        self.skip_newlines();
        
        let mut body = Vec::new();
        let mut algorithms = Vec::new();
        
        // Парсим содержимое модуля (алгоритмы и глобальные переменные)
        while !self.is_eof() && !self.check(&Token::End) {
            match self.peek().clone() {
                // Алгоритмы
                Token::Alg => {
                    let alg = self.parse_algorithm()?;
                    algorithms.push(alg);
                }
                
                // Объявления переменных
                Token::IntType | Token::FloatType | Token::BoolType |
                Token::CharType | Token::StringType | Token::ArrayType |
                Token::AutoType | Token::PointerType => {
                    body.push(self.parse_var_decl_global()?);
                }
                
                Token::Newline | Token::Comment(_) => { self.advance(); }
                
                _ => {
                    return Err(ParseError::unexpected(
                        "алгоритм или объявление переменной в модуле",
                        self.peek(), self.span()
                    ));
                }
            }
        }
        
        self.expect(&Token::End, "кон")?;
        self.skip_newlines();
        
        Ok(Stmt::ModuleDecl { name, body, algorithms })
    }
    
    /// Парсить struct.
    fn parse_struct_decl(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Struct, "структура")?;
        let name = self.expect_ident("имя структуры")?;
        self.skip_newlines();
        
        let fields = self.parse_fields_until(&Token::End)?;
        
        self.expect(&Token::End, "кон")?;
        self.skip_newlines();
        
        Ok(Stmt::StructDecl { name, fields })
    }
    
    /// Парсить interface.
    fn parse_interface_decl(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Interface, "интерфейс")?;
        let name = self.expect_ident("имя интерфейса")?;
        self.skip_newlines();
        
        let methods = self.parse_method_signatures_until(&Token::End)?;
        
        self.expect(&Token::End, "кон")?;
        self.skip_newlines();
        
        Ok(Stmt::InterfaceDecl { name, methods })
    }
}
