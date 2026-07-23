//! Type registry (TypeRegistry) v2.
//!
//! Unified type registration and management system for Kumir 3.
//!
//! Key principles:
//! - Single TypeDef for all types (primitives, Kumir classes, natives)
//! - Methods are stored as regular algorithms with type bindings
//! - Simple inheritance model (educational language!)
//! - Support for `new` keyword for object creation

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::value::Value;

// =============================================================================
//                           IDENTIFIERS
// =============================================================================

/// Unique type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u64);

impl TypeId {
    /// Reserved IDs for primitive types
    pub const INT: TypeId = TypeId(1);
    pub const FLOAT: TypeId = TypeId(2);
    pub const STRING: TypeId = TypeId(3);
    pub const BOOL: TypeId = TypeId(4);
    pub const CHAR: TypeId = TypeId(5);
    pub const ARRAY: TypeId = TypeId(6);
    pub const VOID: TypeId = TypeId(7);

    /// Check whether a type is primitive.
    pub fn is_primitive(&self) -> bool {
        self.0 <= 10
    }
}

// =============================================================================
//                           TYPE DEFINITION
// =============================================================================

/// Type definition — unified structure for all types.
///
/// In Kumir 3, all types (primitives, classes, natives) are described uniformly.
#[derive(Debug, Clone)]
pub struct TypeDef {
    /// Unique identifier
    pub id: TypeId,

    /// Type name (for display and lookup)
    pub name: String,

    /// Alternative names (synonyms).
    /// Example: ["HTTPServer", "HttpServer", "HTTP_Server"]
    pub aliases: Vec<String>,

    /// Module/library to which this type belongs.
    /// Example: "HTTP", "Files", "Graphics"
    pub module: Option<String>,

    /// Parent type (for inheritance)
    pub parent: Option<TypeId>,

    /// Implemented interfaces/traits
    pub implements: Vec<TypeId>,

    /// Fields of this type
    pub fields: Vec<FieldDef>,

    /// Whether this type is native (implemented in Rust)
    pub is_native: bool,

    /// Whether instances of this type can be created
    pub is_instantiable: bool,

    /// Type description (for documentation)
    pub description: String,
}

/// Type field definition.
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Field name
    pub name: String,

    /// Field type (type name)
    pub type_name: String,

    /// Field type ID (filled during registration)
    pub type_id: Option<TypeId>,

    /// Default value (if any)
    pub default: Option<Value>,

    /// Read-only?
    pub readonly: bool,

    /// Private field?
    pub private: bool,
}

// =============================================================================
//                           METHOD DEFINITION
// =============================================================================

/// Method definition for a type.
///
/// Methods are stored separately from the type and bound by name:
/// `TypeName.MethodName` or `TypeName::MethodName`
#[derive(Debug, Clone)]
pub struct MethodDef {
    /// Full method name: "HTTPServer.Start"
    pub full_name: String,

    /// Short name: "Start"
    pub name: String,

    /// Type ID to which this method belongs
    pub owner_type: TypeId,

    /// Method parameters
    pub params: Vec<MethodParam>,

    /// Return type (None = procedure)
    pub return_type: Option<String>,

    /// Static method? (called via Type.Method, not object.method)
    pub is_static: bool,

    /// Is this a constructor/initializer?
    pub is_constructor: bool,

    /// Method description
    pub description: String,
}

/// Method parameter.
#[derive(Debug, Clone)]
pub struct MethodParam {
    /// Parameter name
    pub name: String,

    /// Parameter type
    pub type_name: String,

    /// Passing mode
    pub mode: ParamMode,

    /// Default value
    pub default: Option<Value>,
}

/// Parameter passing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamMode {
    /// Input parameter (by value)
    In,
    /// Output parameter
    Out,
    /// Input/output parameter
    InOut,
}

// =============================================================================
//                           HANDLERS (for native types)
// =============================================================================

/// Factory for creating native objects.
/// Called when `new TypeName(args)`
pub type NativeFactory =
    Arc<dyn Fn(Vec<Value>) -> Result<Arc<dyn Any + Send + Sync>, String> + Send + Sync>;

/// Instance method handler.
/// Called when `object.method(args)`
pub type InstanceMethodHandler = Arc<
    dyn Fn(&Arc<dyn Any + Send + Sync>, &str, Vec<Value>) -> Result<Value, String> + Send + Sync,
>;

/// Static method handler.
/// Called when `TypeName.Method(args)`
pub type StaticMethodHandler = Arc<dyn Fn(&str, Vec<Value>) -> Result<Value, String> + Send + Sync>;

