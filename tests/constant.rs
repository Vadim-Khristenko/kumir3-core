//! Тесты для модуля констант и встроенных функций

use kumir3_corelib::shared::constants::*;
use kumir3_corelib::shared::libraries::registry::is_known_library;

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

#[test]
fn test_ident_chars() {
    assert!(is_ident_start('a'));
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
    assert!(!is_unicode_combining_mark('a'));
}
