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

//!
//! ## Module layout
//!
//! - [`kind`] — [`TypeKind`] methods (constructors, predicates, russian names)
//! - [`construct`] — [`Value`] constructors, predicates, accessors, conversions
//! - [`typing`] — [`Value::type_kind`] type introspection
//! - [`display`] — [`Value`] rendering
//! - [`compare`] — [`Value`] equality, ordering and hashing

mod compare;
mod construct;
mod display;
mod kind;
mod typing;

use std::any::Any;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::expr::Expr;
use super::number::Number;
use super::registry::TypeId;

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
    /// Body expression
    pub body: Box<Expr>,
    /// Captured variables from enclosing scope
    pub captures: BTreeMap<String, Value>,
}

impl PartialEq for LambdaValue {
    fn eq(&self, other: &Self) -> bool {
        self.params == other.params
            && self.param_types == other.param_types
            && self.return_type == other.return_type
            && self.body == other.body
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
