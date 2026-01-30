//! Система библиотек Kumir 3
//!
//! Этот модуль определяет мощную систему библиотек с поддержкой:
//! - Функций со статической типизацией параметров и возвращаемых значений
//! - Нативных обработчиков (ссылки на Rust функции)
//! - Типов и классов
//! - Констант
//! - Версионирования
//! - Зависимостей между библиотеками

use std::collections::HashMap;
use std::sync::Arc;
use super::type_spec::TypeSpec;
use super::value::Value;
use super::config::DependencySpec;

// ============================================================================
//                         ОПРЕДЕЛЕНИЕ ФУНКЦИЙ
// ============================================================================

/// Режим передачи параметра
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamPassMode {
    /// Передача по значению (арг)
    ByValue,
    /// Передача по ссылке для изменения (рез)
    ByRef,
    /// Передача по значению с возможностью изменения (аргрез)
    ByValueRef,
}

/// Определение параметра функции библиотеки
#[derive(Debug, Clone)]
pub struct LibParamDef {
    /// Имя параметра
    pub name: &'static str,
    /// Тип параметра
    pub param_type: TypeSpec,
    /// Режим передачи
    pub mode: ParamPassMode,
    /// Значение по умолчанию (опционально)
    pub default: Option<Value>,
    /// Описание параметра
    pub description: &'static str,
}

impl LibParamDef {
    /// Создаёт параметр с передачей по значению
    pub const fn value(name: &'static str, param_type: TypeSpec) -> Self {
        Self {
            name,
            param_type,
            mode: ParamPassMode::ByValue,
            default: None,
            description: "",
        }
    }

    /// Создаёт параметр с передачей по ссылке (результат)
    pub const fn result(name: &'static str, param_type: TypeSpec) -> Self {
        Self {
            name,
            param_type,
            mode: ParamPassMode::ByRef,
            default: None,
            description: "",
        }
    }

    /// Создаёт параметр аргрез
    pub const fn value_ref(name: &'static str, param_type: TypeSpec) -> Self {
        Self {
            name,
            param_type,
            mode: ParamPassMode::ByValueRef,
            default: None,
            description: "",
        }
    }

    /// Добавляет описание
    pub const fn with_desc(mut self, desc: &'static str) -> Self {
        self.description = desc;
        self
    }
}

// ============================================================================
//                    НАТИВНЫЕ ОБРАБОТЧИКИ
// ============================================================================

/// Результат выполнения нативной функции
pub type NativeResult = Result<Value, String>;

/// Тип нативного обработчика функции
pub type NativeFn = Arc<dyn Fn(&[Value]) -> NativeResult + Send + Sync>;

/// Определение функции библиотеки
#[derive(Clone)]
pub struct LibFunctionDef {
    /// Основное имя функции (русское)
    pub name: &'static str,
    /// Альтернативные имена (английские и другие)
    pub aliases: &'static [&'static str],
    /// Описание функции
    pub description: &'static str,
    /// Параметры функции
    pub params: Vec<LibParamDef>,
    /// Возвращаемый тип
    pub returns: TypeSpec,
    /// Нативный обработчик (опционально)
    pub handler: Option<NativeFn>,
    /// Является ли функция процедурой (не возвращает значение)
    pub is_procedure: bool,
    /// Пример использования
    pub example: &'static str,
    /// Начиная с какой версии доступна
    pub since_version: &'static str,
    /// Является ли устаревшей
    pub deprecated: Option<&'static str>,
}

impl std::fmt::Debug for LibFunctionDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibFunctionDef")
            .field("name", &self.name)
            .field("aliases", &self.aliases)
            .field("params", &self.params)
            .field("returns", &self.returns)
            .field("is_procedure", &self.is_procedure)
            .finish()
    }
}

