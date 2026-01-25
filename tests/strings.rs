// ============================================================================
//                    ТЕСТЫ МОДУЛЯ РАБОТЫ СО СТРОКАМИ
// ============================================================================

use kumir3_corelib::shared::strings::StringOperations;

// ============================================================================
//                         ТЕСТЫ КОДИРОВАНИЯ
// ============================================================================

#[test]
fn test_code_cp1251_ascii() {
    assert_eq!(StringOperations::code_cp1251('A'), 65);
    assert_eq!(StringOperations::code_cp1251('a'), 97);
    assert_eq!(StringOperations::code_cp1251('0'), 48);
    assert_eq!(StringOperations::code_cp1251(' '), 32);
}

#[test]
fn test_code_cp1251_cyrillic() {
    // Заглавные буквы А-Я (0xC0-0xDF в CP-1251)
    assert_eq!(StringOperations::code_cp1251('А'), 192);
    assert_eq!(StringOperations::code_cp1251('Б'), 193);
    assert_eq!(StringOperations::code_cp1251('Я'), 223);
    
    // Строчные буквы а-я (0xE0-0xFF в CP-1251)
    assert_eq!(StringOperations::code_cp1251('а'), 224);
    assert_eq!(StringOperations::code_cp1251('б'), 225);
    assert_eq!(StringOperations::code_cp1251('я'), 255);
}

#[test]
fn test_code_cp1251_not_representable() {
    // Символы, не представимые в CP-1251
    assert_eq!(StringOperations::code_cp1251('中'), -1);
    assert_eq!(StringOperations::code_cp1251('日'), -1);
    assert_eq!(StringOperations::code_cp1251('€'), 136); // Евро есть в CP-1251
}

#[test]
fn test_code_unicode() {
    assert_eq!(StringOperations::code_unicode('A'), 65);
    assert_eq!(StringOperations::code_unicode('А'), 0x0410);
    assert_eq!(StringOperations::code_unicode('Я'), 0x042F);
    assert_eq!(StringOperations::code_unicode('中'), 0x4E2D);
    assert_eq!(StringOperations::code_unicode('😀'), 0x1F600);
}

#[test]
fn test_char_from_cp1251_ascii() {
    assert_eq!(StringOperations::char_from_cp1251(65), Ok('A'));
    assert_eq!(StringOperations::char_from_cp1251(97), Ok('a'));
    assert_eq!(StringOperations::char_from_cp1251(48), Ok('0'));
}

#[test]
fn test_char_from_cp1251_cyrillic() {
    assert_eq!(StringOperations::char_from_cp1251(192), Ok('А'));
    assert_eq!(StringOperations::char_from_cp1251(223), Ok('Я'));
    assert_eq!(StringOperations::char_from_cp1251(224), Ok('а'));
    assert_eq!(StringOperations::char_from_cp1251(255), Ok('я'));
}

#[test]
fn test_char_from_cp1251_errors() {
    assert!(StringOperations::char_from_cp1251(-1).is_err());
    assert!(StringOperations::char_from_cp1251(256).is_err());
    assert!(StringOperations::char_from_cp1251(1000).is_err());
}

#[test]
fn test_char_from_unicode() {
    assert_eq!(StringOperations::char_from_unicode(65), Ok('A'));
    assert_eq!(StringOperations::char_from_unicode(0x0410), Ok('А'));
    assert_eq!(StringOperations::char_from_unicode(0x4E2D), Ok('中'));
    assert_eq!(StringOperations::char_from_unicode(0x1F600), Ok('😀'));
}

#[test]
fn test_char_from_unicode_errors() {
    assert!(StringOperations::char_from_unicode(-1).is_err());
    assert!(StringOperations::char_from_unicode(0x110000).is_err()); // Выше допустимого диапазона
}

// ============================================================================
//                         ТЕСТЫ ДЛИНЫ
// ============================================================================

#[test]
fn test_length_empty() {
    assert_eq!(StringOperations::length(""), 0);
}

#[test]
fn test_length_ascii() {
    assert_eq!(StringOperations::length("Hello"), 5);
    assert_eq!(StringOperations::length("Hello World"), 11);
}

