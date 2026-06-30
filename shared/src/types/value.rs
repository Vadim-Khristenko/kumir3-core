//! Runtime Values and Type System for Kumir 3.
//!
//! [STABLE] This module provides a unified type system where `Value` serves as both
//! runtime value container and type descriptor. The `TypeKind` enum describes types
//! statically, while `Value` holds actual values at runtime.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                        Value (Runtime)                          │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Scalar: Number, String, Boolean, Char                          │
//! │  Collections: Array, Tuple, Set, Map                            │
//! │  Wrappers: Option, Result, Pointer, Reference                   │
//! │  Objects: Enum, Object, NativeObject                            │
//! │  Functional: Lambda, Closure                                    │
//! │  Async: Promise, Generator, Channel                             │
//! │  Special: Null, Undefined, Type (for reflection)                │
//! └─────────────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      TypeKind (Static)                          │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Mirrors Value variants for compile-time type checking          │
//! │  Used in AST, parser, semantic analysis                         │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use std::any::Any;
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::number::Number;
use super::registry::TypeId;
use crate::f128::F128;

// =============================================================================
//         SECTION: TYPE KIND (STATIC TYPE DESCRIPTOR)
// =============================================================================

/// [STABLE] Static type descriptor for compile-time type checking.
///
/// Replaces the old `TypeSpec` with a unified approach. Use this in AST nodes,
/// function signatures, and variable declarations.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum TypeKind {
    // -------------------------------------------------------------------------
    // Numeric Types
    // -------------------------------------------------------------------------
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    Float32,
    Float64,
    Float128,

    // -------------------------------------------------------------------------
    // Basic Scalar Types
    // -------------------------------------------------------------------------
    String,
    Bool,
    Char,

    // -------------------------------------------------------------------------
    // Collections
    // -------------------------------------------------------------------------
    Array(Box<TypeKind>),
    /// Range of an ordered element type: `диапазон<цел>` (e.g. `1..10`).
    Range(Box<TypeKind>),
    Pair(Box<TypeKind>, Box<TypeKind>),
    Triple(Box<TypeKind>, Box<TypeKind>, Box<TypeKind>),
    Tuple(Vec<TypeKind>),
    Set(Box<TypeKind>),
    Map(Box<TypeKind>, Box<TypeKind>),

    // -------------------------------------------------------------------------
    // Wrappers
    // -------------------------------------------------------------------------
    Option(Box<TypeKind>),
    Result {
        ok: Box<TypeKind>,
        err: Box<TypeKind>,
    },
    Pointer(Box<TypeKind>),
    Reference {
        inner: Box<TypeKind>,
        mutable: bool,
    },

    // -------------------------------------------------------------------------
    // User-Defined Types
    // -------------------------------------------------------------------------
    Enum(String),
    Object(String),
    Native(String),
    /// Parameterised (generic) type: `Список<цел>`, `Словарь<лит, цел>`.
    Generic {
        name: String,
        type_args: Vec<TypeKind>,
    },

    // -------------------------------------------------------------------------
    // Functional Types
    // -------------------------------------------------------------------------
    Function {
        params: Vec<TypeKind>,
        result: Option<Box<TypeKind>>,
    },
    Lambda {
        params: Vec<TypeKind>,
        result: Option<Box<TypeKind>>,
        captures: Vec<String>,
    },

    // -------------------------------------------------------------------------
    // Async Types
    // -------------------------------------------------------------------------
    Promise(Box<TypeKind>),
    Generator {
        yield_type: Box<TypeKind>,
        return_type: Box<TypeKind>,
    },
    Channel(Box<TypeKind>),

    // -------------------------------------------------------------------------
    // Special Types
    // -------------------------------------------------------------------------
    Null,
    Undefined,
    #[default]
    Auto,
    Any,
    Void,
    Never,
    Type, // For reflection: holds a TypeKind as a value
}

// =============================================================================
//         SECTION: OWNERSHIP & MUTABILITY MODIFIERS
// =============================================================================

/// [EXPERIMENTAL] Ownership semantics for values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Ownership {
    /// Value is owned (default)
    #[default]
    Owned,
    /// Value is borrowed immutably
    Borrowed,
    /// Value is borrowed mutably
    BorrowedMut,
    /// Value was moved (invalidated)
    Moved,
}

