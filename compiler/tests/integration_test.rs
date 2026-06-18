// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Интеграционные тесты компилятора

use kumir3_compiler::Compiler;
use std::process::Command;

#[test]
#[ignore] // Игнорируем по умолчанию, так как требует rustc
fn test_compile_and_run_simple_program() {
    let mut compiler = Compiler::new();

    let source = r#"
алг Главный
нач
    вывод 42
кон
"#;

    let temp_dir = std::env::temp_dir();
    let output = temp_dir.join("test_kumir_program");

    // Компилируем в исполняемый файл
    let result = compiler.compile_to_exe(source, &output);
    assert!(
        result.is_ok(),
        "Компиляция должна пройти успешно: {:?}",
        result
    );

    // Проверяем что файл создан
    #[cfg(windows)]
    let exe_path = output.with_extension("exe");
    #[cfg(not(windows))]
    let exe_path = output;

    assert!(exe_path.exists(), "Исполняемый файл должен быть создан");

    // Запускаем программу
    let output = Command::new(&exe_path)
        .output()
        .expect("Не удалось запустить программу");

    // Проверяем вывод
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("42"),
        "Программа должна вывести 42, получено: {}",
        stdout
    );

    // Удаляем временный файл
    let _ = std::fs::remove_file(&exe_path);
}

#[test]
fn test_compile_to_rust_code() {
    let mut compiler = Compiler::new();

    let source = r#"
алг Тест
нач
    цел x
    x := 10
    вывод x
кон
"#;

    let temp_dir = std::env::temp_dir();
    let output = temp_dir.join("test_kumir.rs");

    let result = compiler.compile_to_rust(source, &output);
    assert!(result.is_ok(), "Генерация Rust кода должна пройти успешно");

    // Проверяем что файл создан
    assert!(output.exists(), "Rust файл должен быть создан");

    // Читаем и проверяем содержимое
    let rust_code = std::fs::read_to_string(&output).unwrap();
    assert!(rust_code.contains("fn Тест("), "Должна быть функция Тест");
    assert!(rust_code.contains("let mut v"), "Должны быть переменные");

    // Удаляем временный файл
    let _ = std::fs::remove_file(&output);
}

#[test]
fn test_compile_to_ir() {
    let mut compiler = Compiler::new();

    let source = r#"
алг Сумма
нач
    цел a, b, c
    a := 5
    b := 10
    c := a + b
    вывод c
кон
"#;

    let temp_dir = std::env::temp_dir();
    let output = temp_dir.join("test_kumir.ir");

    let result = compiler.compile_to_ir(source, &output);
    assert!(result.is_ok(), "Генерация IR должна пройти успешно");

    // Проверяем что файл создан
    assert!(output.exists(), "IR файл должен быть создан");

    // Читаем и проверяем содержимое
    let ir_code = std::fs::read_to_string(&output).unwrap();
    assert!(ir_code.contains("IrModule"), "Должен быть IrModule");
    assert!(ir_code.contains("functions"), "Должны быть функции");

    // Удаляем временный файл
    let _ = std::fs::remove_file(&output);
}