impl LibFunctionDef {
    /// Создаёт определение функции
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            aliases: &[],
            description: "",
            params: Vec::new(),
            returns: TypeSpec::Null,
            handler: None,
            is_procedure: false,
            example: "",
            since_version: "3.0",
            deprecated: None,
        }
    }

    /// Добавляет альтернативные имена
    pub fn with_aliases(mut self, aliases: &'static [&'static str]) -> Self {
        self.aliases = aliases;
        self
    }

    /// Добавляет описание
    pub fn with_description(mut self, desc: &'static str) -> Self {
        self.description = desc;
        self
    }

    /// Добавляет параметр
    pub fn with_param(mut self, param: LibParamDef) -> Self {
        self.params.push(param);
        self
    }

    /// Устанавливает возвращаемый тип
    pub fn returns(mut self, ret_type: TypeSpec) -> Self {
        self.returns = ret_type;
        self
    }

    /// Устанавливает нативный обработчик
    pub fn with_handler<F>(mut self, handler: F) -> Self
    where
        F: Fn(&[Value]) -> NativeResult + Send + Sync + 'static,
    {
        self.handler = Some(Arc::new(handler));
        self
    }

    /// Помечает как процедуру
    pub fn as_procedure(mut self) -> Self {
        self.is_procedure = true;
        self.returns = TypeSpec::Null;
        self
    }

    /// Добавляет пример
    pub fn with_example(mut self, example: &'static str) -> Self {
        self.example = example;
        self
    }

    /// Помечает как устаревшую
    pub fn deprecate(mut self, message: &'static str) -> Self {
        self.deprecated = Some(message);
        self
    }

    /// Вызывает нативный обработчик
    pub fn call(&self, args: &[Value]) -> NativeResult {
        match &self.handler {
            Some(h) => h(args),
            None => Err(format!("Функция '{}' не имеет нативного обработчика", self.name)),
        }
    }

    /// Проверяет, соответствует ли имя функции
    pub fn matches_name(&self, name: &str) -> bool {
        self.name == name || self.aliases.contains(&name)
    }
}

// ============================================================================
//                         ОПРЕДЕЛЕНИЕ ТИПОВ
// ============================================================================

/// Определение поля типа
#[derive(Debug, Clone)]
pub struct LibFieldDef {
    /// Имя поля
    pub name: &'static str,
    /// Тип поля
    pub field_type: TypeSpec,
    /// Описание
    pub description: &'static str,
    /// Доступно только для чтения
    pub readonly: bool,
}

/// Определение типа/класса в библиотеке
#[derive(Debug, Clone)]
pub struct ValueDef {
    /// Основное имя типа
    pub name: &'static str,
    /// Альтернативные имена
    pub aliases: &'static [&'static str],
    /// Описание типа
    pub description: &'static str,
    /// Поля (для структур/классов)
    pub fields: Vec<LibFieldDef>,
    /// Методы экземпляра
    pub methods: Vec<&'static str>,
    /// Статические методы
    pub static_methods: Vec<&'static str>,
    /// Является ли нативным типом (обёртка над Rust типом)
    pub is_native: bool,
}

impl ValueDef {
    /// Создаёт определение типа
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            aliases: &[],
            description: "",
            fields: Vec::new(),
            methods: Vec::new(),
            static_methods: Vec::new(),
            is_native: false,
        }
    }

    /// Проверяет, соответствует ли имя типа
    pub fn matches_name(&self, name: &str) -> bool {
        self.name == name || self.aliases.contains(&name)
    }
}

// ============================================================================
//                         ОПРЕДЕЛЕНИЕ КЛАССОВ
// ============================================================================

/// Определение класса библиотеки
#[derive(Debug, Clone)]
pub struct ClassDef {
    /// Имя класса
    pub name: &'static str,
    /// Алиасы
    pub aliases: &'static [&'static str],
    /// Описание
    pub description: &'static str,
    /// Поля класса
    pub fields: Vec<LibFieldDef>,
    /// Экземплярные методы
    pub methods: Vec<&'static str>,
    /// Статические методы
    pub static_methods: Vec<&'static str>,
    /// Конструкторы
    pub constructors: Vec<&'static str>,
    /// Базовый класс (для наследования), опционально
    pub base_class: Option<&'static str>,
    /// Реализуемые интерфейсы (имена)
    pub interfaces: Vec<&'static str>,
    /// Является ли нативным классом
    pub is_native: bool,
}

