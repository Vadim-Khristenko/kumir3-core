//! Kumir 3 Parser — Pattern Parsing
//!
//! [STABLE] Parses all pattern forms used in `match` expressions,
//! `match` statements, destructuring assignments, and function parameter
//! matching.
//!
//! ## Architecture
//!
//! ```text
//!   parse_pattern_with_or()       ← top-level: p1 | p2 | p3
//!     └── parse_pattern()         ← single pattern dispatch
//!           ├── parse_guard_or_binding()
//!           │     └── parse_atomic_pattern()
//!           │           ├── Wildcard: _
//!           │           ├── Literal: 42, 3.14, "text", 'c', true, false
//!           │           ├── Rest: ..
//!           │           ├── Optional: Some(p) / None  →  OptionSome/OptionNone
//!           │           ├── Result: Ok(p) / Err(p)    →  ResultOk/ResultErr
//!           │           ├── Negative literal: -42
//!           │           ├── Tuple: (p1, p2, ...)
//!           │           ├── Array: [p1, p2, ...rest]
//!           │           ├── Struct: Name { f1: p, f2: p, .. }
//!           │           ├── EnumVariant: Enum::Variant(bindings)
//!           │           ├── MutableVariable: mut x
//!           │           ├── Typed: p : Type
//!           │           ├── Range: a..b  /  a..=b
//!           │           ├── Ref: &p  /  &mut p
//!           │           ├── Deref: *p
//!           │           └── Variable: x  (fallback binding)
//!           └── parse_range_continuation()  ← ..end / ..=end
//! ```
//!
//! ## Token → Pattern Mapping
//!
//! | Token(s)                       | Pattern variant              |
//! |--------------------------------|------------------------------|
//! | `Ident("_")`                   | `Wildcard`                   |
//! | `IntLiteral(n)`                | `Literal(Number::I64(n))`    |
//! | `FloatLiteral(n)`              | `Literal(Number::F64(n))`    |
//! | `StringLiteral(s)`             | `Literal(String(s))`         |
//! | `CharLiteral(c)`               | `Literal(Char(c))`           |
//! | `True` / `False`               | `Literal(Boolean(b))`        |
//! | `None`                         | `Literal(Null)`              |
//! | `Minus IntLiteral`             | `Literal(I64(-n))`           |
//! | `Minus FloatLiteral`           | `Literal(F64(-n))`           |
//! | `Ellipsis` (..)                | `Rest`                       |
//! | `LParen`                       | `Tuple(patterns)`            |
//! | `LBracket`                     | `Array { elements, rest }`   |
//! | `Ampersand [Mut]`              | `Ref { pattern, mutable }`   |
//! | `Star`                         | `Deref(pattern)`             |
//! | `Mut` ident                    | `MutableVariable(name)`      |
//! | ident `{` fields `}`           | `Struct { .. }`              |
//! | ident `::` variant             | `EnumVariant { .. }`         |
//! | ident `@` pattern              | `Binding { name, pattern }`  |
//! | pattern `:` type               | `Typed { pattern, type_kind}`|
//! | pattern `если` expr            | `Guard { pattern, condition}`|
//! | p1 `\|` p2                     | `Or(patterns)`               |
//! | pattern `..` end               | `Range { .. }`               |

use super::core::Parser;
use super::error::{ParseError, ParseErrorKind, ParseResult};
use crate::types::{Expr, Number, Pattern, Token, Value};

impl Parser {
    // =========================================================================
    //         SECTION: TOP-LEVEL PATTERN (with OR)
    // =========================================================================

    /// Parses a pattern with optional OR alternatives: `p1 | p2 | p3`.
    ///
    /// This is the entry point used by match arms in both statement
    /// and expression forms.
    ///
    /// ```text
    /// 1 | 2 | 3          → Or([Literal(1), Literal(2), Literal(3)])
    /// Some(x) | None     → Or([OptionSome(Variable("x")), OptionNone])
    /// ```
    pub fn parse_pattern_with_or(&mut self) -> ParseResult<Pattern> {
        let first = self.parse_pattern()?;

        if !matches!(self.peek(), Token::Or) {
            return Ok(first);
        }

        let mut alternatives = vec![first];
        while self.match_token(&Token::Or) {
            alternatives.push(self.parse_pattern()?);
        }

        Ok(Pattern::Or(alternatives))
    }