/// [EXPERIMENTAL] Value metadata for ownership tracking.
#[derive(Debug, Clone, PartialEq)]
pub struct ValueMeta {
    /// Type of the value
    pub type_kind: TypeKind,
    /// Ownership state
    pub ownership: Ownership,
    /// Is value mutable
    pub mutable: bool,
    /// Source location (for error messages)
    pub source_loc: Option<(usize, usize)>,
}

impl Default for ValueMeta {
    fn default() -> Self {
        Self {
            type_kind: TypeKind::Auto,
            ownership: Ownership::Owned,
            mutable: false,
            source_loc: None,
        }
    }
}

// =============================================================================
//         SECTION: PROMISE STATUS
// =============================================================================

/// Promise execution status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromiseStatus {
    Pending,
    Resolved,
    Rejected,
}

// =============================================================================
//         SECTION: GENERATOR STATE
// =============================================================================

/// [EXPERIMENTAL] Generator execution state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeneratorState {
    /// Not started yet
    Created,
    /// Yielded a value, can be resumed
    Suspended,
    /// Completed with a return value
    Completed,
    /// Failed with an error
    Failed,
}

// =============================================================================
//         SECTION: LAMBDA / CLOSURE
// =============================================================================

/// [STABLE] Lambda function representation.
#[derive(Debug, Clone)]
pub struct LambdaValue {
    /// Parameter names
    pub params: Vec<String>,
    /// Parameter types (optional, for type inference)
    pub param_types: Vec<Option<TypeKind>>,
    /// Return type (optional)
    pub return_type: Option<TypeKind>,
    /// Body expression index (points to AST)
    pub body_id: usize,
    /// Captured variables from enclosing scope
    pub captures: BTreeMap<String, Value>,
}

impl PartialEq for LambdaValue {
    fn eq(&self, other: &Self) -> bool {
        self.body_id == other.body_id && self.params == other.params
    }
}

// =============================================================================
//         SECTION: VALUE (RUNTIME VALUE)
// =============================================================================

/// [STABLE] Universal runtime value for Kumir 3.
///
/// # Design Principles
///
/// 1. **Unified**: Holds any value the language can express
/// 2. **Self-describing**: Can report its own type via `type_kind()`
/// 3. **Extensible**: New variants can be added for language extensions
/// 4. **Efficient**: Uses `Box` for recursive types to minimize stack usage
#[derive(Debug, Clone)]
pub enum Value {
    // -------------------------------------------------------------------------
    // Basic Scalar Types
    // -------------------------------------------------------------------------
    Number(Number),
    String(String),
    Boolean(bool),
    Char(char),

    // -------------------------------------------------------------------------
    // Collections
    // -------------------------------------------------------------------------
    Array(Vec<Value>),
    /// Integer range value: `1..10` (exclusive) or `1..=10` (inclusive),
    /// optionally with step (`1..10 шаг 2`).
    Range {
        start: i64,
        end: i64,
        inclusive: bool,
        step: i64,
    },
    /// Byte buffer: `байты` (e.g. UTF-8 bytes of a string, file/network data).
    Bytes(Vec<u8>),
    Pair(Box<Value>, Box<Value>),
    Triple(Box<Value>, Box<Value>, Box<Value>),
    Tuple(Vec<Value>),
    Set(BTreeSet<Value>),
    Map(BTreeMap<Value, Value>),

    // -------------------------------------------------------------------------
    // Wrappers
    // -------------------------------------------------------------------------
    Option(Box<Option<Value>>),
    Result(Box<Result<Value, Value>>),
    Pointer(Box<Value>),
    /// [EXPERIMENTAL] Reference with mutability tracking
    Reference {
        target: Box<Value>,
        mutable: bool,
    },

    // -------------------------------------------------------------------------
    // User-Defined Types
    // -------------------------------------------------------------------------
    Enum {
        name: String,
        variant: String,
        data: Option<Box<Value>>,
    },
    Object {
        type_id: TypeId,
        fields: BTreeMap<String, Value>,
    },
    NativeObject {
        type_id: TypeId,
        type_name: String,
        object: Arc<dyn Any + Send + Sync>,
    },

