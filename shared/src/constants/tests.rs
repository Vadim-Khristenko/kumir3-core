//! Tests for constants and built-in functions.

use crate::constants::*;
use crate::libraries::registry::is_known_library;
use rstest::rstest;

#[test]
fn test_keywords() {
    // Standard Kumir keywords
    assert!(is_keyword("алг"));
    assert!(is_keyword("если"));
    assert!(is_keyword("подключить"));
    assert!(!is_keyword("unknown"));
}

#[test]
fn test_builtin_constants() {
    // Built-in constants are available in Russian and other forms
    assert!(is_builtin_constant("ПИ"));
    assert!(is_builtin_constant("pi"));
    assert!(!is_builtin_constant("xyz"));

    let pi = get_builtin_constant("ПИ").unwrap();
    assert!((pi - std::f64::consts::PI).abs() < 1e-10);
}

#[test]
fn test_builtin_functions() {
    // Built-in functions in English and Russian
    assert!(is_builtin_function("sin"));
    assert!(is_builtin_function("корень"));
    assert!(is_builtin_function("длин"));
    assert!(!is_builtin_function("unknown"));
}
#[rstest]
#[case('a')]
#[case('я')]
#[case('_')]
#[should_panic]
#[case('1')]
fn test_ident_chars_start(#[case] c: char) {
    // Only letters and underscore can start an identifier
    assert!(is_ident_start(c))
}
#[rstest]
#[case('a')]
#[case('я')]
#[case('_')]
#[case('1')]
#[case('\u{0301}')]
#[case('\u{0308}')]
fn test_ident_chars_continue(#[case] c: char) {
    // Identifier continuation allows letters, digits, underscore, and combining marks
    assert!(is_ident_continue(c))
}
#[rstest]
#[case('\u{0301}')]
#[should_panic]
#[case('a')]
fn test_is_unicode_combining_mark(#[case] c: char) {
    // Only combining marks, not regular letters
    assert!(is_unicode_combining_mark(c))
}
