// =============================================================================
//                       МОДУЛЬ: ТЕСТЫ ИНТЕРПРЕТАТОРА
// =============================================================================
// Интеграционные тесты публичного API `Interpreter`: базовые выражения,
// управляющие конструкции, алгоритмы, ООП (KITE-0011), диапазоны (KITE-0002),
// модули (KITE-0015), лямбды и композиция (KITE-0013), строгий режим [W0].
use super::*;
use shared::types::Value;

#[test]
fn test_simple_output() {
    let source = r#"
алг Тест
нач
    вывод 42
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("42"));
}

#[test]
fn test_arithmetic() {
    let mut interpreter = Interpreter::new();

    assert_eq!(
        interpreter.eval("2 + 3").unwrap(),
        Value::Number(shared::types::Number::I64(5))
    );
    assert_eq!(
        interpreter.eval("10 - 4").unwrap(),
        Value::Number(shared::types::Number::I64(6))
    );
    assert_eq!(
        interpreter.eval("3 * 4").unwrap(),
        Value::Number(shared::types::Number::I64(12))
    );

    // Деление возвращает F128 (вещественное деление)
    let div_result = interpreter.eval("15 / 3").unwrap();
    match div_result {
        Value::Number(n) => {
            let f_val = match n {
                shared::types::Number::F128(f) => f.to_f64(),
                shared::types::Number::F64(f) => f,
                shared::types::Number::I64(i) => i as f64,
                _ => panic!("Unexpected number type"),
            };
            assert!((f_val - 5.0).abs() < 0.0001, "Expected ~5.0, got {}", f_val);
        }
        _ => panic!("Expected Number"),
    }

    // Возведение в степень также может вернуть F128
    let pow_result = interpreter.eval("2 ** 3").unwrap();
    match pow_result {
        Value::Number(n) => {
            let f_val = match n {
                shared::types::Number::F128(f) => f.to_f64(),
                shared::types::Number::F64(f) => f,
                shared::types::Number::I64(i) => i as f64,
                _ => panic!("Unexpected number type"),
            };
            assert!((f_val - 8.0).abs() < 0.0001, "Expected ~8.0, got {}", f_val);
        }
        _ => panic!("Expected Number"),
    }
}

#[test]
fn test_comparison() {
    let mut interpreter = Interpreter::new();

    assert_eq!(interpreter.eval("5 > 3").unwrap(), Value::Boolean(true));
    assert_eq!(interpreter.eval("5 < 3").unwrap(), Value::Boolean(false));
    assert_eq!(interpreter.eval("5 = 5").unwrap(), Value::Boolean(true));
    assert_eq!(interpreter.eval("5 <> 3").unwrap(), Value::Boolean(true));
}

#[test]
fn test_logical() {
    let mut interpreter = Interpreter::new();

    assert_eq!(interpreter.eval("да и да").unwrap(), Value::Boolean(true));
    assert_eq!(interpreter.eval("да и нет").unwrap(), Value::Boolean(false));
    assert_eq!(
        interpreter.eval("да или нет").unwrap(),
        Value::Boolean(true)
    );
    assert_eq!(interpreter.eval("не да").unwrap(), Value::Boolean(false));
}

#[test]
fn test_variables() {
    let source = r#"
алг Тест
нач
    цел x := 10
    цел y := 20
    вывод x + y
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("30"));
}

#[test]
fn test_if_statement() {
    let source = r#"
алг Тест
нач
    цел x := 5
    если x > 0 то
        вывод "положительное"
    иначе
        вывод "неположительное"
    все
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("положительное"));
}

#[test]
fn test_for_loop() {
    let source = r#"
алг Тест
нач
    цел сумма := 0
    нц для i от 1 до 5
        сумма := сумма + i
    кц
    вывод сумма
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("15"));
}

#[test]
fn test_while_loop() {
    let source = r#"
алг Тест
нач
    цел n := 5
    цел факт := 1
    нц пока n > 0
        факт := факт * n
        n := n - 1
    кц
    вывод факт
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("120"));
}

#[test]
fn test_algorithm_call() {
    let source = r#"
алг цел Квадрат(арг цел x)
нач
    знач := x * x
кон

алг Тест
нач
    вывод Квадрат(5)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("25"));
}

