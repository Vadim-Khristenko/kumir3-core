//! Спецификация типов языка КуМир 3
//!
//! `TypeSpec` — это статическое представление типа, которое точно соответствует
//! вариантам `Value`. Используется в AST, парсере и компиляторе для описания
//! типов переменных, параметров и возвращаемых значений.
//!
//! ## Соответствие Value ↔ TypeSpec
//!
//! | Value variant     | TypeSpec variant        |
//! |-------------------|-------------------------|
//! | Number(I8..I128)  | Int8, Int16, Int32...   |
//! | Number(U8..U128)  | UInt8, UInt16...        |
//! | Number(F32..F128) | Float32, Float64...     |
//! | String            | String                  |
//! | Boolean           | Bool                    |
//! | Char              | Char                    |
//! | Array             | Array(Box<TypeSpec>)    |
//! | Pair              | Pair(Box, Box)          |
//! | Triple            | Triple(Box, Box, Box)   |
//! | Tuple             | Tuple(Vec<TypeSpec>)    |
//! | Set               | Set(Box<TypeSpec>)      |
//! | Map               | Map(Box, Box)           |
//! | Option            | Option(Box<TypeSpec>)   |
//! | Result            | Result(Box, Box)        |
//! | Pointer           | Pointer(Box<TypeSpec>)  |
//! | Enum              | Enum(String)            |
//! | Object            | Object(String)          |
//! | NativeObject      | Native(String)          |
//! | Null              | Null                    |
//! | Undefined         | Undefined               |

use std::fmt;

/// Спецификация типа — статическое описание типа в AST.
///
/// Точно соответствует вариантам `Value`, чтобы обеспечить
/// корректную проверку типов и кодогенерацию.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeSpec {
    // =========================================================================
    // Числовые типы (соответствуют Number)
    // =========================================================================
    
    /// цел_8 (i8)
    Int8,
    /// цел_16 (i16)
    Int16,
    /// цел_32, малое_цел (i32)
    Int32,
    /// цел_64, цел — тип по умолчанию (i64)
    Int64,
    /// цел_128, большое_цел (i128)
    Int128,
    
    /// нат_8 (u8)
    UInt8,
    /// нат_16 (u16)
    UInt16,
    /// нат_32 (u32)
    UInt32,
    /// нат_64, нат (u64)
    UInt64,
    /// нат_128 (u128)
    UInt128,
    
    /// вещ_32, малое_вещ (f32)
    Float32,
    /// вещ_64, вещ — тип по умолчанию (f64)
    Float64,
    /// вещ_128, большое_вещ (f128)
    Float128,

    // =========================================================================
    // Базовые скалярные типы
    // =========================================================================
    
    /// лит (строка)
    String,
    /// лог (да/нет)
    Bool,
    /// сим (символ)
    Char,

    // =========================================================================
    // Коллекции
    // =========================================================================
    
    /// таб T[] — массив элементов типа T
    Array(Box<TypeSpec>),
    
    /// пара (T, U)
    Pair(Box<TypeSpec>, Box<TypeSpec>),
    
    /// тройка (T, U, V)
    Triple(Box<TypeSpec>, Box<TypeSpec>, Box<TypeSpec>),
    
    /// кортеж (T1, T2, ..., Tn)
    Tuple(Vec<TypeSpec>),
    
    /// множество {T}
    Set(Box<TypeSpec>),
    
    /// словарь [K: V]
    Map(Box<TypeSpec>, Box<TypeSpec>),

    // =========================================================================
    // Обёртки (Option/Result)
    // =========================================================================
    
    /// опция T? — может содержать значение или быть пустым
    Option(Box<TypeSpec>),
    
    /// результат T!E — либо успех (T), либо ошибка (E)
    Result {
        ok: Box<TypeSpec>,
        err: Box<TypeSpec>,
    },

    // =========================================================================
    // Указатели и ссылки
    // =========================================================================
    
    /// указатель на T
    Pointer(Box<TypeSpec>),

    // =========================================================================
    // Пользовательские типы
    // =========================================================================
    
    /// перечисление по имени
    Enum(String),
    
    /// объект (экземпляр класса) по имени типа
    Object(String),
    
    /// нативный объект (HTTP сервер, файл, сокет и т.д.)
    Native(String),

    // =========================================================================
    // Функциональные типы
    // =========================================================================
    
    /// функция (T1, T2, ...) -> R
    Function {
        params: Vec<TypeSpec>,
        result: std::option::Option<Box<TypeSpec>>,
    },

    // =========================================================================
    // Специальные типы
    // =========================================================================
    
    /// null / пусто
    Null,
    
    /// неопределено (неинициализированная переменная)
    Undefined,
    
    /// авто — тип выводится автоматически
    Auto,
    
    /// любой тип (для дженериков и динамической типизации)
    Any,
    
    /// void — отсутствие возвращаемого значения (для процедур)
    Void,
}

