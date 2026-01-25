//! Значения времени выполнения (Value)
//! 
//! В Kumir 3 поддерживается автоматический вывод типа (авто), поэтому
//! Value может содержать любой допустимый тип данных.

use std::fmt;
use std::collections::{BTreeMap, BTreeSet};
use std::any::Any;
use std::cmp::Ordering;
use std::sync::Arc;

use super::number::Number;
use super::registry::TypeId;
use crate::shared::f128::F128;

/// Универсальное значение времени выполнения.
#[derive(Debug, Clone)]
pub enum Value {
    // -------------------------------------------------------------------------
    // Базовые скалярные типы
    // -------------------------------------------------------------------------
    Number(Number),     // числовые типы (цел, вещ, ...)
    String(String),     // лит (строка)
    Boolean(bool),      // лог (да/нет)
    Char(char),         // сим (символ)

    // -------------------------------------------------------------------------
    // Коллекции
    // -------------------------------------------------------------------------
    Array(Vec<Value>),                      // таб (массив)
    Pair(Box<Value>, Box<Value>),           // пара (T, U)
    Triple(Box<Value>, Box<Value>, Box<Value>), // тройка (T, U, V)
    Tuple(Vec<Value>),                      // кортеж (T1, T2, ..., Tn)
    Set(BTreeSet<Value>),                   // множество
    Map(BTreeMap<Value, Value>),            // словарь / ассоциативный массив

    // -------------------------------------------------------------------------
    // Обёртки (Option/Result)
    // -------------------------------------------------------------------------
    Option(Box<Option<Value>>),             // опция: Некоторое(x) | Ничего
    Result(Box<Result<Value, Value>>),      // результат: Успех(x) | Ошибка(e)

    // -------------------------------------------------------------------------
    // Kumir 3: Указатели и перечисления
    // -------------------------------------------------------------------------
    Pointer(Box<Value>),                    // указатель на значение
    Enum {                                  // перечисление
        name: String,                       // имя типа перечисления
        variant: String,                    // выбранный вариант
        data: Option<Box<Value>>,           // ассоциированные данные (если есть)
    },

    // -------------------------------------------------------------------------
    // Объекты (Kumir 3) - экземпляры классов
    // -------------------------------------------------------------------------
    /// Объект КуМир-класса
    Object {
        type_id: TypeId,                    // ID типа в TypeRegistry
        fields: BTreeMap<String, Value>,    // поля объекта
    },

    // -------------------------------------------------------------------------
    // Нативные объекты (HTTP сервер, файлы, сокеты и т.д.)
    // -------------------------------------------------------------------------
    NativeObject {
        type_id: TypeId,                    // ID типа в TypeRegistry
        type_name: String,                  // Имя типа (для отладки и переходного периода)
        object: Arc<dyn Any + Send + Sync>, // нативный объект Rust
    },

    // -------------------------------------------------------------------------
    // Асинхронное программирование (Kumir 3)
    // -------------------------------------------------------------------------
    /// Promise - отложенный результат асинхронной операции
    Promise {
        task_id: u64,                       // ID задачи в runtime
        /// Статус: pending, resolved, rejected
        status: PromiseStatus,
        /// Результат (если resolved)
        result: Option<Box<Value>>,
        /// Ошибка (если rejected)
        error: Option<String>,
    },

    // -------------------------------------------------------------------------
    // Специальные значения
    // -------------------------------------------------------------------------
    Null,       // null / пусто
    Undefined,  // неопределено (неинициализированная переменная)
}

/// Статус Promise
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromiseStatus {
    /// Ожидает выполнения
    Pending,
    /// Успешно выполнен
    Resolved,
    /// Завершился с ошибкой
    Rejected,
}

