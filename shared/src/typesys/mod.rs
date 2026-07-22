// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! [EXPERIMENTAL] Type System Engine for Kumir 3.
//!
//! A small, **extensible** engine that unifies the structural type descriptor
//! [`TypeKind`] with the nominal [`TypeRegistry`]. Relations are expressed as
//! composable [`TypeRule`]s that can be registered at runtime — adding a new
//! type or relation means registering a rule, not editing a closed function.
//!
//! The engine is pure (no interpreter/compiler dependencies) and lives in
//! `shared`, so both the interpreter and the compiler share one source of truth
//! for assignability, unification, coercion planning, operator typing and
//! default values. See KITE 10 (arch/kite) for the design rationale.
//!
//! ## Module layout
//! - this module — the [`TypeSystem`] engine + [`Conformance`], [`TypeError`],
//!   [`TypeOp`], [`Coercion`];
//! - [`rules`] — the [`TypeRule`] trait and the built-in rule set;
//! - tests — engine behaviour (see `tests.rs`).

use std::sync::{Arc, RwLock};

use once_cell::sync::Lazy;

use crate::types::Number;
use crate::types::TypeRegistry;
use crate::types::value::{TypeKind, Value};

pub mod rules;
#[cfg(test)]
mod tests;

pub use rules::{
    CollectionRule, IdentityRule, NominalRule, NullOptionRule, NumericRule, TopBottomRule,
    TypeRule, default_rules,
};

// =============================================================================
//         SECTION: CONFORMANCE
// =============================================================================

/// How well a source type conforms to a target type.
///
/// Ordered best→worst: `Exact` > `Widening` > `Coercible` > `Incompatible`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conformance {
    /// Identical types.
    Exact,
    /// Safe implicit conversion (e.g. `цел_16` → `цел_64`, `T` → `T?`).
    Widening,
    /// Requires an explicit/automatic coercion (narrowing, `любой` → `цел`).
    Coercible,
    /// No relation.
    Incompatible,
}

impl Conformance {
    /// Implicitly assignable without an explicit cast.
    pub fn is_implicit(self) -> bool {
        matches!(self, Conformance::Exact | Conformance::Widening)
    }
    /// Compatible at all (possibly via coercion).
    pub fn is_compatible(self) -> bool {
        !matches!(self, Conformance::Incompatible)
    }
    /// Severity rank (lower is better).
    pub(crate) fn rank(self) -> u8 {
        match self {
            Conformance::Exact => 0,
            Conformance::Widening => 1,
            Conformance::Coercible => 2,
            Conformance::Incompatible => 3,
        }
    }
    /// The worse (higher-rank) of two conformances — for covariant composition.
    pub(crate) fn worst(self, other: Conformance) -> Conformance {
        if self.rank() >= other.rank() {
            self
        } else {
            other
        }
    }
}

// =============================================================================
//         SECTION: COERCION PLAN
// =============================================================================

/// What conversion a consumer (interpreter cast / compiler codegen) must apply
/// to turn a `source` value into a `target` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coercion {
    /// No conversion — types match.
    Identity,
    /// Lossless widening (e.g. integer/float promotion).
    Widen,
    /// Wrap into an optional (`T` → `T?`).
    Wrap,
    /// Explicit/checked conversion (narrowing, dynamic `любой`).
    Cast,
    /// Not convertible.
    Forbidden,
}

// =============================================================================
//         SECTION: ERRORS
// =============================================================================

/// Errors produced by checked type operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    NotAssignable {
        target: String,
        source: String,
    },
    NoCommonType {
        a: String,
        b: String,
    },
    BinaryOpUnsupported {
        op: &'static str,
        left: String,
        right: String,
    },
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::NotAssignable { target, source } => write!(
                f,
                "Тип '{}' нельзя присвоить переменной типа '{}'",
                source, target
            ),
            TypeError::NoCommonType { a, b } => {
                write!(f, "У типов '{}' и '{}' нет общего типа", a, b)
            }
            TypeError::BinaryOpUnsupported { op, left, right } => write!(
                f,
                "Операция '{}' не определена для типов '{}' и '{}'",
                op, left, right
            ),
        }
    }
}

impl std::error::Error for TypeError {}

// =============================================================================
//         SECTION: BINARY OPERATORS
// =============================================================================

