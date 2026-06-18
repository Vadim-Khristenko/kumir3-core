// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Unit tests for the type-system engine.

use std::sync::{Arc, RwLock};

use crate::types::value::{TypeKind as TK, Value};
use crate::types::{TypeDefBuilder, TypeRegistry};
use crate::typesys::*;

fn ts() -> TypeSystem {
    TypeSystem::new()
}

// --- assignability -----------------------------------------------------------

#[test]
fn identity_is_exact() {
    assert_eq!(ts().conformance(&TK::Int64, &TK::Int64), Conformance::Exact);
}

#[test]
fn any_accepts_everything() {
    let s = ts();
    assert!(s.is_assignable(&TK::Any, &TK::Int64));
    assert!(s.is_assignable(&TK::Auto, &TK::String));
    assert_eq!(s.conformance(&TK::Int64, &TK::Any), Conformance::Coercible);
}

#[test]
fn never_is_bottom() {
    assert!(ts().is_assignable(&TK::String, &TK::Never));
}

#[test]
fn numeric_widening_and_narrowing() {
    let s = ts();
    assert_eq!(s.conformance(&TK::Int64, &TK::Int16), Conformance::Widening);
    assert_eq!(
        s.conformance(&TK::Int16, &TK::Int64),
        Conformance::Coercible
    );
    assert_eq!(
        s.conformance(&TK::Float64, &TK::Int32),
        Conformance::Widening
    );
    assert_eq!(
        s.conformance(&TK::Int32, &TK::Float64),
        Conformance::Coercible
    );
    assert!(!s.is_assignable(&TK::Int16, &TK::Int64));
    assert!(s.is_assignable(&TK::Int64, &TK::Int16));
}

#[test]
fn null_and_option_wrapping() {
    let s = ts();
    let opt_int = TK::Option(Box::new(TK::Int64));
    assert!(s.is_assignable(&opt_int, &TK::Null));
    assert!(s.is_assignable(&opt_int, &TK::Int64));
    assert!(s.is_assignable(&opt_int, &TK::Int16));
}

#[test]
fn array_covariance() {
    let s = ts();
    let arr_i64 = TK::Array(Box::new(TK::Int64));
    let arr_i16 = TK::Array(Box::new(TK::Int16));
    assert!(s.is_assignable(&arr_i64, &arr_i16));
    assert!(!s.is_assignable(&arr_i16, &arr_i64));
    let arr_str = TK::Array(Box::new(TK::String));
    assert!(!s.conformance(&arr_i64, &arr_str).is_compatible());
}

#[test]
fn is_subtype_helper() {
    let s = ts();
    assert!(s.is_subtype(&TK::Int16, &TK::Int64));
    assert!(!s.is_subtype(&TK::Int64, &TK::Int16));
}

// --- unification -------------------------------------------------------------

#[test]
fn unify_numeric_picks_wider() {
    let s = ts();
    assert_eq!(s.unify(&TK::Int16, &TK::Int64), Some(TK::Int64));
    assert_eq!(s.unify(&TK::Int32, &TK::Float64), Some(TK::Float64));
}

#[test]
fn unify_arrays_recurses() {
    let s = ts();
    let a = TK::Array(Box::new(TK::Int16));
    let b = TK::Array(Box::new(TK::Int64));
    assert_eq!(s.unify(&a, &b), Some(TK::Array(Box::new(TK::Int64))));
}

// --- operators ---------------------------------------------------------------

#[test]
fn binop_arithmetic_and_concat() {
    let s = ts();
    assert_eq!(
        s.result_of_binop(TypeOp::Add, &TK::Int16, &TK::Int64)
            .unwrap(),
        TK::Int64
    );
    assert_eq!(
        s.result_of_binop(TypeOp::Mul, &TK::Int32, &TK::Float64)
            .unwrap(),
        TK::Float64
    );
    assert_eq!(
        s.result_of_binop(TypeOp::Add, &TK::String, &TK::Int64)
            .unwrap(),
        TK::String
    );
}

#[test]
fn binop_comparison_and_logic() {
    let s = ts();
    assert_eq!(
        s.result_of_binop(TypeOp::Lt, &TK::Int16, &TK::Int64)
            .unwrap(),
        TK::Bool
    );
    assert_eq!(
        s.result_of_binop(TypeOp::And, &TK::Bool, &TK::Bool)
            .unwrap(),
        TK::Bool
    );
    assert!(
        s.result_of_binop(TypeOp::And, &TK::Bool, &TK::Int64)
            .is_err()
    );
    assert!(
        s.result_of_binop(TypeOp::Add, &TK::Bool, &TK::Bool)
            .is_err()
    );
}

