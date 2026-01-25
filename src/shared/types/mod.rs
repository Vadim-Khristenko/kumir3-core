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

mod number;
mod value;
mod token;
mod expr;
mod stmt;
mod pattern;
mod class;
mod algorithm;
mod program;
mod registry;
pub mod library;
pub mod version;
pub mod environment;
pub mod import_spec;
pub mod type_spec;
pub mod config;
pub mod resolver;
pub mod venv_loader;

// Re-export всех публичных типов
pub use number::Number;
pub use value::{Value, PromiseStatus};
pub use token::Token;
pub use expr::Expr;
pub use stmt::Stmt;
pub use pattern::Pattern;
pub use type_spec::TypeSpec;
pub use class::{
    ClassDef, Field, Method, MethodSignature, Constructor, Visibility,
    InterfaceDef, TraitDef, TraitMethod, ImplDef,
};
pub use algorithm::{Algorithm, Parameter, ParamMode, OverloadedAlgorithm};
pub use program::Program;

// TypeRegistry v2
pub use registry::{
    TypeRegistry, TypeId, TypeDef, FieldDef, 
    MethodDef, MethodParam, ParamMode as RegistryParamMode,
    TypeDefBuilder, MethodDefBuilder,
    NativeFactory, InstanceMethodHandler, StaticMethodHandler,
    FieldAccessHandler, FieldSetHandler,
};

// Library system
pub use library::{
    LibraryDef, LibFunctionDef, LibParamDef, LibConstantDef,
    LibVersion, LibDependency, LibFieldDef,
    ParamPassMode, NativeFn, NativeResult, LibraryRegistry,
};

// Version system
pub use version::{Version, VersionSpec, VersionReq, VersionOp, VersionParseError};

// Environment system
pub use environment::{
    VirtualEnvironment, VersionedLibrary, LibrarySource, EnvPaths,
    ResolvedDependency, EnvironmentManager,
};

// Import system
pub use import_spec::{ImportSpec, ImportItem, ImportSource, ImportParser, parse_import};

// Config system
pub use config::{
    KumirConfig, ProjectMetadata, DependencySpec, BuildSettings, BuildProfile,
    LockFile, LockEntry, ConfigError, GitSource,
};

// Resolver
pub use resolver::{
    DependencyResolver, DependencyGraph, DependencyNode, ResolutionResult,
    ResolvedPackage, ResolutionStatus, ResolutionError, VersionConflict,
    ConflictStrategy, LibraryProvider,
};

// Integrated loader
pub use venv_loader::{
    IntegratedLoader, LoadedLibrary, LoaderError, LoaderResult,
    LibraryManifest, ManifestDependency,
    loader, register_builtin, load_library, load_library_versioned,
    load_library_with_deps, activate_project, deactivate_project, list_available,
};

// Обратная совместимость (будет удалено)
pub use registry::{TypeInfo, TypeKind, NativeType, InitMethod};

// Вспомогательные структуры
pub use stmt::{EnumVariant, MatchArm};