    // =========================================================================
    //         SECTION: SINGLE PATTERN (guard / binding / typed)
    // =========================================================================

    /// Parses a single pattern, including guard and binding layers.
    ///
    /// After the atomic pattern is parsed, we check for:
    /// 1. Type annotation:  `pattern : Type`
    /// 2. Binding:          `name @ pattern`  (handled inside atomic)
    /// 3. Guard:            `pattern если condition`
    pub fn parse_pattern(&mut self) -> ParseResult<Pattern> {
        let pat = self.parse_atomic_pattern()?;

        // ── Type annotation: pattern : Type ─────────────────────────
        let pat = if self.match_token(&Token::Colon) {
            let type_kind = self.parse_type()?;
            Pattern::Typed {
                pattern: Box::new(pat),
                type_kind,
            }
        } else {
            pat
        };

        // ── Range continuation: pattern .. end  or  pattern ..= end ─
        let pat = self.try_range_continuation(pat)?;

        // ── Guard: pattern если condition ──────────────────────────
        let pat = if self.match_token(&Token::If) {
            let condition = self.parse_expr()?;
            Pattern::Guard {
                pattern: Box::new(pat),
                condition: Box::new(condition),
            }
        } else {
            pat
        };

        Ok(pat)
    }

    // =========================================================================
    //         SECTION: ATOMIC PATTERN
    // =========================================================================

    /// Parses an atomic pattern — a single complete pattern token or
    /// construct without trailing annotations (guard, type, OR).
    fn parse_atomic_pattern(&mut self) -> ParseResult<Pattern> {
        match self.peek().clone() {
            // ── Wildcard: _ ─────────────────────────────────────────
            Token::Ident(ref s) if s == "_" => {
                self.advance();
                Ok(Pattern::Wildcard)
            }

            // ── Rest / spread: .. ───────────────────────────────────
            Token::Ellipsis | Token::DoubleDot => {
                self.advance();
                // ..name  → Array rest binding handled by caller
                Ok(Pattern::Rest)
            }

            // ── Negative literal: -42, -3.14 ────────────────────────
            Token::Minus => {
                self.advance();
                match self.peek().clone() {
                    Token::IntLiteral(n) => {
                        self.advance();
                        Ok(Pattern::Literal(Value::Number(Number::I64(-n))))
                    }
                    Token::FloatLiteral(n) => {
                        self.advance();
                        Ok(Pattern::Literal(Value::Number(Number::F64(-n))))
                    }
                    _ => Err(ParseError::new(
                        ParseErrorKind::InvalidPattern,
                        "expected number after '-' in pattern",
                        self.span(),
                    )),
                }
            }

            // ── Integer literal (possibly range start) ──────────────
            Token::IntLiteral(n) => {
                self.advance();
                Ok(Pattern::Literal(Value::Number(Number::I64(n))))
            }

            // ── Float literal ───────────────────────────────────────
            Token::FloatLiteral(n) => {
                self.advance();
                Ok(Pattern::Literal(Value::Number(Number::F64(n))))
            }

            // ── String literal ──────────────────────────────────────
            Token::StringLiteral(s) => {
                self.advance();
                Ok(Pattern::Literal(Value::String(s)))
            }

            // ── Raw string literal ──────────────────────────────────
            Token::RawStringLiteral(s) => {
                self.advance();
                Ok(Pattern::Literal(Value::String(s)))
            }

            // ── Char literal ────────────────────────────────────────
            Token::CharLiteral(c) => {
                self.advance();
                Ok(Pattern::Literal(Value::Char(c)))
            }

            // ── Boolean literals ────────────────────────────────────
            Token::True => {
                self.advance();
                Ok(Pattern::Literal(Value::Boolean(true)))
            }
            Token::False => {
                self.advance();
                Ok(Pattern::Literal(Value::Boolean(false)))
            }

            // ── None / null ─────────────────────────────────────────
            Token::None => {
                self.advance();
                Ok(Pattern::Literal(Value::Null))
            }

            // ── Tuple pattern: (p1, p2, ...) ────────────────────────
            Token::LParen => self.parse_tuple_pattern(),

            // ── Array pattern: [p1, p2, ...rest] ────────────────────
            Token::LBracket => self.parse_array_pattern(),

            // ── Reference pattern: &p  or  &mut p ───────────────────
            Token::Ampersand => {
                self.advance();
                let mutable = self.match_token(&Token::Mut);
                let inner = self.parse_atomic_pattern()?;
                Ok(Pattern::Ref {
                    pattern: Box::new(inner),
                    mutable,
                })
            }

            // ── Deref pattern: *p ───────────────────────────────────
            Token::Star => {
                self.advance();
                let inner = self.parse_atomic_pattern()?;
                Ok(Pattern::Deref(Box::new(inner)))
            }

            // ── Mutable binding: mut x ──────────────────────────────
            Token::Mut => {
                self.advance();
                let name = self.expect_ident("variable name after 'mut'")?;
                Ok(Pattern::MutableVariable(name))
            }

            // ── Identifier-led patterns ─────────────────────────────
            //    Variable / Enum / Struct / Binding / Some/None/Ok/Err
            Token::Ident(_)
            | Token::VarIdent(_)
            | Token::FuncIdent(_)
            | Token::TypeIdent(_)
            | Token::ClassIdent(_)
            | Token::NamespaceIdent(_) => self.parse_ident_pattern(),

            _ => Err(ParseError::new(
                ParseErrorKind::ExpectedPattern,
                "expected pattern",
                self.span(),
            )),
        }
    }

