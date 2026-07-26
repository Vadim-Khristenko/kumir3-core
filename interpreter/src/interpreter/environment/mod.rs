//! Runtime environment for the Kumir 3 interpreter.
//!
//! The environment stores variables, algorithms, classes, and manages scoping.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::file_importer::FileImporter;
use super::library_bridge::LibraryManager;
use shared::runtime::KumirRuntime;
use shared::types::library::NativeFn;
use shared::types::{
    Algorithm, ClassDef, ImplDef, InterfaceDef, OverloadedAlgorithm, Program, TraitDef,
    TypeRegistry, Value,
};

pub mod frame;
pub mod scope;

use frame::CallFrame;
use scope::Scope;

// =============================================================================
//                            ENVIRONMENT
// =============================================================================

/// Runtime environment for program execution.
pub struct Environment {
    /// Global variables
    globals: Scope,

    /// Call stack (algorithm invocation frames)
    call_stack: Vec<CallFrame>,

    /// Defined algorithms
    algorithms: HashMap<String, Algorithm>,

    /// Overloaded algorithms
    overloaded_algorithms: HashMap<String, OverloadedAlgorithm>,

    /// Defined classes
    classes: HashMap<String, ClassDef>,

    /// Defined interfaces
    interfaces: HashMap<String, InterfaceDef>,

    /// Defined traits
    traits: HashMap<String, TraitDef>,

    /// Trait implementations (target_type → trait_name → ImplDef)
    impls: HashMap<String, HashMap<String, ImplDef>>,

    /// Defined enumerations (enum_name → variants)
    enums: HashMap<String, Vec<String>>,

    /// Native library functions (name → handler)
    native_functions: HashMap<String, NativeFn>,

    /// Type registry
    type_registry: Arc<RwLock<TypeRegistry>>,

    /// Output buffer (for testing)
    output_buffer: Vec<String>,

    /// Debug mode
    debug_mode: bool,

    /// [W0] Strict mode: assignment to undeclared variables is an error (disabled by default).
    strict: bool,

    /// [W0] Collected warnings (e.g., undeclared variables).
    /// Not included in program output buffer.
    warnings: Vec<String>,

    /// Call depth limit (stack overflow protection)
    max_call_depth: usize,

    /// Library manager (shared reference)
    library_manager: Option<Arc<RwLock<LibraryManager>>>,

    /// .kum file importer
    file_importer: Option<Arc<RwLock<FileImporter>>>,

    /// Async runtime
    kumir_runtime: Arc<KumirRuntime>,
}

