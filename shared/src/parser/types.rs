//! Kumir 3 Parser — Type Parsing
//!
//! [STABLE] Parses type annotations in declarations, parameters,
//! return types, casts, and generic arguments.
//!
//! ## Architecture
//!
//! ```text
//! Type grammar (EBNF-ish):
//!
//!   type          = builtin_type | composite_type | function_type | custom_type
//!   builtin_type  = "цел" | "вещ" | "лог" | "сим" | "лит" | "авто" | "пусто"
//!   composite_type = array_type | pointer_type | optional_type | result_type
//!   array_type    = "таб" type?
//!   pointer_type  = "указатель" type?
//!   optional_type = "необязательно" ("<" type ">" | type)
//!   result_type   = "результат" "<" type "," type ">"
//!   function_type = "(" type_list? ")" "->" type
//!   custom_type   = ident type_args?
//!   type_args     = "<" type ("," type)* ">"
//! ```
//!
//! ## Token → TypeKind Mapping
//!
//! | Token          | TypeKind       | Note                       |
//! |----------------|----------------|----------------------------|
//! | `IntType`      | `Int64`        | Default integer width      |
//! | `FloatType`    | `Float64`      | Default float width        |
//! | `BoolType`     | `Bool`         |                            |
//! | `CharType`     | `Char`         |                            |
//! | `StringType`   | `String`       |                            |
//! | `AutoType`     | `Auto`         | Type inference placeholder |
//! | `NoneType`     | `Null`         | Absence-of-value type      |
//! | `ArrayType`    | `Array(elem)`  | Element type follows       |
//! | `PointerType`  | `Pointer(t)`   | Pointee type follows       |
//! | `OptionalType` | `Option(t)`    | Inner type follows         |
//! | identifier     | `Object(name)` | User-defined / class type  |

use super::core::Parser;
use super::error::{ParseError, ParseResult};
use crate::types::{Token, TypeKind};

impl Parser {
    // =========================================================================
    //         SECTION: BUILTIN TYPE PARSING
    // =========================================================================

    /// Attempts to parse a **builtin** type keyword.
    ///
    /// Returns `None` if the current token is not a type keyword.
    /// Does **not** recognise identifiers as types — use
    /// [`try_parse_type_with_custom`] for that.
    pub fn try_parse_type(&mut self) -> Option<TypeKind> {
        let kind = match self.peek() {
            // Scalar builtins
            Token::IntType => TypeKind::Int64,
            Token::FloatType => TypeKind::Float64,
            Token::BoolType => TypeKind::Bool,
            Token::CharType => TypeKind::Char,
            Token::StringType => TypeKind::String,
            Token::AutoType => TypeKind::Auto,
            Token::NoneType => TypeKind::Null,

            // Composite builtins (consume keyword, then recurse for inner type)
            Token::ArrayType => {
                self.advance();
                let elem = self.try_parse_type().unwrap_or(TypeKind::Auto);
                return Some(TypeKind::Array(Box::new(elem)));
            }

            Token::PointerType => {
                self.advance();
                let pointee = self.try_parse_type().unwrap_or(TypeKind::Auto);
                return Some(TypeKind::Pointer(Box::new(pointee)));
            }

            Token::OptionalType => {
                self.advance();
                return Some(self.parse_optional_inner());
            }

            _ => return None,
        };

        self.advance();
        Some(kind)
    }

    /// Parses the inner type of an optional/result wrapper.
    ///
    /// Supports both `Необязательно<T>` and `Необязательно T` syntaxes.
    fn parse_optional_inner(&mut self) -> TypeKind {
        // Generic bracket syntax: Необязательно<T>
        if self.match_token(&Token::Less) {
            if let Some(inner) = self.try_parse_type_with_custom() {
                let _ = self.match_token(&Token::Greater);
                return TypeKind::option(inner);
            }
            // Failed to parse inner type — fallback to Auto
            return TypeKind::option(TypeKind::Auto);
        }
        // Space syntax: Необязательно T
        let inner = self.try_parse_type().unwrap_or(TypeKind::Auto);
        TypeKind::option(inner)
    }

    // =========================================================================
    //         SECTION: FULL TYPE PARSING (builtins + custom)
    // =========================================================================