impl ClassDef {
    /// Создаёт определение класса
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            aliases: &[],
            description: "",
            fields: Vec::new(),
            methods: Vec::new(),
            static_methods: Vec::new(),
            constructors: Vec::new(),
            base_class: None,
            interfaces: Vec::new(),
            is_native: false,
        }
    }

    /// Проверяет, соответствует ли имя класса
    pub fn matches_name(&self, name: &str) -> bool {
        self.name == name || self.aliases.contains(&name)
    }
}

// ============================================================================
//                         ОПРЕДЕЛЕНИЕ КОНСТАНТ
// ============================================================================

/// Определение константы библиотеки
#[derive(Debug, Clone)]
pub struct LibConstantDef {
    /// Имя константы
    pub name: &'static str,
    /// Альтернативные имена
    pub aliases: &'static [&'static str],
    /// Тип константы
    pub const_type: TypeSpec,
    /// Значение
    pub value: Value,
    /// Описание
    pub description: &'static str,
}

// ============================================================================
//                         ОПРЕДЕЛЕНИЕ БИБЛИОТЕКИ
// ============================================================================

/// Версия библиотеки
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LibVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl LibVersion {
    pub const fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }

    /// Конвертирует в Version из модуля version
    pub fn to_version(&self) -> super::version::Version {
        super::version::Version::new(self.major, self.minor, self.patch)
    }

    /// Создаёт из Version
    pub fn from_version(v: &super::version::Version) -> Self {
        Self::new(v.major, v.minor, v.patch)
    }
}

impl std::fmt::Display for LibVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl From<super::version::Version> for LibVersion {
    fn from(v: super::version::Version) -> Self {
        Self::new(v.major, v.minor, v.patch)
    }
}

impl From<LibVersion> for super::version::Version {
    fn from(v: LibVersion) -> Self {
        Self::new(v.major, v.minor, v.patch)
    }
}

/// Зависимость от другой библиотеки
#[derive(Debug, Clone)]
pub struct LibDependency {
    /// Имя библиотеки
    pub name: &'static str,
    /// Минимальная версия (включительно)
    pub min_version: Option<LibVersion>,
    /// Максимальная версия (не включительно - для мажорных обновлений с поломкой совместимости)
    pub max_version: Option<LibVersion>,
    /// Является ли обязательной
    pub required: bool,
}

impl LibDependency {
    /// Создаёт обязательную зависимость без ограничений версии
    pub const fn required(name: &'static str) -> Self {
        Self {
            name,
            min_version: None,
            max_version: None,
            required: true,
        }
    }

    /// Создаёт опциональную зависимость
    pub const fn optional(name: &'static str) -> Self {
        Self {
            name,
            min_version: None,
            max_version: None,
            required: false,
        }
    }

    /// Создаёт зависимость с минимальной версией (>=)
    pub const fn min(name: &'static str, version: LibVersion) -> Self {
        Self {
            name,
            min_version: Some(version),
            max_version: None,
            required: true,
        }
    }

    /// Создаёт зависимость с диапазоном версий [min, max)
    pub const fn range(name: &'static str, min: LibVersion, max: LibVersion) -> Self {
        Self {
            name,
            min_version: Some(min),
            max_version: Some(max),
            required: true,
        }
    }

    /// Создаёт зависимость совместимую с мажорной версией (^X.Y.Z)
    /// Эквивалентно [X.Y.Z, X+1.0.0)
    pub const fn compatible(name: &'static str, version: LibVersion) -> Self {
        Self {
            name,
            min_version: Some(version),
            max_version: Some(LibVersion::new(version.major + 1, 0, 0)),
            required: true,
        }
    }

