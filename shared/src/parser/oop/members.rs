//! Kumir 3 Parser — Class & Impl Members
//!
//! [STABLE] Parses the members that live inside classes, traits, interfaces
//! and impl blocks: constructors, destructors, methods, method signatures,
//! fields and parameter lists.

use std::sync::Arc;

use super::MemberModifiers;
use crate::parser::core::Parser;
use crate::parser::error::{ParseError, ParseErrorKind, ParseResult};
use crate::types::{
    Algorithm, AlgorithmKind, CallConvention, Constructor, EffectFlags, Field, Method,
    MethodSignature, NodeId, ParamMode, Parameter, SourceSpan, Token, Visibility,
};

impl Parser {
    // =========================================================================
    //         SECTION: CONSTRUCTOR
    // =========================================================================

    /// Parses a constructor definition.
    ///
    /// ```text
    /// конструктор(арг цел x, арг цел y)
    ///   предок(x)           ← optional super call
    /// нач
    ///   это.x := x
    ///   это.y := y
    /// кон
    /// ```
    pub(super) fn parse_constructor(&mut self, visibility: Visibility) -> ParseResult<Constructor> {
        let m = self.mark();
        self.expect(&Token::Constructor, "конструктор")?;

        let params = if self.match_token(&Token::LParen) {
            let p = self.parse_method_params()?;
            self.expect(&Token::RParen, ")")?;
            p
        } else {
            Vec::new()
        };

        self.skip_newlines();

        // Optional parent constructor call: предок(args)
        let super_call = if self.match_token(&Token::Super) {
            if self.match_token(&Token::LParen) {
                let args = self.parse_args()?;
                self.expect(&Token::RParen, ")")?;
                self.skip_newlines();
                Some(args)
            } else {
                None
            }
        } else {
            None
        };

        self.expect(&Token::Begin, "нач")?;
        self.skip_newlines();

        let body = self.parse_stmts_until(&[Token::End])?;

        self.expect(&Token::End, "кон")?;
        let span = self.since(m);
        self.skip_newlines();

        Ok(Constructor {
            algorithm: Algorithm {
                id: NodeId::default(),
                name: Arc::from("constructor"),
                kind: AlgorithmKind::Constructor,
                type_params: Vec::new(),
                return_type: None,
                params,
                precondition: None,
                postcondition: None,
                body: Some(body),
                effects: EffectFlags::default(),
                attributes: Vec::new(),
                call_conv: CallConvention::Kumir,
                span: Some(SourceSpan {
                    file_id: None,
                    start: span.start.offset,
                    end: span.end.offset,
                }),
                doc: None,
            },
            super_call,
            visibility,
            attributes: Vec::new(),
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
        })
    }

    // =========================================================================
    //         SECTION: DESTRUCTOR
    // =========================================================================

    /// Parses a destructor definition.
    ///
    /// ```text
    /// деструктор
    /// нач
    ///   .. cleanup ..
    /// кон
    /// ```
    pub(super) fn parse_destructor(&mut self, visibility: Visibility) -> ParseResult<Method> {
        let m = self.mark();
        self.expect(&Token::Destructor, "деструктор")?;
        self.skip_newlines();

        self.expect(&Token::Begin, "нач")?;
        self.skip_newlines();

        let body = self.parse_stmts_until(&[Token::End])?;

        self.expect(&Token::End, "кон")?;
        let span = self.since(m);
        self.skip_newlines();

        Ok(Method {
            algorithm: Algorithm {
                id: NodeId::default(),
                name: Arc::from("~destructor"),
                kind: AlgorithmKind::Destructor,
                type_params: Vec::new(),
                return_type: None,
                params: Vec::new(),
                precondition: None,
                postcondition: None,
                body: Some(body),
                effects: EffectFlags::default(),
                attributes: Vec::new(),
                call_conv: CallConvention::Kumir,
                span: Some(SourceSpan {
                    file_id: None,
                    start: span.start.offset,
                    end: span.end.offset,
                }),
                doc: None,
            },
            visibility,
            is_static: false,
            is_virtual: false,
            is_override: false,
            is_final: false,
            is_abstract: false,
            attributes: Vec::new(),
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
        })
    }

    // =========================================================================
    //         SECTION: METHOD
    // =========================================================================