#[test]
fn test_recursion() {
    let source = r#"
алг цел Фиб(арг цел n)
нач
    если n <= 1 то
        знач := n
    иначе
        знач := Фиб(n - 1) + Фиб(n - 2)
    все
кон

алг Тест
нач
    вывод Фиб(10)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("55"));
}

#[test]
fn test_inheritance_method_dispatch() {
    // [KITE 11] Унаследованный метод должен находиться по иерархии (Кот → Животное).
    let source = r#"
класс Животное
алг лит звук()
нач
    знач := "животное"
кон
кон

класс Кот расширяет Животное
конструктор()
нач
кон
кон

алг Тест
нач
    к := новый Кот()
    вывод к.звук()
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("животное"));
}

#[test]
fn test_method_override_polymorphism() {
    // [KITE 11] Переопределённый метод подкласса должен выигрывать у родителя
    // — это работает только при корректной идентичности объекта (type_id).
    let source = r#"
класс Животное
алг лит звук()
нач
    знач := "животное"
кон
кон

класс Собака расширяет Животное
конструктор()
нач
кон
алг лит звук()
нач
    знач := "гав"
кон
кон

алг Тест
нач
    с := новый Собака()
    вывод с.звук()
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    let out = interpreter.get_output();
    assert!(
        out.contains("гав"),
        "ожидали переопределённый метод, вывод: {}",
        out
    );
}

#[test]
fn test_super_method_call() {
    // [KITE 11] предок.метод() вызывает реализацию родителя, обходя переопределение.
    let source = r#"
класс Животное
алг лит звук()
нач
    знач := "животное"
кон
кон

класс Собака расширяет Животное
конструктор()
нач
кон
алг лит звук()
нач
    знач := предок.звук() + "-гав"
кон
кон

алг Тест
нач
    с := новый Собака()
    вывод с.звук()
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    let out = interpreter.get_output();
    assert!(out.contains("животное-гав"), "вывод: {}", out);
}

#[test]
fn test_abstract_cannot_instantiate() {
    // [KITE 11] Создание экземпляра абстрактного класса запрещено.
    let source = r#"
абстрактный класс Фигура
конструктор()
нач
кон
кон

алг Тест
нач
    ф := новый Фигура()
кон
"#;
    let mut interpreter = Interpreter::new();
    assert!(
        interpreter.run(source).is_err(),
        "абстрактный класс не должен создаваться"
    );
}

#[test]
fn test_final_method_cannot_be_overridden() {
    // [KITE 11] Переопределение `финал`-метода запрещено.
    let source = r#"
класс Основа
финал алг лит метка()
нач
    знач := "основа"
кон
кон

класс Потомок расширяет Основа
алг лит метка()
нач
    знач := "потомок"
кон
кон

алг Тест
нач
кон
"#;
    let mut interpreter = Interpreter::new();
    let err = interpreter.run(source).unwrap_err();
    assert!(
        err.message.contains("переопредел"),
        "сообщение: {}",
        err.message
    );
}

#[test]
fn test_abstract_method_must_be_implemented() {
    // [KITE 11] Неабстрактный класс обязан реализовать абстрактный метод предка.
    let source = r#"
абстрактный класс Фигура
абстрактный алг вещ площадь()
кон

класс Круг расширяет Фигура
конструктор()
нач
кон
кон

алг Тест
нач
кон
"#;
    let mut interpreter = Interpreter::new();
    let err = interpreter.run(source).unwrap_err();
    assert!(
        err.message.contains("реализовать"),
        "сообщение: {}",
        err.message
    );
}

#[test]
fn test_impl_method_dispatch() {
    // [KITE 11, шаг 4] Метод из impl-блока (`реализация Тип`) должен диспетчеризоваться.
    let source = r#"
класс Точка
конструктор()
нач
кон
кон

реализация Точка
алг лит показать()
нач
    знач := "точка!"
кон
кон

алг Тест
нач
    т := новый Точка()
    вывод т.показать()
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    let out = interpreter.get_output();
    assert!(out.contains("точка!"), "вывод: {}", out);
}

#[test]
fn test_private_field_access_denied() {
    // [KITE 11, шаг 5] Доступ к закрытому полю извне класса запрещён.
    let source = r#"
класс Счёт
закрытый:
цел баланс
открытый:
конструктор()
нач
кон
кон

алг Тест
нач
    с := новый Счёт()
    вывод с.баланс
кон
"#;
    let mut interpreter = Interpreter::new();
    let err = interpreter.run(source).unwrap_err();
    assert!(err.message.contains("закрыт"), "сообщение: {}", err.message);
}