    // -------------------------------------------------------------------------
    // Functional Values
    // -------------------------------------------------------------------------
    /// Lambda / anonymous function
    Lambda(Box<LambdaValue>),
    /// [EXPERIMENTAL] Partial application
    PartialApp {
        func: Box<Value>,
        applied_args: Vec<Value>,
    },

    // -------------------------------------------------------------------------
    // Async Values
    // -------------------------------------------------------------------------
    Promise {
        task_id: u64,
        status: PromiseStatus,
        result: Option<Box<Value>>,
        error: Option<String>,
    },
    /// [EXPERIMENTAL] Generator / iterator
    Generator {
        id: u64,
        state: GeneratorState,
        current_value: Option<Box<Value>>,
    },
    /// [EXPERIMENTAL] Channel for concurrent communication
    Channel {
        id: u64,
        capacity: usize,
        closed: bool,
    },

    // -------------------------------------------------------------------------
    // Special Values
    // -------------------------------------------------------------------------
    Null,
    Undefined,
    /// Holds a type as a first-class value (for reflection)
    Type(TypeKind),
    /// [EXPERIMENTAL] Error value for propagation
    Error {
        message: String,
        kind: String,
        source: Option<Box<Value>>,
    },
}

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
//         SECTION: TYPE INTROSPECTION
// =============================================================================

impl Value {
    /// Returns the static type descriptor for this value.
    pub fn type_kind(&self) -> TypeKind {
        match self {
            Value::Number(n) => match n {
                Number::I8(_) => TypeKind::Int8,
                Number::I16(_) => TypeKind::Int16,
                Number::I32(_) => TypeKind::Int32,
                Number::I64(_) => TypeKind::Int64,
                Number::I128(_) => TypeKind::Int128,
                Number::U8(_) => TypeKind::UInt8,
                Number::U16(_) => TypeKind::UInt16,
                Number::U32(_) => TypeKind::UInt32,
                Number::U64(_) => TypeKind::UInt64,
                Number::U128(_) => TypeKind::UInt128,
                Number::F32(_) => TypeKind::Float32,
                Number::F64(_) => TypeKind::Float64,
                Number::F128(_) => TypeKind::Float128,
            },
            Value::String(_) => TypeKind::String,
            Value::Boolean(_) => TypeKind::Bool,
            Value::Char(_) => TypeKind::Char,
            Value::Array(arr) => {
                let elem_type = arr.first().map(|v| v.type_kind()).unwrap_or(TypeKind::Any);
                TypeKind::Array(Box::new(elem_type))
            }
            Value::Range { .. } => TypeKind::Range(Box::new(TypeKind::Int64)),
            Value::Bytes(_) => TypeKind::Array(Box::new(TypeKind::UInt8)),
            Value::Pair(a, b) => TypeKind::Pair(Box::new(a.type_kind()), Box::new(b.type_kind())),
            Value::Triple(a, b, c) => TypeKind::Triple(
                Box::new(a.type_kind()),
                Box::new(b.type_kind()),
                Box::new(c.type_kind()),
            ),
            Value::Tuple(items) => TypeKind::Tuple(items.iter().map(|v| v.type_kind()).collect()),
            Value::Set(s) => {
                let elem_type = s.first().map(|v| v.type_kind()).unwrap_or(TypeKind::Any);
                TypeKind::Set(Box::new(elem_type))
            }
            Value::Map(m) => {
                let (k, v) = m
                    .first_key_value()
                    .map(|(k, v)| (k.type_kind(), v.type_kind()))
                    .unwrap_or((TypeKind::Any, TypeKind::Any));
                TypeKind::Map(Box::new(k), Box::new(v))
            }
            Value::Option(inner) => {
                let inner_type = inner
                    .as_ref()
                    .as_ref()
                    .map(|v| v.type_kind())
                    .unwrap_or(TypeKind::Any);
                TypeKind::Option(Box::new(inner_type))
            }
            Value::Result(inner) => {
                let (ok, err) = match inner.as_ref() {
                    Ok(v) => (v.type_kind(), TypeKind::Any),
                    Err(e) => (TypeKind::Any, e.type_kind()),
                };
                TypeKind::Result {
                    ok: Box::new(ok),
                    err: Box::new(err),
                }
            }
            Value::Pointer(inner) => TypeKind::Pointer(Box::new(inner.type_kind())),
            Value::Reference { target, mutable } => TypeKind::Reference {
                inner: Box::new(target.type_kind()),
                mutable: *mutable,
            },
            Value::Enum { name, .. } => TypeKind::Enum(name.clone()),
            Value::Object { type_id, .. } => TypeKind::Object(format!("#{}", type_id.0)),
            Value::NativeObject { type_name, .. } => TypeKind::Native(type_name.clone()),
            Value::Lambda(lambda) => TypeKind::Lambda {
                params: lambda
                    .param_types
                    .iter()
                    .map(|t| t.clone().unwrap_or(TypeKind::Any))
                    .collect(),
                result: lambda.return_type.clone().map(Box::new),
                captures: lambda.captures.keys().cloned().collect(),
            },
            Value::PartialApp { func, .. } => func.type_kind(),
            Value::Promise { .. } => TypeKind::Promise(Box::new(TypeKind::Any)),
            Value::Generator { .. } => TypeKind::Generator {
                yield_type: Box::new(TypeKind::Any),
                return_type: Box::new(TypeKind::Any),
            },
            Value::Channel { .. } => TypeKind::Channel(Box::new(TypeKind::Any)),
            Value::Null => TypeKind::Null,
            Value::Undefined => TypeKind::Undefined,
            Value::Type(_) => TypeKind::Type,
            Value::Error { .. } => TypeKind::Result {
                ok: Box::new(TypeKind::Never),
                err: Box::new(TypeKind::String),
            },
        }
    }

