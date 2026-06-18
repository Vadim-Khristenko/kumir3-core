// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! [EXPERIMENTAL] Built-in type rules for the engine.
//!
//! Each [`TypeRule`] expresses one relation (assignability and/or unification).
//! For **assignability** the engine keeps the *best* (most permissive)
//! conformance any rule reports, short-circuiting on `Exact`. For **unification**
//! the *first* rule that returns a type wins, so registration order matters.
//! New types or relations are added by registering more rules — the closed set
//! below is just the default.

use std::sync::Arc;

use crate::types::TypeId;
use crate::types::value::TypeKind;

use super::{Conformance, TypeSystem};

// =============================================================================
//         SECTION: RULE TRAIT
// =============================================================================

/// An extensible relation between types.
pub trait TypeRule: Send + Sync {
    /// Diagnostic name.
    fn name(&self) -> &'static str;

    /// How does `source` conform to `target`? `None` = "no opinion".
    fn assignable(
        &self,
        _ts: &TypeSystem,
        _target: &TypeKind,
        _source: &TypeKind,
    ) -> Option<Conformance> {
        None
    }

    /// Least common supertype of `a` and `b`, if this rule knows one.
    fn unify(&self, _ts: &TypeSystem, _a: &TypeKind, _b: &TypeKind) -> Option<TypeKind> {
        None
    }
}

// =============================================================================
//         SECTION: DEFAULT RULE SET
// =============================================================================

/// The standard rule set, in priority order.
pub fn default_rules() -> Vec<Arc<dyn TypeRule>> {
    vec![
        Arc::new(IdentityRule),
        Arc::new(TopBottomRule),
        Arc::new(NullOptionRule),
        Arc::new(NumericRule),
        Arc::new(CollectionRule),
        Arc::new(NominalRule),
    ]
}

// =============================================================================
//         SECTION: IDENTITY
// =============================================================================

/// Identical types conform exactly.
pub struct IdentityRule;
impl TypeRule for IdentityRule {
    fn name(&self) -> &'static str {
        "identity"
    }
    fn assignable(
        &self,
        _ts: &TypeSystem,
        target: &TypeKind,
        source: &TypeKind,
    ) -> Option<Conformance> {
        if target == source {
            Some(Conformance::Exact)
        } else {
            None
        }
    }
    fn unify(&self, _ts: &TypeSystem, a: &TypeKind, b: &TypeKind) -> Option<TypeKind> {
        if a == b { Some(a.clone()) } else { None }
    }
}

// =============================================================================
//         SECTION: TOP / BOTTOM
// =============================================================================

/// Top (`любой`/`авто`) and bottom (`никогда`) types.
pub struct TopBottomRule;
impl TypeRule for TopBottomRule {
    fn name(&self) -> &'static str {
        "top-bottom"
    }
    fn assignable(
        &self,
        _ts: &TypeSystem,
        target: &TypeKind,
        source: &TypeKind,
    ) -> Option<Conformance> {
        if matches!(source, TypeKind::Never) {
            return Some(Conformance::Widening);
        }
        if matches!(target, TypeKind::Any | TypeKind::Auto) {
            return Some(Conformance::Widening);
        }
        if matches!(source, TypeKind::Any) {
            return Some(Conformance::Coercible);
        }
        None
    }
    fn unify(&self, _ts: &TypeSystem, a: &TypeKind, b: &TypeKind) -> Option<TypeKind> {
        if matches!(a, TypeKind::Any) || matches!(b, TypeKind::Any) {
            return Some(TypeKind::Any);
        }
        if matches!(a, TypeKind::Never) {
            return Some(b.clone());
        }
        if matches!(b, TypeKind::Never) {
            return Some(a.clone());
        }
        None
    }
}

// =============================================================================
//         SECTION: NULL / OPTION
// =============================================================================

