use shared::f128::F128;
use shared::math::MathErr;
use shared::math::MathOperators;
use shared::types::{Number, Value};

#[test]
fn string_subtraction_basic() {
    let a = Value::String("banana".to_string());
    let b = Value::String("na".to_string());
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(r, Value::String("ba".to_string()));
}

#[test]
fn string_subtraction_empty_rhs() {
    let a = Value::String("hello".to_string());
    let b = Value::String("".to_string());
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(r, Value::String("hello".to_string()));
}

#[test]
fn array_subtraction_basic() {
    let a = Value::Array(vec![
        Value::from(1i64),
        Value::from(2i64),
        Value::from(2i64),
        Value::from(3i64),
    ]);
    let b = Value::Array(vec![Value::from(2i64)]);
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(
        r,
        Value::Array(vec![
            Value::from(1i64),
            Value::from(2i64),
            Value::from(3i64)
        ])
    );
}

#[test]
fn array_subtraction_multiple() {
    let a = Value::Array(vec![
        Value::from(1i64),
        Value::from(2i64),
        Value::from(2i64),
        Value::from(3i64),
    ]);
    let b = Value::Array(vec![Value::from(2i64), Value::from(2i64)]);
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(r, Value::Array(vec![Value::from(1i64), Value::from(3i64)]));
}

#[test]
fn array_subtraction_collision_small_vb() {
    let a = Value::Array(vec![Value::from(1i32), Value::from(1.0_f64)]);
    let b = Value::Array(vec![Value::from(1i32)]);
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(r, Value::Array(vec![Value::from(1.0_f64)]));
}

#[test]
fn array_subtraction_collision_large_vb() {
    let a = Value::Array(vec![
        Value::from(1i32),
        Value::from(1.0_f64),
        Value::from(2i64),
        Value::from(3i64),
    ]);
    let mut vb = Vec::new();
    vb.push(Value::from(99i32));
    for i in 0..9 {
        vb.push(Value::from(i as i64 + 100));
    }
    vb.push(Value::from(1i32));
    let b = Value::Array(vb);
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(
        r,
        Value::Array(vec![
            Value::from(1.0_f64),
            Value::from(2i64),
            Value::from(3i64)
        ])
    );
}

#[test]
fn string_subtraction_overlap() {
    let a = Value::String("banana".to_string());
    let b = Value::String("ana".to_string());
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(r, Value::String("bna".to_string()));
}

#[test]
fn string_subtraction_unicode() {
    let a = Value::String("🙂🙂🙂".to_string());
    let b = Value::String("🙂".to_string());
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(r, Value::String("".to_string()));
}

#[test]
fn string_subtraction_no_occurrence() {
    let a = Value::String("hello".to_string());
    let b = Value::String("z".to_string());
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(r, Value::String("hello".to_string()));
}

#[test]
fn string_subtraction_multiple_non_overlapping() {
    let a = Value::String("aaaaa".to_string());
    let b = Value::String("aa".to_string());
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(r, Value::String("a".to_string()));
}

#[test]
fn array_subtraction_nested_arrays() {
    let a = Value::Array(vec![
        Value::Array(vec![Value::from(1i64)]),
        Value::Array(vec![Value::from(2i64)]),
        Value::Array(vec![Value::from(1i64)]),
    ]);
    let b = Value::Array(vec![Value::Array(vec![Value::from(1i64)])]);
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(
        r,
        Value::Array(vec![
            Value::Array(vec![Value::from(2i64)]),
            Value::Array(vec![Value::from(1i64)])
        ])
    );
}

#[test]
fn array_subtraction_not_present() {
    let a = Value::Array(vec![
        Value::from(1i64),
        Value::from(2i64),
        Value::from(3i64),
    ]);
    let b = Value::Array(vec![Value::from(9i32)]);
    let r = MathOperators::sub(a.clone(), b, true).unwrap();
    assert_eq!(r, a);
}

