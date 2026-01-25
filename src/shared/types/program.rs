//! Полная программа на языке Кумир

use super::stmt::Stmt;
use super::algorithm::{Algorithm, OverloadedAlgorithm};
use super::class::ClassDef;

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
}