/// `пусто` and automatic `T` → `T?` wrapping.
pub struct NullOptionRule;
impl TypeRule for NullOptionRule {
    fn name(&self) -> &'static str {
        "null-option"
    }
    fn assignable(
        &self,
        ts: &TypeSystem,
        target: &TypeKind,
        source: &TypeKind,
    ) -> Option<Conformance> {
        if let TypeKind::Option(inner) = target {
            if matches!(source, TypeKind::Null) {
                return Some(Conformance::Widening);
            }
            if !matches!(source, TypeKind::Option(_)) {
                let inner_conf = ts.conformance(inner, source);
                if inner_conf.is_compatible() {
                    return Some(inner_conf.worst(Conformance::Widening));
                }
            }
        }
        None
    }
    fn unify(&self, ts: &TypeSystem, a: &TypeKind, b: &TypeKind) -> Option<TypeKind> {
        let (null_side, other) = match (a, b) {
            (TypeKind::Null, x) | (x, TypeKind::Null) => (true, x),
            _ => (false, a),
        };
        if null_side {
            return Some(match other {
                TypeKind::Option(_) => other.clone(),
                _ => TypeKind::Option(Box::new(other.clone())),
            });
        }
        match (a, b) {
            (TypeKind::Option(ia), other) | (other, TypeKind::Option(ia))
                if !matches!(other, TypeKind::Option(_)) =>
            {
                ts.unify(ia, other).map(|u| TypeKind::Option(Box::new(u)))
            }
            _ => None,
        }
    }
}

// =============================================================================
//         SECTION: NUMERIC
// =============================================================================

/// Numeric widening / coercion using the numeric lattice.
pub struct NumericRule;
impl NumericRule {
    fn conf(target: &TypeKind, source: &TypeKind) -> Conformance {
        let (ts_b, ss_b) = match (target.size_bytes(), source.size_bytes()) {
            (Some(t), Some(s)) => (t, s),
            _ => return Conformance::Coercible,
        };
        if target.is_float() {
            if source.is_float() {
                if ts_b >= ss_b {
                    Conformance::Widening
                } else {
                    Conformance::Coercible
                }
            } else if ts_b > ss_b {
                Conformance::Widening
            } else {
                Conformance::Coercible
            }
        } else if source.is_float() {
            Conformance::Coercible
        } else if target.is_signed() == source.is_signed() {
            if ts_b >= ss_b {
                Conformance::Widening
            } else {
                Conformance::Coercible
            }
        } else if target.is_signed() && source.is_unsigned() {
            if ts_b > ss_b {
                Conformance::Widening
            } else {
                Conformance::Coercible
            }
        } else {
            Conformance::Coercible
        }
    }
}
impl TypeRule for NumericRule {
    fn name(&self) -> &'static str {
        "numeric"
    }
    fn assignable(
        &self,
        _ts: &TypeSystem,
        target: &TypeKind,
        source: &TypeKind,
    ) -> Option<Conformance> {
        if target.is_numeric() && source.is_numeric() {
            Some(Self::conf(target, source))
        } else {
            None
        }
    }
    fn unify(&self, _ts: &TypeSystem, a: &TypeKind, b: &TypeKind) -> Option<TypeKind> {
        if a.is_numeric() && b.is_numeric() {
            let pick_float = a.is_float() || b.is_float();
            let sa = a.size_bytes().unwrap_or(0);
            let sb = b.size_bytes().unwrap_or(0);
            let bigger = if sa >= sb { a } else { b };
            if pick_float && !bigger.is_float() {
                return Some(if bigger.size_bytes().unwrap_or(8) > 8 {
                    TypeKind::Float128
                } else {
                    TypeKind::Float64
                });
            }
            Some(bigger.clone())
        } else {
            None
        }
    }
}

// =============================================================================
//         SECTION: COLLECTIONS (structural, covariant)
// =============================================================================