    /// Устанавливает минимальную версию
    pub const fn with_min(mut self, version: LibVersion) -> Self {
        self.min_version = Some(version);
        self
    }

    /// Устанавливает максимальную версию (не включительно)
    pub const fn with_max(mut self, version: LibVersion) -> Self {
        self.max_version = Some(version);
        self
    }

    /// Делает зависимость опциональной
    pub const fn as_optional(mut self) -> Self {
        self.required = false;
        self
    }

    /// Проверяет, совместима ли версия с зависимостью
    pub fn is_compatible(&self, version: &LibVersion) -> bool {
        // Проверяем минимальную версию (>=)
        if let Some(ref min) = self.min_version {
            if version < min {
                return false;
            }
        }

        // Проверяем максимальную версию (<)
        if let Some(ref max) = self.max_version {
            if version >= max {
                return false;
            }
        }

        true
    }

    /// Форматирует диапазон версий в читаемый вид
    pub fn version_range_display(&self) -> String {
        match (&self.min_version, &self.max_version) {
            (None, None) => "*".to_string(),
            (Some(min), None) => format!(">={}", min),
            (None, Some(max)) => format!("<{}", max),
            (Some(min), Some(max)) => {
                // Проверяем, похоже ли это на ^X.Y.Z (compatible)
                if max.major == min.major + 1 && max.minor == 0 && max.patch == 0 {
                    format!("^{}", min)
                } else {
                    format!(">={}, <{}", min, max)
                }
            }
        }
    }

    /// Конвертирует в VersionSpec
    pub fn to_version_spec(&self) -> super::version::VersionSpec {
        use super::version::{VersionSpec, VersionReq, VersionOp};
        
        let mut spec = VersionSpec::any();
        
        // Добавляем минимальную версию (>=)
        if let Some(min) = &self.min_version {
            spec.add_requirement(VersionReq {
                op: VersionOp::GreaterEq,
                version: min.to_version(),
            });
        }
        
        // Добавляем максимальную версию (<)
        if let Some(max) = &self.max_version {
            spec.add_requirement(VersionReq {
                op: VersionOp::Less,
                version: max.to_version(),
            });
        }
        
        spec
    }

    /// Конвертирует в DependencySpec для резолвера
    pub fn to_dependency_spec(&self) -> DependencySpec {
        let mut dep = DependencySpec::version(self.name.to_string(), self.to_version_spec());
        if !self.required {
            dep = dep.optional();
        }
        dep
    }
}

/// Полное определение библиотеки Kumir 3
#[derive(Clone)]
pub struct LibraryDef {
    /// Уникальный идентификатор библиотеки
    pub id: &'static str,
    /// Основное имя библиотеки (русское)
    pub name: &'static str,
    /// Альтернативные имена
    pub aliases: &'static [&'static str],
    /// Описание библиотеки
    pub description: &'static str,
    /// Версия библиотеки
    pub version: LibVersion,
    /// Автор
    pub author: &'static str,
    /// Зависимости
    pub dependencies: Vec<LibDependency>,
    /// Функции библиотеки
    pub functions: Vec<LibFunctionDef>,
    /// Типы библиотеки
    pub types: Vec<ValueDef>,
    /// Классы библиотеки
    pub classes: Vec<ClassDef>,
    /// Константы библиотеки
    pub constants: Vec<LibConstantDef>,
    /// Начиная с какой версии Kumir доступна
    pub kumir_version: &'static str,
    /// Является ли стабильной
    pub stable: bool,
}

impl std::fmt::Debug for LibraryDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibraryDef")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("version", &self.version)
            .field("functions_count", &self.functions.len())
            .field("types_count", &self.types.len())
            .field("classes_count", &self.classes.len())
            .finish()
    }
}