#[test]
fn test_public_field_access_allowed() {
    // [KITE 11, шаг 5] Открытое поле доступно извне.
    let source = r#"
класс Точка2
открытый:
цел x
конструктор()
нач
кон
кон

алг Тест
нач
    т := новый Точка2()
    вывод т.x
кон
"#;
    let mut interpreter = Interpreter::new();
    assert!(
        interpreter.run(source).is_ok(),
        "открытое поле должно быть доступно"
    );
}

#[test]
fn test_kumir_toml_library_dir() {
    // [KITE 5] Библиотека-проект (директория с kumir.toml): её функции вызываемы.
    use std::fs;
    let dir = std::env::temp_dir().join(format!("kumir_lib_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("kumir.toml"),
        "[package]\nname = \"л\"\nmain = \"src/lib.kum\"\n",
    )
    .unwrap();
    fs::write(
        dir.join("src").join("lib.kum"),
        "алг цел дв(цел x)\nнач\n  знач := x * 2\nкон\n",
    )
    .unwrap();

    let parent = dir.parent().unwrap().to_path_buf();
    let libname = dir.file_name().unwrap().to_string_lossy().to_string();

    let mut interp = Interpreter::new();
    interp.set_base_dir(&parent);
    let src = format!(
        "использовать \"{}\"\nалг Тест\nнач\n    вывод дв(21)\nкон\n",
        libname
    );
    interp.run(&src).unwrap();
    assert!(
        interp.get_output().contains("42"),
        "вывод: {}",
        interp.get_output()
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn test_bytes_type() {
    // [KITE 2] Тип байты: создание из строки, длина, обратное преобразование.
    let source = r#"
алг Тест
нач
    б := байты("AB")
    вывод длина(б)
    вывод строка_из_байт(б)
кон
"#;
    let mut interp = Interpreter::new();
    interp.run(source).unwrap();
    let out = interp.get_output();
    assert!(out.contains("2"), "длина байтов: {}", out);
    assert!(out.contains("AB"), "обратное преобразование: {}", out);
}

#[test]
fn test_range_value_display() {
    // [KITE 2] Диапазон как значение печатается как 1..10 / 1..=10.
    let mut interpreter = Interpreter::new();
    interpreter
        .run("алг Тест\nнач\n    д := 1..10\n    вывод д\nкон\n")
        .unwrap();
    assert!(
        interpreter.get_output().contains("1..10"),
        "вывод: {}",
        interpreter.get_output()
    );

    let mut i2 = Interpreter::new();
    i2.run("алг Тест\nнач\n    д := 1..=10\n    вывод д\nкон\n")
        .unwrap();
    assert!(
        i2.get_output().contains("1..=10"),
        "вывод: {}",
        i2.get_output()
    );
}

#[test]
fn test_range_loop_iteration() {
    // [KITE 2/4] Итерация по диапазону в `нц для … в …`.
    let source = r#"
алг Тест
нач
    цел сумма
    сумма := 0
    нц для к в 1..=5
        сумма := сумма + к
    кц
    вывод сумма
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("15"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_range_value_display_with_step() {
    // [KITE-0002] Диапазон со шагом печатается как 1..10 шаг 2.
    let mut interpreter = Interpreter::new();
    interpreter
        .run("алг Тест\nнач\n    д := 1..10 шаг 2\n    вывод д\nкон\n")
        .unwrap();
    let out = interpreter.get_output();
    assert!(out.contains("1..10 шаг 2"), "вывод: {}", out);
}

#[test]
fn test_range_loop_iteration_with_step() {
    // [KITE-0002] Итерация по диапазону со шагом: 1+3+5+7+9 = 25.
    let source = r#"
алг Тест
нач
    цел сумма
    сумма := 0
    нц для к в 1..10 шаг 2
        сумма := сумма + к
    кц
    вывод сумма
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("25"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_range_inclusive_loop_iteration_with_step() {
    // [KITE-0002] Включительный диапазон со шагом: 1+4+7+10 = 22.
    let source = r#"
алг Тест
нач
    цел сумма
    сумма := 0
    нц для к в 1..=10 шаг 3
        сумма := сумма + к
    кц
    вывод сумма
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("22"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_range_pattern_match_with_step() {
    // [KITE-0002] Сопоставление с образцом диапазона со шагом.
    let source = r#"
алг Тест
нач
    цел x
    x := 7
    совпадение x
        при 1..10 шаг 2 => вывод "yes"
        при _ => вывод "no"
    все
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("yes"),
        "вывод: {}",
        interpreter.get_output()
    );

    let source_no = r#"
алг Тест
нач
    цел x
    x := 8
    совпадение x
        при 1..10 шаг 2 => вывод "yes"
        при _ => вывод "no"
    все
кон
"#;
    let mut i2 = Interpreter::new();
    i2.run(source_no).unwrap();
    assert!(i2.get_output().contains("no"), "вывод: {}", i2.get_output());
}

#[test]
fn test_builtin_functions() {
    let mut interpreter = Interpreter::new();

    // Математика
    assert_eq!(
        interpreter.eval("abs(-5)").unwrap(),
        Value::Number(shared::types::Number::I64(5))
    );
    assert_eq!(
        interpreter.eval("min(3, 7)").unwrap(),
        Value::Number(shared::types::Number::I64(3))
    );
    assert_eq!(
        interpreter.eval("max(3, 7)").unwrap(),
        Value::Number(shared::types::Number::I64(7))
    );

    // Строки
    assert_eq!(
        interpreter.eval("длина(\"привет\")").unwrap(),
        Value::Number(shared::types::Number::I64(6))
    );
}

#[test]
fn test_string_operations() {
    let source = r#"
алг Тест
нач
    лит s := "Привет, мир!"
    вывод длина(s)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("12"));
}

#[test]
fn test_array() {
    let _source = r#"
алг Тест
нач
    таб цел[] arr := таб(1, 2, 3, 4, 5)
    вывод сумма(arr)
кон
"#;
    // Примечание: синтаксис массивов может отличаться
    // Этот тест показывает концепцию
}

#[test]
fn test_conditional_expression() {
    let mut interpreter = Interpreter::new();

    let result = interpreter.eval("если 5 > 3 то 1 иначе 0 все").unwrap();
    assert_eq!(result, Value::Number(shared::types::Number::I64(1)));
}

#[test]
fn test_try_catch() {
    let source = r#"
алг Тест
нач
    попытка
        бросить "ошибка"
    перехват e
        вывод "перехвачено"
    кон
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("перехвачено"));
}

#[test]
fn test_type_alias_statement() {
    let source = r#"
алг Тест
нач
    type MyInt = цел
    вывод 42
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("42"));
}

#[test]
fn test_coalesce_operator() {
    let mut interpreter = Interpreter::new();

    assert_eq!(
        interpreter.eval("ничего ?? 42").unwrap(),
        Value::Number(shared::types::Number::I64(42))
    );
    assert_eq!(
        interpreter.eval("некоторое(5) ?? 42").unwrap(),
        Value::Number(shared::types::Number::I64(5))
    );
}

#[test]
fn test_coalesce_with_regular_value() {
    let mut interpreter = Interpreter::new();

    assert_eq!(
        interpreter.eval("10 ?? 42").unwrap(),
        Value::Number(shared::types::Number::I64(10))
    );
}

#[test]
fn test_lambda_execution() {
    let source = r#"
алг Тест
нач
    пусть f := лямбда(x) -> x + 1
    вывод f(5)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("6"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_short_lambda() {
    let source = r#"
алг Тест
нач
    пусть f := x => x * 2
    вывод f(7)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("14"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_short_lambda_multi_param() {
    let source = r#"
алг Тест
нач
    пусть f := (x, y) => x + y
    вывод f(3, 4)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("7"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_lambda_capture() {
    let source = r#"
алг Тест
нач
    пусть a := 10
    пусть f := x => x + a
    вывод f(5)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("15"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_safe_field_on_null() {
    let source = r#"
класс Контейнер
открытый:
цел value
конструктор()
нач
кон
кон

алг Тест
нач
    пусть c := ничего
    вывод c?.value
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("пусто"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_safe_field_on_object() {
    let source = r#"
класс Контейнер
открытый:
цел value := 42
конструктор()
нач
кон
кон

алг Тест
нач
    пусть c := новый Контейнер()
    вывод c?.value
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("42"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_safe_method_on_null() {
    let source = r#"
класс Счёт
открытый:
алг цел получить()
нач
    знач := 100
кон
кон

алг Тест
нач
    пусть c := ничего
    вывод c?.получить()
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("пусто"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_safe_method_on_object() {
    let source = r#"
класс Счёт
открытый:
алг цел получить()
нач
    знач := 100
кон
кон

алг Тест
нач
    пусть c := новый Счёт()
    вывод c?.получить()
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("100"),
        "вывод: {}",
        interpreter.get_output()
    );
}

// =========================================================================
//         [W0] Диагностика необъявленных переменных + строгий режим
// =========================================================================

#[test]
fn test_undeclared_lenient_default_unchanged_but_warns() {
    // Мягкий режим (по умолчанию): присваивание необъявленной переменной
    // ПРЕЖНЕЕ поведение — переменная создаётся, вывод не меняется, но в
    // коллекторе предупреждений появляется запись с её именем.
    let source = r#"
алг Тест
нач
    хyz := 5
    вывод хyz
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("5"),
        "вывод должен остаться прежним: {}",
        interpreter.get_output()
    );
    let warnings = interpreter.warnings();
    assert_eq!(warnings.len(), 1, "ожидалось ровно одно предупреждение");
    assert!(
        warnings[0].contains("хyz"),
        "предупреждение должно называть переменную: {}",
        warnings[0]
    );
}

#[test]
fn test_declared_variable_no_warning() {
    // Объявленная переменная (`цел x`) с последующим присваиванием —
    // никаких предупреждений.
    let source = r#"
алг Тест
нач
    цел x
    x := 5
    вывод x
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("5"));
    assert_eq!(
        interpreter.warnings().len(),
        0,
        "объявленная переменная не должна давать предупреждений: {:?}",
        interpreter.warnings()
    );
}

#[test]
fn test_loop_var_and_result_no_warning() {
    // Переменная цикла `для i` и возврат через `знач` — легитимные неявные
    // имена, не должны считаться необъявленными.
    let source = r#"
алг цел Сумма
нач
    цел с
    с := 0
    нц для i от 1 до 3
        с := с + i
    кц
    знач := с
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert_eq!(
        interpreter.warnings().len(),
        0,
        "переменная цикла и знач не должны предупреждать: {:?}",
        interpreter.warnings()
    );
}

#[test]
fn test_strict_mode_errors_on_undeclared() {
    // Строгий режим: присваивание необъявленной переменной — ошибка (Err),
    // а не паника и не молчаливое создание.
    let source = r#"
алг Тест
нач
    хyz := 5
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.set_strict(true);
    let err = interpreter.run(source).unwrap_err();
    assert_eq!(err.kind, RuntimeErrorKind::UndefinedVariable);
    assert!(
        err.message.contains("хyz"),
        "ошибка должна называть переменную: {}",
        err.message
    );
}

#[test]
fn test_strict_mode_ok_for_declared() {
    // Строгий режим не мешает объявленным переменным и параметрам.
    let source = r#"
алг цел Квадрат(арг цел x)
нач
    цел r
    r := x * x
    знач := r
кон

алг Тест
нач
    вывод Квадрат(6)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.set_strict(true);
    interpreter.run(source).unwrap();
    assert!(interpreter.get_output().contains("36"));
    assert_eq!(interpreter.warnings().len(), 0);
}

#[test]
fn test_safe_chain() {
    let source = r#"
класс Внутренний
открытый:
цел y := 7
конструктор()
нач
кон
кон

класс Внешний
открытый:
Внутренний inner := ничего
конструктор()
нач
кон
кон

алг Тест
нач
    пусть o := новый Внешний()
    вывод o?.inner?.y
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("пусто"),
        "вывод: {}",
        interpreter.get_output()
    );
}

// =====================================================================
//   [W0] «Разбирается, но ничего не делает» — закрытие пробелов
// =====================================================================

#[test]
fn test_module_access_double_colon_call() {
    // [KITE-0015] `Модуль::алг(...)` разрешается так же, как `Модуль.алг(...)`.
    let source = r#"
модуль Математика
    алг цел Удвоить(арг цел x)
    нач
        знач := x * 2
    кон
кон

алг Тест
нач
    вывод Математика::Удвоить(21)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("42"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_module_access_dot_call_still_works() {
    // Точечная форма не должна пострадать.
    let source = r#"
модуль Математика
    алг цел Удвоить(арг цел x)
    нач
        знач := x * 2
    кон
кон

алг Тест
нач
    вывод Математика.Удвоить(21)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("42"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_module_access_unknown_member_error() {
    // Несуществующий член модуля — ясная ошибка, называющая модуль и член.
    let source = r#"
модуль Математика
    алг цел Удвоить(арг цел x)
    нач
        знач := x * 2
    кон
кон

алг Тест
нач
    вывод Математика::Утроить(21)
кон
"#;
    let mut interpreter = Interpreter::new();
    let err = interpreter.run(source).unwrap_err();
    assert!(
        err.message.contains("Математика") && err.message.contains("Утроить"),
        "ошибка должна называть модуль и член: {}",
        err.message
    );
}

#[test]
fn test_enum_variant_double_colon() {
    // `Перечисление::Вариант` — значение перечисления (тоже идёт через `::`).
    let source = r#"
перечисление Цвет
Красный
Зелёный
Синий
кон

алг Тест
нач
    ц := Цвет::Зелёный
    вывод ц
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("Зелёный"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_module_access_bare_algorithm_is_clear_error() {
    // Голая ссылка `Модуль::алг` без вызова — не значение: ясная ошибка.
    let source = r#"
модуль Математика
    алг цел Удвоить(арг цел x)
    нач
        знач := x * 2
    кон
кон

алг Тест
нач
    ф := Математика::Удвоить
    вывод ф
кон
"#;
    let mut interpreter = Interpreter::new();
    let err = interpreter.run(source).unwrap_err();
    assert!(
        err.message.contains("Удвоить") && err.message.contains("("),
        "ошибка должна подсказывать вызов: {}",
        err.message
    );
}

#[test]
fn test_bare_super_ref_is_clear_error() {
    // [KITE-0011] Голый `предок` — не значение; ясная ошибка с подсказкой.
    let source = r#"
класс Базовый
открытый:
конструктор()
нач
кон
алг цел Значение
нач
    знач := 1
кон
кон

класс Производный расширяет Базовый
открытый:
конструктор()
нач
кон
алг цел Значение
нач
    знач := предок
кон
кон

алг Тест
нач
    пусть o := новый Производный()
    вывод o.Значение()
кон
"#;
    let mut interpreter = Interpreter::new();
    let err = interpreter.run(source).unwrap_err();
    assert!(
        err.message.contains("предок.метод"),
        "ошибка должна подсказывать форму вызова: {}",
        err.message
    );
}

#[test]
fn test_destructor_declaration_warns() {
    // [KITE-0011] Деструкторы не исполняются (детерминированной точки
    // разрушения в текущей объектной модели нет) — но молчать нельзя.
    let source = r#"
класс Ресурс
открытый:
конструктор()
нач
кон
деструктор
нач
    вывод "закрыто"
кон
кон

алг Тест
нач
    пусть r := новый Ресурс()
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter
            .warnings()
            .iter()
            .any(|w| w.contains("деструктор") && w.contains("Ресурс")),
        "ожидалось предупреждение о деструкторе: {:?}",
        interpreter.warnings()
    );
    // И тело деструктора действительно не исполняется.
    assert!(
        !interpreter.get_output().contains("закрыто"),
        "деструктор не должен исполняться: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_compose_operator() {
    // [KITE-0013] `f >> g` — композиция: сначала f, затем g.
    let source = r#"
алг цел Удвоить(арг цел x)
нач
    знач := x * 2
кон

алг цел Плюс1(арг цел x)
нач
    знач := x + 1
кон

алг Тест
нач
    ф := Удвоить >> Плюс1
    вывод ф(5)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("11"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_compose_operator_with_lambdas() {
    let source = r#"
алг Тест
нач
    ф := лямбда(x) -> x * 2
    г := лямбда(x) -> x + 1
    к := ф >> г
    вывод к(5)
кон
"#;
    let mut interpreter = Interpreter::new();
    interpreter.run(source).unwrap();
    assert!(
        interpreter.get_output().contains("11"),
        "вывод: {}",
        interpreter.get_output()
    );
}

#[test]
fn test_compose_operator_rejects_non_function() {
    let source = r#"
алг Тест
нач
    ц := 5
    ф := ц >> ц
    вывод ф(1)
кон
"#;
    let mut interpreter = Interpreter::new();
    let err = interpreter.run(source).unwrap_err();
    assert!(
        err.message.contains(">>"),
        "ошибка должна называть оператор: {}",
        err.message
    );
}