    /// Returns the Russian name of this value's type.
    pub fn type_name_ru(&self) -> String {
        self.type_kind().russian_name()
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
//         SECTION: TRAIT IMPLEMENTATIONS
// =============================================================================

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => a == b,
            (
                Value::Range {
                    start: s1,
                    end: e1,
                    inclusive: i1,
                    step: p1,
                },
                Value::Range {
                    start: s2,
                    end: e2,
                    inclusive: i2,
                    step: p2,
                },
            ) => s1 == s2 && e1 == e2 && i1 == i2 && p1 == p2,
            (Value::Bytes(a), Value::Bytes(b)) => a == b,
            (Value::Pair(a1, a2), Value::Pair(b1, b2)) => a1 == b1 && a2 == b2,
            (Value::Triple(a1, a2, a3), Value::Triple(b1, b2, b3)) => {
                a1 == b1 && a2 == b2 && a3 == b3
            }
            (Value::Tuple(a), Value::Tuple(b)) => a == b,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Option(a), Value::Option(b)) => a == b,
            (Value::Result(a), Value::Result(b)) => a == b,
            (Value::Pointer(a), Value::Pointer(b)) => a == b,
            (
                Value::Reference {
                    target: t1,
                    mutable: m1,
                },
                Value::Reference {
                    target: t2,
                    mutable: m2,
                },
            ) => t1 == t2 && m1 == m2,
            (
                Value::Enum {
                    name: n1,
                    variant: v1,
                    data: d1,
                },
                Value::Enum {
                    name: n2,
                    variant: v2,
                    data: d2,
                },
            ) => n1 == n2 && v1 == v2 && d1 == d2,
            (
                Value::Object {
                    type_id: t1,
                    fields: f1,
                },
                Value::Object {
                    type_id: t2,
                    fields: f2,
                },
            ) => t1 == t2 && f1 == f2,
            (
                Value::NativeObject {
                    type_id: t1,
                    object: o1,
                    ..
                },
                Value::NativeObject {
                    type_id: t2,
                    object: o2,
                    ..
                },
            ) => t1 == t2 && Arc::ptr_eq(o1, o2),
            (Value::Lambda(a), Value::Lambda(b)) => a == b,
            (
                Value::Promise {
                    task_id: t1,
                    status: s1,
                    ..
                },
                Value::Promise {
                    task_id: t2,
                    status: s2,
                    ..
                },
            ) => t1 == t2 && s1 == s2,
            (
                Value::Generator {
                    id: i1, state: s1, ..
                },
                Value::Generator {
                    id: i2, state: s2, ..
                },
            ) => i1 == i2 && s1 == s2,
            (Value::Channel { id: i1, .. }, Value::Channel { id: i2, .. }) => i1 == i2,
            (Value::Null, Value::Null) => true,
            (Value::Undefined, Value::Undefined) => true,
            (Value::Type(a), Value::Type(b)) => a == b,
            (
                Value::Error {
                    message: m1,
                    kind: k1,
                    ..
                },
                Value::Error {
                    message: m2,
                    kind: k2,
                    ..
                },
            ) => m1 == m2 && k1 == k2,
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
                Value::Reference { .. } => 13,
                Value::Enum { .. } => 14,
                Value::Object { .. } => 15,
                Value::NativeObject { .. } => 16,
                Value::Lambda(_) => 17,
                Value::PartialApp { .. } => 18,
                Value::Promise { .. } => 19,
                Value::Generator { .. } => 20,
                Value::Channel { .. } => 21,
                Value::Null => 22,
                Value::Undefined => 23,
                Value::Type(_) => 24,
                Value::Error { .. } => 25,
                Value::Range { .. } => 26,
                Value::Bytes(_) => 27,
            }
        }

        let d = tag(self).cmp(&tag(other));
        if d != Ordering::Equal {
            return d;
        }
        self.to_string().cmp(&other.to_string())
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        self.to_string().hash(state);
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

