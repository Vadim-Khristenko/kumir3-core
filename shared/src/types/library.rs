//! Kumir 3 library registry and metadata.
//!
//! [STABLE] Modernized API for registering library functions, types, classes,
//! native handlers, and dependencies. Uses the unified type system (`TypeKind`)
//! and runtime `Value`. Designed to be resilient for plugins, package manager,
//! LSP, and runtime loader.

use std::collections::HashMap;
use std::sync::Arc;

use super::algorithm::ParamMode;
use super::config::DependencySpec;
use super::value::{TypeKind, Value};
use super::version::Version;

// =============================================================================
//         SECTION: PARAMS & HANDLERS
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibParamDef {
    pub name: Arc<str>,
    pub type_kind: TypeKind,
    pub mode: ParamMode,
    pub default: Option<Value>,
    pub description: Option<Arc<str>>,
    pub optional: bool,
}

impl LibParamDef {
    pub fn value(name: impl Into<Arc<str>>, type_kind: TypeKind) -> Self {
        Self {
            name: name.into(),
            type_kind,
            mode: ParamMode::In,
            default: None,
            description: None,
            optional: false,
        }
    }

    pub fn result(name: impl Into<Arc<str>>, type_kind: TypeKind) -> Self {
        Self {
            mode: ParamMode::Out,
            ..Self::value(name, type_kind)
        }
    }

    pub fn value_ref(name: impl Into<Arc<str>>, type_kind: TypeKind) -> Self {
        Self {
            mode: ParamMode::InOut,
            ..Self::value(name, type_kind)
        }
    }

    pub fn with_description(mut self, desc: impl Into<Arc<str>>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }

    pub fn with_default(mut self, val: Value) -> Self {
        self.default = Some(val);
        self
    }
}

pub type NativeResult = Result<Value, String>;
pub type NativeFn = Arc<dyn Fn(&[Value]) -> NativeResult + Send + Sync>;

// =============================================================================
//         SECTION: FUNCTIONS
// =============================================================================

#[derive(Clone)]
pub struct LibFunctionDef {
    pub name: Arc<str>,
    pub aliases: Vec<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub params: Vec<LibParamDef>,
    pub returns: Option<TypeKind>,
    pub handler: Option<NativeFn>,
    pub is_procedure: bool,
    pub is_async: bool,
    pub is_pure: bool,
    pub since: Option<Version>,
    pub deprecated: Option<Arc<str>>,
    pub example: Option<Arc<str>>,
}

impl std::fmt::Debug for LibFunctionDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibFunctionDef")
            .field("name", &self.name)
            .field("aliases", &self.aliases)
            .field("returns", &self.returns)
            .field("is_procedure", &self.is_procedure)
            .field("is_async", &self.is_async)
            .finish()
    }
}

impl LibFunctionDef {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            description: None,
            params: Vec::new(),
            returns: None,
            handler: None,
            is_procedure: false,
            is_async: false,
            is_pure: false,
            since: None,
            deprecated: None,
            example: None,
        }
    }

    pub fn with_aliases(mut self, aliases: impl Into<Vec<Arc<str>>>) -> Self {
        self.aliases = aliases.into();
        self
    }
    pub fn with_description(mut self, desc: impl Into<Arc<str>>) -> Self {
        self.description = Some(desc.into());
        self
    }
    pub fn with_param(mut self, param: LibParamDef) -> Self {
        self.params.push(param);
        self
    }
    pub fn returns(mut self, ret: TypeKind) -> Self {
        self.returns = Some(ret);
        self
    }
    pub fn with_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(&[Value]) -> NativeResult + Send + Sync + 'static,
    {
        self.handler = Some(Arc::new(handler));
        self
    }
    pub fn as_procedure(mut self) -> Self {
        self.is_procedure = true;
        self
    }
    pub fn as_async(mut self) -> Self {
        self.is_async = true;
        self
    }
    pub fn as_pure(mut self) -> Self {
        self.is_pure = true;
        self
    }
    pub fn since(mut self, v: Version) -> Self {
        self.since = Some(v);
        self
    }
    pub fn deprecate(mut self, msg: impl Into<Arc<str>>) -> Self {
        self.deprecated = Some(msg.into());
        self
    }
    pub fn with_example(mut self, ex: impl Into<Arc<str>>) -> Self {
        self.example = Some(ex.into());
        self
    }

    pub fn call(&self, args: &[Value]) -> NativeResult {
        match &self.handler {
            Some(h) => h(args),
            None => Err(format!("Function {} has no native handler", self.name)),
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        if self.name.as_ref() == name {
            return true;
        }
        self.aliases.iter().any(|a| a.as_ref() == name)
    }
}

// =============================================================================
//         SECTION: TYPE & CLASS DEFINITIONS
// =============================================================================

#[derive(Debug, Clone)]
pub struct LibFieldDef {
    pub name: Arc<str>,
    pub type_kind: TypeKind,
    pub description: Option<Arc<str>>,
    pub readonly: bool,
}

#[derive(Debug, Clone)]
pub struct ValueDef {
    pub name: Arc<str>,
    pub aliases: Vec<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub fields: Vec<LibFieldDef>,
    pub methods: Vec<Arc<str>>,
    pub static_methods: Vec<Arc<str>>,
    pub is_native: bool,
}

