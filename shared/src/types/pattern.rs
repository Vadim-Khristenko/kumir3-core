//! Patterns for pattern matching (Kumir 3).
//!
//! [STABLE] Patterns are used in match expressions, destructuring assignments,
//! and function parameter matching. This module provides all pattern types
//! needed for expressive pattern matching in Kumir 3.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        Pattern (Matching)                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Basic: Wildcard, Literal, Variable                             │
//! │  Composite: Tuple, Array, Struct, Enum                          │
//! │  Advanced: Range, Or, Guard, Rest, Type                         │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use super::expr::Expr;
use super::value::{TypeKind, Value};

// =============================================================================
//         SECTION: PATTERN ENUM
// =============================================================================

/// [STABLE] Pattern for pattern matching.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    // =========================================================================
    //         BASIC PATTERNS
    // =========================================================================
    /// Wildcard: _ (matches any value, discarded)
    Wildcard,

    /// Literal: concrete value (42, "string", true)
    Literal(Value),

    /// Variable binding: x (value is bound to x)
    Variable(String),

    /// Mutable variable binding: mut x
    MutableVariable(String),

    // =========================================================================
    //         ENUM AND STRUCT PATTERNS
    // =========================================================================
    /// Enum variant: Color::Red or Option::Some(x)
    EnumVariant {
        enum_name: String,
        variant: String,
        /// Bindings for associated data
        bindings: Vec<Pattern>,
    },

    /// Struct destructuring: Point { x, y } or Point { x: a, y: b }
    Struct {
        struct_name: String,
        /// Field name -> pattern pairs
        fields: Vec<(String, Pattern)>,
        /// Allow unmatched fields (.. syntax)
        rest: bool,
    },

    // =========================================================================
    //         COMPOSITE PATTERNS
    // =========================================================================
    /// Tuple: (x, y, _)
    Tuple(Vec<Pattern>),

    /// Array: [first, second, ...rest]
    Array {
        elements: Vec<Pattern>,
        /// Binding for remaining elements (None if exact match required)
        rest: Option<String>,
    },

    /// Pair: (a, b) — specialized 2-element pattern
    Pair(Box<Pattern>, Box<Pattern>),

    /// Triple: (a, b, c) — specialized 3-element pattern
    Triple(Box<Pattern>, Box<Pattern>, Box<Pattern>),

    // =========================================================================
    //         RANGE PATTERNS
    // =========================================================================
    /// Range: 1..10 or 1..=10
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },

    // =========================================================================
    //         COMBINING PATTERNS
    // =========================================================================
    /// Logical OR for patterns: 1 | 2 | 3
    Or(Vec<Pattern>),

    /// Pattern with guard: x if x > 0
    Guard {
        pattern: Box<Pattern>,
        condition: Box<Expr>,
    },

    /// Binding with nested pattern: x @ Some(_)
    Binding {
        name: String,
        pattern: Box<Pattern>,
    },

    // =========================================================================
    //         TYPE PATTERNS
    // =========================================================================
    /// Type check pattern: x: Int
    Typed {
        pattern: Box<Pattern>,
        type_kind: TypeKind,
    },

    /// Reference pattern: &x or &mut x
    Ref {
        pattern: Box<Pattern>,
        mutable: bool,
    },

    /// Pointer dereference pattern: *ptr
    Deref(Box<Pattern>),

    // =========================================================================
    //         SPECIAL PATTERNS
    // =========================================================================
    /// Rest pattern: .. (matches remaining elements)
    Rest,

    /// Option shorthand: Some(x) or None
    OptionSome(Box<Pattern>),
    OptionNone,

    /// Result shorthand: Ok(x) or Err(e)
    ResultOk(Box<Pattern>),
    ResultErr(Box<Pattern>),
}

// =============================================================================
//         SECTION: PATTERN CONSTRUCTORS
// =============================================================================

impl Pattern {
    /// Creates a wildcard pattern
    pub fn wildcard() -> Self {
        Pattern::Wildcard
    }

    /// Creates a variable binding pattern
    pub fn var(name: impl Into<String>) -> Self {
        Pattern::Variable(name.into())
    }

    /// Creates a literal pattern
    pub fn literal(value: Value) -> Self {
        Pattern::Literal(value)
    }

    /// Creates a tuple pattern
    pub fn tuple(patterns: Vec<Pattern>) -> Self {
        Pattern::Tuple(patterns)
    }

    /// Creates an array pattern with optional rest binding
    pub fn array(elements: Vec<Pattern>, rest: Option<String>) -> Self {
        Pattern::Array { elements, rest }
    }

