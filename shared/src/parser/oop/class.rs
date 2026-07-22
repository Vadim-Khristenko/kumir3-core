//! Kumir 3 Parser — Class & Struct Declarations
//!
//! [STABLE] Parses `класс` / `структура` declarations together with their
//! inheritance clauses (`расширяет`, `реализует`) and dispatches the body
//! members to the parsers in [`super::members`].

use std::sync::Arc;

use crate::parser::core::Parser;
use crate::parser::error::{ParseError, ParseErrorKind, ParseResult};
use crate::types::{ClassDef, ClassKind, Method, NodeId, SourceSpan, Token, Visibility};

impl Parser {
    // =========================================================================
    //         SECTION: CLASS DECLARATION
    // =========================================================================

    /// Parses a class or struct declaration.
    ///
    /// ```text
    /// [абстрактный] [финальный] класс Name [<T, U>] [расширяет Parent]
    ///                                      [реализует Iface1, Trait1]
    ///   [открытый:  ]
    ///     поля / методы / конструкторы / деструктор
    /// кон
    /// ```
    ///
    /// Struct syntax uses `структура` instead of `класс` and produces
    /// `ClassKind::Struct`.
    pub fn parse_class_decl(&mut self) -> ParseResult<ClassDef> {
        let m = self.mark();

        // ── Pre-modifiers ───────────────────────────────────────────
        let is_abstract = self.match_token(&Token::Abstract);
        let is_final = self.match_token(&Token::Final);

        // ── Class vs Struct ─────────────────────────────────────────
        let kind = if self.match_token(&Token::Class) {
            ClassKind::Class
        } else if self.match_token(&Token::Struct) {
            ClassKind::Struct
        } else {
            return Err(ParseError::new(
                ParseErrorKind::InvalidClassMember,
                "expected 'класс' or 'структура'",
                self.span(),
            )
            .into());
        };

        let name: Arc<str> = Arc::from(self.expect_ident("class name")?.as_str());

        // ── Generic type parameters: <T, U: Trait> ──────────────────
        let type_params = self.try_parse_type_params();

        // ── Inheritance: расширяет Parent ────────────────────────────
        let parent: Option<Arc<str>> = if self.match_token(&Token::Extends) {
            Some(Arc::from(self.expect_ident("parent class")?.as_str()))
        } else {
            None
        };

        // ── Interfaces / Traits: реализует Iface1, Iface2 ─────────────
        // At parse time we cannot distinguish interfaces from traits,
        // so all names go into `interfaces`. The semantic analysis pass
        // moves trait names to `traits` once it resolves the declarations.
        let mut interfaces: Vec<Arc<str>> = Vec::new();
        let traits: Vec<Arc<str>> = Vec::new();

        if self.match_token(&Token::Implements) {
            loop {
                interfaces.push(Arc::from(
                    self.expect_ident("interface/trait name")?.as_str(),
                ));
                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }

        self.skip_newlines();

        // ── Body ────────────────────────────────────────────────────
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        let mut constructors = Vec::new();
        let mut destructor: Option<Method> = None;
        let mut current_visibility = Visibility::Public;

        while !self.check(&Token::End) && !self.is_eof() {
            self.skip_newlines();
            if self.check(&Token::End) {
                break;
            }

            // ── Visibility sections: открытый: / закрытый: / защищённый:
            if let Some(vis) = self.try_parse_visibility_section() {
                current_visibility = vis;
                continue;
            }

            // ── Member modifiers ────────────────────────────────────
            let mods = self.parse_member_modifiers();

            match self.peek() {
                // ── Constructor ──────────────────────────────────────
                Token::Constructor => {
                    constructors.push(self.parse_constructor(current_visibility)?);
                }

                // ── Destructor ──────────────────────────────────────
                Token::Destructor => {
                    destructor = Some(self.parse_destructor(current_visibility)?);
                }

                // ── Method (алг) ────────────────────────────────────
                Token::Alg => {
                    methods.push(self.parse_method(current_visibility, &mods)?);
                }

                // ── Field (type-led) ────────────────────────────────
                _ if self.is_type_start_with_custom() => {
                    let new_fields = self.parse_fields(current_visibility, &mods)?;
                    fields.extend(new_fields);
                }

                // ── Unknown member ──────────────────────────────────
                _ => {
                    self.report_error(ParseError::new(
                        ParseErrorKind::InvalidClassMember,
                        "expected field, method, constructor, or destructor",
                        self.span(),
                    ));
                    self.advance(); // skip to avoid infinite loop
                }
            }
        }

        self.expect(&Token::End, "кон")?;
        let span = self.since(m);
        self.skip_newlines();

        Ok(ClassDef {
            id: NodeId::default(),
            name,
            kind,
            type_params,
            parent,
            interfaces,
            traits,
            fields,
            methods,
            constructors,
            destructor,
            is_abstract,
            is_final,
            attributes: Vec::new(),
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
            doc: None,
        })
    }
}