#[test]
fn test_length_unicode() {
    assert_eq!(StringOperations::length("Привет"), 6);
    assert_eq!(StringOperations::length("中文"), 2);
    assert_eq!(StringOperations::length("😀😁😂"), 3);
}

#[test]
fn test_length_bytes() {
    // UTF-8: кириллица = 2 байта на символ
    assert_eq!(StringOperations::length_bytes("Привет"), 12);
    // ASCII = 1 байт на символ
    assert_eq!(StringOperations::length_bytes("Hello"), 5);
    // Китайские иероглифы = 3 байта на символ
    assert_eq!(StringOperations::length_bytes("中文"), 6);
}

// ============================================================================
//                         ТЕСТЫ РЕГИСТРА
// ============================================================================

#[test]
fn test_to_upper() {
    assert_eq!(StringOperations::to_upper("hello"), "HELLO");
    assert_eq!(StringOperations::to_upper("Hello World"), "HELLO WORLD");
    assert_eq!(StringOperations::to_upper("привет"), "ПРИВЕТ");
    assert_eq!(StringOperations::to_upper("Привет Мир"), "ПРИВЕТ МИР");
    assert_eq!(StringOperations::to_upper("123"), "123");
}

#[test]
fn test_to_lower() {
    assert_eq!(StringOperations::to_lower("HELLO"), "hello");
    assert_eq!(StringOperations::to_lower("Hello World"), "hello world");
    assert_eq!(StringOperations::to_lower("ПРИВЕТ"), "привет");
    assert_eq!(StringOperations::to_lower("ПРИВЕТ МИР"), "привет мир");
}

#[test]
fn test_capitalize() {
    assert_eq!(StringOperations::capitalize("hello"), "Hello");
    assert_eq!(StringOperations::capitalize("HELLO"), "Hello");
    assert_eq!(StringOperations::capitalize("привет"), "Привет");
    assert_eq!(StringOperations::capitalize(""), "");
}

#[test]
fn test_title_case() {
    assert_eq!(StringOperations::title_case("hello world"), "Hello World");
    assert_eq!(StringOperations::title_case("привет мир"), "Привет Мир");
    assert_eq!(StringOperations::title_case("  multiple   spaces  "), "  Multiple   Spaces  ");
}

#[test]
fn test_swap_case() {
    assert_eq!(StringOperations::swap_case("Hello"), "hELLO");
    assert_eq!(StringOperations::swap_case("Привет"), "пРИВЕТ");
    assert_eq!(StringOperations::swap_case("HeLLo WoRLD"), "hEllO wOrld");
}

// ============================================================================
//                         ТЕСТЫ ПОИСКА
// ============================================================================

#[test]
fn test_position_found() {
    assert_eq!(StringOperations::position("Hello World", "World"), 7);
    assert_eq!(StringOperations::position("Hello World", "Hello"), 1);
    assert_eq!(StringOperations::position("Hello World", "o"), 5);
    assert_eq!(StringOperations::position("Привет Мир", "Мир"), 8);
}

#[test]
fn test_position_not_found() {
    assert_eq!(StringOperations::position("Hello", "xyz"), 0);
    assert_eq!(StringOperations::position("", "x"), 0);
    assert_eq!(StringOperations::position("short", "longer string"), 0);
}

#[test]
fn test_position_empty_fragment() {
    assert_eq!(StringOperations::position("Hello", ""), 1);
    assert_eq!(StringOperations::position("", ""), 0);
}

#[test]
fn test_position_after() {
    assert_eq!(StringOperations::position_after(1, "abcabc", "bc"), 2);
    assert_eq!(StringOperations::position_after(3, "abcabc", "bc"), 5);
    assert_eq!(StringOperations::position_after(6, "abcabc", "bc"), 0);
    assert_eq!(StringOperations::position_after(1, "aaa", "a"), 1);
    assert_eq!(StringOperations::position_after(2, "aaa", "a"), 2);
}

#[test]
fn test_position_last() {
    assert_eq!(StringOperations::position_last("abcabc", "bc"), 5);
    assert_eq!(StringOperations::position_last("abcabc", "a"), 4);
    assert_eq!(StringOperations::position_last("abcabc", "xyz"), 0);
    assert_eq!(StringOperations::position_last("aaa", "a"), 3);
}

