// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Тесты компилятора Kumir 3

use kumir3_compiler::Compiler;

#[test]
fn test_compiler_creation() {
    let _compiler = Compiler::new();
    // Компилятор создан успешно
}

#[test]
fn test_syntax_check_valid() {
    let compiler = Compiler::new();
    let source = r#"
алг Главный
нач
    вывод 42
кон
"#;

    let result = compiler.check(source);
    assert!(result.is_ok(), "Синтаксис должен быть корректным");
}

#[test]
fn test_syntax_check_invalid() {
    let compiler = Compiler::new();
    let source = "это не валидный код";

    let result = compiler.check(source);
    assert!(result.is_err(), "Должна быть ошибка парсинга");
}

#[test]
fn test_ast_to_ir_simple() {
    let compiler = Compiler::new();
    let source = r#"
алг Тест
нач
    цел x
    x := 5
кон
"#;

    // Парсим и конвертируем в IR
    let result = compiler.check(source);
    assert!(result.is_ok(), "Простая программа должна парситься");
}

#[test]
fn test_compiler_with_debug() {
    let mut compiler = Compiler::new();
    compiler.set_debug(true);
    compiler.set_opt_level(2);

    // Проверяем что настройки применились
    // (нет публичных геттеров, но можем проверить что не паникует)
    let source = "алг Тест\nнач\nкон";
    let _ = compiler.check(source);
}

#[test]
fn test_ast_to_ir_for_loop() {
    let compiler = Compiler::new();
    let source = r#"
алг ЦиклДля
нач
    цел сумма
    сумма := 0
    нц для i от 1 до 10
        сумма := сумма + i
    кц
кон
"#;

    let result = compiler.check(source);
    assert!(result.is_ok(), "Программа с циклом для должна парситься");
}

#[test]
fn test_ast_to_ir_input_output() {
    let compiler = Compiler::new();
    let source = r#"
алг ВводВывод
нач
    цел x, y
    ввод x, y
    вывод x + y
кон
"#;

    let result = compiler.check(source);
    assert!(
        result.is_ok(),
        "Программа с вводом-выводом должна парситься"
    );
}

#[test]
fn test_ast_to_ir_nested_conditions() {
    let compiler = Compiler::new();
    let source = r#"
алг ВложенныеУсловия
нач
    цел x
    x := 10
    если x > 5 то
        если x > 8 то
            вывод "большое"
        иначе
            вывод "среднее"
        все
    иначе
        вывод "маленькое"
    все
кон
"#;

    let result = compiler.check(source);
    if let Err(e) = &result {
        eprintln!("Ошибка парсинга: {}", e);
    }
    assert!(
        result.is_ok(),
        "Программа с вложенными условиями должна парситься"
    );
}

#[test]
fn test_ast_to_ir_arithmetic() {
    let compiler = Compiler::new();
    let source = r#"
алг Арифметика
нач
    цел a, b, c
    a := 10
    b := 20
    c := (a + b) * 2 - 5
    вывод c
кон
"#;

    let result = compiler.check(source);
    assert!(result.is_ok(), "Программа с арифметикой должна парситься");
}
