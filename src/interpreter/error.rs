//! Ошибки интерпретатора Кумир 3
//!
//! Модуль содержит типы ошибок, возникающих во время выполнения программы.
//! Использует стандартные сообщения из `shared/constants/errors`.

use std::fmt;
use crate::shared::constants::errors::errors;

/// Ошибка времени выполнения.
#[derive(Debug, Clone)]
pub struct RuntimeError {
    /// Сообщение об ошибке
    pub message: String,
    /// Номер строки (если известен)
    pub line: Option<usize>,
    /// Контекст (имя алгоритма, класса и т.д.)
    pub context: Option<String>,
    /// Тип ошибки
    pub kind: RuntimeErrorKind,
}

/// Тип ошибки времени выполнения.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeErrorKind {
    /// Деление на ноль
    DivisionByZero,
    /// Переполнение числа
    Overflow,
    /// Неопределённая переменная
    UndefinedVariable,
    /// Неопределённый алгоритм
    UndefinedAlgorithm,
    /// Неопределённый тип
    UndefinedType,
    /// Несоответствие типов
    TypeMismatch,
    /// Индекс вне границ массива
    IndexOutOfBounds,
    /// Неверное количество аргументов
    ArgumentCount,
    /// Утверждение не выполнено
    AssertionFailed,
    /// Ошибка ввода/вывода
    IOError,
    /// Исключение пользователя
    UserException,
    /// Не реализовано
    NotImplemented,
    /// Прочая ошибка
    Other,
}

impl RuntimeError {
    /// Создаёт новую ошибку.
    pub fn new(message: impl Into<String>, kind: RuntimeErrorKind) -> Self {
        Self {
            message: message.into(),
            line: None,
            context: None,
            kind,
        }
    }

    /// Добавляет номер строки.
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Добавляет контекст.
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    // === Конструкторы для распространённых ошибок ===

    pub fn division_by_zero() -> Self {
        Self::new(errors::DIVISION_BY_ZERO, RuntimeErrorKind::DivisionByZero)
    }

    pub fn undefined_variable(name: &str) -> Self {
        Self::new(
            format!("{}: '{}'", errors::UNDEFINED_VARIABLE, name),
            RuntimeErrorKind::UndefinedVariable,
        )
    }

    pub fn undefined_algorithm(name: &str) -> Self {
        Self::new(
            format!("{}: '{}'", errors::UNDEFINED_FUNCTION, name),
            RuntimeErrorKind::UndefinedAlgorithm,
        )
    }

    pub fn undefined_type(name: &str) -> Self {
        Self::new(
            format!("Тип '{}' не определён", name),
            RuntimeErrorKind::UndefinedType,
        )
    }

    pub fn type_mismatch(expected: &str, got: &str) -> Self {
        Self::new(
            format!("{}: ожидался {}, получен {}", errors::TYPE_MISMATCH, expected, got),
            RuntimeErrorKind::TypeMismatch,
        )
    }

    pub fn index_out_of_bounds(index: i64, length: usize) -> Self {
        Self::new(
            format!("{}: индекс {} при размере {}", errors::INDEX_OUT_OF_BOUNDS, index, length),
            RuntimeErrorKind::IndexOutOfBounds,
        )
    }

    pub fn argument_count(name: &str, expected: usize, got: usize) -> Self {
        Self::new(
            format!(
                "{} для '{}': ожидалось {}, получено {}",
                errors::INVALID_ARGUMENT, name, expected, got
            ),
            RuntimeErrorKind::ArgumentCount,
        )
    }

    pub fn assertion_failed(condition: &str) -> Self {
        Self::new(
            format!("Утверждение не выполнено: {}", condition),
            RuntimeErrorKind::AssertionFailed,
        )
    }

    pub fn not_implemented(feature: &str) -> Self {
        Self::new(
            format!("Не реализовано: {}", feature),
            RuntimeErrorKind::NotImplemented,
        )
    }

    pub fn user_exception(message: impl Into<String>) -> Self {
        Self::new(message, RuntimeErrorKind::UserException)
    }

    pub fn io_error(message: impl Into<String>) -> Self {
        Self::new(message, RuntimeErrorKind::IOError)
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Ошибка выполнения] {}", self.message)?;
        if let Some(line) = self.line {
            write!(f, " (строка {})", line)?;
        }
        if let Some(ctx) = &self.context {
            write!(f, " в {}", ctx)?;
        }
        Ok(())
    }
}

impl std::error::Error for RuntimeError {}

/// Результат выполнения интерпретатора.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

/// Сигнал управления потоком (break, continue, return).
#[derive(Debug, Clone)]
pub enum ControlFlow {
    /// Обычное продолжение
    None,
    /// Выход из цикла (break)
    Break,
    /// Переход к следующей итерации (continue)
    Continue,
    /// Возврат из алгоритма
    Return(Option<crate::shared::types::Value>),
}