// =============================================================================
//         SECTION: DISPLAY IMPLEMENTATION
// =============================================================================

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", if *b { "да" } else { "нет" }),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::Array(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Range {
                start,
                end,
                inclusive,
                step,
            } => {
                write!(
                    f,
                    "{}..{}{}{}",
                    start,
                    if *inclusive { "=" } else { "" },
                    end,
                    if *step != 1 {
                        format!(" шаг {}", step)
                    } else {
                        String::new()
                    }
                )
            }
            Value::Bytes(b) => {
                write!(f, "байты[")?;
                for (i, byte) in b.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", byte)?;
                }
                write!(f, "]")
            }
            Value::Tuple(t) => {
                write!(f, "(")?;
                for (i, v) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, ")")
            }
            Value::Pair(a, b) => write!(f, "({}, {})", a, b),
            Value::Triple(a, b, c) => write!(f, "({}, {}, {})", a, b, c),
            Value::Set(s) => {
                write!(f, "{{")?;
                for (i, v) in s.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "}}")
            }
            Value::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Option(o) => match o.as_ref() {
                Some(v) => write!(f, "Некоторое({})", v),
                None => write!(f, "Ничего"),
            },
            Value::Result(r) => match r.as_ref() {
                Ok(v) => write!(f, "Успех({})", v),
                Err(e) => write!(f, "Ошибка({})", e),
            },
            Value::Pointer(p) => write!(f, "&{}", p),
            Value::Reference { target, mutable } => {
                write!(f, "&{}{}", if *mutable { "измен " } else { "" }, target)
            }
            Value::Enum {
                name,
                variant,
                data,
            } => {
                write!(f, "{}::{}", name, variant)?;
                if let Some(d) = data {
                    write!(f, "({})", d)?;
                }
                Ok(())
            }
            Value::Object { type_id, fields } => {
                write!(f, "<Объект #{} {{", type_id.0)?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}>")
            }
            Value::NativeObject { type_name, .. } => write!(f, "<{}>", type_name),
            Value::Lambda(l) => {
                write!(f, "<лямбда({})>", l.params.join(", "))
            }
            Value::PartialApp { func, applied_args } => {
                write!(
                    f,
                    "<частичное применение: {} с {} арг>",
                    func,
                    applied_args.len()
                )
            }
            Value::Promise {
                task_id,
                status,
                result,
                error,
            } => match status {
                PromiseStatus::Pending => write!(f, "<Promise #{} ожидает>", task_id),
                PromiseStatus::Resolved => {
                    if let Some(v) = result {
                        write!(f, "<Promise #{} выполнен: {}>", task_id, v)
                    } else {
                        write!(f, "<Promise #{} выполнен>", task_id)
                    }
                }
                PromiseStatus::Rejected => {
                    if let Some(e) = error {
                        write!(f, "<Promise #{} отклонён: {}>", task_id, e)
                    } else {
                        write!(f, "<Promise #{} отклонён>", task_id)
                    }
                }
            },
            Value::Generator { id, state, .. } => {
                write!(f, "<Генератор #{} {:?}>", id, state)
            }
            Value::Channel {
                id,
                capacity,
                closed,
            } => {
                write!(
                    f,
                    "<Канал #{} вместимость={} закрыт={}>",
                    id, capacity, closed
                )
            }
            Value::Null => write!(f, "пусто"),
            Value::Undefined => write!(f, "неопределено"),
            Value::Type(t) => write!(f, "<Тип: {}>", t),
            Value::Error { message, kind, .. } => {
                write!(f, "<Ошибка {}: {}>", kind, message)
            }
        }
    }
}