#[test]
fn array_subtraction_large_mixed_types() {
    let a = Value::Array(vec![
        Value::from(1i32),
        Value::from(1.0_f64),
        Value::from(2i64),
    ]);
    let mut vb = Vec::new();
    for i in 0..10 {
        vb.push(Value::from(i as i64 + 100));
    }
    vb.push(Value::from(1i32));
    let b = Value::Array(vb);
    let r = MathOperators::sub(a, b, true).unwrap();
    assert_eq!(
        r,
        Value::Array(vec![Value::from(1.0_f64), Value::from(2i64)])
    );
}

#[test]
fn round_default_and_sign() {
    let a = Value::from(1.4);
    let r = MathOperators::round(a, None, None, true).unwrap();
    assert_eq!(r, Value::from(1.0));

    let a = Value::from(1.5);
    let r = MathOperators::round(a, None, None, true).unwrap();
    assert_eq!(r, Value::from(2.0));
    let a = Value::from(-1.5);
    let r = MathOperators::round(a, None, None, true).unwrap();
    assert_eq!(r, Value::from(-2.0));
}

#[test]
fn round_precision_examples() {
    let a = Value::from(1.234);
    let r = MathOperators::round(a, Some(Value::from(2i32)), None, true).unwrap();
    assert_eq!(r, Value::from(1.23));

    let a = Value::from(1.235);
    let r = MathOperators::round(a, Some(Value::from(2i32)), None, true).unwrap();
    assert_eq!(r, Value::from(1.24));
}

#[test]
fn round_with_rf_threshold() {
    let a = Value::from(1.25);
    let r =
        MathOperators::round(a, Some(Value::from(1i32)), Some(Value::from(6i32)), true).unwrap();
    assert_eq!(r, Value::from(1.2));

    let a = Value::from(1.25);
    let r =
        MathOperators::round(a, Some(Value::from(1i32)), Some(Value::from(4i32)), true).unwrap();
    assert_eq!(r, Value::from(1.3));
}

#[test]
fn round_preserves_f32_type() {
    let a = Value::from(1.235f32);
    let r = MathOperators::round(a, Some(Value::from(2i32)), None, true).unwrap();
    assert_eq!(r, Value::from(1.24f32));
}

#[test]
fn round_preserves_f128_type() {
    let a = Value::from(F128::from(1.235));
    let r = MathOperators::round(a, Some(Value::from(2i32)), None, true).unwrap();
    if let Value::Number(Number::F128(v)) = r {
        assert!(
            (v.to_f64() - 1.24).abs() < 1e-10,
            "Expected ~1.24, got {}",
            v.to_f64()
        );
    } else {
        panic!("Expected F128, got {:?}", r);
    }
}

#[test]
fn string_divide_by_delimiter_returns_array_and_count() {
    let s = Value::from("Яблоко, Банан, Виноград, Помело, Кокос");
    let delim = Value::from(", ");
    let r = MathOperators::div(s, delim, true).unwrap();
    match r {
        Value::Pair(left, right) => {
            if let Value::Array(arr) = *left {
                assert_eq!(arr.len(), 5);
                assert_eq!(arr[0], Value::from("Яблоко"));
                assert_eq!(arr[4], Value::from("Кокос"));
            } else {
                panic!("left is not array");
            }
            if let Value::Number(Number::I64(v)) = *right {
                assert_eq!(v, 4);
            } else {
                panic!("right is not I64");
            }
        }
        _ => panic!("expected pair"),
    }
}

#[test]
fn string_divide_by_empty_delim_errors() {
    let s = Value::from("a,b");
    let delim = Value::from("");
    let e = MathOperators::div(s, delim, true);
    assert!(e.is_err());
}

#[test]
fn sqrt_minus_one_special_error() {
    let a = Value::from(-1.0);
    let e = MathOperators::sqrt(a, true);
    assert_eq!(e.unwrap_err(), MathErr::NotRealOneSqrt.msg());

    let b = Value::from(-1i64);
    let e2 = MathOperators::sqrt(b, true);
    assert_eq!(e2.unwrap_err(), MathErr::NotRealOneSqrt.msg());
}

