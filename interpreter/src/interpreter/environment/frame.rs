use shared::types::Value;

use super::scope::Scope;

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
    pub(crate) fn define(&mut self, name: String, value: Value) {
        self.scopes
            .last_mut()
            .expect("frame has at least one scope")
            .define(name, value);
    }

    /// Ищет значение от внутренней области к внешней.
    pub(crate) fn get(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    /// Есть ли имя в любой области кадра.
    pub(crate) fn contains(&self, name: &str) -> bool {
        self.scopes.iter().any(|s| s.contains(name))
    }

    /// Является ли имя константой в кадре.
    pub(crate) fn is_const(&self, name: &str) -> bool {
        self.scopes.iter().any(|s| s.is_const(name))
    }

    /// Присваивает значение существующей переменной (в области, где она объявлена).
    pub(crate) fn assign(&mut self, name: &str, value: Value) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(var) = scope.get_mut(name) {
                *var = value;
                return true;
            }
        }
        false
    }
}
