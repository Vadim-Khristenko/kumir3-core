//! Классы и ООП (Kumir 3)

use super::type_spec::TypeSpec;
use super::expr::Expr;
use super::stmt::Stmt;
use super::algorithm::Parameter;

/// Модификатор доступа.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Public,     // открытый
    Private,    // закрытый
    Protected,  // защищённый
}

// =============================================================================
//                          ИНТЕРФЕЙСЫ
// =============================================================================

/// Определение интерфейса.
/// 
/// Интерфейс — набор сигнатур методов, которые класс обязан реализовать.
/// 
/// Пример:
/// ```kumir
/// интерфейс Сравнимый
///     алг лог меньше(арг Сравнимый другой)
///     алг лог равно(арг Сравнимый другой)
/// кон
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDef {
    /// Имя интерфейса
    pub name: String,
    
    /// Родительские интерфейсы (множественное наследование для интерфейсов)
    pub extends: Vec<String>,
    
    /// Сигнатуры методов (без реализации)
    pub methods: Vec<MethodSignature>,
}

// =============================================================================
//                          TRAIT (ТИПАЖ)
// =============================================================================

/// Определение trait (типажа).
/// 
/// Trait похож на интерфейс, но может содержать реализации методов по умолчанию.
/// 
/// Пример:
/// ```kumir
/// типаж Отображаемый
///     алг лит в_строку()  | обязательный метод
///     
///     алг вывести()       | метод с реализацией по умолчанию
///     нач
///         вывод в_строку(), нс
///     кон
/// кон
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDef {
    /// Имя типажа
    pub name: String,
    
    /// Супер-типажи (требуемые типажи)
    pub supertraits: Vec<String>,
    
    /// Методы типажа (могут иметь реализацию по умолчанию)
    pub methods: Vec<TraitMethod>,
}

/// Метод в типаже
#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    /// Сигнатура метода
    pub signature: MethodSignature,
    
    /// Реализация по умолчанию (None = обязательный для реализации)
    pub default_impl: Option<Vec<Stmt>>,
}

// =============================================================================
//                          IMPL-БЛОК
// =============================================================================

/// Блок реализации (impl).
/// 
/// Используется для:
/// 1. Реализации методов для типа: `реализация для Точка`
/// 2. Реализации типажа для типа: `реализация Отображаемый для Точка`
/// 
/// Пример:
/// ```kumir
/// реализация Отображаемый для Точка
///     алг лит в_строку()
///     нач
///         знач := "(" + строка(я.x) + ", " + строка(я.y) + ")"
///     кон
/// кон
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDef {
    /// Имя типажа (None = собственные методы типа)
    pub trait_name: Option<String>,
    
    /// Целевой тип
    pub target_type: String,
    
    /// Реализуемые методы
    pub methods: Vec<Method>,
}

// =============================================================================
//                          КЛАССЫ
// =============================================================================

/// Определение класса.
/// 
/// Пример:
/// ```kumir
/// класс Точка
///     закрытый:
///         вещ x, y
///     открытый:
///         конструктор(арг вещ x, арг вещ y)
///         нач
///             я.x := x
///             я.y := y
///         кон
///         
///         алг вещ расстояние(арг Точка другая)
///         нач
///             знач := sqrt((я.x - другая.x)**2 + (я.y - другая.y)**2)
///         кон
/// кон
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDef {
    /// Имя класса
    pub name: String,
    
    /// Родительский класс (наследование)
    pub parent: Option<String>,
    
    /// Реализуемые интерфейсы
    pub interfaces: Vec<String>,
    
    /// Поля класса
    pub fields: Vec<Field>,
    
    /// Методы класса
    pub methods: Vec<Method>,
    
    /// Конструкторы (может быть несколько — перегрузка)
    pub constructors: Vec<Constructor>,
    
    /// Деструктор (опционально)
    pub destructor: Option<Method>,
    
    /// Является ли класс абстрактным
    pub is_abstract: bool,
    
    /// Является ли класс финальным (нельзя наследовать)
    pub is_final: bool,
}

/// Поле класса или структуры.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    /// Имя поля
    pub name: String,
    
    /// Тип поля
    pub type_spec: TypeSpec,
    
    /// Модификатор доступа
    pub visibility: Visibility,
    
    /// Начальное значение (по умолчанию)
    pub default: Option<Expr>,
    
    /// Является ли поле статическим
    pub is_static: bool,
}

/// Метод класса.
#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    /// Имя метода
    pub name: String,
    
    /// Параметры
    pub params: Vec<Parameter>,
    
    /// Возвращаемый тип
    pub return_type: Option<TypeSpec>,
    
    /// Тело метода (None для абстрактных методов)
    pub body: Option<Vec<Stmt>>,
    
    /// Модификатор доступа
    pub visibility: Visibility,
    
    /// Является ли метод статическим
    pub is_static: bool,
    
    /// Является ли метод виртуальным
    pub is_virtual: bool,
    
    /// Является ли метод абстрактным
    pub is_abstract: bool,
    
    /// Переопределяет ли метод родительский
    pub is_override: bool,
    
    /// Является ли метод финальным
    pub is_final: bool,
    
    /// Является ли метод асинхронным
    pub is_async: bool,
}

/// Сигнатура метода (для интерфейсов).
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSignature {
    /// Имя метода
    pub name: String,
    
    /// Параметры
    pub params: Vec<Parameter>,
    
    /// Возвращаемый тип
    pub return_type: Option<TypeSpec>,
}

/// Конструктор класса.
/// 
/// Поддерживает перегрузку — может быть несколько конструкторов
/// с разными параметрами.
#[derive(Debug, Clone, PartialEq)]
pub struct Constructor {
    /// Параметры конструктора
    pub params: Vec<Parameter>,
    
    /// Вызов конструктора родителя (если есть)
    pub super_call: Option<Vec<Expr>>,
    
    /// Тело конструктора
    pub body: Vec<Stmt>,
    
    /// Модификатор доступа
    pub visibility: Visibility,
}