    /// Parses a method definition (inside a class or impl block).
    ///
    /// ```text
    /// [static] [virtual] [override] [final] [abstract] [async]
    /// алг [тип] Name [<T>] (params)
    /// [дано precondition]
    /// [надо postcondition]
    /// нач
    ///   body
    /// кон
    /// ```
    ///
    /// Abstract methods have no body.
    pub(super) fn parse_method(
        &mut self,
        visibility: Visibility,
        mods: &MemberModifiers,
    ) -> ParseResult<Method> {
        let m = self.mark();
        self.expect(&Token::Alg, "алг")?;

        // ── Effects ─────────────────────────────────────────────────
        let mut effects = EffectFlags::default();
        if mods.is_async || self.match_token(&Token::Async) {
            effects.is_async = true;
        }

        // ── Optional return type before name ────────────────────────
        let mut return_type = self.try_parse_type();

        // ── Name ────────────────────────────────────────────────────
        let name: Arc<str> = Arc::from(self.expect_ident("имя метода")?.as_str());

        // ── Type parameters ─────────────────────────────────────────
        let type_params = self.try_parse_type_params();

        // ── Parameters ──────────────────────────────────────────────
        let params = if self.match_token(&Token::LParen) {
            let p = self.parse_method_params()?;
            self.expect(&Token::RParen, ")")?;
            p
        } else {
            Vec::new()
        };

        // ── Return type after params: алг Name(...): Type ───────────
        if self.match_token(&Token::Colon) {
            return_type = Some(self.parse_type()?);
        }

        self.skip_newlines();

        let kind = AlgorithmKind::Method;

        // ── Precondition (дано) ─────────────────────────────────────
        let precondition = if self.match_token(&Token::Given) {
            let e = self.parse_expr()?;
            self.skip_newlines();
            Some(e)
        } else {
            None
        };

        // ── Postcondition (надо) ────────────────────────────────────
        let postcondition = if self.match_token(&Token::Need) {
            let e = self.parse_expr()?;
            self.skip_newlines();
            Some(e)
        } else {
            None
        };

        // ── Body (abstract methods have none) ───────────────────────
        let body = if mods.is_abstract || !self.check(&Token::Begin) {
            None
        } else {
            self.expect(&Token::Begin, "нач")?;
            self.skip_newlines();
            let stmts = self.parse_stmts_until(&[Token::End])?;
            self.expect(&Token::End, "кон")?;
            self.skip_newlines();
            Some(stmts)
        };

        let span = self.since(m);

        Ok(Method {
            algorithm: Algorithm {
                id: NodeId::default(),
                name,
                kind,
                type_params,
                return_type,
                params,
                precondition,
                postcondition,
                body,
                effects,
                attributes: Vec::new(),
                call_conv: CallConvention::Kumir,
                span: Some(SourceSpan {
                    file_id: None,
                    start: span.start.offset,
                    end: span.end.offset,
                }),
                doc: None,
            },
            visibility,
            is_static: mods.is_static,
            is_virtual: mods.is_virtual,
            is_override: mods.is_override,
            is_final: mods.is_final,
            is_abstract: mods.is_abstract,
            attributes: Vec::new(),
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
        })
    }

    // =========================================================================
    //         SECTION: METHOD SIGNATURES (for interfaces)
    // =========================================================================

    /// Parses method signatures until a stop token.
    ///
    /// Used by interface declarations to collect a list of method
    /// signatures without bodies.
    pub fn parse_method_signatures_until(
        &mut self,
        end: &Token,
    ) -> ParseResult<Vec<MethodSignature>> {
        let mut methods = Vec::new();

        while !self.check(end) && !self.is_eof() {
            self.skip_newlines();
            if self.check(end) {
                break;
            }

            if self.check(&Token::Alg) {
                methods.push(self.parse_single_method_signature()?);
                self.skip_newlines();
            } else {
                self.report_error(ParseError::new(
                    ParseErrorKind::InvalidClassMember,
                    "expected method signature in interface",
                    self.span(),
                ));
                self.advance();
            }
        }

        Ok(methods)
    }

    /// Parses a single method signature: `алг [type] Name [<T>] (params)`.
    pub(super) fn parse_single_method_signature(&mut self) -> ParseResult<MethodSignature> {
        let m = self.mark();
        self.expect(&Token::Alg, "алг")?;

        let mut return_type = self.try_parse_type();
        let name: Arc<str> = Arc::from(self.expect_ident("имя метода")?.as_str());
        let type_params = self.try_parse_type_params();

        let params = if self.match_token(&Token::LParen) {
            let p = self.parse_method_params()?;
            self.expect(&Token::RParen, ")")?;
            p
        } else {
            Vec::new()
        };

        // Return type after params
        if self.match_token(&Token::Colon) {
            return_type = Some(self.parse_type()?);
        }

        let span = self.since(m);

        Ok(MethodSignature {
            name,
            type_params,
            params,
            return_type,
            attributes: Vec::new(),
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
        })
    }