// =============================================================================
//         SECTION: TYPE KIND METHODS
// =============================================================================

impl TypeKind {
    // -------------------------------------------------------------------------
    // Default Type Constants
    // -------------------------------------------------------------------------

    pub const INT: TypeKind = TypeKind::Int64;
    pub const UINT: TypeKind = TypeKind::UInt64;
    pub const FLOAT: TypeKind = TypeKind::Float64;

    // -------------------------------------------------------------------------
    // Constructors
    // -------------------------------------------------------------------------

    pub fn array(elem: TypeKind) -> Self {
        TypeKind::Array(Box::new(elem))
    }
    pub fn range(elem: TypeKind) -> Self {
        TypeKind::Range(Box::new(elem))
    }
    pub fn pair(a: TypeKind, b: TypeKind) -> Self {
        TypeKind::Pair(Box::new(a), Box::new(b))
    }
    pub fn triple(a: TypeKind, b: TypeKind, c: TypeKind) -> Self {
        TypeKind::Triple(Box::new(a), Box::new(b), Box::new(c))
    }
    pub fn tuple(elements: Vec<TypeKind>) -> Self {
        TypeKind::Tuple(elements)
    }
    pub fn set(elem: TypeKind) -> Self {
        TypeKind::Set(Box::new(elem))
    }
    pub fn map(key: TypeKind, value: TypeKind) -> Self {
        TypeKind::Map(Box::new(key), Box::new(value))
    }
    pub fn option(inner: TypeKind) -> Self {
        TypeKind::Option(Box::new(inner))
    }
    pub fn result(ok: TypeKind, err: TypeKind) -> Self {
        TypeKind::Result {
            ok: Box::new(ok),
            err: Box::new(err),
        }
    }
    pub fn pointer(inner: TypeKind) -> Self {
        TypeKind::Pointer(Box::new(inner))
    }
    pub fn reference(inner: TypeKind, mutable: bool) -> Self {
        TypeKind::Reference {
            inner: Box::new(inner),
            mutable,
        }
    }
    pub fn function(params: Vec<TypeKind>, result: Option<TypeKind>) -> Self {
        TypeKind::Function {
            params,
            result: result.map(Box::new),
        }
    }
    pub fn promise(inner: TypeKind) -> Self {
        TypeKind::Promise(Box::new(inner))
    }
    pub fn channel(inner: TypeKind) -> Self {
        TypeKind::Channel(Box::new(inner))
    }