/// Structural recursion for parameterised types.
pub struct CollectionRule;
impl CollectionRule {
    fn combine(parts: &[Conformance]) -> Conformance {
        let mut acc = Conformance::Exact;
        for &p in parts {
            if matches!(p, Conformance::Incompatible) {
                return Conformance::Incompatible;
            }
            acc = acc.worst(p);
        }
        acc
    }
}
impl TypeRule for CollectionRule {
    fn name(&self) -> &'static str {
        "collection"
    }
    fn assignable(
        &self,
        ts: &TypeSystem,
        target: &TypeKind,
        source: &TypeKind,
    ) -> Option<Conformance> {
        use TypeKind::*;
        match (target, source) {
            (Array(t), Array(s))
            | (Set(t), Set(s))
            | (Pointer(t), Pointer(s))
            | (Promise(t), Promise(s))
            | (Channel(t), Channel(s))
            | (Range(t), Range(s)) => Some(ts.conformance(t, s)),
            (Option(t), Option(s)) => Some(ts.conformance(t, s)),
            (Map(tk, tv), Map(sk, sv)) => Some(Self::combine(&[
                ts.conformance(tk, sk),
                ts.conformance(tv, sv),
            ])),
            (Pair(a1, a2), Pair(b1, b2)) => Some(Self::combine(&[
                ts.conformance(a1, b1),
                ts.conformance(a2, b2),
            ])),
            (Triple(a1, a2, a3), Triple(b1, b2, b3)) => Some(Self::combine(&[
                ts.conformance(a1, b1),
                ts.conformance(a2, b2),
                ts.conformance(a3, b3),
            ])),
            (Tuple(a), Tuple(b)) => {
                if a.len() != b.len() {
                    return Some(Conformance::Incompatible);
                }
                let parts: Vec<_> = a.iter().zip(b).map(|(x, y)| ts.conformance(x, y)).collect();
                Some(Self::combine(&parts))
            }
            (Result { ok: tok, err: terr }, Result { ok: sok, err: serr }) => {
                Some(Self::combine(&[
                    ts.conformance(tok, sok),
                    ts.conformance(terr, serr),
                ]))
            }
            (
                Reference {
                    inner: ti,
                    mutable: tm,
                },
                Reference {
                    inner: si,
                    mutable: sm,
                },
            ) => {
                if *tm && !*sm {
                    return Some(Conformance::Incompatible);
                }
                let inner = ts.conformance(ti, si);
                Some(if *tm == *sm {
                    inner
                } else {
                    inner.worst(Conformance::Widening)
                })
            }
            (
                Generic {
                    name: tn,
                    type_args: ta,
                },
                Generic {
                    name: sn,
                    type_args: sa,
                },
            ) => {
                if tn != sn || ta.len() != sa.len() {
                    return Some(Conformance::Incompatible);
                }
                let parts: Vec<_> = ta
                    .iter()
                    .zip(sa)
                    .map(|(x, y)| ts.conformance(x, y))
                    .collect();
                Some(Self::combine(&parts))
            }
            _ => None,
        }
    }
    fn unify(&self, ts: &TypeSystem, a: &TypeKind, b: &TypeKind) -> Option<TypeKind> {
        use TypeKind::*;
        match (a, b) {
            (Array(x), Array(y)) => ts.unify(x, y).map(|u| Array(Box::new(u))),
            (Range(x), Range(y)) => ts.unify(x, y).map(|u| Range(Box::new(u))),
            (Set(x), Set(y)) => ts.unify(x, y).map(|u| Set(Box::new(u))),
            (Option(x), Option(y)) => ts.unify(x, y).map(|u| Option(Box::new(u))),
            (Map(xk, xv), Map(yk, yv)) => Some(Map(
                Box::new(ts.unify(xk, yk)?),
                Box::new(ts.unify(xv, yv)?),
            )),
            (Pair(x1, x2), Pair(y1, y2)) => Some(Pair(
                Box::new(ts.unify(x1, y1)?),
                Box::new(ts.unify(x2, y2)?),
            )),
            _ => None,
        }
    }
}

// =============================================================================
//         SECTION: NOMINAL (inheritance via registry)
// =============================================================================

/// Nominal subtyping via the attached [`TypeRegistry`](crate::types::TypeRegistry).
pub struct NominalRule;
impl NominalRule {
    fn type_name(t: &TypeKind) -> Option<&str> {
        match t {
            TypeKind::Object(n) | TypeKind::Native(n) | TypeKind::Enum(n) => Some(n.as_str()),
            _ => None,
        }
    }
}
impl TypeRule for NominalRule {
    fn name(&self) -> &'static str {
        "nominal"
    }
    fn assignable(
        &self,
        ts: &TypeSystem,
        target: &TypeKind,
        source: &TypeKind,
    ) -> Option<Conformance> {
        let (tn, sn) = (Self::type_name(target)?, Self::type_name(source)?);
        if tn == sn {
            return Some(Conformance::Exact);
        }
        let registry = ts.registry()?;
        let reg = registry.read().ok()?;
        let tid: TypeId = reg.get_type_id(tn)?;
        let sid: TypeId = reg.get_type_id(sn)?;
        if reg.is_subtype(sid, tid) {
            Some(Conformance::Widening)
        } else {
            None
        }
    }
}