// =============================================================================
// Алиасы для совместимости со старым кодом
// =============================================================================

#[allow(non_upper_case_globals, non_snake_case)]
impl TypeSpec {
    /// Int (алиас для Int64) — для совместимости
    pub const Int: TypeSpec = TypeSpec::Int64;
    /// Float (алиас для Float64) — для совместимости
    pub const Float: TypeSpec = TypeSpec::Float64;
    /// None (алиас для Void) — для совместимости
    pub const None: TypeSpec = TypeSpec::Void;
    /// Optional — алиас для Option (для совместимости)
    pub fn Optional(inner: Box<TypeSpec>) -> TypeSpec {
        TypeSpec::Option(inner)
    }
    /// Custom — алиас для Object (для совместимости)
    pub fn Custom(name: String) -> TypeSpec {
        TypeSpec::Object(name)
    }
    /// Class — алиас для Object (для совместимости)
    pub fn Class(name: String) -> TypeSpec {
        TypeSpec::Object(name)
    }
    /// Interface — алиас для Object (для совместимости)
    pub fn Interface(name: String) -> TypeSpec {
        TypeSpec::Object(name)
    }
}

// =============================================================================
// Конструкторы для удобства
// =============================================================================

impl TypeSpec {
    // -------------------------------------------------------------------------
    // Целые типы по умолчанию
    // -------------------------------------------------------------------------
    
    /// цел — целое число по умолчанию (i64)
    pub const INT: TypeSpec = TypeSpec::Int64;
    
    /// нат — натуральное число по умолчанию (u64)
    pub const UINT: TypeSpec = TypeSpec::UInt64;
    
    // -------------------------------------------------------------------------
    // Вещественные типы по умолчанию
    // -------------------------------------------------------------------------
    
    /// вещ — вещественное число по умолчанию (f64)
    pub const FLOAT: TypeSpec = TypeSpec::Float64;
    
    // -------------------------------------------------------------------------
    // Конструкторы
    // -------------------------------------------------------------------------
    
    /// Создаёт тип массива
    pub fn array(elem: TypeSpec) -> Self {
        TypeSpec::Array(Box::new(elem))
    }
    
    /// Создаёт тип пары
    pub fn pair(first: TypeSpec, second: TypeSpec) -> Self {
        TypeSpec::Pair(Box::new(first), Box::new(second))
    }
    
    /// Создаёт тип тройки
    pub fn triple(first: TypeSpec, second: TypeSpec, third: TypeSpec) -> Self {
        TypeSpec::Triple(Box::new(first), Box::new(second), Box::new(third))
    }
    
    /// Создаёт тип кортежа
    pub fn tuple(elements: Vec<TypeSpec>) -> Self {
        TypeSpec::Tuple(elements)
    }
    
    /// Создаёт тип множества
    pub fn set(elem: TypeSpec) -> Self {
        TypeSpec::Set(Box::new(elem))
    }
    
    /// Создаёт тип словаря
    pub fn map(key: TypeSpec, value: TypeSpec) -> Self {
        TypeSpec::Map(Box::new(key), Box::new(value))
    }
    
    /// Создаёт тип опции
    pub fn option(inner: TypeSpec) -> Self {
        TypeSpec::Option(Box::new(inner))
    }
    