    // -------------------------------------------------------------------------
    // Type Checks
    // -------------------------------------------------------------------------

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            TypeKind::Int8
                | TypeKind::Int16
                | TypeKind::Int32
                | TypeKind::Int64
                | TypeKind::Int128
                | TypeKind::UInt8
                | TypeKind::UInt16
                | TypeKind::UInt32
                | TypeKind::UInt64
                | TypeKind::UInt128
                | TypeKind::Float32
                | TypeKind::Float64
                | TypeKind::Float128
        )
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            TypeKind::Int8
                | TypeKind::Int16
                | TypeKind::Int32
                | TypeKind::Int64
                | TypeKind::Int128
                | TypeKind::UInt8
                | TypeKind::UInt16
                | TypeKind::UInt32
                | TypeKind::UInt64
                | TypeKind::UInt128
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(
            self,
            TypeKind::Float32 | TypeKind::Float64 | TypeKind::Float128
        )
    }

    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            TypeKind::Int8 | TypeKind::Int16 | TypeKind::Int32 | TypeKind::Int64 | TypeKind::Int128
        )
    }

    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            TypeKind::UInt8
                | TypeKind::UInt16
                | TypeKind::UInt32
                | TypeKind::UInt64
                | TypeKind::UInt128
        )
    }

    pub fn is_scalar(&self) -> bool {
        self.is_numeric() || matches!(self, TypeKind::String | TypeKind::Bool | TypeKind::Char)
    }

    pub fn is_collection(&self) -> bool {
        matches!(
            self,
            TypeKind::Array(_)
                | TypeKind::Tuple(_)
                | TypeKind::Set(_)
                | TypeKind::Map(_, _)
                | TypeKind::Pair(_, _)
                | TypeKind::Triple(_, _, _)
                | TypeKind::Range(_)
        )
    }

    pub fn is_nullable(&self) -> bool {
        matches!(self, TypeKind::Option(_) | TypeKind::Null)
    }

    pub fn is_callable(&self) -> bool {
        matches!(self, TypeKind::Function { .. } | TypeKind::Lambda { .. })
    }

    pub fn is_special(&self) -> bool {
        matches!(
            self,
            TypeKind::Auto
                | TypeKind::Any
                | TypeKind::Void
                | TypeKind::Never
                | TypeKind::Null
                | TypeKind::Undefined
        )
    }

    // -------------------------------------------------------------------------
    // Size in Bytes
    // -------------------------------------------------------------------------

    pub fn size_bytes(&self) -> Option<usize> {
        match self {
            TypeKind::Int8 | TypeKind::UInt8 => Some(1),
            TypeKind::Int16 | TypeKind::UInt16 => Some(2),
            TypeKind::Int32 | TypeKind::UInt32 | TypeKind::Float32 => Some(4),
            TypeKind::Int64 | TypeKind::UInt64 | TypeKind::Float64 => Some(8),
            TypeKind::Int128 | TypeKind::UInt128 | TypeKind::Float128 => Some(16),
            TypeKind::Bool => Some(1),
            TypeKind::Char => Some(4),
            _ => None,
        }
    }

    // -------------------------------------------------------------------------
    // Russian Names
    // -------------------------------------------------------------------------

    pub fn russian_name(&self) -> String {
        match self {
            TypeKind::Int8 => "цел_8".into(),
            TypeKind::Int16 => "цел_16".into(),
            TypeKind::Int32 => "цел_32".into(),
            TypeKind::Int64 => "цел".into(),
            TypeKind::Int128 => "цел_128".into(),
            TypeKind::UInt8 => "нат_8".into(),
            TypeKind::UInt16 => "нат_16".into(),
            TypeKind::UInt32 => "нат_32".into(),
            TypeKind::UInt64 => "нат".into(),
            TypeKind::UInt128 => "нат_128".into(),
            TypeKind::Float32 => "вещ_32".into(),
            TypeKind::Float64 => "вещ".into(),
            TypeKind::Float128 => "вещ_128".into(),
            TypeKind::String => "лит".into(),
            TypeKind::Bool => "лог".into(),
            TypeKind::Char => "сим".into(),
            TypeKind::Array(elem) => format!("таб {}", elem.russian_name()),
            TypeKind::Range(elem) => format!("диапазон {}", elem.russian_name()),
            TypeKind::Pair(a, b) => format!("пара({}, {})", a.russian_name(), b.russian_name()),
            TypeKind::Triple(a, b, c) => format!(
                "тройка({}, {}, {})",
                a.russian_name(),
                b.russian_name(),
                c.russian_name()
            ),
            TypeKind::Tuple(elems) => {
                let names: Vec<_> = elems.iter().map(|e| e.russian_name()).collect();
                format!("кортеж({})", names.join(", "))
            }
            TypeKind::Set(elem) => format!("множество {}", elem.russian_name()),
            TypeKind::Map(k, v) => format!("словарь[{}: {}]", k.russian_name(), v.russian_name()),
            TypeKind::Option(inner) => format!("{}?", inner.russian_name()),
            TypeKind::Result { ok, err } => format!("{}!{}", ok.russian_name(), err.russian_name()),
            TypeKind::Pointer(inner) => format!("указатель {}", inner.russian_name()),
            TypeKind::Reference { inner, mutable } => {
                if *mutable {
                    format!("&измен {}", inner.russian_name())
                } else {
                    format!("&{}", inner.russian_name())
                }
            }
            TypeKind::Enum(name) => name.clone(),
            TypeKind::Object(name) => name.clone(),
            TypeKind::Native(name) => format!("@{}", name),
            TypeKind::Generic { name, type_args } => {
                let args: Vec<_> = type_args.iter().map(|a| a.russian_name()).collect();
                format!("{}<{}>", name, args.join(", "))
            }
            TypeKind::Function { params, result } => {
                let param_names: Vec<_> = params.iter().map(|p| p.russian_name()).collect();
                let ret = result
                    .as_ref()
                    .map(|r| r.russian_name())
                    .unwrap_or_else(|| "ничего".into());
                format!("функция({}) -> {}", param_names.join(", "), ret)
            }
            TypeKind::Lambda { params, result, .. } => {
                let param_names: Vec<_> = params.iter().map(|p| p.russian_name()).collect();
                let ret = result
                    .as_ref()
                    .map(|r| r.russian_name())
                    .unwrap_or_else(|| "ничего".into());
                format!("лямбда({}) -> {}", param_names.join(", "), ret)
            }
            TypeKind::Promise(inner) => format!("обещание {}", inner.russian_name()),
            TypeKind::Generator {
                yield_type,
                return_type,
            } => {
                format!(
                    "генератор<{}, {}>",
                    yield_type.russian_name(),
                    return_type.russian_name()
                )
            }
            TypeKind::Channel(inner) => format!("канал {}", inner.russian_name()),
            TypeKind::Null => "пусто".into(),
            TypeKind::Undefined => "неопределено".into(),
            TypeKind::Auto => "авто".into(),
            TypeKind::Any => "любой".into(),
            TypeKind::Void => "ничего".into(),
            TypeKind::Never => "никогда".into(),
            TypeKind::Type => "тип".into(),
        }
    }

    // -------------------------------------------------------------------------
    // Type Compatibility
    // -------------------------------------------------------------------------

    /// Checks if a value of type `from` can be implicitly assigned to a variable
    /// of type `self`.
    ///
    /// Thin facade over the single source of truth — the type engine
    /// ([`crate::typesys::TypeSystem`], see KITE 10). Kept for ergonomics; for
    /// coercion plans, unification or nominal subtyping use the engine directly.
    pub fn is_assignable_from(&self, from: &TypeKind) -> bool {
        crate::typesys::default_engine().is_assignable(self, from)
    }

    /// Finds a common type for two types (for binary operations).
    ///
    /// Delegates to the type engine's unification ([`crate::typesys::TypeSystem::unify`]).
    pub fn common_type(&self, other: &TypeKind) -> Option<TypeKind> {
        crate::typesys::default_engine().unify(self, other)
    }
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.russian_name())
    }
}