// Реализация PartialEq для Value (NativeObject сравнивается по типу и указателю)
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (Value::Pair(a1, a2), Value::Pair(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Triple(a1, a2, a3), Value::Triple(b1, b2, b3)) => a1 == b1 && a2 == b2 && a3 == b3,
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Option(a), Value::Option(b)) => a == b,
            (Value::Result(a), Value::Result(b)) => a == b,
            (Value::Pointer(a), Value::Pointer(b)) => a == b,
            (Value::Enum { name: n1, variant: v1, data: d1 }, 
             Value::Enum { name: n2, variant: v2, data: d2 }) => n1 == n2 && v1 == v2 && d1 == d2,
            (Value::Object { type_id: t1, fields: f1 },
             Value::Object { type_id: t2, fields: f2 }) => t1 == t2 && f1 == f2,
            (Value::NativeObject { type_id: t1, object: o1, .. }, 
             Value::NativeObject { type_id: t2, object: o2, .. }) => t1 == t2 && Arc::ptr_eq(o1, o2),
            (Value::Promise { task_id: t1, status: s1, .. },
             Value::Promise { task_id: t2, status: s2, .. }) => t1 == t2 && s1 == s2,
            (Value::Null, Value::Null) => true,
            (Value::Undefined, Value::Undefined) => true,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        fn tag(v: &Value) -> u8 {
            match v {
                Value::Number(_) => 0,
                Value::String(_) => 1,
                Value::Boolean(_) => 2,
                Value::Char(_) => 3,
                Value::Array(_) => 4,
                Value::Pair(_, _) => 5,
                Value::Triple(_, _, _) => 6,
                Value::Tuple(_) => 7,
                Value::Set(_) => 8,
                Value::Map(_) => 9,
                Value::Option(_) => 10,
                Value::Result(_) => 11,
                Value::Pointer(_) => 12,
                Value::Enum { .. } => 13,
                Value::Object { .. } => 14,
                Value::NativeObject { .. } => 15,
                Value::Promise { .. } => 16,
                Value::Null => 17,
                Value::Undefined => 18,
            }
        }

        let d = tag(self).cmp(&tag(other));
        if d != Ordering::Equal {
            return d;
        }
        self.to_string().cmp(&other.to_string())
    }
}

// --- From implementations ---

macro_rules! impl_from_number_for_value {
    ($($t:ty => $v:ident),+ $(,)?) => {
        $(
            impl From<$t> for Value { 
                fn from(v: $t) -> Self { Value::Number(Number::$v(v)) } 
            }
        )+
    };
}

impl_from_number_for_value!(
    i8 => I8, i16 => I16, i32 => I32, i64 => I64, i128 => I128,
    u8 => U8, u16 => U16, u32 => U32, u64 => U64, u128 => U128,
    f32 => F32, f64 => F64, F128 => F128
);

impl From<Number> for Value { fn from(n: Number) -> Self { Value::Number(n) } }
impl From<String> for Value { fn from(s: String) -> Self { Value::String(s) } }
impl From<&str> for Value { fn from(s: &str) -> Self { Value::String(s.to_owned()) } }
impl From<bool> for Value { fn from(b: bool) -> Self { Value::Boolean(b) } }
impl From<char> for Value { fn from(c: char) -> Self { Value::Char(c) } }

// --- Display Implementation ---

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => match n {
                Number::I8(v) => write!(f, "{}", v),
                Number::I16(v) => write!(f, "{}", v),
                Number::I32(v) => write!(f, "{}", v),
                Number::I64(v) => write!(f, "{}", v),
                Number::I128(v) => write!(f, "{}", v),
                Number::U8(v) => write!(f, "{}", v),
                Number::U16(v) => write!(f, "{}", v),
                Number::U32(v) => write!(f, "{}", v),
                Number::U64(v) => write!(f, "{}", v),
                Number::U128(v) => write!(f, "{}", v),
                Number::F32(x) => write!(f, "{}", x),
                Number::F64(x) => write!(f, "{}", x),
                Number::F128(x) => write!(f, "{}", x),
            },
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::Array(a) => write_collection(f, "[", "]", a),
            Value::Tuple(a) => write_collection(f, "(", ")", a),
            Value::Set(s) => write_collection(f, "{", "}", s),
            Value::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Pair(a, b) => write!(f, "({}, {})", a, b),
            Value::Triple(a, b, c) => write!(f, "({}, {}, {})", a, b, c),
            Value::Option(o) => match o.as_ref() {
                Some(v) => write!(f, "Некоторое({})", v),
                None => write!(f, "Ничего"),
            },
            Value::Result(r) => match r.as_ref() {
                Ok(v) => write!(f, "Успех({})", v),
                Err(e) => write!(f, "Ошибка({})", e),
            },
            Value::Pointer(p) => write!(f, "&{}", p),
            Value::Enum { name, variant, data } => {
                write!(f, "{}::{}", name, variant)?;
                if let Some(d) = data {
                    write!(f, "({})", d)?;
                }
                Ok(())
            },
            Value::Object { type_id, fields } => {
                write!(f, "<Объект #{} {{", type_id.0)?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}>")
            },
            Value::NativeObject { type_name, .. } => write!(f, "<{}>", type_name),
            Value::Promise { task_id, status, result, error } => {
                match status {
                    PromiseStatus::Pending => write!(f, "<Promise #{} pending>", task_id),
                    PromiseStatus::Resolved => {
                        if let Some(v) = result {
                            write!(f, "<Promise #{} resolved: {}>", task_id, v)
                        } else {
                            write!(f, "<Promise #{} resolved>", task_id)
                        }
                    }
                    PromiseStatus::Rejected => {
                        if let Some(e) = error {
                            write!(f, "<Promise #{} rejected: {}>", task_id, e)
                        } else {
                            write!(f, "<Promise #{} rejected>", task_id)
                        }
                    }
                }
            }
            Value::Null => write!(f, "пусто"),
            Value::Undefined => write!(f, "неопределено"),
        }
    }
}

