//! Реестр типов (TypeRegistry) v2
//!
//! Единая система регистрации и управления типами для КуМир 3.
//!
//! Ключевые принципы:
//! - Единый TypeDef для всех типов (примитивы, КуМир-классы, нативные)
//! - Методы хранятся как обычные алгоритмы с привязкой к типу
//! - Простая модель наследования (образовательный язык!)
//! - Поддержка ключевого слова `новый` для создания объектов

use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::value::Value;

// =============================================================================
//                           ИДЕНТИФИКАТОРЫ
// =============================================================================

/// Уникальный идентификатор типа.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeId(pub u64);

impl TypeId {
    /// Зарезервированные ID для примитивных типов
    pub const INT: TypeId = TypeId(1); // цел
    pub const FLOAT: TypeId = TypeId(2); // вещ
    pub const STRING: TypeId = TypeId(3); // лит
    pub const BOOL: TypeId = TypeId(4); // лог
    pub const CHAR: TypeId = TypeId(5); // сим
    pub const ARRAY: TypeId = TypeId(6); // таб
    pub const VOID: TypeId = TypeId(7); // пустота

    /// Проверить, является ли тип примитивным
    pub fn is_primitive(&self) -> bool {
        self.0 <= 10
    }
}

// =============================================================================
//                           ОПРЕДЕЛЕНИЕ ТИПА
// =============================================================================

/// Определение типа — единая структура для всех видов типов.
///
/// В КуМир 3 все типы (примитивы, классы, нативные) описываются единообразно.
#[derive(Debug, Clone)]
pub struct TypeDef {
    /// Уникальный идентификатор
    pub id: TypeId,

    /// Имя типа (для отображения и поиска)
    pub name: String,

    /// Альтернативные имена (синонимы)
    /// Например: ["HTTPСервер", "HttpServer", "HTTP_Сервер"]
    pub aliases: Vec<String>,

    /// Модуль/библиотека, к которой принадлежит тип
    /// Например: "HTTP", "Файлы", "Графика"
    pub module: Option<String>,

    /// Родительский тип (для наследования)
    pub parent: Option<TypeId>,

    /// Реализуемые интерфейсы/трейты
    pub implements: Vec<TypeId>,

    /// Поля типа
    pub fields: Vec<FieldDef>,

    /// Является ли тип нативным (реализован на Rust)
    pub is_native: bool,

    /// Можно ли создавать экземпляры этого типа
    pub is_instantiable: bool,

    /// Описание типа (для документации)
    pub description: String,
}

/// Определение поля типа.
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Имя поля
    pub name: String,

    /// Тип поля (имя типа)
    pub type_name: String,

    /// ID типа поля (заполняется при регистрации)
    pub type_id: Option<TypeId>,

    /// Значение по умолчанию (если есть)
    pub default: Option<Value>,

    /// Только для чтения?
    pub readonly: bool,

    /// Приватное поле?
    pub private: bool,
}

// =============================================================================
//                           ОПРЕДЕЛЕНИЕ МЕТОДА
// =============================================================================

/// Определение метода типа.
///
/// Методы хранятся отдельно от типа и привязываются по имени:
/// `ИмяТипа.ИмяМетода` или `ИмяТипа::ИмяМетода`
#[derive(Debug, Clone)]
pub struct MethodDef {
    /// Полное имя метода: "HTTPСервер.Запустить"
    pub full_name: String,

    /// Короткое имя: "Запустить"  
    pub name: String,

    /// ID типа, которому принадлежит метод
    pub owner_type: TypeId,

    /// Параметры метода
    pub params: Vec<MethodParam>,

    /// Возвращаемый тип (None = процедура)
    pub return_type: Option<String>,

    /// Статический метод? (вызывается через Тип.Метод, а не объект.метод)
    pub is_static: bool,

    /// Это конструктор/инициализатор?
    pub is_constructor: bool,

    /// Описание метода
    pub description: String,
}

