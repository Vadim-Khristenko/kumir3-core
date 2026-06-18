//! Тесты для модуля констант и встроенных функций

use rstest::rstest;
use shared::constants::*;
use shared::libraries::registry::is_known_library;

#[test]
fn test_keywords() {
    assert!(is_keyword("алг"));
    assert!(is_keyword("если"));
    assert!(is_keyword("подключить"));
    assert!(!is_keyword("неизвестное"));
}

#[test]
fn test_builtin_constants() {
    assert!(is_builtin_constant("ПИ"));
    assert!(is_builtin_constant("pi"));
    assert!(!is_builtin_constant("xyz"));

    let pi = get_builtin_constant("ПИ").unwrap();
    assert!((pi - std::f64::consts::PI).abs() < 1e-10);
}

#[test]
fn test_builtin_functions() {
    assert!(is_builtin_function("sin"));
    assert!(is_builtin_function("корень"));
    assert!(is_builtin_function("длин"));
    assert!(!is_builtin_function("неизвестная"));
}

/*  assert!(is_ident_start());
assert!(is_ident_start('я'));
assert!(is_ident_start('_'));
assert!(!is_ident_start('1'));

assert!(is_ident_continue('a'));
assert!(is_ident_continue('1'));
assert!(is_ident_continue('_'));
// Unicode combining marks
assert!(is_ident_continue('\u{0301}')); // combining acute accent
assert!(is_ident_continue('\u{0308}')); // combining diaeresis
assert!(is_unicode_combining_mark('\u{0301}'));
assert!(!is_unicode_combining_mark('a')); */
#[rstest]
#[case('a')]
#[case('я')]
#[case('_')]
#[should_panic]
#[case('1')]
fn test_ident_chars_start(#[case] c: char) {
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
    assert!(is_ident_continue(c))
}
#[rstest]
#[case('\u{0301}')]
#[should_panic]
#[case('a')]
fn test_is_unicode_combining_mark(#[case] c: char) {
    assert!(is_unicode_combining_mark(c))
}
