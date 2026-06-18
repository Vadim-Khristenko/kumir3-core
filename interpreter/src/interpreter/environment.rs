//! Среда выполнения (Environment) для интерпретатора Кумир 3
//!
//! Среда хранит переменные, алгоритмы, классы и управляет областями видимости.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::error::{RuntimeError, RuntimeResult};
use super::library_bridge::LibraryManager;
use shared::runtime::KumirRuntime;
use shared::types::{
    Algorithm, ClassDef, ImplDef, InterfaceDef, OverloadedAlgorithm, Program, TraitDef,
    TypeRegistry, Value,
};

// =============================================================================
//                           SCOPE (Область видимости)
// =============================================================================

/// Область видимости переменных.
#[derive(Debug, Clone)]
pub struct Scope {
    /// Переменные в данной области видимости
    variables: HashMap<String, Value>,
    /// Константы (нельзя переопределить)
    constants: HashMap<String, Value>,
}

impl Scope {
    /// Создаёт новую область видимости.
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            constants: HashMap::new(),
        }
    }

    /// Определяет переменную.
    pub fn define(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    /// Определяет константу.
    pub fn define_const(&mut self, name: String, value: Value) {
        self.constants.insert(name, value);
    }

    /// Получает значение переменной.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables
            .get(name)
            .or_else(|| self.constants.get(name))
    }

    /// Получает изменяемую ссылку на переменную.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.variables.get_mut(name)
    }

    /// Проверяет, существует ли переменная.
    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name) || self.constants.contains_key(name)
    }

    /// Проверяет, является ли переменная константой.
    pub fn is_const(&self, name: &str) -> bool {
        self.constants.contains_key(name)
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
//                           CALL FRAME (Кадр вызова)
// =============================================================================

/// Кадр вызова алгоритма.
///
/// [KITE 4] Лексическая модель: кадр содержит **стек блочных областей**
/// (`scopes`), а не одну область. Поиск имени идёт от внутренней области к
/// внешней внутри этого кадра, затем — в глобальной области; кадры вызывающих
/// **не** просматриваются (нет динамической утечки видимости).
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Имя алгоритма
    pub algorithm_name: String,
    /// Стек лексических областей видимости (scopes[0] — параметры/верхний уровень)
    scopes: Vec<Scope>,
    /// Возвращаемое значение (знач)
    pub result_value: Option<Value>,
    /// Текущий объект (для методов)
    pub this: Option<Value>,
    /// [KITE 11] Класс, в котором определён исполняемый метод (для `предок`).
    pub defining_class: Option<String>,
}

impl CallFrame {
    /// Создаёт новый кадр вызова с одной (верхней) областью.
    pub fn new(algorithm_name: impl Into<String>) -> Self {
        Self {
            algorithm_name: algorithm_name.into(),
            scopes: vec![Scope::new()],
            result_value: None,
            this: None,
            defining_class: None,
        }
    }

    /// Создаёт кадр для метода с объектом this.
    pub fn with_this(algorithm_name: impl Into<String>, this: Value) -> Self {
        Self {
            algorithm_name: algorithm_name.into(),
            scopes: vec![Scope::new()],
            result_value: None,
            this: Some(this),
            defining_class: None,
        }
    }

    /// Открывает вложенную блочную область видимости.
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Закрывает текущую блочную область (верхнюю не трогаем).
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Определяет переменную во внутренней (текущей) области.
    fn define(&mut self, name: String, value: Value) {
        self.scopes
            .last_mut()
            .expect("frame has at least one scope")
            .define(name, value);
    }

    /// Ищет значение от внутренней области к внешней.
    fn get(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    /// Есть ли имя в любой области кадра.
    fn contains(&self, name: &str) -> bool {
        self.scopes.iter().any(|s| s.contains(name))
    }

    /// Является ли имя константой в кадре.
    fn is_const(&self, name: &str) -> bool {
        self.scopes.iter().any(|s| s.is_const(name))
    }

    /// Присваивает значение существующей переменной (в области, где она объявлена).
    fn assign(&mut self, name: &str, value: Value) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(name) {
                *var = value;
                return true;
            }
        }
        false
    }
}

// =============================================================================
//                           ENVIRONMENT (Среда выполнения)
// =============================================================================

use super::file_importer::FileImporter;
use shared::types::library::NativeFn;

/// Среда выполнения программы.
pub struct Environment {
    /// Глобальные переменные
    globals: Scope,

    /// Стек кадров вызова
    call_stack: Vec<CallFrame>,

