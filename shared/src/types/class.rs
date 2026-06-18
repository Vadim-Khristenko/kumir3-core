//! Classes, interfaces, traits, and implementation blocks (Kumir 3).
//!
//! [STABLE] Defines the object model abstractions used by the Kumir 3 compiler
//! and runtime. The data structures are intentionally extensible for future
//! LSP support, debugger integration, and alternative backends (AOT/JIT or
//! transpilers).
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │ ClassDef                                                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  name, kind, parent, interfaces, traits, fields, methods        │
//! │  constructors, destructor, attributes, span, docs               │
//! └─────────────────────────────────────────────────────────────────┘
//!      ▲         ▲             ▲
//!      │         │ implements  │ declares
//!      │         │             │
//! ┌───────────┐  ┌───────────┐  ┌───────────────┐
//! │ TraitDef  │  │ Interface │  │ ImplDef       │
//! └───────────┘  └───────────┘  └───────────────┘
//! ```

use std::sync::Arc;

use super::algorithm::{Algorithm, Attribute, NodeId, Parameter, SourceSpan, TypeParam};
use super::expr::Expr;
use super::stmt::Stmt;
use super::value::TypeKind;

// =============================================================================
//         SECTION: VISIBILITY
// =============================================================================

/// Visibility modifier for members and types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Visibility {
    #[default]
    Public,
    Private,
    Protected,
}

// =============================================================================
//         SECTION: CLASS KINDS
// =============================================================================

/// Kind of type in the object system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    /// Standard class with inheritance and methods
    Class,
    /// Lightweight struct (no virtual dispatch, plain data)
    Struct,
    /// Performer/driver (Robot, Board, etc.)
    Performer,
    /// Native host type provided by runtime or plugin
    Native,
}

// =============================================================================
//         SECTION: INTERFACES
// =============================================================================

/// Interface definition (method signatures only).
#[derive(Debug, Clone, PartialEq)]
pub struct InterfaceDef {
    pub id: NodeId,
    pub name: Arc<str>,
    pub type_params: Vec<TypeParam>,
    pub extends: Vec<Arc<str>>,
    pub methods: Vec<MethodSignature>,
    pub attributes: Vec<Attribute>,
    pub span: Option<SourceSpan>,
    pub doc: Option<Arc<str>>,
}

// =============================================================================
//         SECTION: TRAITS (TYPECLASSES)
// =============================================================================

/// Trait definition with optional default implementations.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitDef {
    pub id: NodeId,
    pub name: Arc<str>,
    pub type_params: Vec<TypeParam>,
    pub supertraits: Vec<Arc<str>>,
    pub methods: Vec<TraitMethod>,
    pub attributes: Vec<Attribute>,
    pub span: Option<SourceSpan>,
    pub doc: Option<Arc<str>>,
}

/// Method inside a trait.
#[derive(Debug, Clone, PartialEq)]
pub struct TraitMethod {
    pub signature: MethodSignature,
    pub default_impl: Option<Vec<Stmt>>,
    pub attributes: Vec<Attribute>,
    pub span: Option<SourceSpan>,
}

// =============================================================================
//         SECTION: IMPL BLOCKS
// =============================================================================

/// Implementation block (impl Trait for Type or inherent impl).
#[derive(Debug, Clone, PartialEq)]
pub struct ImplDef {
    pub id: NodeId,
    pub trait_name: Option<Arc<str>>,
    pub type_params: Vec<TypeParam>,
    /// Target type being implemented (Object/Class/Struct name)
    pub target: Arc<str>,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>,
    pub span: Option<SourceSpan>,
    pub doc: Option<Arc<str>>,
}

// =============================================================================
//         SECTION: FIELDS
// =============================================================================

/// Field of a class or struct.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub id: NodeId,
    pub name: Arc<str>,
    pub type_kind: TypeKind,
    pub visibility: Visibility,
    pub default: Option<Expr>,
    pub is_static: bool,
    pub is_mutable: bool,
    pub attributes: Vec<Attribute>,
    pub span: Option<SourceSpan>,
    pub doc: Option<Arc<str>>,
}

// =============================================================================
//         SECTION: METHODS & SIGNATURES
// =============================================================================

/// Signature of a method (used by interfaces and traits).
#[derive(Debug, Clone, PartialEq)]
pub struct MethodSignature {
    pub name: Arc<str>,
    pub type_params: Vec<TypeParam>,
    pub params: Vec<Parameter>,
    pub return_type: Option<TypeKind>,
    pub attributes: Vec<Attribute>,
    pub span: Option<SourceSpan>,
}

/// Method definition bound to a class or struct.
#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    /// Algorithm payload (kind should be Method/Constructor/Destructor)
    pub algorithm: Algorithm,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_virtual: bool,
    pub is_override: bool,
    pub is_final: bool,
    pub is_abstract: bool,
    pub attributes: Vec<Attribute>,
    pub span: Option<SourceSpan>,
}

/// Constructor definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Constructor {
    pub algorithm: Algorithm,
    /// Optional call to parent constructor
    pub super_call: Option<Vec<Expr>>,
    pub visibility: Visibility,
    pub attributes: Vec<Attribute>,
    pub span: Option<SourceSpan>,
}

// =============================================================================
//         SECTION: CLASSES
// =============================================================================

/// Class or struct definition.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDef {
    pub id: NodeId,
    pub name: Arc<str>,
    pub kind: ClassKind,
    pub type_params: Vec<TypeParam>,
    pub parent: Option<Arc<str>>,
    pub interfaces: Vec<Arc<str>>,
    pub traits: Vec<Arc<str>>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub constructors: Vec<Constructor>,
    pub destructor: Option<Method>,
    pub is_abstract: bool,
    pub is_final: bool,
    pub attributes: Vec<Attribute>,
    pub span: Option<SourceSpan>,
    pub doc: Option<Arc<str>>,
}