/// Field access handler.
/// Called when `object.field`
pub type FieldAccessHandler =
    Arc<dyn Fn(&Arc<dyn Any + Send + Sync>, &str) -> Result<Value, String> + Send + Sync>;

/// Field set handler.
/// Called when `object.field := value`
pub type FieldSetHandler =
    Arc<dyn Fn(&Arc<dyn Any + Send + Sync>, &str, Value) -> Result<(), String> + Send + Sync>;

// =============================================================================
//                           TYPE REGISTRY
// =============================================================================

/// Type registry — central repository of all types in a program.
///
/// Main responsibilities:
/// - Register and lookup types
/// - Create object instances
/// - Call methods and access fields
/// - Check inheritance relationships
pub struct TypeRegistry {
    /// Counter for generating unique IDs
    next_id: RwLock<u64>,

    /// Types by ID
    types: RwLock<HashMap<TypeId, TypeDef>>,

    /// Types by name (including aliases)
    types_by_name: RwLock<HashMap<String, TypeId>>,

    /// Methods by full name: "TypeName.MethodName" -> MethodDef
    methods: RwLock<HashMap<String, MethodDef>>,

    /// Methods by type: TypeId -> Vec<MethodDef>
    methods_by_type: RwLock<HashMap<TypeId, Vec<MethodDef>>>,

    // --- Handlers for native types ---
    /// Object creation factories
    factories: RwLock<HashMap<TypeId, NativeFactory>>,

    /// Instance method handlers
    instance_handlers: RwLock<HashMap<TypeId, InstanceMethodHandler>>,

    /// Static method handlers
    static_handlers: RwLock<HashMap<TypeId, StaticMethodHandler>>,

    /// Field access handlers
    #[allow(dead_code)]
    field_getters: RwLock<HashMap<TypeId, FieldAccessHandler>>,

    /// Field set handlers
    #[allow(dead_code)]
    field_setters: RwLock<HashMap<TypeId, FieldSetHandler>>,
}

impl TypeRegistry {
    /// Create a new registry with pre-registered primitives.
    pub fn new() -> Self {
        let registry = Self {
            next_id: RwLock::new(100), // 0-99 reserved
            types: RwLock::new(HashMap::new()),
            types_by_name: RwLock::new(HashMap::new()),
            methods: RwLock::new(HashMap::new()),
            methods_by_type: RwLock::new(HashMap::new()),
            factories: RwLock::new(HashMap::new()),
            instance_handlers: RwLock::new(HashMap::new()),
            static_handlers: RwLock::new(HashMap::new()),
            field_getters: RwLock::new(HashMap::new()),
            field_setters: RwLock::new(HashMap::new()),
        };

        registry.register_primitives();
        registry
    }

    /// Register primitive types.
    fn register_primitives(&self) {
        let primitives = [
            (TypeId::INT, "цел", vec!["целое", "int", "integer"]),
            (TypeId::FLOAT, "вещ", vec!["вещественное", "float", "real"]),
            (TypeId::STRING, "лит", vec!["строка", "string", "str"]),
            (TypeId::BOOL, "лог", vec!["логическое", "bool", "boolean"]),
            (TypeId::CHAR, "сим", vec!["символ", "char"]),
            (TypeId::ARRAY, "таб", vec!["таблица", "массив", "array"]),
            (TypeId::VOID, "пустота", vec!["void", "unit"]),
        ];

        let mut types = self.types.write().unwrap();
        let mut by_name = self.types_by_name.write().unwrap();

        for (id, name, aliases) in primitives {
            let type_def = TypeDef {
                id,
                name: name.to_string(),
                aliases: aliases.iter().map(|s| s.to_string()).collect(),
                module: None,
                parent: None,
                implements: vec![],
                fields: vec![],
                is_native: true,
                is_instantiable: false,
                description: format!("Примитивный тип {}", name),
            };

            types.insert(id, type_def);
            by_name.insert(name.to_string(), id);
            for alias in aliases {
                by_name.insert(alias.to_string(), id);
            }
        }
    }

    /// Generate a new unique TypeId.
    fn next_type_id(&self) -> TypeId {
        let mut id = self.next_id.write().unwrap();
        let type_id = TypeId(*id);
        *id += 1;
        type_id
    }

    // =========================================================================
    //                       TYPE REGISTRATION
    // =========================================================================

    /// Register a new type.
    pub fn register_type(&self, mut type_def: TypeDef) -> TypeId {
        let type_id = self.next_type_id();
        type_def.id = type_id;

        let name = type_def.name.clone();
        let aliases = type_def.aliases.clone();

        self.types.write().unwrap().insert(type_id, type_def);

        let mut by_name = self.types_by_name.write().unwrap();
        by_name.insert(name, type_id);
        for alias in aliases {
            by_name.insert(alias, type_id);
        }

        type_id
    }

