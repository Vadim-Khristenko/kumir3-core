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
//!
//! ## Module Layout
//!
//! | Submodule      | Responsibility                                    |
//! |----------------|---------------------------------------------------|
//! | `class`        | class/struct declarations + inheritance clauses    |
//! | `members`      | fields, methods, signatures, ctors/dtors, params   |
//! | `trait_impl`   | interfaces, traits, impl blocks                    |
//! | `generics`     | generic type parameters `<T: Trait>`               |
//! | `mod` (here)   | member modifiers + visibility sections (shared)    |

mod class;
mod generics;
mod members;
mod trait_impl;

use super::core::Parser;
use crate::types::{Token, Visibility};

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
}
