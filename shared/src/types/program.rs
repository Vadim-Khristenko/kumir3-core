//! Полная программа на языке Кумир

use super::algorithm::{Algorithm, OverloadedAlgorithm};
use super::class::ClassDef;
use super::stmt::Stmt;

/// Полная программа на языке Кумир.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Подключённые модули
    pub imports: Vec<Stmt>,

    /// Объявления глобальных переменных
    pub globals: Vec<Stmt>,

    /// Определения алгоритмов
    pub algorithms: Vec<Algorithm>,

    /// Перегруженные алгоритмы (Kumir 3)
    pub overloaded_algorithms: Vec<OverloadedAlgorithm>,

    /// Определения классов (Kumir 3)
    pub classes: Vec<ClassDef>,

    /// Определения интерфейсов (Kumir 3)
    pub interfaces: Vec<Stmt>,

    /// Главный алгоритм (точка входа)
    pub main: Option<Algorithm>,

    /// Предупреждения при разборе программы
    pub warnings: Vec<String>,

    /// Программа не содержала объявления алгоритма, и свободные инструкции
    /// были обёрнуты в анонимный алгоритм.
    ///
    /// Признак нужен интерактивному режиму: такие инструкции человек набрал
    /// «просто так», и выполнять их следует в общей области, чтобы набранное
    /// строкой выше было видно строкой ниже.
    pub auto_wrapped: bool,
}