/// Параметр метода.
#[derive(Debug, Clone)]
pub struct MethodParam {
    /// Имя параметра
    pub name: String,

    /// Тип параметра
    pub type_name: String,

    /// Режим: арг, рез, аргрез
    pub mode: ParamMode,

    /// Значение по умолчанию
    pub default: Option<Value>,
}

/// Режим передачи параметра.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamMode {
    /// арг — входной параметр (по значению)
    In,
    /// рез — выходной параметр
    Out,
    /// аргрез — входной и выходной
    InOut,
}

// =============================================================================
//                           ОБРАБОТЧИКИ (для нативных типов)
// =============================================================================

/// Фабрика для создания нативных объектов.
/// Вызывается при `новый ТипИмя(аргументы)`
pub type NativeFactory =
    Arc<dyn Fn(Vec<Value>) -> Result<Arc<dyn Any + Send + Sync>, String> + Send + Sync>;

/// Обработчик методов экземпляра.
/// Вызывается при `объект.метод(аргументы)`
pub type InstanceMethodHandler = Arc<
    dyn Fn(&Arc<dyn Any + Send + Sync>, &str, Vec<Value>) -> Result<Value, String> + Send + Sync,
>;

/// Обработчик статических методов.
/// Вызывается при `ТипИмя.Метод(аргументы)`
pub type StaticMethodHandler = Arc<dyn Fn(&str, Vec<Value>) -> Result<Value, String> + Send + Sync>;

/// Обработчик доступа к полям.
/// Вызывается при `объект.поле`
pub type FieldAccessHandler =
    Arc<dyn Fn(&Arc<dyn Any + Send + Sync>, &str) -> Result<Value, String> + Send + Sync>;

/// Обработчик установки полей.
/// Вызывается при `объект.поле := значение`
pub type FieldSetHandler =
    Arc<dyn Fn(&Arc<dyn Any + Send + Sync>, &str, Value) -> Result<(), String> + Send + Sync>;

// =============================================================================
//                           РЕЕСТР ТИПОВ
// =============================================================================

/// Реестр типов — центральное хранилище всех типов в программе.
///
/// Основные функции:
/// - Регистрация и поиск типов
/// - Создание экземпляров объектов
/// - Вызов методов и доступ к полям
/// - Проверка наследования
pub struct TypeRegistry {
    /// Счётчик для генерации уникальных ID
    next_id: RwLock<u64>,

    /// Типы по ID
    types: RwLock<HashMap<TypeId, TypeDef>>,

    /// Типы по имени (включая aliases)
    types_by_name: RwLock<HashMap<String, TypeId>>,

    /// Методы по полному имени: "ТипИмя.МетодИмя" -> MethodDef
    methods: RwLock<HashMap<String, MethodDef>>,

    /// Методы типа: TypeId -> Vec<MethodDef>
    methods_by_type: RwLock<HashMap<TypeId, Vec<MethodDef>>>,

    // --- Обработчики для нативных типов ---
    /// Фабрики создания объектов
    factories: RwLock<HashMap<TypeId, NativeFactory>>,

    /// Обработчики методов экземпляров
    instance_handlers: RwLock<HashMap<TypeId, InstanceMethodHandler>>,

    /// Обработчики статических методов
    static_handlers: RwLock<HashMap<TypeId, StaticMethodHandler>>,

    /// Обработчики доступа к полям
    #[allow(dead_code)]
    field_getters: RwLock<HashMap<TypeId, FieldAccessHandler>>,

    /// Обработчики установки полей
    #[allow(dead_code)]
    field_setters: RwLock<HashMap<TypeId, FieldSetHandler>>,
}