    // =========================================================================
    //         SECTION: IDENTIFIER-LED PATTERNS
    // =========================================================================

    /// Parses patterns that start with an identifier.
    ///
    /// Disambiguates between:
    /// - `Some(p)` / `None` / `Ok(p)` / `Err(p)` — option/result sugar
    /// - `Name::Variant(bindings)`                — enum variant
    /// - `Name { f1: p, f2: p, .. }`              — struct destructuring
    /// - `name @ pattern`                         — binding with nested pattern
    /// - `name`                                   — plain variable binding
    fn parse_ident_pattern(&mut self) -> ParseResult<Pattern> {
        let name = self.expect_ident("pattern identifier")?;

        match self.peek() {
            // ── Option/Result shorthand: Some(p), Ok(p), Err(p) ─────
            Token::LParen if is_option_result(&name) => {
                self.advance(); // consume `(`
                match name.as_str() {
                    "Some" | "Некоторый" => {
                        let inner = self.parse_pattern()?;
                        self.expect(&Token::RParen, ")")?;
                        Ok(Pattern::OptionSome(Box::new(inner)))
                    }
                    "Ok" | "Успех" => {
                        let inner = self.parse_pattern()?;
                        self.expect(&Token::RParen, ")")?;
                        Ok(Pattern::ResultOk(Box::new(inner)))
                    }
                    "Err" | "Ошибка" => {
                        let inner = self.parse_pattern()?;
                        self.expect(&Token::RParen, ")")?;
                        Ok(Pattern::ResultErr(Box::new(inner)))
                    }
                    _ => unreachable!(),
                }
            }

            // ── None keyword (as ident) ─────────────────────────────
            _ if name == "None" || name == "Ничего" => Ok(Pattern::OptionNone),

            // ── Enum variant: Enum::Variant or Enum::Variant(p1, p2)
            Token::DoubleColon => {
                self.advance();
                self.parse_enum_variant_pattern(name)
            }

            // ── Struct destructuring: Name { field: p, .. } ─────────
            Token::LBrace => {
                self.advance();
                self.parse_struct_pattern(name)
            }

            // ── Binding: name @ pattern ─────────────────────────────
            Token::At => {
                self.advance();
                let inner = self.parse_pattern()?;
                Ok(Pattern::Binding {
                    name,
                    pattern: Box::new(inner),
                })
            }

            // ── Plain variable binding ──────────────────────────────
            _ => Ok(Pattern::Variable(name)),
        }
    }

    // =========================================================================
    //         SECTION: ENUM VARIANT PATTERN
    // =========================================================================

