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
fn char_binary_mod_keyword() {
    // FIXED (slice 2b): `мод` is now an arithmetic remainder operator
    // (keyword lexed to Token::Percent), equivalent to `%`.
    assert_eq!(eval("7 мод 3").unwrap(), Value::Number(Number::I64(1)));
    assert_eq!(eval("10 мод 4").unwrap(), Value::Number(Number::I64(2)));
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

// FIXED (slice 2b): the postfix `это` type-check is now reachable — `это`
// lexes to Token::This and, in postfix position (after an expression), the
// parser treats `expr это Тип` as a type check returning a Boolean.
#[test]
fn char_typecheck_int_true() {
    assert_eq!(eval("5 это цел").unwrap(), Value::Boolean(true));
}

#[test]
fn char_typecheck_string_true() {
    assert_eq!(eval("\"x\" это лит").unwrap(), Value::Boolean(true));
}

#[test]
fn char_typecheck_bool_true() {
    assert_eq!(eval("да это лог").unwrap(), Value::Boolean(true));
}

#[test]
fn char_typecheck_mismatch_false() {
    // A value of the wrong type yields `false`, not the left operand.
    assert_eq!(eval("5 это лит").unwrap(), Value::Boolean(false));
    assert_eq!(eval("\"x\" это цел").unwrap(), Value::Boolean(false));
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

// =============================================================================
//   DEFECT FIXES (W0) — corrected behaviors for crashy / silently-wrong nodes
// =============================================================================
//
// Each test below pins the CORRECTED behavior of a defect that previously
// crashed (runtime "not implemented" / "method not found") or silently
// produced wrong values. See .local/.../W0-spec-truth-audit.md.

// -----------------------------------------------------------------------------
//   DEFECT 1 — `?` error-propagation operator (`__propagate__`)
// -----------------------------------------------------------------------------

// `?` on a present Option (`некоторое(v)`) unwraps to the inner value.
#[test]
fn char_propagate_unwraps_some() {
    // `?` on a present Option does NOT fire early-return; it unwraps to 7,
    // which is then printed normally.
    let prog = "алг Тест\nнач\n    цел y\n    y := некоторое(7)?\n    вывод y\nкон\n";
    assert_eq!(run_and_get_output(prog).unwrap().trim(), "7");
}

// `?` on `пусто` (Null) propagates: the enclosing algorithm returns early
// (before the `вывод`), so nothing is printed.
#[test]
fn char_propagate_null_short_circuits_algorithm() {
    let prog = "алг Тест\nнач\n    цел y\n    y := пусто?\n    вывод \"после\"\nкон\n";
    let out = run_and_get_output(prog).unwrap();
    assert!(
        !out.contains("после"),
        "propagation must short-circuit before the print, got {:?}",
        out
    );
}

// `?` on `пусто` exercises the early-return path from the enclosing algorithm
// and must never crash.
#[test]
fn char_propagate_null_does_not_crash() {
    let prog = "алг Тест\nнач\n    цел y\n    y := пусто?\nкон\n";
    assert!(
        run_and_get_output(prog).is_ok(),
        "propagation must not crash"
    );
}

// `?` on a plain non-propagatable, non-unwrappable value is a CLEAR error
// (TypeMismatch), not a panic and not "method not found".
#[test]
fn char_propagate_on_plain_value_is_clear_error() {
    let err = eval("5?").unwrap_err();
    assert_eq!(err.kind, crate::interpreter::RuntimeErrorKind::TypeMismatch);
    assert!(
        err.message.contains("Оператор '?'"),
        "got {:?}",
        err.message
    );
}

// -----------------------------------------------------------------------------
//   DEFECT 2 — constructor errors must surface (no more `let _ =`)
// -----------------------------------------------------------------------------

// A constructor body that raises a runtime error must make `новый` fail,
// instead of the error being silently swallowed.
#[test]
fn char_constructor_error_propagates() {
    let prog = "\
класс Тчк
цел x
конструктор()
нач
    x := 1 / \"строка\"
кон
кон
алг Тест
нач
    p := новый Тчк()
кон
";
    // The failing constructor body must surface as an error from `новый`.
    assert!(
        run_and_get_output(prog).is_err(),
        "constructor error must propagate out of `новый`"
    );

    // Control: the identical class with a valid constructor body succeeds,
    // proving the error above comes from the body (not from parsing).
    let ok_prog = "\
класс Тчк
цел x
конструктор()
нач
    x := 1
кон
кон
алг Тест
нач
    p := новый Тчк()
кон
";
    assert!(
        run_and_get_output(ok_prog).is_ok(),
        "valid constructor must succeed"
    );
}

// -----------------------------------------------------------------------------
//   DEFECT 3 — `ждать` / `запустить` in expression position (sync passthrough)
// -----------------------------------------------------------------------------

#[test]
fn char_await_in_expression_position() {
    // KITE-3 §6 shape: `y := ждать <expr>` evaluates synchronously.
    let prog = "алг Тест\nнач\n    цел y\n    y := ждать (40 + 2)\n    вывод y\nкон\n";
    assert_eq!(run_and_get_output(prog).unwrap().trim(), "42");
}

#[test]
fn char_spawn_in_expression_position() {
    // `запустить <expr>` in value position does not crash; passthrough value.
    let v = eval("запустить (1 + 2)").unwrap();
    assert_eq!(v, Value::Number(Number::I64(3)));
}

// -----------------------------------------------------------------------------
//   DEFECT 4 — tuple literal `(a, b)` evaluates to Value::Tuple
// -----------------------------------------------------------------------------

#[test]
fn char_tuple_literal_evaluates() {
    let v = eval("(1, 2, 3)").unwrap();
    assert_eq!(
        v,
        Value::Tuple(vec![
            Value::Number(Number::I64(1)),
            Value::Number(Number::I64(2)),
            Value::Number(Number::I64(3)),
        ])
    );
}

#[test]
fn char_tuple_literal_evaluates_elements() {
    // Non-literal elements are evaluated, not dropped.
    let v = eval("(1 + 1, 2 * 3)").unwrap();
    assert_eq!(
        v,
        Value::Tuple(vec![
            Value::Number(Number::I64(2)),
            Value::Number(Number::I64(6)),
        ])
    );
}

// -----------------------------------------------------------------------------
//   DEFECT 5 — array literal evaluates each element (no more Undefined)
// -----------------------------------------------------------------------------

#[test]
fn char_array_literal_evaluates_nonliteral_elements() {
    // Previously `[x, y+1]` degraded to `[Undefined, Undefined]`.
    let prog = "алг Тест\nнач\n    цел x\n    x := 10\n    вывод [x, x + 1, x * 2]\nкон\n";
    let out = run_and_get_output(prog).unwrap();
    assert!(out.contains("10"), "got {:?}", out);
    assert!(out.contains("11"), "got {:?}", out);
    assert!(out.contains("20"), "got {:?}", out);
    assert!(
        !out.to_lowercase().contains("undefined") && !out.contains("неопред"),
        "array must not contain Undefined placeholders, got {:?}",
        out
    );
}

#[test]
fn char_array_literal_of_literals_still_works() {
    let v = eval("[1, 2, 3]").unwrap();
    assert_eq!(
        v,
        Value::Array(vec![
            Value::Number(Number::I64(1)),
            Value::Number(Number::I64(2)),
            Value::Number(Number::I64(3)),
        ])
    );
}

// -----------------------------------------------------------------------------
//   DEFECT 6 — multidimensional indexing `arr[i, j]` is a CLEAN error
// -----------------------------------------------------------------------------

#[test]
fn char_multidim_index_is_clean_not_implemented_error() {
    // Build a 1-D array, then index it with two subscripts `a[1, 2]`.
    // Multidimensional indexing is intentionally a CLEAR NotImplemented error,
    // never a panic.
    let prog = "алг Тест\nнач\n    a := [10, 20, 30]\n    вывод a[1, 2]\nкон\n";
    let err = run_and_get_output(prog).unwrap_err();
    assert_eq!(
        err.kind,
        crate::interpreter::RuntimeErrorKind::NotImplemented,
        "got {:?}: {}",
        err.kind,
        err.message
    );
    assert!(err.message.contains("многомерные"), "got {:?}", err.message);
}