#[test]
fn test_contains() {
    assert!(StringOperations::contains("Hello World", "World"));
    assert!(StringOperations::contains("Hello World", ""));
    assert!(!StringOperations::contains("Hello", "xyz"));
}

#[test]
fn test_starts_with() {
    assert!(StringOperations::starts_with("Hello World", "Hello"));
    assert!(StringOperations::starts_with("Hello World", ""));
    assert!(!StringOperations::starts_with("Hello World", "World"));
}

#[test]
fn test_ends_with() {
    assert!(StringOperations::ends_with("Hello World", "World"));
    assert!(StringOperations::ends_with("Hello World", ""));
    assert!(!StringOperations::ends_with("Hello World", "Hello"));
}

#[test]
fn test_count_occurrences() {
    assert_eq!(StringOperations::count_occurrences("abcabcabc", "abc"), 3);
    assert_eq!(StringOperations::count_occurrences("aaa", "a"), 3);
    assert_eq!(StringOperations::count_occurrences("aaa", "aa"), 1); // Непересекающиеся
    assert_eq!(StringOperations::count_occurrences("hello", "x"), 0);
    assert_eq!(StringOperations::count_occurrences("hello", ""), 0);
}

// ============================================================================
//                         ТЕСТЫ ИЗВЛЕЧЕНИЯ
// ============================================================================

#[test]
fn test_substring_basic() {
    assert_eq!(StringOperations::substring("Hello", 1, 3), Ok("Hel".to_string()));
    assert_eq!(StringOperations::substring("Hello", 2, 3), Ok("ell".to_string()));
    assert_eq!(StringOperations::substring("Hello", 5, 1), Ok("o".to_string()));
}

#[test]
fn test_substring_unicode() {
    assert_eq!(StringOperations::substring("Привет", 1, 3), Ok("При".to_string()));
    assert_eq!(StringOperations::substring("Привет Мир", 8, 3), Ok("Мир".to_string()));
}

#[test]
fn test_substring_overflow() {
    // Запрос больше символов, чем есть — возвращает до конца строки
    assert_eq!(StringOperations::substring("Hello", 3, 100), Ok("llo".to_string()));
}

#[test]
fn test_substring_errors() {
    assert!(StringOperations::substring("Hello", 0, 3).is_err());  // pos < 1
    assert!(StringOperations::substring("Hello", 10, 3).is_err()); // pos > len
    assert!(StringOperations::substring("Hello", 1, -1).is_err()); // count < 0
}

#[test]
fn test_left() {
    assert_eq!(StringOperations::left("Hello", 3), Ok("Hel".to_string()));
    assert_eq!(StringOperations::left("Hello", 10), Ok("Hello".to_string()));
    assert_eq!(StringOperations::left("Hello", 0), Ok("".to_string()));
    assert_eq!(StringOperations::left("Привет", 3), Ok("При".to_string()));
}

#[test]
fn test_right() {
    assert_eq!(StringOperations::right("Hello", 3), Ok("llo".to_string()));
    assert_eq!(StringOperations::right("Hello", 10), Ok("Hello".to_string()));
    assert_eq!(StringOperations::right("Hello", 0), Ok("".to_string()));
    assert_eq!(StringOperations::right("Привет", 3), Ok("вет".to_string()));
}

#[test]
fn test_char_at() {
    assert_eq!(StringOperations::char_at("Hello", 1), Ok('H'));
    assert_eq!(StringOperations::char_at("Hello", 5), Ok('o'));
    assert_eq!(StringOperations::char_at("Привет", 1), Ok('П'));
    assert_eq!(StringOperations::char_at("Привет", 3), Ok('и'));
}

#[test]
fn test_char_at_errors() {
    assert!(StringOperations::char_at("Hello", 0).is_err());
    assert!(StringOperations::char_at("Hello", 6).is_err());
    assert!(StringOperations::char_at("", 1).is_err());
}

// ============================================================================
//                         ТЕСТЫ МОДИФИКАЦИИ
// ============================================================================

#[test]
fn test_insert_middle() {
    let mut s = "Hello".to_string();
    StringOperations::insert("123", &mut s, 3).unwrap();
    assert_eq!(s, "He123llo");
}

#[test]
fn test_insert_start() {
    let mut s = "World".to_string();
    StringOperations::insert("Hello ", &mut s, 1).unwrap();
    assert_eq!(s, "Hello World");
}

