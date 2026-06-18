//! Модуль типов языка Кумир 3
//!
//! Структура модуля:
//! - `number` - числовые типы (Number)
//! - `value` - значения времени выполнения (Value)
//! - `token` - токены (лексемы) языка
//! - `expr` - выражения (Expr)
//! - `stmt` - инструкции (Stmt)
//! - `pattern` - паттерны для pattern matching
//! - `class` - классы и ООП (AST определения)
//! - `algorithm` - алгоритмы и параметры
//! - `program` - полная программа
//! - `registry` - реестр типов (TypeRegistry v2)
//! - `library` - определения библиотек (LibraryDef)
//! - `version` - семантическое версионирование (SemVer)
//! - `environment` - виртуальные окружения
//! - `import_spec` - спецификации импорта
//! - `config` - конфигурация проекта (kumir.toml)
//! - `resolver` - резолвер зависимостей

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

// Re-export всех публичных типов
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

// Вспомогательные структуры
pub use stmt::{EnumVariant, MatchArm, VarModifiers, YieldParam};