/// Binary operators recognised for operator-result typing.
///
/// The set mirrors the language (KITE 13 § 3), not a generic expression
/// algebra: `/` and `див` are **different operators** with different typing
/// rules, so they are different variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeOp {
    Add,
    Sub,
    Mul,
    /// Real division `/` — always yields a real number (KITE 13 § 3.4).
    Div,
    /// Integer division `див` — integers only, truncating (KITE 13 § 3.5).
    IntDiv,
    /// Remainder `мод` / `%` — integers only (KITE 13 § 3.6).
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

impl TypeOp {
    pub fn symbol(self) -> &'static str {
        match self {
            TypeOp::Add => "+",
            TypeOp::Sub => "-",
            TypeOp::Mul => "*",
            TypeOp::Div => "/",
            TypeOp::IntDiv => "див",
            TypeOp::Mod => "мод",
            TypeOp::Pow => "**",
            TypeOp::Eq => "=",
            TypeOp::Ne => "<>",
            TypeOp::Lt => "<",
            TypeOp::Le => "<=",
            TypeOp::Gt => ">",
            TypeOp::Ge => ">=",
            TypeOp::And => "и",
            TypeOp::Or => "или",
        }
    }
    /// Equality (`=`, `<>`) — **total**: defined for every pair of value types.
    fn is_equality(self) -> bool {
        matches!(self, TypeOp::Eq | TypeOp::Ne)
    }
    /// Ordering (`<`, `<=`, `>`, `>=`) — requires an *ordered* type.
    fn is_ordering(self) -> bool {
        matches!(self, TypeOp::Lt | TypeOp::Le | TypeOp::Gt | TypeOp::Ge)
    }
    fn is_logical(self) -> bool {
        matches!(self, TypeOp::And | TypeOp::Or)
    }
}

/// Types the language puts in a total order (KITE 13 § 3.8): numbers, strings
/// and characters. Everything else (`лог`, tables, sets, …) can only be tested
/// for equality.
fn is_ordered(t: &TypeKind) -> bool {
    t.is_numeric() || matches!(t, TypeKind::String | TypeKind::Char)
}

// =============================================================================
//         SECTION: TYPE SYSTEM ENGINE
// =============================================================================

/// The extensible type-system engine.
pub struct TypeSystem {
    rules: Vec<Arc<dyn TypeRule>>,
    registry: Option<Arc<RwLock<TypeRegistry>>>,
}

impl TypeSystem {
    /// A system with the standard built-in rules.
    pub fn new() -> Self {
        let mut ts = Self::bare();
        ts.rules = default_rules();
        ts
    }

    /// A system with no rules (everything incompatible) — build your own.
    pub fn bare() -> Self {
        Self {
            rules: Vec::new(),
            registry: None,
        }
    }

    /// Attach a nominal type registry (enables inheritance-based subtyping).
    pub fn with_registry(mut self, registry: Arc<RwLock<TypeRegistry>>) -> Self {
        self.registry = Some(registry);
        self
    }

    /// Register an additional rule at the end (lowest priority).
    pub fn register_rule(&mut self, rule: Arc<dyn TypeRule>) {
        self.rules.push(rule);
    }

    /// Register a rule at the front — it is consulted first, so it takes
    /// priority for unification and for `Exact` short-circuiting.
    pub fn register_rule_front(&mut self, rule: Arc<dyn TypeRule>) {
        self.rules.insert(0, rule);
    }

    /// Number of active rules.
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Access the attached registry, if any.
    pub fn registry(&self) -> Option<&Arc<RwLock<TypeRegistry>>> {
        self.registry.as_ref()
    }

    // -------------------------------------------------------------------------
    //         ASSIGNABILITY
    // -------------------------------------------------------------------------

    /// Best conformance of `source` to `target` across all rules.
    pub fn conformance(&self, target: &TypeKind, source: &TypeKind) -> Conformance {
        let mut best = Conformance::Incompatible;
        for rule in &self.rules {
            if let Some(c) = rule.assignable(self, target, source) {
                if c.rank() < best.rank() {
                    best = c;
                }
                if best == Conformance::Exact {
                    break;
                }
            }
        }
        best
    }

    /// True if `source` is implicitly assignable to `target` (no explicit cast).
    pub fn is_assignable(&self, target: &TypeKind, source: &TypeKind) -> bool {
        self.conformance(target, source).is_implicit()
    }

    /// True if `sub` is a subtype of `sup` (structural or nominal), i.e. a value
    /// of `sub` can stand in for `sup` without an explicit cast.
    pub fn is_subtype(&self, sub: &TypeKind, sup: &TypeKind) -> bool {
        self.is_assignable(sup, sub)
    }

