//! Паттерны для pattern matching (Kumir 3)

use super::value::Value;
use super::expr::Expr;

/// Паттерн для pattern matching.
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard: _ (любое значение, игнорируется)
    Wildcard,
    
    /// Литерал: конкретное значение (42, "строка", да)
    Literal(Value),
    
    /// Привязка к переменной: x (значение сохраняется в x)
    Variable(String),
    
    /// Вариант перечисления: Цвет::Красный или Опция::Некоторое(x)
    EnumVariant {
        enum_name: String,
        variant: String,
        bindings: Vec<String>,      // имена для привязки данных
    },
    
    /// Диапазон: 1..10
    Range {
        start: Option<Box<Expr>>,
        end: Option<Box<Expr>>,
        inclusive: bool,
    },
    
    /// Кортеж: (x, y, _)
    Tuple(Vec<Pattern>),
    
    /// Массив: [первый, второй, ...остальные]
    Array {
        elements: Vec<Pattern>,
        rest: Option<String>,       // привязка для оставшихся элементов
    },
    
    /// Логическое ИЛИ для паттернов: 1 | 2 | 3
    Or(Vec<Pattern>),
}
