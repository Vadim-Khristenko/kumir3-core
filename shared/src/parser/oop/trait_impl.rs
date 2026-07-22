//! Kumir 3 Parser — Interfaces, Traits & Impl Blocks
//!
//! [STABLE] Parses `интерфейс`, `трейт` and `реализация` declarations,
//! including supertrait lists, `расширяет` chains and trait methods with
//! optional default implementations.

use std::sync::Arc;

use crate::parser::core::Parser;
use crate::parser::error::{ParseError, ParseErrorKind, ParseResult};
use crate::types::{
    ImplDef, InterfaceDef, NodeId, SourceSpan, Token, TraitDef, TraitMethod, Visibility,
};

impl Parser {
    // =========================================================================
    //         SECTION: INTERFACE DECLARATION
    // =========================================================================

    /// Parses an interface declaration.
    ///
    /// ```text
    /// интерфейс Drawable [<T>] [расширяет Base1, Base2]
    ///   алг рисовать()
    ///   алг цвет(): Цвет
    /// кон
    /// ```
    pub fn parse_interface_decl(&mut self) -> ParseResult<InterfaceDef> {
        let m = self.mark();
        self.expect(&Token::Interface, "интерфейс")?;

        let name: Arc<str> = Arc::from(self.expect_ident("interface name")?.as_str());
        let type_params = self.try_parse_type_params();

        let extends: Vec<Arc<str>> = if self.match_token(&Token::Extends) {
            let mut bases = Vec::new();
            loop {
                bases.push(Arc::from(self.expect_ident("parent interface")?.as_str()));
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
            bases
        } else {
            Vec::new()
        };

        self.skip_newlines();

        let methods = self.parse_method_signatures_until(&Token::End)?;

        self.expect(&Token::End, "кон")?;
        let span = self.since(m);
        self.skip_newlines();

        Ok(InterfaceDef {
            id: NodeId::default(),
            name,
            type_params,
            extends,
            methods,
            attributes: Vec::new(),
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
            doc: None,
        })
    }

    // =========================================================================
    //         SECTION: TRAIT DECLARATION
    // =========================================================================

    /// Parses a trait declaration with optional default implementations.
    ///
    /// ```text
    /// трейт Printable [<T>] [: Displayable, Comparable]
    ///   алг вывести()
    ///   алг форматировать(лит формат): лит
    ///   нач
    ///     знач := ""
    ///   кон
    /// кон
    /// ```
    pub fn parse_trait_decl(&mut self) -> ParseResult<TraitDef> {
        let m = self.mark();
        self.expect(&Token::Trait, "трейт")?;

        let name: Arc<str> = Arc::from(self.expect_ident("trait name")?.as_str());
        let type_params = self.try_parse_type_params();

        // Supertraits: трейт Foo : Bar, Baz
        let supertraits: Vec<Arc<str>> = if self.match_token(&Token::Colon) {
            let mut supers = Vec::new();
            loop {
                supers.push(Arc::from(self.expect_ident("supertrait")?.as_str()));
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
            supers
        } else {
            Vec::new()
        };

        self.skip_newlines();

        let mut methods = Vec::new();

        while !self.check(&Token::End) && !self.is_eof() {
            self.skip_newlines();
            if self.check(&Token::End) {
                break;
            }

            if self.check(&Token::Alg) {
                methods.push(self.parse_trait_method()?);
            } else {
                self.report_error(ParseError::new(
                    ParseErrorKind::InvalidClassMember,
                    "expected method declaration in trait",
                    self.span(),
                ));
                self.advance();
            }
        }

        self.expect(&Token::End, "кон")?;
        let span = self.since(m);
        self.skip_newlines();

        Ok(TraitDef {
            id: NodeId::default(),
            name,
            type_params,
            supertraits,
            methods,
            attributes: Vec::new(),
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
            doc: None,
        })
    }

    // =========================================================================
    //         SECTION: IMPL BLOCK
    // =========================================================================

    /// Parses an impl block (inherent or trait impl).
    ///
    /// ```text
    /// реализация Printable для Точка [<T>]
    ///   алг вывести() нач ... кон
    /// кон
    ///
    /// реализация Точка
    ///   алг расстояние(Точка другая): вещ нач ... кон
    /// кон
    /// ```
    pub fn parse_impl_block(&mut self) -> ParseResult<ImplDef> {
        let m = self.mark();
        self.expect(&Token::Impl, "реализация")?;

        let first_name: Arc<str> = Arc::from(self.expect_ident("type or trait name")?.as_str());
        let type_params = self.try_parse_type_params();

        // Check for "для" (for) — trait impl
        let (trait_name, target) = if self.match_keyword("для") {
            let target: Arc<str> = Arc::from(self.expect_ident("target type")?.as_str());
            (Some(first_name), target)
        } else {
            (None, first_name)
        };

        self.skip_newlines();

        let mut methods = Vec::new();

        while !self.check(&Token::End) && !self.is_eof() {
            self.skip_newlines();
            if self.check(&Token::End) {
                break;
            }

            let mods = self.parse_member_modifiers();

            if self.check(&Token::Alg) {
                methods.push(self.parse_method(Visibility::Public, &mods)?);
            } else {
                self.report_error(ParseError::new(
                    ParseErrorKind::InvalidClassMember,
                    "expected method in impl block",
                    self.span(),
                ));
                self.advance();
            }
        }

        self.expect(&Token::End, "кон")?;
        let span = self.since(m);
        self.skip_newlines();

        Ok(ImplDef {
            id: NodeId::default(),
            trait_name,
            type_params,
            target,
            methods,
            attributes: Vec::new(),
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
            doc: None,
        })
    }

    // =========================================================================
    //         SECTION: TRAIT METHOD (with optional default impl)
    // =========================================================================

    /// Parses a method inside a trait — may have a default implementation.
    ///
    /// ```text
    /// алг вывести()               ← abstract (no body)
    ///
    /// алг строка(): лит           ← has default impl
    /// нач
    ///   знач := ""
    /// кон
    /// ```
    fn parse_trait_method(&mut self) -> ParseResult<TraitMethod> {
        let m = self.mark();

        let sig = self.parse_single_method_signature()?;

        self.skip_newlines();

        // Optional default body
        let default_impl = if self.check(&Token::Begin) {
            self.expect(&Token::Begin, "нач")?;
            self.skip_newlines();
            let body = self.parse_stmts_until(&[Token::End])?;
            self.expect(&Token::End, "кон")?;
            self.skip_newlines();
            Some(body)
        } else {
            None
        };

        let span = self.since(m);

        Ok(TraitMethod {
            signature: sig,
            default_impl,
            attributes: Vec::new(),
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
        })
    }
}