    /// Создаёт тип результата
    pub fn result(ok: TypeSpec, err: TypeSpec) -> Self {
        TypeSpec::Result {
            ok: Box::new(ok),
            err: Box::new(err),
        }
    }
    
    /// Создаёт тип указателя
    pub fn pointer(pointee: TypeSpec) -> Self {
        TypeSpec::Pointer(Box::new(pointee))
    }
    
    /// Создаёт тип функции
    pub fn function(params: Vec<TypeSpec>, result: std::option::Option<TypeSpec>) -> Self {
        TypeSpec::Function {
            params,
            result: result.map(Box::new),
        }
    }
    
    /// Создаёт тип перечисления
    pub fn enum_type(name: impl Into<String>) -> Self {
        TypeSpec::Enum(name.into())
    }
    
    /// Создаёт тип объекта
    pub fn object(name: impl Into<String>) -> Self {
        TypeSpec::Object(name.into())
    }
    
    /// Создаёт тип нативного объекта
    pub fn native(name: impl Into<String>) -> Self {
        TypeSpec::Native(name.into())
    }
}

// =============================================================================
// Проверки типов
// =============================================================================

impl TypeSpec {
    /// Является ли тип числовым
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            TypeSpec::Int8 | TypeSpec::Int16 | TypeSpec::Int32 | TypeSpec::Int64 | TypeSpec::Int128 |
            TypeSpec::UInt8 | TypeSpec::UInt16 | TypeSpec::UInt32 | TypeSpec::UInt64 | TypeSpec::UInt128 |
            TypeSpec::Float32 | TypeSpec::Float64 | TypeSpec::Float128
        )
    }
    
    /// Является ли тип целочисленным (знаковым)
    pub fn is_signed_int(&self) -> bool {
        matches!(
            self,
            TypeSpec::Int8 | TypeSpec::Int16 | TypeSpec::Int32 | TypeSpec::Int64 | TypeSpec::Int128
        )
    }
    
    /// Является ли тип целочисленным (беззнаковым)
    pub fn is_unsigned_int(&self) -> bool {
        matches!(
            self,
            TypeSpec::UInt8 | TypeSpec::UInt16 | TypeSpec::UInt32 | TypeSpec::UInt64 | TypeSpec::UInt128
        )
    }
    
    /// Является ли тип целочисленным (любым)
    pub fn is_integer(&self) -> bool {
        self.is_signed_int() || self.is_unsigned_int()
    }
    
    /// Является ли тип вещественным
    pub fn is_float(&self) -> bool {
        matches!(self, TypeSpec::Float32 | TypeSpec::Float64 | TypeSpec::Float128)
    }
    
    /// Является ли тип скалярным (не коллекция)
    pub fn is_scalar(&self) -> bool {
        self.is_numeric() || matches!(self, TypeSpec::String | TypeSpec::Bool | TypeSpec::Char)
    }
    
    /// Является ли тип коллекцией
    pub fn is_collection(&self) -> bool {
        matches!(
            self,
            TypeSpec::Array(_) | TypeSpec::Tuple(_) | TypeSpec::Set(_) | TypeSpec::Map(_, _) |
            TypeSpec::Pair(_, _) | TypeSpec::Triple(_, _, _)
        )
    }
    
    /// Является ли тип nullable (может быть null)
    pub fn is_nullable(&self) -> bool {
        matches!(self, TypeSpec::Option(_) | TypeSpec::Null)
    }
    
    /// Является ли тип пользовательским
    pub fn is_user_defined(&self) -> bool {
        matches!(self, TypeSpec::Enum(_) | TypeSpec::Object(_))
    }
    
    /// Является ли тип специальным (Auto, Any, Void)
    pub fn is_special(&self) -> bool {
        matches!(self, TypeSpec::Auto | TypeSpec::Any | TypeSpec::Void | TypeSpec::Null | TypeSpec::Undefined)
    }
}

// =============================================================================
// Русские названия типов
// =============================================================================

