//! Type system for the Kumir 3 language.
//!
//! Module organization:
//! - `number` — numeric types (Number)
//! - `value` — runtime values (Value)
//! - `token` — lexical tokens
//! - `expr` — expressions (Expr)
//! - `stmt` — statements (Stmt)
//! - `pattern` — pattern matching patterns
//! - `class` — classes and OOP (AST definitions)
//! - `algorithm` — algorithms and parameters
//! - `program` — complete program
//! - `registry` — type registry (TypeRegistry v2)
//! - `library` — library definitions (LibraryDef)
//! - `version` — semantic versioning (SemVer)
//! - `environment` — virtual environments
//! - `import_spec` — import specifications
//! - `config` — project configuration (kumir.toml)
//! - `resolver` — dependency resolver

mod algorithm;
mod class;
pub mod config;
pub mod environment;
mod expr;
pub mod import_spec;
pub mod library;
mod number;
mod pattern;
mod program;
mod registry;
pub mod resolver;
mod stmt;
mod token;
pub mod value;
pub mod venv_loader;
pub mod version;

// Re-export all public types
pub use algorithm::{
    Algorithm, AlgorithmKind, Attribute, CallConvention, EffectFlags, NodeId, OverloadedAlgorithm,
    ParamMode, Parameter, SourceSpan, TypeConstraint, TypeParam,
};
pub use class::{
    ClassDef, ClassKind, Constructor, Field, ImplDef, InterfaceDef, Method, MethodSignature,
    TraitDef, TraitMethod, Visibility,
};
pub use expr::Expr;
pub use number::Number;
pub use pattern::Pattern;
pub use program::Program;
pub use stmt::Stmt;
pub use token::Token;
pub use value::{
    GeneratorState, LambdaValue, Ownership, PromiseStatus, TypeKind, Value, ValueMeta,
};

// TypeRegistry v2
pub use registry::{
    FieldAccessHandler, FieldDef, FieldSetHandler, InstanceMethodHandler, MethodDef,
    MethodDefBuilder, MethodParam, NativeFactory, ParamMode as RegistryParamMode,
    StaticMethodHandler, TypeDef, TypeDefBuilder, TypeId, TypeRegistry,
};

// Library system
pub use library::{
    LibConstantDef, LibDependency, LibFieldDef, LibFunctionDef, LibParamDef, LibVersion,
    LibraryDef, LibraryRegistry, NativeFn, NativeResult,
};

// Version system
pub use version::{Version, VersionOp, VersionParseError, VersionReq, VersionSpec};

// Environment system
pub use environment::{
    EnvPaths, EnvironmentManager, LibrarySource, ResolvedDependency, VersionedLibrary,
    VirtualEnvironment,
};

// Import system
pub use import_spec::{ImportItem, ImportParser, ImportSource, ImportSpec, parse_import};

// Config system
pub use config::{
    BuildProfile, BuildSettings, ConfigError, DependencySpec, GitSource, KumirConfig, LockEntry,
    LockFile, ProjectMetadata,
};

// Resolver
pub use resolver::{
    ConflictStrategy, DependencyGraph, DependencyNode, DependencyResolver, LibraryProvider,
    ResolutionError, ResolutionResult, ResolutionStatus, ResolvedPackage, VersionConflict,
};

// Integrated loader
pub use venv_loader::{
    IntegratedLoader, LibraryManifest, LoadedLibrary, LoaderError, LoaderResult,
    ManifestDependency, activate_project, deactivate_project, list_available, load_library,
    load_library_versioned, load_library_with_deps, loader, register_builtin,
};

// Helper structures
pub use stmt::{EnumVariant, MatchArm, VarModifiers, YieldParam};