#[test]
fn test_insert_end() {
    let mut s = "Hello".to_string();
    StringOperations::insert(" World", &mut s, 6).unwrap();
    assert_eq!(s, "Hello World");
}

#[test]
fn test_insert_unicode() {
    let mut s = "Привет".to_string();
    StringOperations::insert("123", &mut s, 4).unwrap();
    assert_eq!(s, "При123вет");
}

#[test]
fn test_insert_errors() {
    let mut s = "Hello".to_string();
    assert!(StringOperations::insert("X", &mut s, 0).is_err());
    assert!(StringOperations::insert("X", &mut s, 10).is_err());
}

#[test]
fn test_delete_middle() {
    let mut s = "Hello World".to_string();
    StringOperations::delete(&mut s, 6, 6).unwrap();
    assert_eq!(s, "Hello");
}

#[test]
fn test_delete_start() {
    let mut s = "Hello World".to_string();
    StringOperations::delete(&mut s, 1, 6).unwrap();
    assert_eq!(s, "World");
}

#[test]
fn test_delete_unicode() {
    let mut s = "Привет Мир".to_string();
    StringOperations::delete(&mut s, 7, 4).unwrap();
    assert_eq!(s, "Привет");
}

#[test]
fn test_delete_overflow() {
    let mut s = "Hello".to_string();
    StringOperations::delete(&mut s, 3, 100).unwrap();
    assert_eq!(s, "He");
}

#[test]
fn test_delete_errors() {
    let mut s = "Hello".to_string();
    assert!(StringOperations::delete(&mut s, 0, 1).is_err());
    assert!(StringOperations::delete(&mut s, 10, 1).is_err());
    assert!(StringOperations::delete(&mut s, 1, -1).is_err());
}

#[test]
fn test_replace_all() {
    let mut s = "abcabc".to_string();
    StringOperations::replace(&mut s, "bc", "XY", true).unwrap();
    assert_eq!(s, "aXYaXY");
}

#[test]
fn test_replace_first() {
    let mut s = "abcabc".to_string();
    StringOperations::replace(&mut s, "bc", "XY", false).unwrap();
    assert_eq!(s, "aXYabc");
}

#[test]
fn test_replace_unicode() {
    let mut s = "Привет Мир Мир".to_string();
    StringOperations::replace(&mut s, "Мир", "Свет", true).unwrap();
    assert_eq!(s, "Привет Свет Свет");
}

#[test]
fn test_replace_empty_old_error() {
    let mut s = "Hello".to_string();
    assert!(StringOperations::replace(&mut s, "", "X", true).is_err());
}

// ============================================================================
//                         ТЕСТЫ ОБРЕЗКИ
// ============================================================================

#[test]
fn test_trim() {
    assert_eq!(StringOperations::trim("  Hello  "), "Hello");
    assert_eq!(StringOperations::trim("\t\nHello\t\n"), "Hello");
    assert_eq!(StringOperations::trim("Hello"), "Hello");
    assert_eq!(StringOperations::trim("   "), "");
}

#[test]
fn test_trim_left() {
    assert_eq!(StringOperations::trim_left("  Hello  "), "Hello  ");
    assert_eq!(StringOperations::trim_left("Hello"), "Hello");
}

#[test]
fn test_trim_right() {
    assert_eq!(StringOperations::trim_right("  Hello  "), "  Hello");
    assert_eq!(StringOperations::trim_right("Hello"), "Hello");
}

#[test]
fn test_trim_chars() {
    assert_eq!(StringOperations::trim_chars("xxHelloxx", "x"), "Hello");
    assert_eq!(StringOperations::trim_chars("##Hello##", "#"), "Hello");
    assert_eq!(StringOperations::trim_chars("abcHelloabc", "abc"), "Hello");
}

// ============================================================================
//                         ТЕСТЫ РАЗБИЕНИЯ И ОБЪЕДИНЕНИЯ
// ============================================================================

#[test]
fn test_split_by_delimiter() {
    assert_eq!(StringOperations::split("a,b,c", ","), vec!["a", "b", "c"]);
    assert_eq!(StringOperations::split("a::b::c", "::"), vec!["a", "b", "c"]);
}