    // =========================================================================
    //         SECTION: FIELDS
    // =========================================================================

    /// Parses one or more field declarations of the same type.
    ///
    /// ```text
    /// [static] [mut] цел x, y, z
    /// [static] лит имя := "Анонимус"
    /// ```
    pub(super) fn parse_fields(
        &mut self,
        visibility: Visibility,
        mods: &MemberModifiers,
    ) -> ParseResult<Vec<Field>> {
        let _m = self.mark();
        let is_mutable = self.match_token(&Token::Mut);
        let type_kind = self.parse_type()?;

        let mut fields = Vec::new();

        loop {
            let field_m = self.mark();
            let name: Arc<str> = Arc::from(self.expect_ident("имя поля")?.as_str());

            let default = if self.match_token(&Token::Assign) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            let field_span = self.since(field_m);

            fields.push(Field {
                id: NodeId::default(),
                name,
                type_kind: type_kind.clone(),
                visibility,
                default,
                is_static: mods.is_static,
                is_mutable,
                attributes: Vec::new(),
                span: Some(SourceSpan {
                    file_id: None,
                    start: field_span.start.offset,
                    end: field_span.end.offset,
                }),
                doc: None,
            });

            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        self.skip_newlines();
        Ok(fields)
    }

    /// Parses struct-body fields (used by `parse_fields_until` for
    /// structs parsed outside of OOP context, e.g. by `decl.rs`).
    ///
    /// ```text
    /// структура Точка
    ///   вещ x, y
    ///   лит метка := ""
    /// кон
    /// ```
    pub fn parse_fields_until(&mut self, end: &Token) -> ParseResult<Vec<Field>> {
        let mut fields = Vec::new();
        let mods = MemberModifiers::default();

        while !self.check(end) && !self.is_eof() {
            self.skip_newlines();
            if self.check(end) {
                break;
            }

            if self.is_type_start_with_custom() {
                let new_fields = self.parse_fields(Visibility::Public, &mods)?;
                fields.extend(new_fields);
            } else {
                break;
            }
        }

        Ok(fields)
    }

    // =========================================================================
    //         SECTION: METHOD PARAMETERS
    // =========================================================================

    /// Parses a comma-separated method parameter list.
    ///
    /// Supports both classic and modern syntaxes:
    /// - Classic: `арг цел x, арг цел y`
    /// - Modern:  `x: цел, y: цел`
    fn parse_method_params(&mut self) -> ParseResult<Vec<Parameter>> {
        if self.check(&Token::RParen) {
            return Ok(Vec::new());
        }
        self.comma_sep(&Token::RParen, |p| p.parse_method_param())
    }

    /// Parses a single method parameter.
    fn parse_method_param(&mut self) -> ParseResult<Parameter> {
        // ── Optional mode: арг / рез / аргрез ───────────────────────
        let mode = if self.match_token(&Token::Arg) {
            ParamMode::In
        } else if self.match_token(&Token::Res) {
            ParamMode::Out
        } else if self.match_token(&Token::ArgRes) {
            ParamMode::InOut
        } else {
            ParamMode::In
        };

        // ── Modern syntax: name : Type ──────────────────────────────
        if self.is_ident() && matches!(self.peek_n(1), Token::Colon) {
            let name = Arc::from(self.expect_ident("имя параметра")?.as_str());
            self.expect(&Token::Colon, ":")?;
            let type_kind = Some(self.parse_type()?);

            let default = if self.match_token(&Token::Assign) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            return Ok(Parameter {
                id: NodeId::default(),
                name,
                type_kind,
                mode,
                default,
                attributes: Vec::new(),
                span: None,
            });
        }

        // ── Classic syntax: Type name ───────────────────────────────
        let type_kind = Some(self.parse_type()?);
        let name = Arc::from(self.expect_ident("имя параметра")?.as_str());

        let default = if self.match_token(&Token::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(Parameter {
            id: NodeId::default(),
            name,
            type_kind,
            mode,
            default,
            attributes: Vec::new(),
            span: None,
        })
    }
}
