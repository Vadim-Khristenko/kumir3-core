//! Constructors, type-checking predicates, accessors and collection operations
//! for [`Value`], plus the `From` conversions into `Value`.

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::super::number::Number;
use super::super::registry::TypeId;
use super::{LambdaValue, TypeKind, Value};
use crate::f128::F128;

// =============================================================================
//         SECTION: VALUE CONSTRUCTORS
// =============================================================================

impl Value {
    // -------------------------------------------------------------------------
    // Scalar Constructors
    // -------------------------------------------------------------------------

    /// Creates an integer value (i64)
    pub fn int(v: i64) -> Self {
        Value::Number(Number::I64(v))
    }

    /// Creates a floating-point value (f64)
    pub fn float(v: f64) -> Self {
        Value::Number(Number::F64(v))
    }

    /// Creates a boolean value
    pub fn bool(v: bool) -> Self {
        Value::Boolean(v)
    }

    /// Creates a character value
    pub fn char(v: char) -> Self {
        Value::Char(v)
    }

    /// Creates a string value
    pub fn string(v: impl Into<String>) -> Self {
        Value::String(v.into())
    }

    // -------------------------------------------------------------------------
    // Collection Constructors
    // -------------------------------------------------------------------------

    /// Creates an array from iterator
    pub fn array(iter: impl IntoIterator<Item = Value>) -> Self {
        Value::Array(iter.into_iter().collect())
    }

    /// Creates an integer range value with the given step.
    pub fn range(start: i64, end: i64, inclusive: bool, step: i64) -> Self {
        Value::Range {
            start,
            end,
            inclusive,
            step,
        }
    }

    /// Creates a byte buffer value.
    pub fn bytes(data: impl Into<Vec<u8>>) -> Self {
        Value::Bytes(data.into())
    }

    /// Returns the byte slice if this value is a byte buffer.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        if let Value::Bytes(b) = self {
            Some(b)
        } else {
            None
        }
    }

    /// Creates a pair
    pub fn pair(a: Value, b: Value) -> Self {
        Value::Pair(Box::new(a), Box::new(b))
    }

    /// Creates a triple
    pub fn triple(a: Value, b: Value, c: Value) -> Self {
        Value::Triple(Box::new(a), Box::new(b), Box::new(c))
    }

    /// Creates a tuple from iterator
    pub fn tuple(iter: impl IntoIterator<Item = Value>) -> Self {
        Value::Tuple(iter.into_iter().collect())
    }

    /// Creates a set from iterator
    pub fn set(iter: impl IntoIterator<Item = Value>) -> Self {
        Value::Set(iter.into_iter().collect())
    }

    /// Creates a map from iterator of key-value pairs
    pub fn map(iter: impl IntoIterator<Item = (Value, Value)>) -> Self {
        Value::Map(iter.into_iter().collect())
    }

    // -------------------------------------------------------------------------
    // Wrapper Constructors
    // -------------------------------------------------------------------------

    /// Creates Some(value)
    pub fn some(v: Value) -> Self {
        Value::Option(Box::new(Some(v)))
    }

    /// Creates None
    pub fn none() -> Self {
        Value::Option(Box::new(None))
    }

    /// Creates Ok(value)
    pub fn ok(v: Value) -> Self {
        Value::Result(Box::new(Ok(v)))
    }

    /// Creates Err(value)
    pub fn err(e: Value) -> Self {
        Value::Result(Box::new(Err(e)))
    }

    /// Creates a pointer to value
    pub fn pointer(v: Value) -> Self {
        Value::Pointer(Box::new(v))
    }

    /// Creates a reference
    pub fn reference(v: Value, mutable: bool) -> Self {
        Value::Reference {
            target: Box::new(v),
            mutable,
        }
    }

    // -------------------------------------------------------------------------
    // Enum/Object Constructors
    // -------------------------------------------------------------------------

    /// Creates an enum variant
    pub fn enum_variant(
        name: impl Into<String>,
        variant: impl Into<String>,
        data: Option<Value>,
    ) -> Self {
        Value::Enum {
            name: name.into(),
            variant: variant.into(),
            data: data.map(Box::new),
        }
    }

    /// Creates an object
    pub fn object(type_id: TypeId, fields: BTreeMap<String, Value>) -> Self {
        Value::Object { type_id, fields }
    }

    // -------------------------------------------------------------------------
    // Error Constructor
    // -------------------------------------------------------------------------

    /// Creates an error value
    pub fn error(message: impl Into<String>, kind: impl Into<String>) -> Self {
        Value::Error {
            message: message.into(),
            kind: kind.into(),
            source: None,
        }
    }

    /// Creates an error with source
    pub fn error_with_source(
        message: impl Into<String>,
        kind: impl Into<String>,
        source: Value,
    ) -> Self {
        Value::Error {
            message: message.into(),
            kind: kind.into(),
            source: Some(Box::new(source)),
        }
    }
}

// =============================================================================
//         SECTION: TYPE CHECKING HELPERS
// =============================================================================