    /// Parses after `EnumName::`:
    ///
    /// ```text
    /// Color::Red                     → EnumVariant { bindings: [] }
    /// Option::Some(x)                → EnumVariant { bindings: [Variable("x")] }
    /// Result::Ok(val)                → EnumVariant { bindings: [Variable("val")] }
    /// Shape::Rect(w, h)              → EnumVariant { bindings: [Var("w"), Var("h")] }
    /// ```
    fn parse_enum_variant_pattern(&mut self, enum_name: String) -> ParseResult<Pattern> {
        let variant = self.expect_ident("enum variant name")?;

        let bindings = if self.match_token(&Token::LParen) {
            let pats = self.comma_sep(&Token::RParen, |p| p.parse_pattern())?;
            self.expect(&Token::RParen, ")")?;
            pats
        } else {
            Vec::new()
        };

        Ok(Pattern::EnumVariant {
            enum_name,
            variant,
            bindings,
        })
    }

    // =========================================================================
    //         SECTION: STRUCT PATTERN
    // =========================================================================

    /// Parses after `StructName {`:
    ///
    /// ```text
    /// Point { x, y }                 → fields: [(x, Var(x)), (y, Var(y))]
    /// Point { x: a, y: b }           → fields: [(x, Var(a)), (y, Var(b))]
    /// Point { x, .. }                → fields: [(x, Var(x))], rest: true
    /// Rect { min: Point { x, y }, .. }  → nested struct patterns
    /// ```
    fn parse_struct_pattern(&mut self, struct_name: String) -> ParseResult<Pattern> {
        let mut fields = Vec::new();
        let mut rest = false;

        while !self.check(&Token::RBrace) && !self.is_eof() {
            // ── Rest marker: .. ─────────────────────────────────────
            if matches!(self.peek(), Token::Ellipsis | Token::DoubleDot) {
                self.advance();
                rest = true;
                break;
            }

            let field_name = self.expect_ident("field name")?;

            // field: pattern  or  just  field  (shorthand)
            let pattern = if self.match_token(&Token::Colon) {
                self.parse_pattern()?
            } else {
                // Shorthand: `x` is the same as `x: x`
                Pattern::Variable(field_name.clone())
            };

            fields.push((field_name, pattern));

            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        self.expect(&Token::RBrace, "}")?;

        Ok(Pattern::Struct {
            struct_name,
            fields,
            rest,
        })
    }

    // =========================================================================
    //         SECTION: TUPLE PATTERN
    // =========================================================================

    /// Parses `(p1, p2, ...)`.
    ///
    /// Special cases:
    /// - `()`          → empty Tuple
    /// - `(p)`         → same as `p` (grouping)
    /// - `(p1, p2)`    → Pair if exactly 2, else Tuple
    /// - `(p1, p2, p3)` → Triple if exactly 3, else Tuple
    fn parse_tuple_pattern(&mut self) -> ParseResult<Pattern> {
        self.advance(); // consume `(`

        // Empty tuple: ()
        if self.check(&Token::RParen) {
            self.advance();
            return Ok(Pattern::Tuple(Vec::new()));
        }

        let first = self.parse_pattern()?;

        // Single-element — grouping, not a tuple
        if !self.match_token(&Token::Comma) {
            self.expect(&Token::RParen, ")")?;
            return Ok(first);
        }

        let mut elements = vec![first];
        loop {
            if self.check(&Token::RParen) {
                break;
            }
            elements.push(self.parse_pattern()?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }
        self.expect(&Token::RParen, ")")?;

        // Specialized Pair / Triple constructors
        match elements.len() {
            2 => {
                let mut it = elements.into_iter();
                let a = it.next().unwrap();
                let b = it.next().unwrap();
                Ok(Pattern::Pair(Box::new(a), Box::new(b)))
            }
            3 => {
                let mut it = elements.into_iter();
                let a = it.next().unwrap();
                let b = it.next().unwrap();
                let c = it.next().unwrap();
                Ok(Pattern::Triple(Box::new(a), Box::new(b), Box::new(c)))
            }
            _ => Ok(Pattern::Tuple(elements)),
        }
    }

    // =========================================================================
    //         SECTION: ARRAY PATTERN
    // =========================================================================

    /// Parses `[p1, p2, ...rest]`.
    ///
    /// ```text
    /// []                             → Array { elements: [], rest: None }
    /// [x, y]                         → Array { elements: [x, y], rest: None }
    /// [head, ..tail]                 → Array { elements: [head], rest: Some("tail") }
    /// [first, second, ..]            → Array { elements: [first, second], rest: Some("") }
    /// ```
    fn parse_array_pattern(&mut self) -> ParseResult<Pattern> {
        self.advance(); // consume `[`

        let mut elements = Vec::new();
        let mut rest = None;

        while !self.check(&Token::RBracket) && !self.is_eof() {
            // ── Rest spread: ..name or just .. ──────────────────────
            if matches!(self.peek(), Token::Ellipsis | Token::DoubleDot) {
                self.advance();
                // Optional rest binding name
                if self.is_ident() {
                    rest = Some(self.expect_ident("rest variable")?);
                } else {
                    rest = Some(String::new()); // anonymous rest
                }
                // Skip trailing comma
                let _ = self.match_token(&Token::Comma);
                break;
            }

            elements.push(self.parse_pattern()?);
            if !self.match_token(&Token::Comma) {
                break;
            }
        }

        self.expect(&Token::RBracket, "]")?;

        Ok(Pattern::Array { elements, rest })
    }

    // =========================================================================
    //         SECTION: RANGE CONTINUATION
    // =========================================================================

    /// Checks if the pattern is followed by `..` or `..=` to form a
    /// range pattern.
    ///
    /// ```text
    /// 1..10      → Range { start: 1, end: 10, inclusive: false }
    /// 1..=10     → Range { start: 1, end: 10, inclusive: true  }
    /// x..        → Range { start: x, end: None, inclusive: false }
    /// ```
    fn try_range_continuation(&mut self, start_pat: Pattern) -> ParseResult<Pattern> {
        if !matches!(self.peek(), Token::DoubleDot) {
            return Ok(start_pat);
        }
        self.advance(); // consume `..`

        // Inclusive range: ..= (DoubleDot was consumed, now check `=`)
        let inclusive = self.match_token(&Token::Equal);

        // Convert start pattern to expression for Range
        let start_expr = pattern_to_expr(&start_pat);

        // Try to parse end value
        let end_expr = self.try_parse_range_end()?;

        Ok(Pattern::Range {
            start: start_expr.map(Box::new),
            end: end_expr.map(Box::new),
            inclusive,
        })
    }

    /// Tries to parse the end value of a range pattern.
    ///
    /// Returns `None` for open-ended ranges: `1..` means "1 to infinity".
    fn try_parse_range_end(&mut self) -> ParseResult<Option<Expr>> {
        match self.peek().clone() {
            Token::IntLiteral(n) => {
                self.advance();
                Ok(Some(Expr::Literal(Value::Number(Number::I64(n)))))
            }
            Token::FloatLiteral(n) => {
                self.advance();
                Ok(Some(Expr::Literal(Value::Number(Number::F64(n)))))
            }
            Token::Minus => {
                self.advance();
                match self.peek().clone() {
                    Token::IntLiteral(n) => {
                        self.advance();
                        Ok(Some(Expr::Literal(Value::Number(Number::I64(-n)))))
                    }
                    Token::FloatLiteral(n) => {
                        self.advance();
                        Ok(Some(Expr::Literal(Value::Number(Number::F64(-n)))))
                    }
                    _ => Ok(None),
                }
            }
            Token::Ident(_)
            | Token::VarIdent(_)
            | Token::FuncIdent(_)
            | Token::TypeIdent(_)
            | Token::ClassIdent(_)
            | Token::NamespaceIdent(_) => {
                let name = self.expect_ident("range end")?;
                Ok(Some(Expr::Variable(name)))
            }
            _ => Ok(None),
        }
    }
}

// =============================================================================
//         SECTION: UTILITY FUNCTIONS
// =============================================================================

/// Returns `true` if the identifier names an Option/Result constructor.
fn is_option_result(name: &str) -> bool {
    matches!(
        name,
        "Some" | "Некоторый" | "Ok" | "Успех" | "Err" | "Ошибка"
    )
}

/// Best-effort conversion of a literal pattern to its Expr equivalent.
///
/// Used to build `Range { start, end }` from a preceding pattern.
/// Returns `None` for non-convertible patterns (variables still convert).
fn pattern_to_expr(pat: &Pattern) -> Option<Expr> {
    match pat {
        Pattern::Literal(v) => Some(Expr::Literal(v.clone())),
        Pattern::Variable(name) => Some(Expr::Variable(name.clone())),
        _ => None,
    }
}