    /// Register a native type with handlers.
    pub fn register_native_type(
        &self,
        type_def: TypeDef,
        factory: NativeFactory,
        instance_handler: InstanceMethodHandler,
        static_handler: StaticMethodHandler,
    ) -> TypeId {
        let type_id = self.register_type(type_def);

        self.factories.write().unwrap().insert(type_id, factory);
        self.instance_handlers
            .write()
            .unwrap()
            .insert(type_id, instance_handler);
        self.static_handlers
            .write()
            .unwrap()
            .insert(type_id, static_handler);

        type_id
    }

    /// Register a type method.
    pub fn register_method(&self, method: MethodDef) {
        let full_name = method.full_name.clone();
        let owner_type = method.owner_type;

        self.methods
            .write()
            .unwrap()
            .insert(full_name, method.clone());

        self.methods_by_type
            .write()
            .unwrap()
            .entry(owner_type)
            .or_default()
            .push(method);
    }

    // =========================================================================
    //                       TYPE LOOKUP
    // =========================================================================

    /// Get TypeId by name.
    pub fn get_type_id(&self, name: &str) -> Option<TypeId> {
        self.types_by_name.read().unwrap().get(name).copied()
    }

    /// Get type definition by ID.
    pub fn get_type(&self, type_id: TypeId) -> Option<TypeDef> {
        self.types.read().unwrap().get(&type_id).cloned()
    }

    /// Get type name by ID.
    pub fn get_type_name(&self, type_id: TypeId) -> Option<String> {
        self.types
            .read()
            .unwrap()
            .get(&type_id)
            .map(|t| t.name.clone())
    }