impl Environment {
    /// Creates a new runtime environment.
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
            strict: false,
            warnings: Vec::new(),
            max_call_depth: 1000,
            library_manager: None,
            file_importer: None,
            kumir_runtime: Arc::new(KumirRuntime::new()),
        }
    }

    /// Initializes an environment from a program (loads algorithms, classes, etc).
    pub fn from_program(program: &Program) -> RuntimeResult<Self> {
        let mut env = Self::new();

        // Load algorithms
        for alg in &program.algorithms {
            env.define_algorithm(alg.clone());
        }

        // Load overloaded algorithms
        for overloaded in &program.overloaded_algorithms {
            env.overloaded_algorithms
                .insert(overloaded.name.to_string(), overloaded.clone());
        }

        // Load classes
        for class in &program.classes {
            env.define_class(class.clone());
        }

        // Load main algorithm
        if let Some(main) = &program.main {
            env.define_algorithm(main.clone());
        }

        Ok(env)
    }

    // =============================================================================
    //                        VARIABLE MANAGEMENT
    // =============================================================================

    /// Defines a global variable.
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

    /// Определяет локальную константу — значение, которое нельзя изменить.
    ///
    /// Отличается от [`Self::define_local`] только тем, в какую половину области
    /// видимости попадает имя: [`Scope`] держит константы отдельно, и
    /// [`Self::set_variable`] отказывает при попытке присвоить им.
    pub fn define_local_const(&mut self, name: String, value: Value) {
        if let Some(frame) = self.call_stack.last_mut() {
            frame.define_const(name, value);
        } else {
            self.globals.define_const(name, value);
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

        // [W0] Переменная не найдена нигде в цепочке областей видимости —
        // присваивание пытается ввести НОВУЮ, ранее необъявленную переменную.
        // Это распространённый учебный «подводный камень» (например, опечатка
        // `хyz := 5`). Легитимные неявные имена (параметры арг/рез/аргрез,
        // `знач`, переменные циклов `для`, привязки совпадений/`for-each`) сюда
        // НЕ попадают: они создаются через `define_local`/`set_result_value`
        // ещё до присваивания, поэтому уже присутствуют в области видимости.
        if self.strict {
            return Err(RuntimeError::new(
                format!("переменная '{}' используется без объявления", name),
                super::error::RuntimeErrorKind::UndefinedVariable,
            ));
        }
        self.warnings
            .push(format!("переменная '{}' используется без объявления", name));
        // Мягкий режим: поведение как прежде — создаём локальную переменную.
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
        self.warn_if_destructor(&name, &class);
        self.register_class_identity(&name);
        self.classes.insert(name, class);
    }

    /// Определяет класс с заданным именем (для импортов с префиксом модуля).
    pub fn define_class_with_name(&mut self, name: &str, class: ClassDef) {
        self.warn_if_destructor(name, &class);
        self.register_class_identity(name);
        self.classes.insert(name.to_string(), class);
    }

    /// [W0][KITE-0011] Деструкторы разбираются, но НЕ исполняются.
    ///
    /// В текущей объектной модели значение-объект (`Value::Object`) —
    /// обычное значение с семантикой копирования: нет ни идентичности
    /// экземпляра, ни подсчёта ссылок, ни момента «последнего владельца».
    /// Детерминированной точки разрушения, в которой деструктор можно было бы
    /// вызвать ровно один раз, попросту не существует — вызов «на выходе из
    /// кадра» исполнял бы `деструктор` для каждой копии и для возвращаемых
    /// объектов. Изобретать такую семантику здесь нельзя, но и молчать тоже:
    /// объявление деструктора порождает явное предупреждение.
    fn warn_if_destructor(&mut self, name: &str, class: &ClassDef) {
        if class.destructor.is_some() {
            self.warnings.push(format!(
                "деструктор класса '{}' объявлен, но не будет исполнен: \
                 детерминированная точка разрушения объекта не определена \
                 (KITE-0011); освобождайте ресурсы явным методом",
                name
            ));
        }
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

    /// [W0] Включает/выключает строгий режим (необъявленная переменная — ошибка).
    pub fn set_strict(&mut self, enabled: bool) {
        self.strict = enabled;
    }

    /// [W0] Проверяет, включён ли строгий режим.
    pub fn is_strict(&self) -> bool {
        self.strict
    }

    /// [W0] Возвращает собранные предупреждения (не попадают в вывод программы).
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// [W0] Очищает собранные предупреждения.
    pub fn clear_warnings(&mut self) {
        self.warnings.clear();
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

    /// Все глобальные имена со значениями, отсортированные по имени.
    ///
    /// Служит интерактивному режиму: он показывает состояние программы, а
    /// перебрать его иначе было нечем. Порядок задан, чтобы список не
    /// перетасовывался при каждой перерисовке.
    pub fn globals_snapshot(&self) -> Vec<(String, Value, bool)> {
        let mut items: Vec<(String, Value, bool)> = self
            .globals
            .entries()
            .map(|(name, value, is_const)| (name.clone(), value.clone(), is_const))
            .collect();
        items.sort_by(|a, b| a.0.cmp(&b.0));
        items
    }

    /// Имена определённых алгоритмов, отсортированные.
    pub fn algorithm_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.algorithms.keys().cloned().collect();
        names.sort();
        names
    }

    /// Известно ли имя как функция подключённой библиотеки.
    ///
    /// Без менеджера (или при отравленной блокировке) отвечает «нет»: вызов
    /// тогда разрешается обычным путём и даёт понятную ошибку об алгоритме.
    pub fn is_library_function(&self, name: &str) -> bool {
        self.library_manager
            .as_ref()
            .and_then(|m| m.read().ok())
            .is_some_and(|m| m.is_library_function(name))
    }

    /// Вызывает функцию подключённой библиотеки.
    ///
    /// `Ok(None)` означает «такой функции нет» — вызывающий продолжает поиск.
    pub fn call_library_function(
        &self,
        name: &str,
        args: &[Value],
    ) -> RuntimeResult<Option<Value>> {
        let Some(manager) = self.library_manager.as_ref() else {
            return Ok(None);
        };
        let manager = manager.read().map_err(|_| {
            RuntimeError::new(
                "Не удалось получить доступ к библиотекам",
                RuntimeErrorKind::Other,
            )
        })?;
        manager.call_function(name, args)
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
            strict: self.strict,
            warnings: self.warnings.clone(),
            max_call_depth: self.max_call_depth,
            library_manager: self.library_manager.clone(),
            file_importer: self.file_importer.clone(),
            kumir_runtime: Arc::clone(&self.kumir_runtime),
        }
    }
}