    /// Checked assignment: `Ok(conformance)` if compatible, else an error.
    pub fn check_assignable(
        &self,
        target: &TypeKind,
        source: &TypeKind,
    ) -> Result<Conformance, TypeError> {
        let c = self.conformance(target, source);
        if c.is_compatible() {
            Ok(c)
        } else {
            Err(TypeError::NotAssignable {
                target: target.russian_name(),
                source: source.russian_name(),
            })
        }
    }

    /// Plan the conversion needed to assign `source` into `target`.
    ///
    /// Lets the interpreter (runtime cast) and compiler (codegen) act on the
    /// *same* decision the engine made.
    pub fn coercion(&self, target: &TypeKind, source: &TypeKind) -> Coercion {
        match self.conformance(target, source) {
            Conformance::Exact => Coercion::Identity,
            Conformance::Widening => {
                // Distinguish optional-wrapping from plain widening.
                if matches!(target, TypeKind::Option(_)) && !matches!(source, TypeKind::Option(_)) {
                    Coercion::Wrap
                } else {
                    Coercion::Widen
                }
            }
            Conformance::Coercible => Coercion::Cast,
            Conformance::Incompatible => Coercion::Forbidden,
        }
    }

    // -------------------------------------------------------------------------
    //         UNIFICATION / COMMON TYPE
    // -------------------------------------------------------------------------

    /// Least common supertype of `a` and `b`, if any.
    pub fn unify(&self, a: &TypeKind, b: &TypeKind) -> Option<TypeKind> {
        for rule in &self.rules {
            if let Some(t) = rule.unify(self, a, b) {
                return Some(t);
            }
        }
        if self.is_assignable(a, b) {
            Some(a.clone())
        } else if self.is_assignable(b, a) {
            Some(b.clone())
        } else {
            None
        }
    }

    /// Checked common type.
    pub fn common_type(&self, a: &TypeKind, b: &TypeKind) -> Result<TypeKind, TypeError> {
        self.unify(a, b).ok_or_else(|| TypeError::NoCommonType {
            a: a.russian_name(),
            b: b.russian_name(),
        })
    }

    // -------------------------------------------------------------------------
    //         OPERATOR TYPING
    // -------------------------------------------------------------------------

    /// Result type of a binary operation, or an error if undefined.
    ///
    /// The rules follow KITE 13 § 3 exactly, including the operators the
    /// language defines over strings and tables:
    ///
    /// | operation | result |
    /// |---|---|
    /// | number `+ - *` number | common numeric type (§ 3.3) |
    /// | number `/` number | `вещ_128` — *always* real (§ 3.4) |
    /// | integer `див` / `мод` integer | common integer type (§ 3.5, § 3.6) |
    /// | `лит + лит` | `лит` (concatenation) |
    /// | `лит - лит` | `лит` (substring removal) |
    /// | `лит * цел`, `цел * лит` | `лит` (repetition) |
    /// | `лит / цел` | `таб лит` (chunking) |
    /// | `лит / лит` | `пара(таб лит, цел)` (splitting) |
    /// | `таб + таб` | `таб` of the common element type |
    /// | `таб - таб` | the left table's type (multiset difference) |
    /// | `=` `<>` | `лог`, **always** — equality is total |
    /// | `<` `<=` `>` `>=` | `лог`, only for compatible *ordered* types |
    /// | `и` `или` | `лог`, only for `лог` operands |
    pub fn result_of_binop(
        &self,
        op: TypeOp,
        left: &TypeKind,
        right: &TypeKind,
    ) -> Result<TypeKind, TypeError> {
        let unsupported = || TypeError::BinaryOpUnsupported {
            op: op.symbol(),
            left: left.russian_name(),
            right: right.russian_name(),
        };

        if op.is_logical() {
            return if matches!(left, TypeKind::Bool) && matches!(right, TypeKind::Bool) {
                Ok(TypeKind::Bool)
            } else {
                Err(unsupported())
            };
        }

        // Equality is structural and TOTAL: values of unrelated types are
        // simply not equal, never a type error (KITE 13 § 3.8).
        if op.is_equality() {
            return Ok(TypeKind::Bool);
        }

        // Ordering needs an ordered type on BOTH sides, and the two must be
        // comparable with each other (`цел < вещ` yes, `сим < лит` no).
        if op.is_ordering() {
            let comparable = self.conformance(left, right).is_compatible()
                || self.conformance(right, left).is_compatible();
            return if is_ordered(left) && is_ordered(right) && comparable {
                Ok(TypeKind::Bool)
            } else {
                Err(unsupported())
            };
        }

        self.result_of_arith(op, left, right)
            .ok_or_else(unsupported)
    }