// --- coercion planning -------------------------------------------------------

#[test]
fn coercion_plan() {
    let s = ts();
    assert_eq!(s.coercion(&TK::Int64, &TK::Int64), Coercion::Identity);
    assert_eq!(s.coercion(&TK::Int64, &TK::Int16), Coercion::Widen);
    assert_eq!(
        s.coercion(&TK::Option(Box::new(TK::Int64)), &TK::Int64),
        Coercion::Wrap
    );
    assert_eq!(s.coercion(&TK::Int16, &TK::Int64), Coercion::Cast); // narrowing
    assert_eq!(s.coercion(&TK::Int64, &TK::String), Coercion::Forbidden);
}

// --- default values ----------------------------------------------------------

#[test]
fn default_values() {
    let s = ts();
    assert_eq!(s.default_value(&TK::Int64), Some(Value::int(0)));
    assert_eq!(s.default_value(&TK::Bool), Some(Value::bool(false)));
    assert_eq!(s.default_value(&TK::String), Some(Value::string("")));
    assert_eq!(
        s.default_value(&TK::Array(Box::new(TK::Int64))),
        Some(Value::Array(vec![]))
    );
    assert_eq!(
        s.default_value(&TK::Option(Box::new(TK::Int64))),
        Some(Value::none())
    );
    assert_eq!(
        s.default_value(&TK::Tuple(vec![TK::Int64, TK::Bool])),
        Some(Value::Tuple(vec![Value::int(0), Value::bool(false)]))
    );
    assert_eq!(
        s.default_value(&TK::Function {
            params: vec![],
            result: None
        }),
        None
    );
}

// --- nominal -----------------------------------------------------------------

#[test]
fn nominal_subtyping_via_registry() {
    let reg = TypeRegistry::new();
    let animal = reg.register_type(TypeDefBuilder::new("Животное").build());
    let _cat = reg.register_type(TypeDefBuilder::new("Кот").parent(animal).build());
    let s = TypeSystem::new().with_registry(Arc::new(RwLock::new(reg)));
    let animal_t = TK::Object("Животное".into());
    let cat_t = TK::Object("Кот".into());
    assert!(s.is_assignable(&animal_t, &cat_t));
    assert!(!s.is_assignable(&cat_t, &animal_t));
}

// --- extensibility -----------------------------------------------------------

#[test]
fn extensibility_register_custom_rule() {
    struct BoolFromInt;
    impl TypeRule for BoolFromInt {
        fn name(&self) -> &'static str {
            "bool-from-int"
        }
        fn assignable(&self, _ts: &TypeSystem, target: &TK, source: &TK) -> Option<Conformance> {
            if matches!(target, TK::Bool) && source.is_integer() {
                Some(Conformance::Coercible)
            } else {
                None
            }
        }
    }
    let mut s = TypeSystem::new();
    let before = s.rule_count();
    assert!(!s.conformance(&TK::Bool, &TK::Int64).is_compatible());
    s.register_rule(Arc::new(BoolFromInt));
    assert_eq!(s.rule_count(), before + 1);
    assert_eq!(s.conformance(&TK::Bool, &TK::Int64), Conformance::Coercible);
}

// --- new type: range --------------------------------------------------------

#[test]
fn range_covariance_and_unify() {
    let s = ts();
    let r64 = TK::range(TK::Int64);
    let r16 = TK::range(TK::Int16);
    assert!(s.is_assignable(&r64, &r16));
    assert!(!s.is_assignable(&r16, &r64));
    assert_eq!(s.unify(&r16, &r64), Some(TK::range(TK::Int64)));
    assert_eq!(TK::range(TK::Int64).russian_name(), "диапазон цел");
    assert!(TK::range(TK::Int64).is_collection());
    // range has no natural default value
    assert_eq!(s.default_value(&r64), None);
}

// --- extended coverage -------------------------------------------------------

