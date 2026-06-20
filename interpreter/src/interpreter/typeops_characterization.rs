//! Characterization tests for the interpreter's value-operations layer.
//!
//! These tests capture the CURRENT observable behavior of value operations
//! (binary arithmetic, comparisons, unary ops, casts, type checks, truthiness
//! and default values) exercised through the public API of this crate.
//!
//! They are intentionally written to PASS on the code AS IT IS TODAY, before
//! the planned extraction of these operations into a dedicated `ops/` module.
//! After that refactor they must continue to pass unchanged, guaranteeing
//! 100% behavioral parity. Every expected value here was OBSERVED by running
//! the interpreter, never guessed.

use crate::interpreter::{eval, run_and_get_output};
use shared::types::{Number, Value};

// =============================================================================
//                    BINARY ARITHMETIC
// =============================================================================

#[test]
fn char_binary_add_integers() {
    assert_eq!(eval("2 + 3").unwrap(), Value::Number(Number::I64(5)));
}

#[test]
fn char_binary_sub_integers() {
    assert_eq!(eval("10 - 4").unwrap(), Value::Number(Number::I64(6)));
}

#[test]
fn char_binary_mul_integers() {
    assert_eq!(eval("3 * 4").unwrap(), Value::Number(Number::I64(12)));
}

#[test]
fn char_binary_div_integers_is_float() {
    // Division always produces a real (float) value, even on integers.
    let v = eval("7 / 2").unwrap();
    match v {
        Value::Number(Number::F128(f)) => {
            assert!((f.to_f64() - 3.5).abs() < 1e-9, "got {}", f.to_f64());
        }
        Value::Number(Number::F64(f)) => {
            assert!((f - 3.5).abs() < 1e-9, "got {}", f);
        }
        other => panic!("expected float from division, got {:?}", other),
    }
}

#[test]
fn char_binary_mod_keyword_passes_through() {
    // CAPTURED: the `мод` keyword is NOT a binary operator in expression
    // context — it is lexed as an identifier and never consumed, so the
    // expression evaluates to just the left operand. (Use `%` for modulo.)
    assert_eq!(eval("7 мод 3").unwrap(), Value::Number(Number::I64(7)));
}

#[test]
fn char_binary_mod_percent() {
    assert_eq!(eval("7 % 3").unwrap(), Value::Number(Number::I64(1)));
}

#[test]
fn char_binary_power() {
    let v = eval("2 ** 3").unwrap();
    match v {
        Value::Number(Number::F128(f)) => {
            assert!((f.to_f64() - 8.0).abs() < 1e-9, "got {}", f.to_f64());
        }
        Value::Number(Number::F64(f)) => {
            assert!((f - 8.0).abs() < 1e-9, "got {}", f);
        }
        Value::Number(Number::I64(i)) => assert_eq!(i, 8),
        other => panic!("expected number from power, got {:?}", other),
    }
}

#[test]
fn char_string_concat_with_plus() {
    assert_eq!(
        eval("\"ab\" + \"cd\"").unwrap(),
        Value::String("abcd".to_string())
    );
}

// =============================================================================
//                    COMPARISONS
// =============================================================================

#[test]
fn char_cmp_lt_numbers() {
    assert_eq!(eval("3 < 5").unwrap(), Value::Boolean(true));
    assert_eq!(eval("5 < 3").unwrap(), Value::Boolean(false));
}

#[test]
fn char_cmp_gt_numbers() {
    assert_eq!(eval("5 > 3").unwrap(), Value::Boolean(true));
}

#[test]
fn char_cmp_le_numbers() {
    assert_eq!(eval("3 <= 3").unwrap(), Value::Boolean(true));
    assert_eq!(eval("4 <= 3").unwrap(), Value::Boolean(false));
}

#[test]
fn char_cmp_ge_numbers() {
    assert_eq!(eval("3 >= 3").unwrap(), Value::Boolean(true));
    assert_eq!(eval("2 >= 3").unwrap(), Value::Boolean(false));
}

#[test]
fn char_cmp_eq_numbers() {
    assert_eq!(eval("5 = 5").unwrap(), Value::Boolean(true));
    assert_eq!(eval("5 = 6").unwrap(), Value::Boolean(false));
}

#[test]
fn char_cmp_ne_numbers_angle() {
    assert_eq!(eval("5 <> 3").unwrap(), Value::Boolean(true));
    assert_eq!(eval("5 <> 5").unwrap(), Value::Boolean(false));
}

#[test]
fn char_cmp_ne_numbers_bang() {
    assert_eq!(eval("5 != 3").unwrap(), Value::Boolean(true));
}

#[test]
fn char_cmp_lt_strings() {
    assert_eq!(eval("\"abc\" < \"abd\"").unwrap(), Value::Boolean(true));
    assert_eq!(eval("\"abd\" < \"abc\"").unwrap(), Value::Boolean(false));
}