impl Value {
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }
    pub fn is_boolean(&self) -> bool {
        matches!(self, Value::Boolean(_))
    }
    pub fn is_char(&self) -> bool {
        matches!(self, Value::Char(_))
    }
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }
    pub fn is_pair(&self) -> bool {
        matches!(self, Value::Pair(_, _))
    }
    pub fn is_triple(&self) -> bool {
        matches!(self, Value::Triple(_, _, _))
    }
    pub fn is_tuple(&self) -> bool {
        matches!(self, Value::Tuple(_))
    }
    pub fn is_set(&self) -> bool {
        matches!(self, Value::Set(_))
    }
    pub fn is_map(&self) -> bool {
        matches!(self, Value::Map(_))
    }
    pub fn is_option(&self) -> bool {
        matches!(self, Value::Option(_))
    }
    pub fn is_result(&self) -> bool {
        matches!(self, Value::Result(_))
    }
    pub fn is_pointer(&self) -> bool {
        matches!(self, Value::Pointer(_))
    }
    pub fn is_reference(&self) -> bool {
        matches!(self, Value::Reference { .. })
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
    pub fn is_undefined(&self) -> bool {
        matches!(self, Value::Undefined)
    }
    pub fn is_enum(&self) -> bool {
        matches!(self, Value::Enum { .. })
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Value::Object { .. })
    }
    pub fn is_native_object(&self) -> bool {
        matches!(self, Value::NativeObject { .. })
    }
    pub fn is_lambda(&self) -> bool {
        matches!(self, Value::Lambda(_))
    }
    pub fn is_promise(&self) -> bool {
        matches!(self, Value::Promise { .. })
    }
    pub fn is_generator(&self) -> bool {
        matches!(self, Value::Generator { .. })
    }
    pub fn is_channel(&self) -> bool {
        matches!(self, Value::Channel { .. })
    }
    pub fn is_error(&self) -> bool {
        matches!(self, Value::Error { .. })
    }
    pub fn is_type(&self) -> bool {
        matches!(self, Value::Type(_))
    }

    /// Checks if value is "truthy" (for conditionals)
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Null | Value::Undefined => false,
            Value::Number(n) => !n.is_zero(),
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Option(o) => o.is_some(),
            Value::Result(r) => r.is_ok(),
            _ => true,
        }
    }

    /// Checks if value is "falsy"
    pub fn is_falsy(&self) -> bool {
        !self.is_truthy()
    }

    /// Checks if value is a collection type
    pub fn is_collection(&self) -> bool {
        matches!(
            self,
            Value::Array(_)
                | Value::Tuple(_)
                | Value::Set(_)
                | Value::Map(_)
                | Value::Pair(_, _)
                | Value::Triple(_, _, _)
        )
    }

    /// Checks if value is callable (function, lambda, etc.)
    pub fn is_callable(&self) -> bool {
        matches!(self, Value::Lambda(_) | Value::PartialApp { .. })
    }
}

// =============================================================================
//         SECTION: VALUE EXTRACTION
// =============================================================================

