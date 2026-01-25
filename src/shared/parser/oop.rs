// ============================================================================
//                    ПАРСИНГ КЛАССОВ И ООП
// ============================================================================

use crate::shared::types::{
    Token, ClassDef, Field, Method, MethodSignature, Constructor,
    Visibility, Parameter, ParamMode,
};
use super::core::Parser;
use super::error::ParseResult;

impl Parser {
    /// Парсить класс.
    pub fn parse_class_decl(&mut self) -> ParseResult<ClassDef> {
        // Модификаторы перед class
        let is_abstract = self.match_token(&Token::Abstract);
        let is_final = self.match_token(&Token::Final);
        
        self.expect(&Token::Class, "класс")?;
        let name = self.expect_ident("имя класса")?;
        
        // Наследование: расширяет Родитель
        let parent = if self.match_token(&Token::Extends) {
            Some(self.expect_ident("родительский класс")?)
        } else { None };
        
        // Интерфейсы: реализует Интерфейс1, Интерфейс2
        let interfaces = if self.match_token(&Token::Implements) {
            let mut ifaces = Vec::new();
            loop {
                ifaces.push(self.expect_ident("интерфейс")?);
                if !self.match_token(&Token::Comma) { break; }
            }
            ifaces
        } else { Vec::new() };
        
        self.skip_newlines();
        
        // Тело класса
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut constructors = Vec::new();
        let mut destructor = None;
        let mut current_visibility = Visibility::Public;
        
        while !self.check(&Token::End) && !self.is_eof() {
            self.skip_newlines();
            if self.check(&Token::End) { break; }
            
            // Секции видимости: открытый:/закрытый:/защищённый:
            if let Some(vis) = self.try_parse_visibility_section() {
                current_visibility = vis;
                continue;
            }
            
            // Модификаторы члена
            let member_modifiers = self.parse_member_modifiers();
            
            match self.peek() {
                // Конструктор
                Token::Constructor => {
                    constructors.push(self.parse_constructor(current_visibility)?);
                }
                
                // Деструктор
                Token::Destructor => {
                    destructor = Some(self.parse_destructor(current_visibility)?);
                }
                
                // Метод
                Token::Alg => {
                    methods.push(self.parse_method(current_visibility, member_modifiers)?);
                }
                
                // Поле (тип имя)
                _ if self.is_type_start_with_custom() => {
                    fields.extend(self.parse_field(current_visibility, member_modifiers)?);
                }
                
                _ => break,
            }
        }
        
        self.expect(&Token::End, "кон")?;
        self.skip_newlines();
        
        Ok(ClassDef {
            name,
            parent,
            interfaces,
            fields,
            methods,
            constructors,
            destructor,
            is_abstract,
            is_final,
        })
    }
    
    /// Попытаться распарсить секцию видимости.
    fn try_parse_visibility_section(&mut self) -> Option<Visibility> {
        let vis = match self.peek() {
            Token::Public => Visibility::Public,
            Token::Private => Visibility::Private,
            Token::Protected => Visibility::Protected,
            _ => return None,
        };
        
        self.advance();
        
        // Ожидаем двоеточие после модификатора
        if self.match_token(&Token::Colon) {
            self.skip_newlines();
            Some(vis)
        } else {
            // Это не секция, а модификатор перед членом - откатываемся
            // (но это сложно сделать, поэтому считаем что двоеточие обязательно)
            Some(vis)
        }
    }
    
    /// Парсить модификаторы члена.
    fn parse_member_modifiers(&mut self) -> MemberModifiers {
        let mut m = MemberModifiers::default();
        
        loop {
            match self.peek() {
                Token::Static => { self.advance(); m.is_static = true; }
                Token::Virtual => { self.advance(); m.is_virtual = true; }
                Token::Abstract => { self.advance(); m.is_abstract = true; }
                Token::Override => { self.advance(); m.is_override = true; }
                Token::Final => { self.advance(); m.is_final = true; }
                Token::Async => { self.advance(); m.is_async = true; }
                _ => break,
            }
        }
        
        m
    }
    
    /// Парсить конструктор.
    fn parse_constructor(&mut self, visibility: Visibility) -> ParseResult<Constructor> {
        self.expect(&Token::Constructor, "конструктор")?;
        
        let params = if self.match_token(&Token::LParen) {
            let p = self.parse_method_params()?;
            self.expect(&Token::RParen, ")")?;
            p
        } else { Vec::new() };
        
        self.skip_newlines();
        
        // Вызов родительского конструктора: родитель(args)
        let super_call = if self.match_token(&Token::Super) {
            if self.match_token(&Token::LParen) {
                let args = self.parse_args()?;
                self.expect(&Token::RParen, ")")?;
                Some(args)
            } else { None }
        } else { None };
        
        self.expect(&Token::Begin, "нач")?;
        self.skip_newlines();
        
        let body = self.parse_stmts_until(&[Token::End])?;
        
        self.expect(&Token::End, "кон")?;
        self.skip_newlines();
        
        Ok(Constructor { params, super_call, body, visibility })
    }
    
