// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Тесты для системы пользовательских библиотек


#[test]
fn test_user_library_loader_creation() {
    let loader = UserLibraryLoader::new();
    // Просто проверяем, что создаётся без паники
    drop(loader);
}

#[test]
fn test_parse_simple_library() {
    let source = r#"
алг цел удвоить(цел x)
нач
    знач := x * 2
кон

алг цел утроить(цел x)
нач
    знач := x * 3
кон
"#;

    let result = parse(source);
    assert!(result.is_ok(), "Простая библиотека должна парситься");

    let program = result.unwrap();
    assert_eq!(program.algorithms.len(), 2, "Должно быть 2 алгоритма");
    assert_eq!(program.algorithms[0].name.as_ref(), "удвоить");
    assert_eq!(program.algorithms[1].name.as_ref(), "утроить");
}

#[test]
fn test_library_with_multiple_functions() {
    let source = r#"
алг цел сумма(цел a, цел b)
нач
    знач := a + b
кон

алг цел разность(цел a, цел b)
нач
    знач := a - b
кон

алг цел произведение(цел a, цел b)
нач
    знач := a * b
кон
"#;

    let result = parse(source);
    assert!(result.is_ok());

    let program = result.unwrap();
    assert_eq!(program.algorithms.len(), 3);
}

#[test]
fn test_library_with_different_types() {
    let source = r#"
алг цел целая_функция(цел x)
нач
    знач := x
кон

алг вещ вещественная_функция(вещ x)
нач
    знач := x
кон

алг лог логическая_функция(лог x)
нач
    знач := x
кон

алг лит строковая_функция(лит x)
нач
    знач := x
кон
"#;

    let result = parse(source);
    assert!(result.is_ok());

    let program = result.unwrap();
    assert_eq!(program.algorithms.len(), 4);
}

#[test]
fn test_library_with_recursion() {
    let source = r#"
алг цел факториал(цел n)
нач
    если n <= 1 то
        знач := 1
    иначе
        знач := n * факториал(n - 1)
    все
кон
"#;

    let result = parse(source);
    assert!(result.is_ok());
}

#[test]
fn test_library_with_loops() {
    let source = r#"
алг цел сумма_до_n(цел n)
нач
    цел сумма, i
    сумма := 0
    нц для i от 1 до n
        сумма := сумма + i
    кц
    знач := сумма
кон
"#;

    let result = parse(source);
    assert!(result.is_ok());
}

#[test]
fn test_example_library_structure() {
    let example = shared::libraries::create_example_library();
    let result = parse(&example);
    assert!(result.is_ok(), "Пример библиотеки должен парситься");

    let program = result.unwrap();
    assert!(
        program.algorithms.len() >= 2,
        "В примере должно быть минимум 2 функции"
    );
}

#[test]
fn test_library_cache() {
    let mut loader = UserLibraryLoader::new();

    // Создаём временный файл
    let temp_dir = std::env::temp_dir();
    let lib_path = temp_dir.join("test_cache_lib.kum");

    let source = r#"
алг цел тест()
нач
    знач := 42
кон
"#;

    std::fs::write(&lib_path, source).unwrap();

    // Первая загрузка
    let result1 = loader.load_from_file(&lib_path);
    assert!(result1.is_ok());

    // Вторая загрузка (из кэша)
    let result2 = loader.load_from_file(&lib_path);
    assert!(result2.is_ok());

    // Очистка
    let _ = std::fs::remove_file(&lib_path);
}

#[test]
fn test_clear_cache() {
    let mut loader = UserLibraryLoader::new();

    let temp_dir = std::env::temp_dir();
    let lib_path = temp_dir.join("test_clear_cache.kum");

    let source = "алг цел тест()\nнач\n    знач := 1\nкон";
    std::fs::write(&lib_path, source).unwrap();

    // Загружаем
    let _ = loader.load_from_file(&lib_path);

    // Очищаем кэш
    loader.clear_cache();

    // Загружаем снова (не из кэша)
    let result = loader.load_from_file(&lib_path);
    assert!(result.is_ok());

    // Очистка
    let _ = std::fs::remove_file(&lib_path);
}