#[test]
fn result_type_covariance() {
    let s = ts();
    let r_wide = TK::Result {
        ok: Box::new(TK::Int64),
        err: Box::new(TK::String),
    };
    let r_narrow = TK::Result {
        ok: Box::new(TK::Int16),
        err: Box::new(TK::String),
    };
    assert!(s.is_assignable(&r_wide, &r_narrow));
    assert!(!s.is_assignable(&r_narrow, &r_wide));
}

#[test]
fn reference_mutability() {
    let s = ts();
    let imm = TK::Reference {
        inner: Box::new(TK::Int64),
        mutable: false,
    };
    let mutbl = TK::Reference {
        inner: Box::new(TK::Int64),
        mutable: true,
    };
    // &изм source can satisfy & target (downgrade) ...
    assert!(s.conformance(&imm, &mutbl).is_compatible());
    // ... but & cannot satisfy &изм target.
    assert!(!s.conformance(&mutbl, &imm).is_compatible());
}

#[test]
fn generic_match_and_mismatch() {
    let s = ts();
    let g_a = TK::Generic {
        name: "Ящик".into(),
        type_args: vec![TK::Int16],
    };
    let g_b = TK::Generic {
        name: "Ящик".into(),
        type_args: vec![TK::Int64],
    };
    let g_c = TK::Generic {
        name: "Коробка".into(),
        type_args: vec![TK::Int64],
    };
    assert!(s.is_assignable(&g_b, &g_a)); // covariant arg
    assert!(!s.conformance(&g_c, &g_b).is_compatible()); // different name
}

#[test]
fn map_covariance() {
    let s = ts();
    let m_wide = TK::Map(Box::new(TK::String), Box::new(TK::Int64));
    let m_narrow = TK::Map(Box::new(TK::String), Box::new(TK::Int16));
    assert!(s.is_assignable(&m_wide, &m_narrow));
    assert!(!s.is_assignable(&m_narrow, &m_wide));
}

#[test]
fn tuple_length_mismatch() {
    let s = ts();
    let t2 = TK::Tuple(vec![TK::Int64, TK::Bool]);
    let t3 = TK::Tuple(vec![TK::Int64, TK::Bool, TK::String]);
    assert!(!s.conformance(&t2, &t3).is_compatible());
    assert!(s.conformance(&t2, &t2).is_implicit());
}

#[test]
fn nested_option_widening() {
    let s = ts();
    let oo_wide = TK::Option(Box::new(TK::Option(Box::new(TK::Int64))));
    let oo_narrow = TK::Option(Box::new(TK::Option(Box::new(TK::Int16))));
    assert!(s.is_assignable(&oo_wide, &oo_narrow));
}

#[test]
fn errors_are_descriptive() {
    let s = ts();
    let err = s.check_assignable(&TK::Int64, &TK::String).unwrap_err();
    assert!(matches!(err, TypeError::NotAssignable { .. }));
    let err2 = s.common_type(&TK::String, &TK::Bool).unwrap_err();
    assert!(matches!(err2, TypeError::NoCommonType { .. }));
    let err3 = s
        .result_of_binop(TypeOp::Sub, &TK::String, &TK::String)
        .unwrap_err();
    assert!(matches!(err3, TypeError::BinaryOpUnsupported { .. }));
}

#[test]
fn unsigned_signed_rules() {
    let s = ts();
    // unsigned -> larger signed is widening (нат_8 -> цел_64)
    assert_eq!(s.conformance(&TK::Int64, &TK::UInt8), Conformance::Widening);
    // signed -> unsigned loses sign => coercible, not implicit
    assert_eq!(
        s.conformance(&TK::UInt64, &TK::Int64),
        Conformance::Coercible
    );
}

#[test]
fn front_rule_overrides_builtin() {
    // A front rule that declares narrowing цел_64 -> цел_16 as exact.
    struct ForceExact;
    impl TypeRule for ForceExact {
        fn name(&self) -> &'static str {
            "force-exact"
        }
        fn assignable(&self, _ts: &TypeSystem, target: &TK, source: &TK) -> Option<Conformance> {
            if matches!(target, TK::Int16) && matches!(source, TK::Int64) {
                Some(Conformance::Exact)
            } else {
                None
            }
        }
    }
    let mut s = TypeSystem::new();
    assert_eq!(
        s.conformance(&TK::Int16, &TK::Int64),
        Conformance::Coercible
    );
    s.register_rule_front(Arc::new(ForceExact));
    assert_eq!(s.conformance(&TK::Int16, &TK::Int64), Conformance::Exact);
}