impl TypeSpec {
    /// Возвращает русское название типа
    pub fn russian_name(&self) -> String {
        match self {
            // Целые
            TypeSpec::Int8 => "цел_8".to_string(),
            TypeSpec::Int16 => "цел_16".to_string(),
            TypeSpec::Int32 => "цел_32".to_string(),
            TypeSpec::Int64 => "цел".to_string(),
            TypeSpec::Int128 => "цел_128".to_string(),
            
            // Беззнаковые
            TypeSpec::UInt8 => "нат_8".to_string(),
            TypeSpec::UInt16 => "нат_16".to_string(),
            TypeSpec::UInt32 => "нат_32".to_string(),
            TypeSpec::UInt64 => "нат".to_string(),
            TypeSpec::UInt128 => "нат_128".to_string(),
            
            // Вещественные
            TypeSpec::Float32 => "вещ_32".to_string(),
            TypeSpec::Float64 => "вещ".to_string(),
            TypeSpec::Float128 => "вещ_128".to_string(),
            
            // Скалярные
            TypeSpec::String => "лит".to_string(),
            TypeSpec::Bool => "лог".to_string(),
            TypeSpec::Char => "сим".to_string(),
            
            // Коллекции
            TypeSpec::Array(elem) => format!("таб {}", elem.russian_name()),
            TypeSpec::Pair(a, b) => format!("пара({}, {})", a.russian_name(), b.russian_name()),
            TypeSpec::Triple(a, b, c) => format!("тройка({}, {}, {})", a.russian_name(), b.russian_name(), c.russian_name()),
            TypeSpec::Tuple(elems) => {
                let names: Vec<_> = elems.iter().map(|e| e.russian_name()).collect();
                format!("кортеж({})", names.join(", "))
            }
            TypeSpec::Set(elem) => format!("множество {}", elem.russian_name()),
            TypeSpec::Map(k, v) => format!("словарь[{}: {}]", k.russian_name(), v.russian_name()),
            
            // Обёртки
            TypeSpec::Option(inner) => format!("{}?", inner.russian_name()),
            TypeSpec::Result { ok, err } => format!("{}!{}", ok.russian_name(), err.russian_name()),
            
            // Указатели
            TypeSpec::Pointer(inner) => format!("указатель {}", inner.russian_name()),
            
            // Пользовательские
            TypeSpec::Enum(name) => name.clone(),
            TypeSpec::Object(name) => name.clone(),
            TypeSpec::Native(name) => format!("@{}", name),
            
            // Функции
            TypeSpec::Function { params, result } => {
                let param_names: Vec<_> = params.iter().map(|p| p.russian_name()).collect();
                let ret = result.as_ref().map(|r| r.russian_name()).unwrap_or_else(|| "ничего".to_string());
                format!("функция({}) -> {}", param_names.join(", "), ret)
            }
            
            // Специальные
            TypeSpec::Null => "пусто".to_string(),
            TypeSpec::Undefined => "неопределено".to_string(),
            TypeSpec::Auto => "авто".to_string(),
            TypeSpec::Any => "любой".to_string(),
            TypeSpec::Void => "ничего".to_string(),
        }
    }
}

// =============================================================================
// Display
// =============================================================================

impl fmt::Display for TypeSpec {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Целые
            TypeSpec::Int8 => write!(f, "i8"),
            TypeSpec::Int16 => write!(f, "i16"),
            TypeSpec::Int32 => write!(f, "i32"),
            TypeSpec::Int64 => write!(f, "i64"),
            TypeSpec::Int128 => write!(f, "i128"),
            
            // Беззнаковые
            TypeSpec::UInt8 => write!(f, "u8"),
            TypeSpec::UInt16 => write!(f, "u16"),
            TypeSpec::UInt32 => write!(f, "u32"),
            TypeSpec::UInt64 => write!(f, "u64"),
            TypeSpec::UInt128 => write!(f, "u128"),
            
            // Вещественные
            TypeSpec::Float32 => write!(f, "f32"),
            TypeSpec::Float64 => write!(f, "f64"),
            TypeSpec::Float128 => write!(f, "f128"),
            