fn write_collection<T: fmt::Display, I: IntoIterator<Item = T>>(
    f: &mut fmt::Formatter<'_>, 
    start: &str, 
    end: &str, 
    iter: I
) -> fmt::Result {
    write!(f, "{}", start)?;
    for (i, v) in iter.into_iter().enumerate() {
        if i > 0 { write!(f, ", ")?; }
        write!(f, "{}", v)?;
    }
    write!(f, "{}", end)
}

// --- Helper Methods ---

impl Value {
    // Type checks
    pub fn is_number(&self) -> bool { matches!(self, Value::Number(_)) }
    pub fn is_string(&self) -> bool { matches!(self, Value::String(_)) }
    pub fn is_boolean(&self) -> bool { matches!(self, Value::Boolean(_)) }
    pub fn is_char(&self) -> bool { matches!(self, Value::Char(_)) }
    pub fn is_array(&self) -> bool { matches!(self, Value::Array(_)) }
    pub fn is_pair(&self) -> bool { matches!(self, Value::Pair(_, _)) }
    pub fn is_tuple(&self) -> bool { matches!(self, Value::Tuple(_)) }
    pub fn is_set(&self) -> bool { matches!(self, Value::Set(_)) }
    pub fn is_map(&self) -> bool { matches!(self, Value::Map(_)) }
    pub fn is_option(&self) -> bool { matches!(self, Value::Option(_)) }
    pub fn is_result(&self) -> bool { matches!(self, Value::Result(_)) }
    pub fn is_null(&self) -> bool { matches!(self, Value::Null) }
    pub fn is_undefined(&self) -> bool { matches!(self, Value::Undefined) }
    pub fn is_pointer(&self) -> bool { matches!(self, Value::Pointer(_)) }
    pub fn is_enum(&self) -> bool { matches!(self, Value::Enum { .. }) }
    pub fn is_object(&self) -> bool { matches!(self, Value::Object { .. }) }
    pub fn is_native_object(&self) -> bool { matches!(self, Value::NativeObject { .. }) }

    // Conversions
    pub fn as_number(&self) -> Option<&Number> {
        if let Value::Number(n) = self { Some(n) } else { None }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Boolean(b) => Some(b.to_string()),
            Value::Char(c) => Some(c.to_string()),
            Value::Pair(left, right) => {
                match (left.as_string(), right.as_string()) {
                    (Some(l), Some(r)) => Some(format!("({}, {})", l, r)),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Получить значение как целое число
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Number(n) => n.to_i64(),
            _ => None,
        }
    }

    /// Получить значение как вещественное число
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Number(n) => n.to_f64(),
            _ => None,
        }
    }

    /// Получить значение как логическое
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Получить нативный объект (возвращает Arc<dyn Any + Send + Sync>)
    pub fn as_native_object(&self) -> Option<&Arc<dyn Any + Send + Sync>> {
        match self {
            Value::NativeObject { object, .. } => Some(object),
            _ => None,
        }
    }

    /// Получить массив
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Получить type_id для объектов
    pub fn type_id(&self) -> Option<TypeId> {
        match self {
            Value::Object { type_id, .. } => Some(*type_id),
            Value::NativeObject { type_id, .. } => Some(*type_id),
            _ => None,
        }
    }

    /// Получить поле объекта
    pub fn get_field(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Object { fields, .. } => fields.get(name),
            _ => None,
        }
    }

    /// Установить поле объекта
    pub fn set_field(&mut self, name: &str, value: Value) -> Result<(), String> {
        match self {
            Value::Object { fields, .. } => {
                fields.insert(name.to_string(), value);
                Ok(())
            }
            _ => Err("Невозможно установить поле: значение не является объектом".to_string()),
        }
    }
}