#[test]
fn test_split_empty_delimiter() {
    // Разбить на символы
    assert_eq!(StringOperations::split("abc", ""), vec!["a", "b", "c"]);
    assert_eq!(StringOperations::split("Привет", ""), vec!["П", "р", "и", "в", "е", "т"]);
}

#[test]
fn test_split_words() {
    assert_eq!(StringOperations::split_words("Hello World"), vec!["Hello", "World"]);
    assert_eq!(StringOperations::split_words("  multiple   spaces  "), vec!["multiple", "spaces"]);
}

#[test]
fn test_split_lines() {
    assert_eq!(StringOperations::split_lines("a\nb\nc"), vec!["a", "b", "c"]);
    assert_eq!(StringOperations::split_lines("a\r\nb\r\nc"), vec!["a", "b", "c"]);
}

#[test]
fn test_join() {
    assert_eq!(StringOperations::join(&["a", "b", "c"], ","), "a,b,c");
    assert_eq!(StringOperations::join(&["a", "b", "c"], ""), "abc");
    assert_eq!(StringOperations::join(&["a"], ","), "a");
    assert_eq!(StringOperations::join(&[], ","), "");
}

// ============================================================================
//                         ТЕСТЫ ПОВТОРЕНИЯ И ЗАПОЛНЕНИЯ
// ============================================================================

#[test]
fn test_repeat() {
    assert_eq!(StringOperations::repeat("ab", 3), Ok("ababab".to_string()));
    assert_eq!(StringOperations::repeat("ab", 0), Ok("".to_string()));
    assert_eq!(StringOperations::repeat("", 5), Ok("".to_string()));
}

#[test]
fn test_repeat_error() {
    assert!(StringOperations::repeat("ab", -1).is_err());
}

#[test]
fn test_pad_left() {
    assert_eq!(StringOperations::pad_left("42", 5, '0'), "00042");
    assert_eq!(StringOperations::pad_left("Hello", 3, ' '), "Hello"); // Уже длиннее
    assert_eq!(StringOperations::pad_left("", 3, 'X'), "XXX");
}

#[test]
fn test_pad_right() {
    assert_eq!(StringOperations::pad_right("42", 5, '0'), "42000");
    assert_eq!(StringOperations::pad_right("Hello", 3, ' '), "Hello");
}

#[test]
fn test_center() {
    assert_eq!(StringOperations::center("ab", 6, '-'), "--ab--");
    assert_eq!(StringOperations::center("abc", 6, '-'), "-abc--"); // Нечётное — больше справа
    assert_eq!(StringOperations::center("Hello", 3, '-'), "Hello");
}

// ============================================================================
//                         ТЕСТЫ РЕВЕРСА
// ============================================================================

#[test]
fn test_reverse() {
    assert_eq!(StringOperations::reverse("Hello"), "olleH");
    assert_eq!(StringOperations::reverse("Привет"), "тевирП");
    assert_eq!(StringOperations::reverse(""), "");
    assert_eq!(StringOperations::reverse("a"), "a");
}

// ============================================================================
//                         ТЕСТЫ ПРОВЕРОК
// ============================================================================

#[test]
fn test_is_empty() {
    assert!(StringOperations::is_empty(""));
    assert!(!StringOperations::is_empty(" "));
    assert!(!StringOperations::is_empty("x"));
}

#[test]
fn test_is_whitespace() {
    assert!(StringOperations::is_whitespace("   "));
    assert!(StringOperations::is_whitespace("\t\n\r"));
    assert!(!StringOperations::is_whitespace(""));
    assert!(!StringOperations::is_whitespace(" x "));
}

#[test]
fn test_is_digits() {
    assert!(StringOperations::is_digits("12345"));
    assert!(StringOperations::is_digits("0"));
    assert!(!StringOperations::is_digits(""));
    assert!(!StringOperations::is_digits("123a"));
    assert!(!StringOperations::is_digits("-123"));
}

#[test]
fn test_is_alpha() {
    assert!(StringOperations::is_alpha("Hello"));
    assert!(StringOperations::is_alpha("Привет"));
    assert!(!StringOperations::is_alpha(""));
    assert!(!StringOperations::is_alpha("Hello1"));
    assert!(!StringOperations::is_alpha("Hello World"));
}