    /// Get all methods of a type.
    pub fn get_methods(&self, type_id: TypeId) -> Vec<MethodDef> {
        self.methods_by_type
            .read()
            .unwrap()
            .get(&type_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Find method by full name.
    pub fn get_method(&self, full_name: &str) -> Option<MethodDef> {
        self.methods.read().unwrap().get(full_name).cloned()
    }

    /// Find static method of a type.
    pub fn find_static_method(&self, type_name: &str, method_name: &str) -> Option<MethodDef> {
        let full_name = format!("{}.{}", type_name, method_name);
        self.methods.read().unwrap().get(&full_name).cloned()
    }

    // =========================================================================
    //                       OBJECT CREATION
    // =========================================================================

    /// Create an instance of a native type.
    ///
    /// Called when `new TypeName(args)`
    pub fn create_instance(&self, type_id: TypeId, args: Vec<Value>) -> Result<Value, String> {
        // Check if instances can be created
        let type_def = self
            .get_type(type_id)
            .ok_or_else(|| format!("Тип с ID {} не найден", type_id.0))?;

        if !type_def.is_instantiable {
            return Err(format!(
                "Невозможно создать экземпляр типа '{}'",
                type_def.name
            ));
        }

        // For native types, use the factory
        if type_def.is_native {
            let factory = self
                .factories
                .read()
                .unwrap()
                .get(&type_id)
                .cloned()
                .ok_or_else(|| format!("Тип '{}' не имеет фабрики", type_def.name))?;

            let object = factory(args)?;
            let type_name = type_def.name.clone();
            return Ok(Value::NativeObject {
                type_id,
                type_name,
                object,
            });
        }

        // For Kumir classes, create Object with fields
        let mut fields = std::collections::BTreeMap::new();
        for field in &type_def.fields {
            let default_value = field.default.clone().unwrap_or(Value::Undefined);
            fields.insert(field.name.clone(), default_value);
        }

        Ok(Value::Object { type_id, fields })
    }

    // =========================================================================
    //                       METHOD CALLING
    // =========================================================================

    /// Call a static method of a type.
    ///
    /// Used for `TypeName.Method(args)`
    pub fn call_static_method(
        &self,
        type_id: TypeId,
        method: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let handler = self
            .static_handlers
            .read()
            .unwrap()
            .get(&type_id)
            .cloned()
            .ok_or_else(|| {
                let name = self
                    .get_type_name(type_id)
                    .unwrap_or_else(|| type_id.0.to_string());
                format!("Тип '{}' не поддерживает статические методы", name)
            })?;

        handler(method, args)
    }

    /// Call an instance method.
    ///
    /// Used for `object.method(args)`
    pub fn call_instance_method(
        &self,
        type_id: TypeId,
        object: &Arc<dyn Any + Send + Sync>,
        method: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let handler = self
            .instance_handlers
            .read()
            .unwrap()
            .get(&type_id)
            .cloned()
            .ok_or_else(|| {
                let name = self
                    .get_type_name(type_id)
                    .unwrap_or_else(|| type_id.0.to_string());
                format!("Тип '{}' не поддерживает методы экземпляра", name)
            })?;

        handler(object, method, args)
    }

    // =========================================================================
    //                       INHERITANCE
    // =========================================================================

    /// Check if one type is a subtype of another.
    pub fn is_subtype(&self, child: TypeId, parent: TypeId) -> bool {
        if child == parent {
            return true;
        }

        let types = self.types.read().unwrap();
        let mut current = child;

        while let Some(type_def) = types.get(&current) {
            if let Some(p) = type_def.parent {
                if p == parent {
                    return true;
                }
                current = p;
            } else {
                break;
            }
        }

        false
    }

    /// Get the inheritance chain of a type.
    pub fn get_inheritance_chain(&self, type_id: TypeId) -> Vec<TypeId> {
        let mut chain = vec![type_id];
        let types = self.types.read().unwrap();
        let mut current = type_id;

        while let Some(type_def) = types.get(&current) {
            if let Some(parent) = type_def.parent {
                chain.push(parent);
                current = parent;
            } else {
                break;
            }
        }

        chain
    }

    // =========================================================================
    //                       UTILITIES
    // =========================================================================

    /// Get all registered types.
    pub fn all_types(&self) -> Vec<TypeDef> {
        self.types.read().unwrap().values().cloned().collect()
    }

    /// Get all types in a module.
    pub fn types_in_module(&self, module: &str) -> Vec<TypeDef> {
        self.types
            .read()
            .unwrap()
            .values()
            .filter(|t| t.module.as_deref() == Some(module))
            .cloned()
            .collect()
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Thread-safe
unsafe impl Send for TypeRegistry {}
unsafe impl Sync for TypeRegistry {}

// =============================================================================
//                           BUILDERS
// =============================================================================

/// Builder for convenient TypeDef creation.
pub struct TypeDefBuilder {
    type_def: TypeDef,
}

impl TypeDefBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            type_def: TypeDef {
                id: TypeId(0), // will be filled during registration
                name: name.to_string(),
                aliases: vec![],
                module: None,
                parent: None,
                implements: vec![],
                fields: vec![],
                is_native: false,
                is_instantiable: true,
                description: String::new(),
            },
        }
    }

    pub fn alias(mut self, alias: &str) -> Self {
        self.type_def.aliases.push(alias.to_string());
        self
    }

    pub fn module(mut self, module: &str) -> Self {
        self.type_def.module = Some(module.to_string());
        self
    }

    pub fn parent(mut self, parent: TypeId) -> Self {
        self.type_def.parent = Some(parent);
        self
    }

    pub fn field(mut self, name: &str, type_name: &str) -> Self {
        self.type_def.fields.push(FieldDef {
            name: name.to_string(),
            type_name: type_name.to_string(),
            type_id: None,
            default: None,
            readonly: false,
            private: false,
        });
        self
    }

    pub fn native(mut self) -> Self {
        self.type_def.is_native = true;
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.type_def.description = desc.to_string();
        self
    }

    pub fn build(self) -> TypeDef {
        self.type_def
    }
}

/// Builder for MethodDef creation.
pub struct MethodDefBuilder {
    method: MethodDef,
}

impl MethodDefBuilder {
    pub fn new(type_id: TypeId, type_name: &str, method_name: &str) -> Self {
        Self {
            method: MethodDef {
                full_name: format!("{}.{}", type_name, method_name),
                name: method_name.to_string(),
                owner_type: type_id,
                params: vec![],
                return_type: None,
                is_static: false,
                is_constructor: false,
                description: String::new(),
            },
        }
    }

    pub fn param(mut self, name: &str, type_name: &str) -> Self {
        self.method.params.push(MethodParam {
            name: name.to_string(),
            type_name: type_name.to_string(),
            mode: ParamMode::In,
            default: None,
        });
        self
    }

    pub fn param_out(mut self, name: &str, type_name: &str) -> Self {
        self.method.params.push(MethodParam {
            name: name.to_string(),
            type_name: type_name.to_string(),
            mode: ParamMode::Out,
            default: None,
        });
        self
    }

    pub fn returns(mut self, type_name: &str) -> Self {
        self.method.return_type = Some(type_name.to_string());
        self
    }

    pub fn static_method(mut self) -> Self {
        self.method.is_static = true;
        self
    }

    pub fn constructor(mut self) -> Self {
        self.method.is_constructor = true;
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.method.description = desc.to_string();
        self
    }

    pub fn build(self) -> MethodDef {
        self.method
    }
}
