//! Algorithms, parameters, and callable metadata (Kumir 3).
//!
//! [STABLE] Defines algorithms (functions/procedures/methods), parameters,
//! attributes, and spans for tooling. Designed for extensibility toward LSP,
//! debugger hooks, AOT/JIT backends, and cross-language translation.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ Algorithm                                                      │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  id, name, kind, type params, params, return type               │
//! │  pre/post conditions, effects, attributes, span, docs           │
//! └─────────────────────────────────────────────────────────────────┘
//!            ▲                              ▲
//!            │ uses                          │ contains
//! ┌────────────────────────┐        ┌──────────────────────────────┐
//! │ Parameter              │        │ Attribute, SourceSpan,       │
//! │ name, type, mode,      │        │ TypeConstraint, EffectFlags  │
//! │ default, attributes    │        └──────────────────────────────┘
//! └────────────────────────┘
//! ```

use std::sync::Arc;

use super::expr::Expr;
use super::stmt::Stmt;
use super::value::TypeKind;

// =============================================================================
//         SECTION: IDS AND SPANS
// =============================================================================

/// Stable identifier for AST and symbol-table entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct NodeId(pub u64);

/// Source mapping for diagnostics, LSP, and debugging.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SourceSpan {
    pub file_id: Option<Arc<str>>,
    pub start: usize,
    pub end: usize,
}

// =============================================================================
//         SECTION: ATTRIBUTES
// =============================================================================

/// User-defined attributes for algorithms, parameters, and types.
#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: Arc<str>,
    pub args: Vec<Expr>,
    pub span: Option<SourceSpan>,
}

// =============================================================================
//         SECTION: TYPE PARAMETERS & CONSTRAINTS
// =============================================================================

/// Generic type parameter.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeParam {
    pub name: Arc<str>,
    pub constraints: Vec<TypeConstraint>,
    pub span: Option<SourceSpan>,
}

/// Constraint for a type parameter.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeConstraint {
    /// Must implement the given trait/interface
    Implements(Arc<str>),
    /// Must be equal to a specific type
    Equals(TypeKind),
    /// Custom predicate expressed as an expression
    WhereExpr(Expr),
}

// =============================================================================
//         SECTION: PARAMETER DEFINITION
// =============================================================================

/// Passing mode for parameters.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParamMode {
    /// Input (by value)
    In,
    /// Output (by reference or implicit out slot)
    Out,
    /// In/out parameter
    InOut,
    /// Borrowed reference (immutable)
    Borrow,
    /// Borrowed reference (mutable)
    BorrowMut,
    /// Move semantics (consumes the argument)
    Move,
}

/// Parameter definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub id: NodeId,
    pub name: Arc<str>,
    pub type_kind: Option<TypeKind>,
    pub mode: ParamMode,
    pub default: Option<Expr>,
    pub attributes: Vec<Attribute>,
    pub span: Option<SourceSpan>,
}

// =============================================================================
//         SECTION: EFFECTS & CALLING CONVENTIONS
// =============================================================================

/// Behavioral flags for algorithms to aid optimizers and analyzers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectFlags {
    /// Async algorithm (returns Promise/Task)
    pub is_async: bool,
    /// Generator (can yield values)
    pub is_generator: bool,
    /// Pure function (no observable side effects)
    pub is_pure: bool,
    /// Can throw/propagate errors
    pub can_throw: bool,
    /// Does not capture environment (good for AOT/JIT)
    pub is_static_env: bool,
}

impl Default for EffectFlags {
    fn default() -> Self {
        Self {
            is_async: false,
            is_generator: false,
            is_pure: false,
            can_throw: true,
            is_static_env: false,
        }
    }
}

/// Calling convention for interoperability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallConvention {
    /// Kumir-native call ABI
    Kumir,
    /// Rust ABI (for native plugins)
    Rust,
    /// C ABI (FFI interop)
    C,
    /// Custom adapter name
    Custom(Arc<str>),
}

// =============================================================================
//         SECTION: ALGORITHM DEFINITION
// =============================================================================

/// Kind of algorithm/function.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlgorithmKind {
    Function,
    Procedure,
    Method,
    Constructor,
    Destructor,
    /// Native/extern function provided by runtime or plugin
    Native,
    /// Closure/lambda produced at runtime
    Closure,
}

/// Algorithm (function/procedure/method) definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Algorithm {
    pub id: NodeId,
    pub name: Arc<str>,
    pub kind: AlgorithmKind,
    pub type_params: Vec<TypeParam>,
    pub return_type: Option<TypeKind>,
    pub params: Vec<Parameter>,
    pub precondition: Option<Expr>,
    pub postcondition: Option<Expr>,
    /// None for abstract/native algorithms
    pub body: Option<Vec<Stmt>>,
    pub effects: EffectFlags,
    pub attributes: Vec<Attribute>,
    pub call_conv: CallConvention,
    pub span: Option<SourceSpan>,
    pub doc: Option<Arc<str>>,
}

// =============================================================================
//         SECTION: OVERLOADING GROUPS
// =============================================================================

/// Group of overloaded algorithms sharing the same name.
#[derive(Debug, Clone, PartialEq)]
pub struct OverloadedAlgorithm {
    pub name: Arc<str>,
    pub overloads: Vec<Algorithm>,
    pub span: Option<SourceSpan>,
}