// =============================================================================
//         SECTION: TYPE SPEC COMPATIBILITY (DEPRECATED)
// =============================================================================

/// [DEPRECATED] Alias for backward compatibility. Use `TypeKind` instead.
pub type TypeSpec = TypeKind;

// =============================================================================
//         SECTION: TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_constructors() {
        assert!(Value::int(42).is_number());
        assert!(Value::string("hello").is_string());
        assert!(Value::bool(true).is_boolean());
    }

    #[test]
    fn test_type_kind_from_value() {
        assert_eq!(Value::int(42).type_kind(), TypeKind::Int64);
        assert_eq!(Value::string("test").type_kind(), TypeKind::String);
        assert_eq!(Value::bool(false).type_kind(), TypeKind::Bool);
    }

    #[test]
    fn test_russian_names() {
        assert_eq!(TypeKind::Int64.russian_name(), "цел");
        assert_eq!(TypeKind::Float64.russian_name(), "вещ");
        assert_eq!(TypeKind::Bool.russian_name(), "лог");
    }

    #[test]
    fn test_truthy_falsy() {
        assert!(Value::bool(true).is_truthy());
        assert!(Value::bool(false).is_falsy());
        assert!(Value::Null.is_falsy());
        assert!(Value::int(1).is_truthy());
        assert!(Value::int(0).is_falsy());
    }

    #[test]
    fn test_collections() {
        let arr = Value::array(vec![Value::int(1), Value::int(2)]);
        assert_eq!(arr.len(), Some(2));
        assert_eq!(arr.get(0), Some(&Value::int(1)));
    }
}