            // Скалярные
            TypeSpec::String => write!(f, "String"),
            TypeSpec::Bool => write!(f, "bool"),
            TypeSpec::Char => write!(f, "char"),
            
            // Коллекции
            TypeSpec::Array(elem) => write!(f, "[{}]", elem),
            TypeSpec::Pair(a, b) => write!(f, "({}, {})", a, b),
            TypeSpec::Triple(a, b, c) => write!(f, "({}, {}, {})", a, b, c),
            TypeSpec::Tuple(elems) => {
                write!(f, "(")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", elem)?;
                }
                write!(f, ")")
            }
            TypeSpec::Set(elem) => write!(f, "{{{}}}", elem),
            TypeSpec::Map(k, v) => write!(f, "[{}: {}]", k, v),
            
            // Обёртки
            TypeSpec::Option(inner) => write!(f, "{}?", inner),
            TypeSpec::Result { ok, err } => write!(f, "{}!{}", ok, err),
            
            // Указатели
            TypeSpec::Pointer(inner) => write!(f, "*{}", inner),
            
            // Пользовательские
            TypeSpec::Enum(name) => write!(f, "enum {}", name),
            TypeSpec::Object(name) => write!(f, "{}", name),
            TypeSpec::Native(name) => write!(f, "@{}", name),
            
            // Функции
            TypeSpec::Function { params, result } => {
                write!(f, "fn(")?;
                for (i, param) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}", param)?;
                }
                write!(f, ")")?;
                if let Some(ret) = result {
                    write!(f, " -> {}", ret)?;
                }
                Ok(())
            }
            
            // Специальные
            TypeSpec::Null => write!(f, "null"),
            TypeSpec::Undefined => write!(f, "undefined"),
            TypeSpec::Auto => write!(f, "auto"),
            TypeSpec::Any => write!(f, "any"),
            TypeSpec::Void => write!(f, "void"),
        }
    }
}

// =============================================================================
// Размер типа в байтах (для числовых типов)
// =============================================================================

impl TypeSpec {
    /// Возвращает размер типа в байтах (для числовых типов)
    pub fn size_bytes(&self) -> std::option::Option<usize> {
        match self {
            TypeSpec::Int8 | TypeSpec::UInt8 => Some(1),
            TypeSpec::Int16 | TypeSpec::UInt16 => Some(2),
            TypeSpec::Int32 | TypeSpec::UInt32 | TypeSpec::Float32 => Some(4),
            TypeSpec::Int64 | TypeSpec::UInt64 | TypeSpec::Float64 => Some(8),
            TypeSpec::Int128 | TypeSpec::UInt128 | TypeSpec::Float128 => Some(16),
            TypeSpec::Bool => Some(1),
            TypeSpec::Char => Some(4), // UTF-32
            _ => None,
        }
    }
}

// =============================================================================
// Совместимость типов
// =============================================================================

impl TypeSpec {
    /// Проверяет, можно ли присвоить значение типа `from` переменной типа `to`
    pub fn is_assignable_from(&self, from: &TypeSpec) -> bool {
        // Точное совпадение
        if self == from {
            return true;
        }
        
        // Any принимает всё
        if matches!(self, TypeSpec::Any) {
            return true;
        }
        
        // Auto принимает всё (тип будет выведен)
        if matches!(self, TypeSpec::Auto) {
            return true;
        }
        
        // Null можно присвоить Option
        if matches!(from, TypeSpec::Null) && matches!(self, TypeSpec::Option(_)) {
            return true;
        }
        
        // Числовые типы: расширение разрешено (i8 -> i16 -> i32 -> i64)
        if self.is_numeric() && from.is_numeric() {
            // Упрощённая проверка: разрешаем присваивание, если размер from <= размер self
            if let (Some(from_size), Some(self_size)) = (from.size_bytes(), self.size_bytes()) {
                // Для вещественных всегда разрешаем расширение
                if self.is_float() && from.is_numeric() {
                    return from_size <= self_size;
                }
                // Для целых — только если знаковость совпадает или from меньше
                if self.is_integer() && from.is_integer() {
                    if self.is_signed_int() == from.is_signed_int() {
                        return from_size <= self_size;
                    }
                    // Беззнаковое меньшего размера можно присвоить знаковому
                    if self.is_signed_int() && from.is_unsigned_int() {
                        return from_size < self_size;
                    }
                }
            }
        }
        
        false
    }
    