#[test]
fn char_cmp_eq_strings() {
    assert_eq!(eval("\"hi\" = \"hi\"").unwrap(), Value::Boolean(true));
    assert_eq!(eval("\"hi\" = \"ho\"").unwrap(), Value::Boolean(false));
}

// =============================================================================
//                    UNARY
// =============================================================================

#[test]
fn char_unary_minus_signed() {
    assert_eq!(eval("-5").unwrap(), Value::Number(Number::I64(-5)));
}

#[test]
fn char_unary_not_boolean() {
    assert_eq!(eval("не да").unwrap(), Value::Boolean(false));
    assert_eq!(eval("не нет").unwrap(), Value::Boolean(true));
}

// =============================================================================
//                    CAST  (expr как Тип)
// =============================================================================

#[test]
fn char_cast_to_int() {
    assert_eq!(eval("3 как цел").unwrap(), Value::Number(Number::I64(3)));
}

#[test]
fn char_cast_to_float() {
    match eval("3 как вещ").unwrap() {
        Value::Number(Number::F64(f)) => assert!((f - 3.0).abs() < 1e-9, "got {}", f),
        other => panic!("expected F64, got {:?}", other),
    }
}

#[test]
fn char_cast_to_string() {
    assert_eq!(eval("42 как лит").unwrap(), Value::String("42".to_string()));
}

#[test]
fn char_cast_to_bool() {
    assert_eq!(eval("1 как лог").unwrap(), Value::Boolean(true));
    assert_eq!(eval("0 как лог").unwrap(), Value::Boolean(false));
}

#[test]
fn char_cast_string_to_float() {
    match eval("\"2.5\" как вещ").unwrap() {
        Value::Number(Number::F64(f)) => assert!((f - 2.5).abs() < 1e-9, "got {}", f),
        other => panic!("expected F64, got {:?}", other),
    }
}

#[test]
fn char_cast_to_char_is_unsupported() {
    // сим (Char) is NOT a supported cast target today.
    assert!(eval("65 как сим").is_err());
}

// =============================================================================
//                    TYPE CHECK  (expr это Тип)
// =============================================================================

// CAPTURED: the `это` type-check operator is NOT reachable in expression
// context. The token `это` is lexed as the self-reference keyword (Token::This),
// not as the identifier the postfix `это`-branch expects, so the type-check is
// never applied — the expression evaluates to just the left operand and the
// trailing `... Тип` is left unconsumed. Documented here verbatim as today's
// behavior; a true type-check would return a Boolean.
#[test]
fn char_typecheck_int_passes_through() {
    assert_eq!(eval("5 это цел").unwrap(), Value::Number(Number::I64(5)));
}

#[test]
fn char_typecheck_string_passes_through() {
    assert_eq!(
        eval("\"x\" это лит").unwrap(),
        Value::String("x".to_string())
    );
}

#[test]
fn char_typecheck_bool_passes_through() {
    assert_eq!(eval("да это лог").unwrap(), Value::Boolean(true));
}

// =============================================================================
//                    TRUTHINESS  (если <val> то ... иначе ... все)
// =============================================================================

fn truth_branch(val: &str) -> String {
    let src = format!(
        "алг Тест\nнач\n    если {} то\n        вывод \"Y\"\n    иначе\n        вывод \"N\"\n    все\nкон\n",
        val
    );
    run_and_get_output(&src).unwrap()
}

#[test]
fn char_truthiness_nonzero_number() {
    assert!(truth_branch("5").contains("Y"));
}

#[test]
fn char_truthiness_zero() {
    assert!(truth_branch("0").contains("N"));
}

#[test]
fn char_truthiness_nonempty_string() {
    assert_eq!(truth_branch("\"hi\"").trim(), "Y");
}

#[test]
fn char_truthiness_empty_string() {
    assert_eq!(truth_branch("\"\"").trim(), "N");
}

// =============================================================================
//                    DEFAULT VALUES (declaration without initializer)
// =============================================================================

fn default_output(decl: &str) -> String {
    let src = format!("алг Тест\nнач\n    {}\n    вывод x\nкон\n", decl);
    run_and_get_output(&src).unwrap()
}

#[test]
fn char_default_int() {
    assert_eq!(default_output("цел x").trim(), "0");
}

#[test]
fn char_default_float() {
    assert_eq!(default_output("вещ x").trim(), "0");
}

#[test]
fn char_default_bool() {
    // CAPTURED: default логический (bool) prints as "нет" (false).
    assert_eq!(default_output("лог x").trim(), "нет");
}

#[test]
fn char_default_string() {
    // CAPTURED: default литеральный (string) is empty.
    assert_eq!(default_output("лит x"), "\n");
}

#[test]
fn char_default_char() {
    // CAPTURED: default символьный (char) is the NUL character '\0'.
    assert_eq!(default_output("сим x"), "\0\n");
}