    /// Attempts to parse any type: builtin, function, or user-defined.
    ///
    /// Returns `None` if the current token cannot begin a type expression.
    pub fn try_parse_type_with_custom(&mut self) -> Option<TypeKind> {
        // 1. Function type: (T1, T2) -> R
        if self.check(&Token::LParen)
            && let Some(ft) = self.try_parse(|p| p.parse_function_type())
        {
            return Some(ft);
        }

        // 2. Builtin types
        if let Some(t) = self.try_parse_type() {
            return Some(t);
        }

        // 3. User-defined type (class, enum, generic)
        self.try_parse_custom_type()
    }

    /// Parses a user-defined type: identifier with optional generic args.
    ///
    /// Examples: `Точка`, `Список<цел>`, `Словарь<лит, цел>`.
    fn try_parse_custom_type(&mut self) -> Option<TypeKind> {
        let name = match self.peek() {
            Token::Ident(s)
            | Token::TypeIdent(s)
            | Token::ClassIdent(s)
            | Token::VarIdent(s)
            | Token::FuncIdent(s)
            | Token::NamespaceIdent(s) => s.clone(),
            _ => return None,
        };
        self.advance();

        // Generic arguments: Тип<A, B>
        if self.match_token(&Token::Less) {
            let type_args = self.parse_generic_args();
            if !type_args.is_empty() {
                return Some(TypeKind::Generic { name, type_args });
            }
        }

        Some(TypeKind::Object(name))
    }

    /// Parses comma-separated type arguments inside `<...>`.
    ///
    /// Handles nested `<>` via depth tracking.  Returns the parsed
    /// types (may be empty if parsing fails).
    fn parse_generic_args(&mut self) -> Vec<TypeKind> {
        let mut args = Vec::new();
        let mut depth = 1u32;

        // Try to parse proper type args
        loop {
            if self.is_eof() || depth == 0 {
                break;
            }

            // Check for closing `>`
            if self.check(&Token::Greater) {
                depth -= 1;
                if depth == 0 {
                    self.advance();
                    break;
                }
            }

            // Try to parse a type argument
            if let Some(t) = self.try_parse_type_with_custom() {
                args.push(t);
            } else {
                // Skip unknown tokens inside <...>
                match self.peek() {
                    Token::Less => {
                        depth += 1;
                        self.advance();
                    }
                    Token::Greater => {
                        depth -= 1;
                        self.advance();
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {
                        self.advance();
                    }
                }
                continue;
            }

            // Separator or end
            if !self.match_token(&Token::Comma) {
                // Expect closing >
                if self.check(&Token::Greater) {
                    self.advance();
                    break;
                }
                // Broken generic — skip to >
                self.skip_until(|t| matches!(t, Token::Greater | Token::Newline | Token::EOF));
                let _ = self.match_token(&Token::Greater);
                break;
            }
        }

        args
    }

    // =========================================================================
    //         SECTION: REQUIRED TYPE PARSING
    // =========================================================================

    /// Parses a type expression (required — returns error if absent).
    ///
    /// Recognises builtins, function types, and user-defined types.
    pub fn parse_type(&mut self) -> ParseResult<TypeKind> {
        self.try_parse_type_with_custom()
            .ok_or_else(|| ParseError::expected_type(self.span()).into())
    }

    // =========================================================================
    //         SECTION: FUNCTION TYPE
    // =========================================================================

    /// Parses a function type: `(T1, T2) -> R`.
    ///
    /// Called speculatively via `try_parse()` — safe to fail.
    fn parse_function_type(&mut self) -> ParseResult<TypeKind> {
        self.expect(&Token::LParen, "(")?;
        let params = self.comma_sep(&Token::RParen, |p| p.parse_type())?;
        self.expect(&Token::RParen, ")")?;
        self.expect(&Token::Arrow, "->")?;
        let result = self.parse_type()?;
        Ok(TypeKind::Function {
            params,
            result: Some(Box::new(result)),
        })
    }

    // =========================================================================
    //         SECTION: TYPE START PREDICATES
    // =========================================================================

    /// `true` if the current token is a builtin type keyword.
    #[inline]
    pub fn is_type_start(&self) -> bool {
        matches!(
            self.peek(),
            Token::IntType
                | Token::FloatType
                | Token::BoolType
                | Token::CharType
                | Token::StringType
                | Token::ArrayType
                | Token::AutoType
                | Token::PointerType
                | Token::OptionalType
                | Token::NoneType
        )
    }

    /// `true` if the current token can begin any type (builtin or custom).
    #[inline]
    pub fn is_type_start_with_custom(&self) -> bool {
        self.is_type_start() || self.is_ident() || self.check(&Token::LParen)
    }
}