    /// Находит общий тип для двух типов (для операций)
    pub fn common_type(&self, other: &TypeSpec) -> std::option::Option<TypeSpec> {
        if self == other {
            return Some(self.clone());
        }
        
        // Числовые типы: выбираем больший
        if self.is_numeric() && other.is_numeric() {
            let self_size = self.size_bytes()?;
            let other_size = other.size_bytes()?;
            
            // Если есть вещественное — результат вещественный
            if self.is_float() || other.is_float() {
                return Some(if self_size >= other_size { self.clone() } else { other.clone() });
            }
            
            // Оба целые — выбираем больший
            return Some(if self_size >= other_size { self.clone() } else { other.clone() });
        }
        
        // Any совместим со всем
        if matches!(self, TypeSpec::Any) {
            return Some(other.clone());
        }
        if matches!(other, TypeSpec::Any) {
            return Some(self.clone());
        }
        
        None
    }
}

// =============================================================================
// Значение по умолчанию
// =============================================================================

impl Default for TypeSpec {
    fn default() -> Self {
        TypeSpec::Auto
    }
}

// =============================================================================
// Тесты
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_russian_names() {
        assert_eq!(TypeSpec::Int64.russian_name(), "цел");
        assert_eq!(TypeSpec::Float64.russian_name(), "вещ");
        assert_eq!(TypeSpec::Bool.russian_name(), "лог");
        assert_eq!(TypeSpec::String.russian_name(), "лит");
        assert_eq!(TypeSpec::array(TypeSpec::Int64).russian_name(), "таб цел");
    }
    
    #[test]
    fn test_is_numeric() {
        assert!(TypeSpec::Int64.is_numeric());
        assert!(TypeSpec::Float32.is_numeric());
        assert!(!TypeSpec::String.is_numeric());
        assert!(!TypeSpec::Bool.is_numeric());
    }
    
    #[test]
    fn test_size_bytes() {
        assert_eq!(TypeSpec::Int8.size_bytes(), Some(1));
        assert_eq!(TypeSpec::Int32.size_bytes(), Some(4));
        assert_eq!(TypeSpec::Int64.size_bytes(), Some(8));
        assert_eq!(TypeSpec::Float128.size_bytes(), Some(16));
        assert_eq!(TypeSpec::String.size_bytes(), None);
    }
    
    #[test]
    fn test_assignable() {
        // Точное совпадение
        assert!(TypeSpec::Int64.is_assignable_from(&TypeSpec::Int64));
        
        // Расширение
        assert!(TypeSpec::Int64.is_assignable_from(&TypeSpec::Int32));
        assert!(TypeSpec::Float64.is_assignable_from(&TypeSpec::Int32));
        
        // Null -> Option
        assert!(TypeSpec::option(TypeSpec::Int64).is_assignable_from(&TypeSpec::Null));
        
        // Any принимает всё
        assert!(TypeSpec::Any.is_assignable_from(&TypeSpec::Int64));
        assert!(TypeSpec::Any.is_assignable_from(&TypeSpec::String));
    }
    
    #[test]
    fn test_display() {
        assert_eq!(format!("{}", TypeSpec::Int64), "i64");
        assert_eq!(format!("{}", TypeSpec::array(TypeSpec::Int64)), "[i64]");
        assert_eq!(format!("{}", TypeSpec::option(TypeSpec::String)), "String?");
    }
    
    #[test]
    fn test_compatibility_aliases() {
        // Проверяем что старые алиасы работают
        assert_eq!(TypeSpec::Int, TypeSpec::Int64);
        assert_eq!(TypeSpec::Float, TypeSpec::Float64);
        assert_eq!(TypeSpec::None, TypeSpec::Void);
    }
}