impl Value {
    pub fn as_number(&self) -> Option<&Number> {
        if let Value::Number(n) = self {
            Some(n)
        } else {
            None
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Value::String(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Boolean(b) => Some(if *b { "да" } else { "нет" }.to_string()),
            Value::Char(c) => Some(c.to_string()),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Value::Number(n) => n.to_i64(),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Number(n) => n.to_f64(),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Boolean(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    pub fn as_char(&self) -> Option<char> {
        if let Value::Char(c) = self {
            Some(*c)
        } else {
            None
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        if let Value::Array(arr) = self {
            Some(arr)
        } else {
            None
        }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Value>> {
        if let Value::Array(arr) = self {
            Some(arr)
        } else {
            None
        }
    }

    pub fn as_tuple(&self) -> Option<&Vec<Value>> {
        if let Value::Tuple(items) = self {
            Some(items)
        } else {
            None
        }
    }

    pub fn as_map(&self) -> Option<&BTreeMap<Value, Value>> {
        if let Value::Map(m) = self {
            Some(m)
        } else {
            None
        }
    }

    pub fn as_set(&self) -> Option<&BTreeSet<Value>> {
        if let Value::Set(s) = self {
            Some(s)
        } else {
            None
        }
    }

    pub fn as_option(&self) -> Option<&Option<Value>> {
        if let Value::Option(o) = self {
            Some(o.as_ref())
        } else {
            None
        }
    }

    pub fn as_result(&self) -> Option<&Result<Value, Value>> {
        if let Value::Result(r) = self {
            Some(r.as_ref())
        } else {
            None
        }
    }

    pub fn as_native_object(&self) -> Option<&Arc<dyn Any + Send + Sync>> {
        if let Value::NativeObject { object, .. } = self {
            Some(object)
        } else {
            None
        }
    }

    pub fn as_lambda(&self) -> Option<&LambdaValue> {
        if let Value::Lambda(l) = self {
            Some(l.as_ref())
        } else {
            None
        }
    }

    pub fn as_type(&self) -> Option<&TypeKind> {
        if let Value::Type(t) = self {
            Some(t)
        } else {
            None
        }
    }

    /// Gets the type_id for objects
    pub fn type_id(&self) -> Option<TypeId> {
        match self {
            Value::Object { type_id, .. } => Some(*type_id),
            Value::NativeObject { type_id, .. } => Some(*type_id),
            _ => None,
        }
    }

    /// Gets a field from an object
    pub fn get_field(&self, name: &str) -> Option<&Value> {
        if let Value::Object { fields, .. } = self {
            fields.get(name)
        } else {
            None
        }
    }

    /// Sets a field on an object
    pub fn set_field(&mut self, name: &str, value: Value) -> Result<(), String> {
        if let Value::Object { fields, .. } = self {
            fields.insert(name.to_string(), value);
            Ok(())
        } else {
            Err("Value is not an object".to_string())
        }
    }

    /// Unwraps option, panics if None
    pub fn unwrap_option(self) -> Value {
        if let Value::Option(o) = self {
            o.expect("Called unwrap_option on None")
        } else {
            panic!("Called unwrap_option on non-Option value")
        }
    }

    /// Unwraps result, panics if Err
    pub fn unwrap_result(self) -> Value {
        if let Value::Result(r) = self {
            match *r {
                Ok(v) => v,
                Err(e) => panic!("Called unwrap_result on Err: {}", e),
            }
        } else {
            panic!("Called unwrap_result on non-Result value")
        }
    }

    /// Dereferences a pointer or reference
    pub fn deref(&self) -> Option<&Value> {
        match self {
            Value::Pointer(inner) => Some(inner.as_ref()),
            Value::Reference { target, .. } => Some(target.as_ref()),
            _ => None,
        }
    }
}

// =============================================================================
//         SECTION: COLLECTION OPERATIONS
// =============================================================================

impl Value {
    /// Gets the length of a collection
    pub fn len(&self) -> Option<usize> {
        match self {
            Value::String(s) => Some(s.len()),
            Value::Array(a) => Some(a.len()),
            Value::Range {
                start,
                end,
                inclusive,
                step,
            } => {
                if *step == 0 {
                    return Some(0);
                }
                let last = if *step > 0 {
                    if *inclusive { *end } else { *end - 1 }
                } else {
                    if *inclusive { *end } else { *end + 1 }
                };
                let n = if *step > 0 && start <= &last {
                    (last - start) / step + 1
                } else if *step < 0 && start >= &last {
                    (start - last) / (-step) + 1
                } else {
                    0
                };
                Some(n.max(0) as usize)
            }
            Value::Bytes(b) => Some(b.len()),
            Value::Tuple(t) => Some(t.len()),
            Value::Set(s) => Some(s.len()),
            Value::Map(m) => Some(m.len()),
            _ => None,
        }
    }

    /// Checks if collection is empty
    pub fn is_empty(&self) -> Option<bool> {
        self.len().map(|l| l == 0)
    }

    /// Gets element by index
    pub fn get(&self, index: usize) -> Option<&Value> {
        match self {
            Value::Array(a) => a.get(index),
            Value::Tuple(t) => t.get(index),
            _ => None,
        }
    }

    /// Gets element by key (for maps)
    pub fn get_by_key(&self, key: &Value) -> Option<&Value> {
        if let Value::Map(m) = self {
            m.get(key)
        } else {
            None
        }
    }

    /// Checks if collection contains a value
    pub fn contains(&self, item: &Value) -> bool {
        match self {
            Value::Array(a) => a.contains(item),
            Value::Set(s) => s.contains(item),
            Value::Map(m) => m.contains_key(item),
            _ => false,
        }
    }
}

// =============================================================================
//         SECTION: CLONE OPERATIONS (for ownership)
// =============================================================================

impl Value {
    /// Deep clone the value
    pub fn deep_clone(&self) -> Self {
        self.clone()
    }

    /// Shallow copy (for Copy types)
    pub fn shallow_copy(&self) -> Option<Self> {
        match self {
            Value::Number(n) => Some(Value::Number(n.clone())),
            Value::Boolean(b) => Some(Value::Boolean(*b)),
            Value::Char(c) => Some(Value::Char(*c)),
            Value::Null => Some(Value::Null),
            _ => None, // Not a Copy type
        }
    }
}

// =============================================================================
//         SECTION: FROM IMPLEMENTATIONS
// =============================================================================

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

impl From<Number> for Value {
    fn from(n: Number) -> Self {
        Value::Number(n)
    }
}
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}
impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_owned())
    }
}
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Boolean(b)
    }
}
impl From<char> for Value {
    fn from(c: char) -> Self {
        Value::Char(c)
    }
}
impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Array(v.into_iter().map(Into::into).collect())
    }
}
impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(o: Option<T>) -> Self {
        Value::Option(Box::new(o.map(Into::into)))
    }
}
impl From<TypeKind> for Value {
    fn from(t: TypeKind) -> Self {
        Value::Type(t)
    }
}