impl TypeRegistry {
    /// Создать новый реестр с предзарегистрированными примитивами.
    pub fn new() -> Self {
        let registry = Self {
            next_id: RwLock::new(100), // 0-99 зарезервированы
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

    /// Зарегистрировать примитивные типы.
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

    /// Сгенерировать новый уникальный TypeId.
    fn next_type_id(&self) -> TypeId {
        let mut id = self.next_id.write().unwrap();
        let type_id = TypeId(*id);
        *id += 1;
        type_id
    }

    // =========================================================================
    //                       РЕГИСТРАЦИЯ ТИПОВ
    // =========================================================================

    /// Зарегистрировать новый тип.
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

    /// Зарегистрировать нативный тип с обработчиками.
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

    /// Зарегистрировать метод типа.
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
    //                       ПОИСК ТИПОВ
    // =========================================================================

    /// Получить TypeId по имени.
    pub fn get_type_id(&self, name: &str) -> Option<TypeId> {
        self.types_by_name.read().unwrap().get(name).copied()
    }

    /// Получить определение типа по ID.
    pub fn get_type(&self, type_id: TypeId) -> Option<TypeDef> {
        self.types.read().unwrap().get(&type_id).cloned()
    }

    /// Получить имя типа по ID.
    pub fn get_type_name(&self, type_id: TypeId) -> Option<String> {
        self.types
            .read()
            .unwrap()
            .get(&type_id)
            .map(|t| t.name.clone())
    }

    /// Получить методы типа.
    pub fn get_methods(&self, type_id: TypeId) -> Vec<MethodDef> {
        self.methods_by_type
            .read()
            .unwrap()
            .get(&type_id)
            .cloned()
            .unwrap_or_default()
    }

    /// Найти метод по полному имени.
    pub fn get_method(&self, full_name: &str) -> Option<MethodDef> {
        self.methods.read().unwrap().get(full_name).cloned()
    }

    /// Найти статический метод типа.
    pub fn find_static_method(&self, type_name: &str, method_name: &str) -> Option<MethodDef> {
        let full_name = format!("{}.{}", type_name, method_name);
        self.methods.read().unwrap().get(&full_name).cloned()
    }

    // =========================================================================
    //                       СОЗДАНИЕ ОБЪЕКТОВ
    // =========================================================================

    /// Создать экземпляр нативного типа.
    ///
    /// Вызывается при `новый ТипИмя(аргументы)`
    pub fn create_instance(&self, type_id: TypeId, args: Vec<Value>) -> Result<Value, String> {
        // Проверяем, можно ли создавать экземпляры
        let type_def = self
            .get_type(type_id)
            .ok_or_else(|| format!("Тип с ID {} не найден", type_id.0))?;

        if !type_def.is_instantiable {
            return Err(format!(
                "Невозможно создать экземпляр типа '{}'",
                type_def.name
            ));
        }

        // Если нативный тип — используем фабрику
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

        // Для КуМир-классов создаём Object с полями
        let mut fields = std::collections::BTreeMap::new();
        for field in &type_def.fields {
            let default_value = field.default.clone().unwrap_or(Value::Undefined);
            fields.insert(field.name.clone(), default_value);
        }

        Ok(Value::Object { type_id, fields })
    }

    // =========================================================================
    //                       ВЫЗОВ МЕТОДОВ
    // =========================================================================

    /// Вызвать статический метод типа.
    ///
    /// Используется для `ТипИмя.Метод(аргументы)`
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

    /// Вызвать метод экземпляра.
    ///
    /// Используется для `объект.метод(аргументы)`
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
    //                       НАСЛЕДОВАНИЕ
    // =========================================================================

    /// Проверить, является ли один тип потомком другого.
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

    /// Получить цепочку наследования типа.
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
    //                       УТИЛИТЫ
    // =========================================================================

    /// Получить все зарегистрированные типы.
    pub fn all_types(&self) -> Vec<TypeDef> {
        self.types.read().unwrap().values().cloned().collect()
    }

    /// Получить все типы модуля.
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
//                           БИЛДЕРЫ
// =============================================================================

/// Билдер для удобного создания TypeDef.
pub struct TypeDefBuilder {
    type_def: TypeDef,
}

impl TypeDefBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            type_def: TypeDef {
                id: TypeId(0), // будет заполнено при регистрации
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

/// Билдер для создания MethodDef.
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