impl ValueDef {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            description: None,
            fields: Vec::new(),
            methods: Vec::new(),
            static_methods: Vec::new(),
            is_native: false,
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        if self.name.as_ref() == name {
            return true;
        }
        self.aliases.iter().any(|a| a.as_ref() == name)
    }
}

#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: Arc<str>,
    pub aliases: Vec<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub fields: Vec<LibFieldDef>,
    pub methods: Vec<Arc<str>>,
    pub static_methods: Vec<Arc<str>>,
    pub constructors: Vec<Arc<str>>,
    pub base_class: Option<Arc<str>>,
    pub interfaces: Vec<Arc<str>>,
    pub is_native: bool,
}

impl ClassDef {
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            aliases: Vec::new(),
            description: None,
            fields: Vec::new(),
            methods: Vec::new(),
            static_methods: Vec::new(),
            constructors: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            is_native: false,
        }
    }

    pub fn matches_name(&self, name: &str) -> bool {
        if self.name.as_ref() == name {
            return true;
        }
        self.aliases.iter().any(|a| a.as_ref() == name)
    }
}

// =============================================================================
//         SECTION: LIBRARY DEFINITION
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LibVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl LibVersion {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn to_version(&self) -> Version {
        Version::new(self.major, self.minor, self.patch)
    }
    pub fn from_version(v: &Version) -> Self {
        Self {
            major: v.major,
            minor: v.minor,
            patch: v.patch,
        }
    }
}

impl std::fmt::Display for LibVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl From<Version> for LibVersion {
    fn from(v: Version) -> Self {
        Self::from_version(&v)
    }
}
impl From<LibVersion> for Version {
    fn from(v: LibVersion) -> Self {
        Version::new(v.major, v.minor, v.patch)
    }
}

#[derive(Debug, Clone)]
pub struct LibDependency {
    pub name: Arc<str>,
    pub version: VersionSpecKind,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub enum VersionSpecKind {
    Any,
    Range(String),
}

impl LibDependency {
    pub fn required(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            version: VersionSpecKind::Any,
            required: true,
        }
    }

    pub fn optional(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            version: VersionSpecKind::Any,
            required: false,
        }
    }
}

#[derive(Clone)]
pub struct LibraryDef {
    pub id: Arc<str>,
    pub name: Arc<str>,
    pub aliases: Vec<Arc<str>>,
    pub description: Option<Arc<str>>,
    pub version: LibVersion,
    pub author: Arc<str>,
    pub dependencies: Vec<DependencySpec>,
    pub functions: Vec<LibFunctionDef>,
    pub types: Vec<ValueDef>,
    pub classes: Vec<ClassDef>,
    pub constants: Vec<LibConstantDef>,
    pub kumir_version: Option<Version>,
    pub stable: bool,
}

impl std::fmt::Debug for LibraryDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibraryDef")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("version", &self.version)
            .field("functions", &self.functions.len())
            .field("types", &self.types.len())
            .field("classes", &self.classes.len())
            .finish()
    }
}

impl LibraryDef {
    pub fn new(id: impl Into<Arc<str>>, name: impl Into<Arc<str>>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            aliases: Vec::new(),
            description: None,
            version: LibVersion::new(0, 1, 0),
            author: Arc::from("unknown"),
            dependencies: Vec::new(),
            functions: Vec::new(),
            types: Vec::new(),
            classes: Vec::new(),
            constants: Vec::new(),
            kumir_version: None,
            stable: true,
        }
    }

    pub fn find_function(&self, name: &str) -> Option<&LibFunctionDef> {
        self.functions.iter().find(|f| f.matches_name(name))
    }

    pub fn find_type(&self, name: &str) -> Option<&ValueDef> {
        self.types.iter().find(|t| t.matches_name(name))
    }

    pub fn find_class(&self, name: &str) -> Option<&ClassDef> {
        self.classes.iter().find(|c| c.matches_name(name))
    }

    pub fn matches_name(&self, name: &str) -> bool {
        if self.name.as_ref() == name || self.id.as_ref() == name {
            return true;
        }
        self.aliases.iter().any(|a| a.as_ref() == name)
    }
}

#[derive(Debug, Clone)]
pub struct LibConstantDef {
    pub name: Arc<str>,
    pub aliases: Vec<Arc<str>>,
    pub const_type: TypeKind,
    pub value: Value,
    pub description: Option<Arc<str>>,
}

// =============================================================================
//         SECTION: REGISTRY
// =============================================================================

#[derive(Default)]
pub struct LibraryRegistry {
    libraries: HashMap<String, LibraryDef>,
}

impl LibraryRegistry {
    pub fn new() -> Self {
        Self {
            libraries: HashMap::new(),
        }
    }

    pub fn register(&mut self, lib: LibraryDef) {
        self.libraries.insert(lib.name.to_string(), lib);
    }

    pub fn get(&self, name: &str) -> Option<&LibraryDef> {
        self.libraries.get(name)
    }

    pub fn all(&self) -> impl Iterator<Item = &LibraryDef> {
        self.libraries.values()
    }
}