impl LibraryDef {
    /// Создаёт новую библиотеку
    pub fn new(id: &'static str, name: &'static str) -> Self {
        Self {
            id,
            name,
            aliases: &[],
            description: "",
            version: LibVersion::new(1, 0, 0),
            author: "",
            dependencies: Vec::new(),
            functions: Vec::new(),
            types: Vec::new(),
            classes: Vec::new(),
            constants: Vec::new(),
            kumir_version: "3.0",
            stable: false,
        }
    }

    /// Ищет функцию по имени
    pub fn find_function(&self, name: &str) -> Option<&LibFunctionDef> {
        self.functions.iter().find(|f| f.matches_name(name))
    }

    /// Ищет тип по имени
    pub fn find_type(&self, name: &str) -> Option<&ValueDef> {
        self.types.iter().find(|t| t.matches_name(name))
    }

    /// Ищет класс по имени
    pub fn find_class(&self, name: &str) -> Option<&ClassDef> {
        self.classes.iter().find(|c| c.matches_name(name))
    }

    /// Проверяет, соответствует ли имя библиотеки
    pub fn matches_name(&self, name: &str) -> bool {
        self.name == name || self.id == name || self.aliases.contains(&name)
    }

    /// Вызывает функцию библиотеки
    pub fn call_function(&self, name: &str, args: &[Value]) -> NativeResult {
        match self.find_function(name) {
            Some(func) => func.call(args),
            None => Err(format!("Функция '{}' не найдена в библиотеке '{}'", name, self.name)),
        }
    }

    /// Получает все имена функций (включая алиасы)
    pub fn all_function_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        for func in &self.functions {
            names.push(func.name);
            names.extend(func.aliases.iter().copied());
        }
        names
    }

    /// Получает все имена типов (включая алиасы)
    pub fn all_type_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        for t in &self.types {
            names.push(t.name);
            names.extend(t.aliases.iter().copied());
        }
        for c in &self.classes {
            names.push(c.name);
            names.extend(c.aliases.iter().copied());
        }
        names
    }

    /// Получает имена всех классов (включая алиасы)
    pub fn all_class_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        for c in &self.classes {
            names.push(c.name);
            names.extend(c.aliases.iter().copied());
        }
        names
    }
}

// ============================================================================
//                         РЕЕСТР БИБЛИОТЕК
// ============================================================================

/// Глобальный реестр библиотек
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

    /// Регистрирует библиотеку
    pub fn register(&mut self, lib: LibraryDef) {
        let id = lib.id.to_string();
        // Также регистрируем по всем алиасам
        for &alias in lib.aliases {
            self.libraries.insert(alias.to_string(), lib.clone());
        }
        self.libraries.insert(lib.name.to_string(), lib.clone());
        self.libraries.insert(id, lib);
    }

    /// Ищет библиотеку по имени
    pub fn find(&self, name: &str) -> Option<&LibraryDef> {
        self.libraries.get(name)
    }

    /// Проверяет, существует ли библиотека
    pub fn exists(&self, name: &str) -> bool {
        self.libraries.contains_key(name)
    }

    /// Получает все зарегистрированные библиотеки (уникальные)
    pub fn all(&self) -> Vec<&LibraryDef> {
        let mut seen = std::collections::HashSet::new();
        self.libraries.values()
            .filter(|lib| seen.insert(lib.id))
            .collect()
    }
}

// ============================================================================
//                         МАКРОСЫ ДЛЯ УДОБСТВА
// ============================================================================

/// Макрос для создания параметра
#[macro_export]
macro_rules! lib_param {
    ($name:expr, $type:expr) => {
        LibParamDef::value($name, $type)
    };
    ($name:expr, $type:expr, ref) => {
        LibParamDef::result($name, $type)
    };
    ($name:expr, $type:expr, inout) => {
        LibParamDef::value_ref($name, $type)
    };
}

/// Макрос для создания функции
#[macro_export]
macro_rules! lib_fn {
    ($name:expr) => {
        LibFunctionDef::new($name)
    };
}