#[test]
fn test_is_alphanumeric() {
    assert!(StringOperations::is_alphanumeric("Hello123"));
    assert!(StringOperations::is_alphanumeric("Привет123"));
    assert!(!StringOperations::is_alphanumeric(""));
    assert!(!StringOperations::is_alphanumeric("Hello 123"));
}

#[test]
fn test_is_uppercase() {
    assert!(StringOperations::is_uppercase("HELLO"));
    assert!(StringOperations::is_uppercase("HELLO123"));
    assert!(StringOperations::is_uppercase("123")); // Нет букв — true
    assert!(!StringOperations::is_uppercase(""));
    assert!(!StringOperations::is_uppercase("Hello"));
}

#[test]
fn test_is_lowercase() {
    assert!(StringOperations::is_lowercase("hello"));
    assert!(StringOperations::is_lowercase("hello123"));
    assert!(StringOperations::is_lowercase("123"));
    assert!(!StringOperations::is_lowercase(""));
    assert!(!StringOperations::is_lowercase("Hello"));
}

#[test]
fn test_is_numeric() {
    assert!(StringOperations::is_numeric("123"));
    assert!(StringOperations::is_numeric("123.45"));
    assert!(StringOperations::is_numeric("-123"));
    assert!(StringOperations::is_numeric("1e10"));
    assert!(!StringOperations::is_numeric(""));
    assert!(!StringOperations::is_numeric("abc"));
    assert!(!StringOperations::is_numeric("12abc"));
}

// ============================================================================
//                         ТЕСТЫ ПРЕОБРАЗОВАНИЯ
// ============================================================================

#[test]
fn test_to_number() {
    assert_eq!(StringOperations::to_number("123"), Ok(123.0));
    assert_eq!(StringOperations::to_number("123.45"), Ok(123.45));
    assert_eq!(StringOperations::to_number("-123"), Ok(-123.0));
    assert_eq!(StringOperations::to_number("  42  "), Ok(42.0));
}

#[test]
fn test_to_number_error() {
    assert!(StringOperations::to_number("abc").is_err());
    assert!(StringOperations::to_number("").is_err());
}

#[test]
fn test_to_integer() {
    assert_eq!(StringOperations::to_integer("123"), Ok(123));
    assert_eq!(StringOperations::to_integer("-456"), Ok(-456));
    assert_eq!(StringOperations::to_integer("  789  "), Ok(789));
}

#[test]
fn test_to_integer_error() {
    assert!(StringOperations::to_integer("12.5").is_err());
    assert!(StringOperations::to_integer("abc").is_err());
}

// ============================================================================
//                         ТЕСТЫ СРАВНЕНИЯ
// ============================================================================

#[test]
fn test_compare() {
    assert_eq!(StringOperations::compare("abc", "abd"), -1);
    assert_eq!(StringOperations::compare("abc", "abc"), 0);
    assert_eq!(StringOperations::compare("abd", "abc"), 1);
    assert_eq!(StringOperations::compare("", "a"), -1);
    assert_eq!(StringOperations::compare("a", ""), 1);
}

#[test]
fn test_compare_ignore_case() {
    assert_eq!(StringOperations::compare_ignore_case("ABC", "abc"), 0);
    assert_eq!(StringOperations::compare_ignore_case("ABC", "abd"), -1);
}

#[test]
fn test_equals_ignore_case() {
    assert!(StringOperations::equals_ignore_case("Hello", "HELLO"));
    assert!(StringOperations::equals_ignore_case("Привет", "ПРИВЕТ"));
    assert!(!StringOperations::equals_ignore_case("Hello", "World"));
}

// ============================================================================
//                         ТЕСТЫ ФОРМАТИРОВАНИЯ
// ============================================================================

#[test]
fn test_format() {
    use kumir3_corelib::shared::types::Value;
    
    let result = StringOperations::format("Hello, {}!", &[Value::from("World")]);
    assert_eq!(result, "Hello, World!");
    
    let result = StringOperations::format("{} + {} = {}", &[
        Value::from(2i64),
        Value::from(3i64),
        Value::from(5i64),
    ]);
    assert_eq!(result, "2 + 3 = 5");
    
    // Больше placeholders, чем аргументов
    let result = StringOperations::format("{} {} {}", &[Value::from("a")]);
    assert_eq!(result, "a {} {}");
}
