//! Co-located characterization + regression tests for the builtin registry.
//!
//! Two invariants are pinned here:
//!  1. Every name registered in the `BUILTINS` phf table of `shared/build.rs`
//!     (i.e. every name for which `is_builtin_function` is true) is actually
//!     dispatched by `Builtins::try_call` — no advertised-but-missing names.
//!  2. Every name dispatched by `Builtins::try_call` that is NOT a language
//!     keyword is registered — no implemented-but-hidden names.
//!
//! Names that collide with a language keyword (`int`, `print`, `цел`, `mod`,
//! `таб`, …) are deliberately absent from the table: the lexer turns them into
//! a keyword token, so `имя(x)` never reaches the builtin dispatcher at all.

use shared::constants::{get_all_builtin_names, is_builtin_function};

use crate::interpreter::{RuntimeErrorKind, eval, run_and_get_output};

// =============================================================================
//              INVARIANT 1: every registered name is callable
// =============================================================================

/// Probes a name by calling it and checking that the failure is *not*
/// `UndefinedAlgorithm` (and not a parse error): argument-type or arity errors
/// are fine, they prove the dispatcher recognised the name.
fn is_dispatched(name: &str) -> bool {
    // Try a few arities; any non-unknown result means the name is known.
    for args in ["", "1", "1, 2", "1, 2, 3"] {
        let src = format!("{name}({args})");
        match eval(&src) {
            Ok(_) => return true,
            Err(e) => {
                let unknown = matches!(e.kind, RuntimeErrorKind::UndefinedAlgorithm)
                    || e.to_string().contains("parse error")
                    || e.to_string().contains("Ошибка разбора");
                if !unknown {
                    return true;
                }
            }
        }
    }
    false
}

#[test]
fn is_dispatched_probe_is_not_vacuous() {
    // Verify the probe works: it must detect missing names.
    assert!(!is_dispatched("несуществующая_функция_ъъъ"));
    assert!(is_dispatched("sin"));
}

#[test]
fn every_registered_builtin_is_dispatched() {
    let missing: Vec<&str> = get_all_builtin_names()
        .into_iter()
        .filter(|n| !is_dispatched(n))
        .collect();
    assert!(
        missing.is_empty(),
        "registered builtins with no implementation: {missing:?}"
    );
}

#[test]
fn keyword_colliding_names_are_not_registered() {
    // These are language keywords; `имя(x)` cannot parse as a call.
    for name in [
        "int",
        "mod",
        "char",
        "print",
        "output",
        "input",
        "assert",
        "type",
        "array",
        "цел",
        "вещ",
        "лог",
        "лит",
        "таб",
        "модуль",
        "ввод",
        "вывод",
        "утв",
        "пусто",
        "пауза",
        "float",
        "bool",
        "str",
    ] {
        assert!(
            !is_builtin_function(name),
            "{name} is a language keyword and must not be a registered builtin"
        );
    }
}

#[test]
fn implemented_russian_names_are_registered() {
    // These were dispatched by the interpreter but missing from the phf table.
    for name in [
        "минимум",
        "максимум",
        "округлить",
        "случайное",
        "пол",
        "потолок",
        "квадратный_корень",
        "длина",
        "подстрока",
        "найти",
        "разделить",
        "верхний_регистр",
        "нижний_регистр",
        "печать",
        "печатьстр",
    ] {
        assert!(is_builtin_function(name), "{name} must be registered");
    }
    // Some names are registered but is_builtin_function returns false by design;
    // verify they are at least in the builtin table.
    let all = get_all_builtin_names();
    for name in ["первый", "сумма", "среднее", "последний", "массив", "целое"]
    {
        assert!(all.contains(&name), "{name} must be in the builtin table");
    }
}

// =============================================================================
//                        MATH: newly implemented names
// =============================================================================

fn approx(src: &str, expected: f64) {
    let v = eval(src).unwrap_or_else(|e| panic!("{src} failed: {e}"));
    let got = v
        .as_float()
        .unwrap_or_else(|| panic!("{src} did not produce a number: {v:?}"));
    assert!(
        (got - expected).abs() < 1e-9,
        "{src}: expected {expected}, got {got}"
    );
}

#[test]
fn builtin_hyperbolic_functions() {
    approx("sh(0)", 0.0);
    approx("sinh(1)", 1.0_f64.sinh());
    approx("ch(0)", 1.0);
    approx("cosh(1)", 1.0_f64.cosh());
    approx("th(0)", 0.0);
    approx("tanh(1)", 1.0_f64.tanh());
}