    /// Определённые алгоритмы
    algorithms: HashMap<String, Algorithm>,

    /// Перегруженные алгоритмы
    overloaded_algorithms: HashMap<String, OverloadedAlgorithm>,

    /// Определённые классы
    classes: HashMap<String, ClassDef>,

    /// Определённые интерфейсы
    interfaces: HashMap<String, InterfaceDef>,

    /// Определённые типажи (trait)
    traits: HashMap<String, TraitDef>,

    /// Реализации типажей (target_type -> trait_name -> ImplDef)
    impls: HashMap<String, HashMap<String, ImplDef>>,

    /// Определённые перечисления (enum_name -> variants)
    enums: HashMap<String, Vec<String>>,

    /// Нативные функции библиотек (имя -> обработчик)
    native_functions: HashMap<String, NativeFn>,

    /// Реестр типов
    type_registry: Arc<RwLock<TypeRegistry>>,

    /// Буфер вывода (для тестирования)
    output_buffer: Vec<String>,

    /// Режим отладки
    debug_mode: bool,

    /// Максимальная глубина вызова (защита от бесконечной рекурсии)
    max_call_depth: usize,

    /// Менеджер библиотек (shared reference)
    library_manager: Option<Arc<RwLock<LibraryManager>>>,

    /// Импортер файлов .kum
    file_importer: Option<Arc<RwLock<FileImporter>>>,

    /// Runtime для async операций
    kumir_runtime: Arc<KumirRuntime>,
}

impl Environment {
    /// Создаёт новую среду выполнения.
    pub fn new() -> Self {
        Self {
            globals: Scope::new(),
            call_stack: Vec::new(),
            algorithms: HashMap::new(),
            overloaded_algorithms: HashMap::new(),
            classes: HashMap::new(),
            interfaces: HashMap::new(),
            traits: HashMap::new(),
            impls: HashMap::new(),
            enums: HashMap::new(),
            native_functions: HashMap::new(),
            type_registry: Arc::new(RwLock::new(TypeRegistry::new())),
            output_buffer: Vec::new(),
            debug_mode: false,
            max_call_depth: 1000,
            library_manager: None,
            file_importer: None,
            kumir_runtime: Arc::new(KumirRuntime::new()),
        }
    }

    /// Создаёт среду из программы (загружает алгоритмы, классы и т.д.)
    pub fn from_program(program: &Program) -> RuntimeResult<Self> {
        let mut env = Self::new();

        // Загружаем алгоритмы
        for alg in &program.algorithms {
            env.define_algorithm(alg.clone());
        }

        // Загружаем перегруженные алгоритмы
        for overloaded in &program.overloaded_algorithms {
            env.overloaded_algorithms
                .insert(overloaded.name.to_string(), overloaded.clone());
        }

        // Загружаем классы
        for class in &program.classes {
            env.define_class(class.clone());
        }

        // Загружаем главный алгоритм
        if let Some(main) = &program.main {
            env.define_algorithm(main.clone());
        }

        Ok(env)
    }

    // =========================================================================
    //                    УПРАВЛЕНИЕ ПЕРЕМЕННЫМИ
    // =========================================================================

    /// Определяет глобальную переменную.
    pub fn define_global(&mut self, name: String, value: Value) {
        self.globals.define(name, value);
    }