    /// Arithmetic-operator typing (numeric, string and table operations).
    fn result_of_arith(&self, op: TypeOp, left: &TypeKind, right: &TypeKind) -> Option<TypeKind> {
        use TypeKind::*;

        // `див` and `мод` are integer-only; the result is the common integer type.
        if matches!(op, TypeOp::IntDiv | TypeOp::Mod) {
            return if left.is_integer() && right.is_integer() {
                self.unify(left, right)
            } else {
                None
            };
        }

        // `/` over numbers is always real, even for two integers.
        if op == TypeOp::Div && left.is_numeric() && right.is_numeric() {
            return Some(Float128);
        }

        if left.is_numeric() && right.is_numeric() {
            return self.unify(left, right);
        }

        match (op, left, right) {
            // Strings.
            (TypeOp::Add, String, String) => Some(String),
            (TypeOp::Sub, String, String) => Some(String),
            (TypeOp::Mul, String, t) | (TypeOp::Mul, t, String) if t.is_integer() => Some(String),
            (TypeOp::Div, String, t) if t.is_integer() => Some(Array(Box::new(String))),
            (TypeOp::Div, String, String) => Some(Pair(
                Box::new(Array(Box::new(String))),
                Box::new(TypeKind::Int64),
            )),
            // Tables: concatenation keeps both element types, difference keeps
            // the left one. A concatenation of unrelated element types is still
            // defined — the result is simply a table of `любой`.
            (TypeOp::Add, Array(a), Array(b)) => {
                Some(Array(Box::new(self.unify(a, b).unwrap_or(TypeKind::Any))))
            }
            (TypeOp::Sub, Array(_), Array(_)) => Some(left.clone()),
            _ => None,
        }
    }

    // -------------------------------------------------------------------------
    //         DEFAULT VALUES
    // -------------------------------------------------------------------------

    /// The natural default value for a type (zero / empty / none), if it has one.
    ///
    /// Delegated to the engine so variable initialisation is consistent across
    /// the interpreter and the compiler.
    pub fn default_value(&self, ty: &TypeKind) -> Option<Value> {
        use TypeKind::*;
        Some(match ty {
            Int8 => Value::Number(Number::I8(0)),
            Int16 => Value::Number(Number::I16(0)),
            Int32 => Value::Number(Number::I32(0)),
            Int64 => Value::Number(Number::I64(0)),
            Int128 => Value::Number(Number::I128(0)),
            UInt8 => Value::Number(Number::U8(0)),
            UInt16 => Value::Number(Number::U16(0)),
            UInt32 => Value::Number(Number::U32(0)),
            UInt64 => Value::Number(Number::U64(0)),
            UInt128 => Value::Number(Number::U128(0)),
            Float32 => Value::Number(Number::F32(0.0)),
            Float64 => Value::Number(Number::F64(0.0)),
            Float128 => Value::Number(Number::F128(crate::f128::F128::from(0.0))),
            Bool => Value::Boolean(false),
            Char => Value::Char('\0'),
            String => Value::String(std::string::String::new()),
            Array(_) => Value::Array(Vec::new()),
            Set(_) => Value::Set(std::collections::BTreeSet::new()),
            Map(_, _) => Value::Map(std::collections::BTreeMap::new()),
            Option(_) => Value::Option(Box::new(None)),
            Tuple(items) => Value::Tuple(
                items
                    .iter()
                    .map(|t| self.default_value(t).unwrap_or(Value::Undefined))
                    .collect(),
            ),
            Null => Value::Null,
            Void | Undefined => Value::Undefined,
            _ => return None,
        })
    }
}

impl Default for TypeSystem {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
//         SECTION: GLOBAL DEFAULT ENGINE
// =============================================================================

/// The shared, process-wide engine with the standard rules (no nominal registry).
///
/// This is the **single source of truth** for structural type relations.
/// Components that don't carry their own [`TypeSystem`] — and the legacy
/// `TypeKind::is_assignable_from` / `TypeKind::common_type` helpers — route
/// through this. For nominal (inheritance) subtyping, build a [`TypeSystem`]
/// with `with_registry`.
pub fn default_engine() -> &'static TypeSystem {
    static ENGINE: Lazy<TypeSystem> = Lazy::new(TypeSystem::new);
    &ENGINE
}
