//! Kumir 3 Parser — Class, Interface, Trait & Impl Parsing
//!
//! [STABLE] Parses all OOP constructs: classes, structs, interfaces, traits,
//! impl blocks, methods, constructors, destructors, and fields.
//!
//! ## Architecture
//!
//! ```text
//!   parse_class_decl()              ← [abstract] [final] класс Name ...
//!     ├── parse_visibility_section()
//!     ├── parse_member_modifiers()
//!     ├── parse_constructor()       ← конструктор(params) нач ... кон
//!     ├── parse_destructor()        ← деструктор нач ... кон
//!     ├── parse_method()            ← алг [тип] Name(params) нач ... кон
//!     └── parse_field()             ← тип name [, name2] [:= init]
//!
//!   parse_interface_decl()          ← интерфейс Name [расширяет ...]
//!     └── parse_method_signature()  ← алг [тип] Name(params)
//!
//!   parse_trait_decl()              ← трейт Name [: SuperTrait1, ...]
//!     └── parse_trait_method()      ← алг ... [нач ... кон]
//!
//!   parse_impl_block()              ← реализация [Trait для] Type
//!     └── parse_method()
//! ```
//!
//! ## Token → OOP Construct Mapping
//!
//! | Token(s)             | Construct                      |
//! |----------------------|--------------------------------|
//! | `[Abstract] Class`   | `ClassDef { kind: Class }`     |
//! | `Struct`             | `ClassDef { kind: Struct }`    |
//! | `Interface`          | `InterfaceDef`                 |
//! | `Trait`              | `TraitDef`                     |
//! | `Impl`               | `ImplDef`                      |
//! | `Constructor`        | `Constructor { algorithm }`    |
//! | `Destructor`         | `Method (destructor)`          |
//! | `Alg`                | `Method { algorithm }`         |
//! | type-start tokens    | `Field { type_kind, name }`    |
//!
//! ## Field Layout (class.rs types)
//!
//! ```text
//! ClassDef: id, name, kind, type_params, parent, interfaces, traits,
//!           fields, methods, constructors, destructor,
//!           is_abstract, is_final, attributes, span, doc
//!
//! Field:    id, name, type_kind, visibility, default,
//!           is_static, is_mutable, attributes, span, doc
//!
//! Method:   algorithm, visibility, is_static, is_virtual,
//!           is_override, is_final, is_abstract, attributes, span
//!
//! Constructor: algorithm, super_call, visibility, attributes, span
//! ```

use std::sync::Arc;

use super::core::Parser;
use super::error::{ParseError, ParseErrorKind, ParseResult};
use crate::types::{
    Algorithm, AlgorithmKind, CallConvention, ClassDef, ClassKind, Constructor, EffectFlags, Field,
    ImplDef, InterfaceDef, Method, MethodSignature, NodeId, ParamMode, Parameter, SourceSpan,
    Token, TraitDef, TraitMethod, TypeConstraint, TypeParam, Visibility,
};

// =============================================================================
//         SECTION: MEMBER MODIFIERS (parser-local)
// =============================================================================

/// Modifiers for class/struct members, collected before the member
/// keyword to determine method/field properties.
#[derive(Debug, Default)]
struct MemberModifiers {
    is_static: bool,
    is_virtual: bool,
    is_abstract: bool,
    is_override: bool,
    is_final: bool,
    is_async: bool,
}

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
    fn parse_constructor(&mut self, visibility: Visibility) -> ParseResult<Constructor> {
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
    fn parse_destructor(&mut self, visibility: Visibility) -> ParseResult<Method> {
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
    fn parse_method(
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
        let name: Arc<str> = Arc::from(self.expect_ident("method name")?.as_str());

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

        let kind = if return_type.is_some() {
            AlgorithmKind::Method
        } else {
            AlgorithmKind::Method
        };

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
    fn parse_single_method_signature(&mut self) -> ParseResult<MethodSignature> {
        let m = self.mark();
        self.expect(&Token::Alg, "алг")?;

        let mut return_type = self.try_parse_type();
        let name: Arc<str> = Arc::from(self.expect_ident("method name")?.as_str());
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
    fn parse_fields(
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
            let name: Arc<str> = Arc::from(self.expect_ident("field name")?.as_str());

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
            let name = Arc::from(self.expect_ident("parameter name")?.as_str());
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
        let name = Arc::from(self.expect_ident("parameter name")?.as_str());

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

    // =========================================================================
    //         SECTION: VISIBILITY SECTIONS
    // =========================================================================

    /// Tries to parse a visibility section header (`открытый:`).
    ///
    /// Returns `Some(Visibility)` if a valid section header was found.
    fn try_parse_visibility_section(&mut self) -> Option<Visibility> {
        let vis = match self.peek() {
            Token::Public => Visibility::Public,
            Token::Private => Visibility::Private,
            Token::Protected => Visibility::Protected,
            _ => return None,
        };

        // Must be followed by `:` to be a section header
        if !matches!(self.peek_n(1), Token::Colon) {
            return None;
        }

        self.advance(); // consume visibility keyword
        self.advance(); // consume `:`
        self.skip_newlines();

        Some(vis)
    }

    // =========================================================================
    //         SECTION: MEMBER MODIFIERS
    // =========================================================================

    /// Parses zero or more member modifiers preceding a method or field.
    ///
    /// ```text
    /// static virtual алг ...
    /// static final цел ...
    /// abstract алг ...
    /// override алг ...
    /// async алг ...
    /// ```
    fn parse_member_modifiers(&mut self) -> MemberModifiers {
        let mut m = MemberModifiers::default();

        loop {
            match self.peek() {
                Token::Static => {
                    self.advance();
                    m.is_static = true;
                }
                Token::Virtual => {
                    self.advance();
                    m.is_virtual = true;
                }
                Token::Abstract => {
                    self.advance();
                    m.is_abstract = true;
                }
                Token::Override => {
                    self.advance();
                    m.is_override = true;
                }
                Token::Final => {
                    self.advance();
                    m.is_final = true;
                }
                Token::Async => {
                    self.advance();
                    m.is_async = true;
                }
                _ => break,
            }
        }

        m
    }

    // =========================================================================
    //         SECTION: TYPE PARAMETERS (Generics)
    // =========================================================================

    /// Tries to parse generic type parameters: `<T, U: Constraint>`.
    ///
    /// Returns empty vec if no `<` follows.
    fn try_parse_type_params(&mut self) -> Vec<TypeParam> {
        if !self.match_token(&Token::Less) {
            return Vec::new();
        }

        let mut params = Vec::new();

        loop {
            if self.check(&Token::Greater) {
                break;
            }

            let name: Arc<str> = match self.expect_ident("type parameter") {
                Ok(n) => Arc::from(n.as_str()),
                Err(_) => break,
            };

            // Constraints: T: Trait1 + Trait2
            let constraints = if self.match_token(&Token::Colon) {
                let mut c = Vec::new();
                loop {
                    let cname = match self.expect_ident("constraint") {
                        Ok(n) => Arc::from(n.as_str()),
                        Err(_) => break,
                    };
                    c.push(TypeConstraint::Implements(cname));
                    if !self.match_token(&Token::Plus) {
                        break;
                    }
                }
                c
            } else {
                Vec::new()
            };

            params.push(TypeParam {
                name,
                constraints,
                span: None,
            });

            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        let _ = self.match_token(&Token::Greater);
        params
    }
}