    /// Creates an enum variant pattern
    pub fn enum_variant(
        enum_name: impl Into<String>,
        variant: impl Into<String>,
        bindings: Vec<Pattern>,
    ) -> Self {
        Pattern::EnumVariant {
            enum_name: enum_name.into(),
            variant: variant.into(),
            bindings,
        }
    }

    /// Creates a struct pattern
    pub fn struct_pattern(
        struct_name: impl Into<String>,
        fields: Vec<(String, Pattern)>,
        rest: bool,
    ) -> Self {
        Pattern::Struct {
            struct_name: struct_name.into(),
            fields,
            rest,
        }
    }

    /// Creates a range pattern
    pub fn range(start: Option<Expr>, end: Option<Expr>, inclusive: bool) -> Self {
        Pattern::Range {
            start: start.map(Box::new),
            end: end.map(Box::new),
            inclusive,
        }
    }

    /// Creates an OR pattern
    pub fn or(patterns: Vec<Pattern>) -> Self {
        Pattern::Or(patterns)
    }

    /// Creates a guarded pattern
    pub fn guard(pattern: Pattern, condition: Expr) -> Self {
        Pattern::Guard {
            pattern: Box::new(pattern),
            condition: Box::new(condition),
        }
    }

    /// Creates a binding pattern (x @ pattern)
    pub fn binding(name: impl Into<String>, pattern: Pattern) -> Self {
        Pattern::Binding {
            name: name.into(),
            pattern: Box::new(pattern),
        }
    }

    /// Creates a typed pattern
    pub fn typed(pattern: Pattern, type_kind: TypeKind) -> Self {
        Pattern::Typed {
            pattern: Box::new(pattern),
            type_kind,
        }
    }
}

// =============================================================================
//         SECTION: PATTERN UTILITIES
// =============================================================================

impl Pattern {
    /// Returns true if this pattern is irrefutable (always matches)
    pub fn is_irrefutable(&self) -> bool {
        match self {
            Pattern::Wildcard => true,
            Pattern::Variable(_) => true,
            Pattern::MutableVariable(_) => true,
            Pattern::Rest => true,
            Pattern::Tuple(patterns) => patterns.iter().all(|p| p.is_irrefutable()),
            Pattern::Array { elements, rest } => {
                rest.is_some() && elements.iter().all(|p| p.is_irrefutable())
            }
            Pattern::Binding { pattern, .. } => pattern.is_irrefutable(),
            Pattern::Typed { pattern, .. } => pattern.is_irrefutable(),
            Pattern::Ref { pattern, .. } => pattern.is_irrefutable(),
            _ => false,
        }
    }

    /// Collects all variable bindings from this pattern
    pub fn bindings(&self) -> Vec<String> {
        let mut result = Vec::new();
        self.collect_bindings(&mut result);
        result
    }

    fn collect_bindings(&self, out: &mut Vec<String>) {
        match self {
            Pattern::Variable(name) | Pattern::MutableVariable(name) => {
                out.push(name.clone());
            }
            Pattern::EnumVariant { bindings, .. } => {
                for p in bindings {
                    p.collect_bindings(out);
                }
            }
            Pattern::Struct { fields, .. } => {
                for (_, p) in fields {
                    p.collect_bindings(out);
                }
            }
            Pattern::Tuple(patterns) | Pattern::Or(patterns) => {
                for p in patterns {
                    p.collect_bindings(out);
                }
            }
            Pattern::Array { elements, rest } => {
                for p in elements {
                    p.collect_bindings(out);
                }
                if let Some(name) = rest {
                    out.push(name.clone());
                }
            }
            Pattern::Pair(a, b) => {
                a.collect_bindings(out);
                b.collect_bindings(out);
            }
            Pattern::Triple(a, b, c) => {
                a.collect_bindings(out);
                b.collect_bindings(out);
                c.collect_bindings(out);
            }
            Pattern::Guard { pattern, .. } => pattern.collect_bindings(out),
            Pattern::Binding { name, pattern } => {
                out.push(name.clone());
                pattern.collect_bindings(out);
            }
            Pattern::Typed { pattern, .. } => pattern.collect_bindings(out),
            Pattern::Ref { pattern, .. } => pattern.collect_bindings(out),
            Pattern::Deref(pattern) => pattern.collect_bindings(out),
            Pattern::OptionSome(p) | Pattern::ResultOk(p) | Pattern::ResultErr(p) => {
                p.collect_bindings(out);
            }
            Pattern::Wildcard
            | Pattern::Literal(_)
            | Pattern::Range { .. }
            | Pattern::Rest
            | Pattern::OptionNone => {}
        }
    }
}