#[test]
fn string_multiplication_and_division() {
    let a = Value::String("abc".to_string());
    let b = Value::from(3i32);
    let r = MathOperators::mul(a.clone(), b.clone(), true).unwrap();
    assert_eq!(r, Value::String("abcabcabc".to_string()));

    // number * string
    let r2 = MathOperators::mul(Value::from(2i32), a.clone(), true).unwrap();
    assert_eq!(r2, Value::String("abcabc".to_string()));

    // division into parts
    let a2 = Value::String("abcdef".to_string());
    let r3 = MathOperators::div(a2.clone(), Value::from(3i32), true).unwrap();
    assert_eq!(
        r3,
        Value::Array(vec![
            Value::String("ab".to_string()),
            Value::String("cd".to_string()),
            Value::String("ef".to_string())
        ])
    );

    // division with remainder distribution
    let a3 = Value::String("abcdefg".to_string());
    let r4 = MathOperators::div(a3.clone(), Value::from(3i32), true).unwrap();
    assert_eq!(
        r4,
        Value::Array(vec![
            Value::String("abc".to_string()),
            Value::String("de".to_string()),
            Value::String("fg".to_string())
        ])
    );
}

#[test]
fn round_invalid_rf_errors() {
    let a = Value::from(1.23);
    let e = MathOperators::round(
        a.clone(),
        Some(Value::from(1i32)),
        Some(Value::from(0i32)),
        true,
    );
    assert!(e.is_err());

    let e = MathOperators::round(a, Some(Value::from(1i32)), Some(Value::from(10i32)), true);
    assert!(e.is_err());
}

#[test]
fn trig_sin_cos_basic() {
    let pi = std::f64::consts::PI;

    // sin(0) = 0
    let r = MathOperators::sin(Value::from(0.0)).unwrap();
    if let Value::Number(Number::F128(v)) = r {
        assert!(v.abs().to_f64().abs() < 1e-10);
    } else {
        panic!("Expected F128 result");
    }

    // sin(pi/2) = 1
    let r = MathOperators::sin(Value::from(pi / 2.0)).unwrap();
    if let Value::Number(Number::F128(v)) = r {
        assert!((v.to_f64() - 1.0).abs() < 1e-10);
    } else {
        panic!("Expected F128 result");
    }

    // cos(0) = 1
    let r = MathOperators::cos(Value::from(0.0)).unwrap();
    if let Value::Number(Number::F128(v)) = r {
        assert!((v.to_f64() - 1.0).abs() < 1e-10);
    } else {
        panic!("Expected F128 result");
    }

    // cos(pi) = -1
    let r = MathOperators::cos(Value::from(pi)).unwrap();
    if let Value::Number(Number::F128(v)) = r {
        assert!((v.to_f64() - -1.0).abs() < 1e-10);
    } else {
        panic!("Expected F128 result");
    }
}

#[test]
fn trig_tg_ctg_basic() {
    let pi = std::f64::consts::PI;
    let val = Value::from(pi / 4.0); // 45 degrees

    // tg(pi/4) = 1
    let r = MathOperators::tg(val.clone()).unwrap();
    if let Value::Number(Number::F128(v)) = r {
        assert!((v.to_f64() - 1.0).abs() < 1e-10);
    } else {
        panic!("Expected F128 result");
    }

    // ctg(pi/4) = 1
    let r = MathOperators::ctg(val).unwrap();
    if let Value::Number(Number::F128(v)) = r {
        assert!((v.to_f64() - 1.0).abs() < 1e-10);
    } else {
        panic!("Expected F128 result");
    }
}

#[test]
fn abs_basic() {
    // Integer
    let r = MathOperators::abs(Value::from(-10)).unwrap();
    assert_eq!(r, Value::from(10));

    // Float
    let r = MathOperators::abs(Value::from(-12.5)).unwrap();
    assert_eq!(r, Value::from(12.5));

    // Positive remains positive
    let r = MathOperators::abs(Value::from(5)).unwrap();
    assert_eq!(r, Value::from(5));
}