    /// Определяет локальную переменную в текущей (внутренней) области видимости.
    pub fn define_local(&mut self, name: String, value: Value) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.define(name, value);
        } else {
            self.globals.define(name, value);
        }
    }

    /// Получает значение переменной.
    ///
    /// [KITE 4] Лексический поиск: только текущий кадр (его стек областей) и
    /// глобальная область. Кадры вызывающих не просматриваются.
    pub fn get_variable(&self, name: &str) -> RuntimeResult<&Value> {
        if let Some(frame) = self.call_stack.last()
            && let Some(value) = frame.get(name)
        {
            return Ok(value);
        }
        self.globals
            .get(name)
            .ok_or_else(|| RuntimeError::undefined_variable(name))
    }

    /// Присваивает значение переменной.
    ///
    /// [KITE 4] Ищет имя в текущем кадре, затем в глобальных; если нигде нет —
    /// создаёт локальную во внутренней области. Кадры вызывающих не трогаются.
    pub fn set_variable(&mut self, name: &str, value: Value) -> RuntimeResult<()> {
        if let Some(frame) = self.call_stack.last_mut()
            && frame.contains(name)
        {
            if frame.is_const(name) {
                return Err(RuntimeError::new(
                    format!("Нельзя изменить константу '{}'", name),
                    super::error::RuntimeErrorKind::Other,
                ));
            }
            frame.assign(name, value);
            return Ok(());
        }

        if self.globals.contains(name) {
            if self.globals.is_const(name) {
                return Err(RuntimeError::new(
                    format!("Нельзя изменить константу '{}'", name),
                    super::error::RuntimeErrorKind::Other,
                ));
            }
            if let Some(var) = self.globals.get_mut(name) {
                *var = value;
                return Ok(());
            }
        }

        // Переменная не найдена — создаём локальную в текущей области.
        self.define_local(name.to_string(), value);
        Ok(())
    }

    /// Проверяет, определена ли переменная.
    pub fn has_variable(&self, name: &str) -> bool {
        if let Some(frame) = self.call_stack.last()
            && frame.contains(name)
        {
            return true;
        }
        self.globals.contains(name)
    }

    /// [KITE 4] Открывает вложенную блочную область видимости в текущем кадре
    /// (для тел `нц`/`если` и т.п.). Без активного кадра — нет эффекта.
    pub fn push_scope(&mut self) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.push_scope();
        }
    }

    /// [KITE 4] Закрывает текущую блочную область видимости в текущем кадре.
    pub fn pop_scope(&mut self) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.pop_scope();
        }
    }

    // =========================================================================
    //                    УПРАВЛЕНИЕ АЛГОРИТМАМИ
    // =========================================================================

    /// Определяет алгоритм.
    pub fn define_algorithm(&mut self, algorithm: Algorithm) {
        self.algorithms
            .insert(algorithm.name.to_string(), algorithm);
    }

    /// Определяет алгоритм с заданным именем (для импортов с префиксом модуля).
    pub fn define_algorithm_with_name(&mut self, name: &str, algorithm: Algorithm) {
        self.algorithms.insert(name.to_string(), algorithm);
    }

    /// Получает алгоритм по имени.
    pub fn get_algorithm(&self, name: &str) -> RuntimeResult<&Algorithm> {
        self.algorithms
            .get(name)
            .ok_or_else(|| RuntimeError::undefined_algorithm(name))
    }

    /// Получает перегруженный алгоритм.
    pub fn get_overloaded_algorithm(&self, name: &str) -> Option<&OverloadedAlgorithm> {
        self.overloaded_algorithms.get(name)
    }

    /// Проверяет, определён ли алгоритм.
    pub fn has_algorithm(&self, name: &str) -> bool {
        self.algorithms.contains_key(name)
    }

    // =========================================================================
    //                    УПРАВЛЕНИЕ КЛАССАМИ
    // =========================================================================

    /// Определяет класс.
    pub fn define_class(&mut self, class: ClassDef) {
        let name = class.name.to_string();
        self.register_class_identity(&name);
        self.classes.insert(name, class);
    }

    /// Определяет класс с заданным именем (для импортов с префиксом модуля).
    pub fn define_class_with_name(&mut self, name: &str, class: ClassDef) {
        self.register_class_identity(name);
        self.classes.insert(name.to_string(), class);
    }

    /// [KITE 11] Регистрирует класс в реестре типов для стабильной идентичности
    /// объектов (`type_id ↔ имя класса`), если он ещё не зарегистрирован.
    fn register_class_identity(&self, name: &str) {
        if let Ok(reg) = self.type_registry.read()
            && reg.get_type_id(name).is_none()
        {
            reg.register_type(shared::types::TypeDefBuilder::new(name).build());
        }
    }

    /// [KITE 11] TypeId зарегистрированного класса (или `TypeId(0)`, если нет).
    pub fn class_type_id(&self, name: &str) -> shared::types::TypeId {
        self.type_registry
            .read()
            .ok()
            .and_then(|r| r.get_type_id(name))
            .unwrap_or(shared::types::TypeId(0))
    }

    /// [KITE 11] Имя класса по TypeId объекта (None для пустого/непользовательского).
    pub fn class_name_by_type_id(&self, id: shared::types::TypeId) -> Option<String> {
        if id.0 == 0 {
            return None;
        }
        self.type_registry
            .read()
            .ok()
            .and_then(|r| r.get_type_name(id))
    }

    /// Получает класс по имени.
    pub fn get_class(&self, name: &str) -> RuntimeResult<&ClassDef> {
        self.classes
            .get(name)
            .ok_or_else(|| RuntimeError::undefined_type(name))
    }

    /// Проверяет, определён ли класс.
    pub fn has_class(&self, name: &str) -> bool {
        self.classes.contains_key(name)
    }

    /// Возвращает итератор по всем классам.
    pub fn all_classes(&self) -> impl Iterator<Item = (&String, &ClassDef)> {
        self.classes.iter()
    }

    // =========================================================================
    //                    УПРАВЛЕНИЕ ИНТЕРФЕЙСАМИ
    // =========================================================================

    /// Определяет интерфейс.
    pub fn define_interface(&mut self, iface: InterfaceDef) {
        self.interfaces.insert(iface.name.to_string(), iface);
    }

    /// Получает интерфейс по имени.
    pub fn get_interface(&self, name: &str) -> RuntimeResult<&InterfaceDef> {
        self.interfaces
            .get(name)
            .ok_or_else(|| RuntimeError::undefined_type(&format!("интерфейс {}", name)))
    }

    /// Проверяет, определён ли интерфейс.
    pub fn has_interface(&self, name: &str) -> bool {
        self.interfaces.contains_key(name)
    }

    // =========================================================================
    //                    УПРАВЛЕНИЕ ТИПАЖАМИ (TRAIT)
    // =========================================================================

    /// Определяет типаж (trait).
    pub fn define_trait(&mut self, trait_def: TraitDef) {
        self.traits.insert(trait_def.name.to_string(), trait_def);
    }

    /// Получает типаж по имени.
    pub fn get_trait(&self, name: &str) -> RuntimeResult<&TraitDef> {
        self.traits
            .get(name)
            .ok_or_else(|| RuntimeError::undefined_type(&format!("типаж {}", name)))
    }

    /// Проверяет, определён ли типаж.
    pub fn has_trait(&self, name: &str) -> bool {
        self.traits.contains_key(name)
    }

    // =========================================================================
    //                    УПРАВЛЕНИЕ РЕАЛИЗАЦИЯМИ (IMPL)
    // =========================================================================

    /// Регистрирует реализацию типажа для типа.
    pub fn define_impl(&mut self, impl_def: ImplDef) {
        let trait_name = impl_def
            .trait_name
            .as_ref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Self".to_string());
        let target_type = impl_def.target.to_string();

        self.impls
            .entry(target_type)
            .or_default()
            .insert(trait_name, impl_def);
    }

    /// Получает реализацию типажа для типа.
    pub fn get_impl(&self, target_type: &str, trait_name: Option<&str>) -> Option<&ImplDef> {
        let trait_key = trait_name.unwrap_or("Self");
        self.impls.get(target_type)?.get(trait_key)
    }

    /// Проверяет, реализует ли тип данный типаж.
    pub fn type_implements_trait(&self, target_type: &str, trait_name: &str) -> bool {
        self.impls
            .get(target_type)
            .map(|impls| impls.contains_key(trait_name))
            .unwrap_or(false)
    }

    /// Получает метод из реализации типажа.
    pub fn get_impl_method(
        &self,
        target_type: &str,
        trait_name: Option<&str>,
        method_name: &str,
    ) -> Option<&shared::types::Method> {
        let impl_def = self.get_impl(target_type, trait_name)?;
        impl_def
            .methods
            .iter()
            .find(|m| m.algorithm.name.as_ref() == method_name)
    }

    /// [KITE 11, шаг 4] Ищет метод в любом impl-блоке типа (по всем типажам).
    /// Возвращает копию метода, чтобы не удерживать заимствование среды.
    pub fn find_impl_method(
        &self,
        target_type: &str,
        method_name: &str,
    ) -> Option<shared::types::Method> {
        self.impls.get(target_type).and_then(|by_trait| {
            by_trait.values().find_map(|impl_def| {
                impl_def
                    .methods
                    .iter()
                    .find(|m| m.algorithm.name.as_ref() == method_name)
                    .cloned()
            })
        })
    }

    // =========================================================================
    //                    УПРАВЛЕНИЕ ПЕРЕЧИСЛЕНИЯМИ
    // =========================================================================

    /// Определяет перечисление.
    pub fn define_enum(&mut self, name: String, variants: Vec<String>) {
        self.enums.insert(name, variants);
    }

    /// Проверяет вариант перечисления.
    pub fn is_valid_enum_variant(&self, enum_name: &str, variant: &str) -> bool {
        self.enums
            .get(enum_name)
            .map(|v| v.contains(&variant.to_string()))
            .unwrap_or(false)
    }

    // =========================================================================
    //                    УПРАВЛЕНИЕ СТЕКОМ ВЫЗОВОВ
    // =========================================================================

    /// Создаёт новый кадр вызова.
    pub fn push_frame(&mut self, algorithm_name: impl Into<String>) -> RuntimeResult<()> {
        if self.call_stack.len() >= self.max_call_depth {
            return Err(RuntimeError::new(
                format!(
                    "Превышена максимальная глубина вызова ({}).\n\
                     Возможно, бесконечная рекурсия.",
                    self.max_call_depth
                ),
                super::error::RuntimeErrorKind::Other,
            ));
        }
        self.call_stack.push(CallFrame::new(algorithm_name));
        Ok(())
    }

    /// Создаёт кадр для метода.
    pub fn push_method_frame(
        &mut self,
        method_name: impl Into<String>,
        this: Value,
    ) -> RuntimeResult<()> {
        if self.call_stack.len() >= self.max_call_depth {
            return Err(RuntimeError::new(
                "Превышена максимальная глубина вызова",
                super::error::RuntimeErrorKind::Other,
            ));
        }
        self.call_stack
            .push(CallFrame::with_this(method_name, this));
        Ok(())
    }

    /// Удаляет верхний кадр вызова.
    pub fn pop_frame(&mut self) -> Option<CallFrame> {
        self.call_stack.pop()
    }

    /// [KITE 11] Запоминает класс, в котором определён исполняемый метод (для `предок`).
    pub fn set_current_defining_class(&mut self, class_name: impl Into<String>) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.defining_class = Some(class_name.into());
        }
    }

    /// [KITE 11] Класс текущего исполняемого метода (для разрешения `предок`).
    pub fn current_defining_class(&self) -> Option<String> {
        self.call_stack
            .last()
            .and_then(|f| f.defining_class.clone())
    }

    /// Получает текущий кадр вызова.
    pub fn current_frame(&self) -> Option<&CallFrame> {
        self.call_stack.last()
    }

    /// Получает изменяемый текущий кадр.
    pub fn current_frame_mut(&mut self) -> Option<&mut CallFrame> {
        self.call_stack.last_mut()
    }

    /// Получает текущий объект this.
    pub fn get_this(&self) -> Option<&Value> {
        self.call_stack.last().and_then(|f| f.this.as_ref())
    }

    /// Устанавливает возвращаемое значение (знач).
    pub fn set_result_value(&mut self, value: Value) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.result_value = Some(value);
        }
    }

    /// Получает возвращаемое значение.
    pub fn get_result_value(&self) -> Option<&Value> {
        self.call_stack.last().and_then(|f| f.result_value.as_ref())
    }

    /// Возвращает глубину стека вызовов.
    pub fn call_depth(&self) -> usize {
        self.call_stack.len()
    }

    // =========================================================================
    //                    ВВОД/ВЫВОД
    // =========================================================================

    /// Добавляет строку в буфер вывода.
    pub fn print(&mut self, s: &str) {
        if self.debug_mode {
            print!("{}", s);
        }
        self.output_buffer.push(s.to_string());
    }

    /// Добавляет строку с переводом строки.
    pub fn println(&mut self, s: &str) {
        if self.debug_mode {
            println!("{}", s);
        }
        self.output_buffer.push(format!("{}\n", s));
    }

    /// Получает буфер вывода.
    pub fn get_output(&self) -> String {
        self.output_buffer.join("")
    }

    /// Очищает буфер вывода.
    pub fn clear_output(&mut self) {
        self.output_buffer.clear();
    }

    // =========================================================================
    //                    НАСТРОЙКИ
    // =========================================================================

    /// Включает/выключает режим отладки.
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_mode = enabled;
    }

    /// Проверяет режим отладки.
    pub fn is_debug_mode(&self) -> bool {
        self.debug_mode
    }

    /// Устанавливает максимальную глубину вызова.
    pub fn set_max_call_depth(&mut self, depth: usize) {
        self.max_call_depth = depth;
    }

    /// Получает реестр типов.
    pub fn type_registry(&self) -> &Arc<RwLock<TypeRegistry>> {
        &self.type_registry
    }

    /// Строит движок системы типов (KITE 10), связанный с реестром этой среды.
    ///
    /// Единый источник правды для проверки совместимости, унификации и
    /// типизации операций — те же правила, что использует компилятор.
    pub fn type_system(&self) -> shared::typesys::TypeSystem {
        shared::typesys::TypeSystem::new().with_registry(Arc::clone(&self.type_registry))
    }

    // =========================================================================
    //                    НАТИВНЫЕ ФУНКЦИИ (БИБЛИОТЕКИ)
    // =========================================================================

    /// Регистрирует нативную функцию.
    pub fn register_native_function(&mut self, name: impl Into<String>, handler: NativeFn) {
        self.native_functions.insert(name.into(), handler);
    }

    /// Проверяет, есть ли нативная функция.
    pub fn has_native_function(&self, name: &str) -> bool {
        self.native_functions.contains_key(name)
    }

    /// Вызывает нативную функцию.
    pub fn call_native_function(&self, name: &str, args: &[Value]) -> RuntimeResult<Value> {
        let handler = self.native_functions.get(name).ok_or_else(|| {
            RuntimeError::new(
                format!("Нативная функция '{}' не найдена", name),
                super::error::RuntimeErrorKind::UndefinedAlgorithm,
            )
        })?;

        handler(args).map_err(|e| RuntimeError::new(e, super::error::RuntimeErrorKind::Other))
    }

    /// Получает нативную функцию (опционально).
    pub fn get_native_function(&self, name: &str) -> Option<&NativeFn> {
        self.native_functions.get(name)
    }

    // =========================================================================
    //                    МЕНЕДЖЕР БИБЛИОТЕК
    // =========================================================================

    /// Устанавливает менеджер библиотек.
    pub fn set_library_manager(&mut self, manager: Arc<RwLock<LibraryManager>>) {
        self.library_manager = Some(manager);
    }

    /// Получает менеджер библиотек.
    pub fn library_manager(&self) -> Option<&Arc<RwLock<LibraryManager>>> {
        self.library_manager.as_ref()
    }

    /// Устанавливает импортер файлов.
    pub fn set_file_importer(&mut self, importer: Arc<RwLock<FileImporter>>) {
        self.file_importer = Some(importer);
    }

    /// Получает импортер файлов.
    pub fn file_importer(&self) -> Option<&Arc<RwLock<FileImporter>>> {
        self.file_importer.as_ref()
    }

    // =========================================================================
    //                    ASYNC RUNTIME
    // =========================================================================

    /// Получает KumirRuntime для async операций.
    pub fn kumir_runtime(&self) -> &Arc<KumirRuntime> {
        &self.kumir_runtime
    }

    /// Получает TaskExecutor для управления задачами.
    pub fn task_executor(&self) -> std::sync::Arc<shared::runtime::TaskExecutor> {
        self.kumir_runtime.executor()
    }

    /// Проверяет, является ли имя загруженной библиотекой.
    pub fn is_loaded_library(&self, name: &str) -> bool {
        self.library_manager
            .as_ref()
            .and_then(|m| m.read().ok())
            .map(|m| m.is_loaded(name))
            .unwrap_or(false)
    }

    /// Вызывает функцию библиотеки по квалифицированному имени (Библиотека.функция).
    pub fn call_library_qualified(
        &self,
        lib_name: &str,
        func_name: &str,
        args: &[Value],
    ) -> RuntimeResult<Option<Value>> {
        let manager = self.library_manager.as_ref().ok_or_else(|| {
            RuntimeError::new(
                "Менеджер библиотек не инициализирован",
                super::error::RuntimeErrorKind::Other,
            )
        })?;

        let manager = manager.read().map_err(|_| {
            RuntimeError::new(
                "Не удалось получить доступ к менеджеру библиотек",
                super::error::RuntimeErrorKind::Other,
            )
        })?;

        manager.call_qualified_function(lib_name, func_name, args)
    }
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        Self {
            globals: self.globals.clone(),
            call_stack: self.call_stack.clone(),
            algorithms: self.algorithms.clone(),
            overloaded_algorithms: self.overloaded_algorithms.clone(),
            classes: self.classes.clone(),
            interfaces: self.interfaces.clone(),
            traits: self.traits.clone(),
            impls: self.impls.clone(),
            enums: self.enums.clone(),
            native_functions: self.native_functions.clone(),
            type_registry: Arc::clone(&self.type_registry),
            output_buffer: self.output_buffer.clone(),
            debug_mode: self.debug_mode,
            max_call_depth: self.max_call_depth,
            library_manager: self.library_manager.clone(),
            file_importer: self.file_importer.clone(),
            kumir_runtime: Arc::clone(&self.kumir_runtime),
        }
    }
}