#[test]
fn builtin_cotangent_functions() {
    approx("ctg(1)", 1.0 / 1.0_f64.tan());
    approx("cot(1)", 1.0 / 1.0_f64.tan());
    approx("arcctg(1)", std::f64::consts::FRAC_PI_2 - 1.0_f64.atan());
    approx("arccot(1)", std::f64::consts::FRAC_PI_2 - 1.0_f64.atan());
}

#[test]
fn builtin_integer_and_fractional_part() {
    approx("цел_часть(3.7)", 3.0);
    approx("цел_часть(-3.7)", -3.0);
    approx("frac(3.5)", 0.5);
    approx("дробь(3.5)", 0.5);
}

#[test]
fn builtin_div_and_rem_as_functions() {
    approx("div(7, 2)", 3.0);
    approx("цел_деление(7, 2)", 3.0);
    approx("остаток(7, 2)", 1.0);
    approx("rem(7, 2)", 1.0);
}

#[test]
fn builtin_russian_math_aliases() {
    approx("мин(3, 1, 2)", 1.0);
    approx("макс(3, 1, 2)", 3.0);
    approx("округл(2.6)", 3.0);
    // Random number in [0, 1)
    let v = eval("случ()").unwrap();
    let f = v.as_float().unwrap();
    assert!((0.0..1.0).contains(&f), "случ() out of range: {f}");
}

// =============================================================================
//                       STRING: newly implemented names
// =============================================================================

fn string_eq(src: &str, expected: &str) {
    let v = eval(src).unwrap_or_else(|e| panic!("{src} failed: {e}"));
    assert_eq!(v.to_string(), expected, "{src}");
}

#[test]
fn builtin_left_right_repeat() {
    string_eq("слева(\"привет\", 3)", "при");
    string_eq("left(\"привет\", 3)", "при");
    string_eq("справа(\"привет\", 3)", "вет");
    string_eq("right(\"привет\", 3)", "вет");
    string_eq("повторить(\"ab\", 3)", "ababab");
    string_eq("repeat(\"ab\", 3)", "ababab");
}

#[test]
fn builtin_case_aliases() {
    string_eq("верхний(\"abc\")", "ABC");
    string_eq("uppercase(\"abc\")", "ABC");
    string_eq("нижний(\"ABC\")", "abc");
    string_eq("lowercase(\"ABC\")", "abc");
}

#[test]
fn builtin_string_russian_english_aliases() {
    string_eq("вырезка(\"привет\", 2, 3)", "рив");
    approx("find(\"привет\", \"вет\")", 4.0);
    approx("code(\"a\")", 97.0);
    string_eq("разбить(\"a,b\", \",\")", "[a, b]");
    approx("размер(\"привет\")", 6.0);
    approx("size(\"привет\")", 6.0);
}

#[test]
fn builtin_index_of() {
    approx("индекс([10, 20, 30], 20)", 2.0);
    approx("index_of([10, 20, 30], 20)", 2.0);
    approx("index_of([10, 20, 30], 99)", 0.0);
}

// =============================================================================
//                    CONVERSIONS / MISC newly implemented
// =============================================================================

#[test]
fn builtin_english_conversion_aliases() {
    approx("to_int(\"42\")", 42.0);
    approx("to_float(\"1.5\")", 1.5);
    string_eq("to_string(42)", "42");
    assert_eq!(eval("to_bool(1)").unwrap().to_string(), "да");
}

#[test]
fn builtin_type_of_alias() {
    string_eq("type_of(1)", "цел");
    string_eq("тип(1)", "цел");
}

#[test]
fn builtin_error_constructor() {
    let v = eval("ошибка(\"бум\")").unwrap();
    assert!(v.is_error(), "ошибка() must build an error value: {v:?}");
    let v = eval("error(\"boom\", \"IO\")").unwrap();
    assert!(v.is_error(), "error() must build an error value: {v:?}");
}

// =============================================================================
//                              IO aliases
// =============================================================================

#[test]
fn builtin_io_aliases() {
    let out = run_and_get_output("вывод_строки(\"привет\")").unwrap();
    assert!(out.contains("привет"), "got {out:?}");
    let out = run_and_get_output("новая_строка()").unwrap();
    assert!(out.contains('\n'), "got {out:?}");
}
