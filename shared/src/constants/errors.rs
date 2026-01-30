//! Сообщения об ошибках на русском языке
//!
//! Содержит все стандартные сообщения об ошибках.

/// Сообщения об ошибках на русском языке.
pub mod errors {
    // =========================================================================
    //                    ЛЕКСИЧЕСКИЕ ОШИБКИ
    // =========================================================================
    pub const UNEXPECTED_CHAR: &str = "Неожиданный символ";
    pub const UNTERMINATED_STRING: &str = "Незавершённая строка";
    pub const UNTERMINATED_CHAR: &str = "Незавершённый символьный литерал";
    pub const INVALID_NUMBER: &str = "Некорректное число";
    pub const INVALID_ESCAPE: &str = "Некорректная escape-последовательность";

    // =========================================================================
    //                    СИНТАКСИЧЕСКИЕ ОШИБКИ
    // =========================================================================
    pub const EXPECTED_EXPRESSION: &str = "Ожидалось выражение";
    pub const EXPECTED_IDENTIFIER: &str = "Ожидался идентификатор";
    pub const EXPECTED_TYPE: &str = "Ожидался тип";
    pub const EXPECTED_SEMICOLON: &str = "Ожидалась точка с запятой";
    pub const EXPECTED_RPAREN: &str = "Ожидалась закрывающая скобка ')'";
    pub const EXPECTED_RBRACKET: &str = "Ожидалась закрывающая скобка ']'";
    pub const EXPECTED_RBRACE: &str = "Ожидалась закрывающая скобка '}'";
    pub const EXPECTED_THEN: &str = "Ожидалось 'то'";
    pub const EXPECTED_END: &str = "Ожидалось 'кон'";
    pub const EXPECTED_FI: &str = "Ожидалось 'все'";
    pub const EXPECTED_END_LOOP: &str = "Ожидалось 'кц'";
    pub const EXPECTED_BEGIN: &str = "Ожидалось 'нач'";
    pub const EXPECTED_ASSIGN: &str = "Ожидалось ':='";
    pub const UNEXPECTED_TOKEN: &str = "Неожиданный токен";

    // =========================================================================
    //                    СЕМАНТИЧЕСКИЕ ОШИБКИ
    // =========================================================================
    pub const UNDEFINED_VARIABLE: &str = "Неопределённая переменная";
    pub const UNDEFINED_FUNCTION: &str = "Неопределённый алгоритм";
    pub const TYPE_MISMATCH: &str = "Несоответствие типов";

    // =========================================================================
    //                    ОШИБКИ ВЫПОЛНЕНИЯ
    // =========================================================================
    pub const DIVISION_BY_ZERO: &str = "Деление на ноль";
    pub const INDEX_OUT_OF_BOUNDS: &str = "Индекс выходит за границы массива";
    pub const STACK_OVERFLOW: &str = "Переполнение стека";
    pub const INVALID_ARGUMENT: &str = "Некорректный аргумент";
}
