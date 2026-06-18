//! Встроенные функции Кумир
//!
//! Содержит списки встроенных функций по категориям.

// ============================================================================
//                    ВСТРОЕННЫЕ ФУНКЦИИ
// ============================================================================

/// Список встроенных математических функций.
pub static BUILTIN_MATH_FUNCTIONS: &[&str] = &[
    // Тригонометрия
    "sin",
    "cos",
    "tg",
    "tan",
    "ctg",
    "cot",
    "arcsin",
    "arccos",
    "arctg",
    "arctan",
    "arcctg",
    "arccot",
    // Гиперболические
    "sh",
    "sinh",
    "ch",
    "cosh",
    "th",
    "tanh",
    // Степенные и логарифмы
    "sqrt",
    "корень",
    "exp",
    "ln",
    "lg",
    "log",
    "степень",
    "pow",
    // Округление
    "abs",
    "модуль",
    "sign",
    "знак",
    "int",
    "цел_часть",
    "frac",
    "дробь",
    "floor",
    "ceil",
    "round",
    "округл",
    // Прочее
    "min",
    "мин",
    "max",
    "макс",
    "mod",
    "div",
    "случ",
    "rand",
    "random",
];

/// Список встроенных функций работы со строками.
pub static BUILTIN_STRING_FUNCTIONS: &[&str] = &[
    "длин",
    "len",
    "length",
    "символ",
    "char",
    "chr",
    "код",
    "ord",
    "code",
    "вырезка",
    "substr",
    "substring",
    "позиция",
    "pos",
    "position",
    "find",
    "заменить",
    "replace",
    "верхний",
    "upper",
    "uppercase",
    "нижний",
    "lower",
    "lowercase",
    "обрезать",
    "trim",
    "слева",
    "left",
    "справа",
    "right",
    "повторить",
    "repeat",
    "разбить",
    "split",
    "соединить",
    "join",
];

/// Список встроенных функций ввода/вывода.
pub static BUILTIN_IO_FUNCTIONS: &[&str] = &[
    "ввод",
    "input",
    "вывод",
    "output",
    "print",
    "вывод_строки",
    "println",
    "новая_строка",
    "newline",
    "нс",
];

/// Проверяет, является ли строка встроенной функцией.
pub fn is_builtin_function(s: &str) -> bool {
    BUILTIN_MATH_FUNCTIONS.contains(&s)
        || BUILTIN_STRING_FUNCTIONS.contains(&s)
        || BUILTIN_IO_FUNCTIONS.contains(&s)
}

/// Получает все имена встроенных функций.
pub fn get_all_builtin_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Vec::new();

    // Добавляем математические функции
    names.extend(BUILTIN_MATH_FUNCTIONS.iter().copied());

    // Добавляем строковые функции
    names.extend(BUILTIN_STRING_FUNCTIONS.iter().copied());

    // Добавляем функции ввода/вывода
    names.extend(BUILTIN_IO_FUNCTIONS.iter().copied());

    // Добавляем дополнительные
    names.extend(&[
        "цел",
        "int",
        "to_int",
        "вещ",
        "float",
        "to_float",
        "строка",
        "str",
        "to_string",
        "лог",
        "bool",
        "to_bool",
        "размер",
        "size",
        "добавить",
        "push",
        "извлечь",
        "pop",
        "обратить",
        "reverse",
        "сортировать",
        "sort",
        "содержит",
        "contains",
        "индекс",
        "index_of",
        "утв",
        "assert",
        "тип",
        "type_of",
        "ошибка",
        "error",
    ]);

    names
}
