use std::collections::HashMap;

use shared::types::Value;

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
