//! Алгоритмы и параметры

use super::type_spec::TypeSpec;
use super::expr::Expr;
use super::stmt::Stmt;

/// Определение алгоритма (функции).
#[derive(Debug, Clone, PartialEq)]
pub struct Algorithm {
    /// Имя алгоритма
    pub name: String,
    
    /// Возвращаемый тип (None для процедур)
    pub return_type: Option<TypeSpec>,
    
    /// Параметры
    pub params: Vec<Parameter>,
    
    /// Предусловие (дано)
    pub precondition: Option<Expr>,
    
    /// Постусловие (надо)  
    pub postcondition: Option<Expr>,
    
    /// Тело алгоритма
    pub body: Vec<Stmt>,
    
    /// Является ли алгоритм асинхронным
    pub is_async: bool,
}

/// Параметр алгоритма.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    /// Имя параметра
    pub name: String,
    
    /// Тип параметра
    pub type_spec: TypeSpec,
    
    /// Режим передачи: арг, рез, аргрез
    pub mode: ParamMode,
    
    /// Значение по умолчанию (Kumir 3)
    pub default: Option<Expr>,
}

/// Режим передачи параметра.
#[derive(Debug, Clone, PartialEq)]
pub enum ParamMode {
    Arg,        // арг — входной параметр (по значению)
    Res,        // рез — выходной параметр
    ArgRes,     // аргрез — входной и выходной параметр
}

/// Группа перегруженных алгоритмов.
/// 
/// Позволяет иметь несколько алгоритмов с одним именем, но разными
/// параметрами. Выбор конкретной версии происходит во время компиляции
/// на основе типов аргументов.
#[derive(Debug, Clone, PartialEq)]
pub struct OverloadedAlgorithm {
    /// Общее имя для всех перегрузок
    pub name: String,
    
    /// Варианты перегрузки
    pub overloads: Vec<Algorithm>,
}