    /// Парсить деструктор.
    fn parse_destructor(&mut self, visibility: Visibility) -> ParseResult<Method> {
        self.expect(&Token::Destructor, "деструктор")?;
        self.skip_newlines();
        
        self.expect(&Token::Begin, "нач")?;
        self.skip_newlines();
        
        let body = self.parse_stmts_until(&[Token::End])?;
        
        self.expect(&Token::End, "кон")?;
        self.skip_newlines();
        
        Ok(Method {
            name: "~destructor".to_string(),
            params: Vec::new(),
            return_type: None,
            body: Some(body),
            visibility,
            is_static: false,
            is_virtual: false,
            is_abstract: false,
            is_override: false,
            is_final: false,
            is_async: false,
        })
    }
    
    /// Парсить метод.
    fn parse_method(&mut self, visibility: Visibility, mods: MemberModifiers) -> ParseResult<Method> {
        self.expect(&Token::Alg, "алг")?;
        
        let return_type = self.try_parse_type();
        let name = self.expect_ident("имя метода")?;
        
        let params = if self.match_token(&Token::LParen) {
            let p = self.parse_method_params()?;
            self.expect(&Token::RParen, ")")?;
            p
        } else { Vec::new() };
        
        self.skip_newlines();
        
        // Абстрактные методы без тела
        let body = if mods.is_abstract || !self.check(&Token::Begin) {
            None
        } else {
            self.expect(&Token::Begin, "нач")?;
            self.skip_newlines();
            let b = self.parse_stmts_until(&[Token::End])?;
            self.expect(&Token::End, "кон")?;
            self.skip_newlines();
            Some(b)
        };
        
        Ok(Method {
            name,
            params,
            return_type,
            body,
            visibility,
            is_static: mods.is_static,
            is_virtual: mods.is_virtual,
            is_abstract: mods.is_abstract,
            is_override: mods.is_override,
            is_final: mods.is_final,
            is_async: mods.is_async,
        })
    }
    
    /// Парсить поле класса.
    fn parse_field(&mut self, visibility: Visibility, mods: MemberModifiers) -> ParseResult<Vec<Field>> {
        let type_spec = self.parse_type()?;
        let mut fields = Vec::new();
        
        loop {
            let name = self.expect_ident("имя поля")?;
            
            let default = if self.match_token(&Token::Assign) {
                Some(self.parse_expr()?)
            } else { None };
            
            fields.push(Field {
                name,
                type_spec: type_spec.clone(),
                visibility,
                default,
                is_static: mods.is_static,
            });
            
            if !self.match_token(&Token::Comma) { break; }
        }
        
        self.skip_newlines();
        Ok(fields)
    }
    
    /// Парсить параметры метода.
    fn parse_method_params(&mut self) -> ParseResult<Vec<Parameter>> {
        let mut params = Vec::new();
        
        if self.check(&Token::RParen) {
            return Ok(params);
        }
        
        loop {
            // Режим
            let mode = if self.match_token(&Token::Arg) { ParamMode::Arg }
                else if self.match_token(&Token::Res) { ParamMode::Res }
                else if self.match_token(&Token::ArgRes) { ParamMode::ArgRes }
                else { ParamMode::Arg };
            
            let type_spec = self.parse_type()?;
            let name = self.expect_ident("имя параметра")?;
            
            let default = if self.match_token(&Token::Assign) {
                Some(self.parse_expr()?)
            } else { None };
            
            params.push(Parameter { name, type_spec, mode, default });
            
            if !self.match_token(&Token::Comma) { break; }
        }
        
        Ok(params)
    }
    
    /// Парсить поля структуры.
    pub fn parse_fields_until(&mut self, end: &Token) -> ParseResult<Vec<Field>> {
        let mut fields = Vec::new();
        
        while !self.check(end) && !self.is_eof() {
            self.skip_newlines();
            if self.check(end) { break; }
            
            if self.is_type_start_with_custom() {
                let type_spec = self.parse_type()?;
                
                loop {
                    let name = self.expect_ident("имя поля")?;
                    
                    let default = if self.match_token(&Token::Assign) {
                        Some(self.parse_expr()?)
                    } else { None };
                    
                    fields.push(Field {
                        name,
                        type_spec: type_spec.clone(),
                        visibility: Visibility::Public,
                        default,
                        is_static: false,
                    });
                    
                    if !self.match_token(&Token::Comma) { break; }
                }
                
                self.skip_newlines();
            } else {
                break;
            }
        }
        
        Ok(fields)
    }
    
    /// Парсить сигнатуры методов (для интерфейсов).
    pub fn parse_method_signatures_until(&mut self, end: &Token) -> ParseResult<Vec<MethodSignature>> {
        let mut methods = Vec::new();
        
        while !self.check(end) && !self.is_eof() {
            self.skip_newlines();
            if self.check(end) { break; }
            
            if self.match_token(&Token::Alg) {
                let return_type = self.try_parse_type();
                let name = self.expect_ident("имя метода")?;
                
                let params = if self.match_token(&Token::LParen) {
                    let p = self.parse_method_params()?;
                    self.expect(&Token::RParen, ")")?;
                    p
                } else { Vec::new() };
                
                methods.push(MethodSignature { name, params, return_type });
                self.skip_newlines();
            } else {
                break;
            }
        }
        
        Ok(methods)
    }
}

/// Модификаторы члена класса.
#[derive(Default)]
struct MemberModifiers {
    is_static: bool,
    is_virtual: bool,
    is_abstract: bool,
    is_override: bool,
    is_final: bool,
    is_async: bool,
}
