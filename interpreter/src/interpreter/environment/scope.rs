//! Variable scope (local scope inside a block or frame).

use std::collections::HashMap;

use shared::types::Value;

// =============================================================================
//                              SCOPE
// =============================================================================

/// A scope (lexical binding region) for variables and constants.
#[derive(Debug, Clone)]
pub struct Scope {
    /// Variables in this scope
    variables: HashMap<String, Value>,
    /// Constants (immutable)
    constants: HashMap<String, Value>,
}

impl Scope {
    /// Creates a new, empty scope.
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            constants: HashMap::new(),
        }
    }

    /// Defines a variable in this scope.
    pub fn define(&mut self, name: String, value: Value) {
        self.variables.insert(name, value);
    }

    /// Defines a constant in this scope.
    pub fn define_const(&mut self, name: String, value: Value) {
        self.constants.insert(name, value);
    }

    /// Looks up a variable or constant by name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.variables
            .get(name)
            .or_else(|| self.constants.get(name))
    }

    /// Looks up a mutable reference to a variable.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.variables.get_mut(name)
    }

    /// Checks if a name is defined (variable or constant).
    pub fn contains(&self, name: &str) -> bool {
        self.variables.contains_key(name) || self.constants.contains_key(name)
    }

    /// Iterates over all bindings: (name, value, is_const).
    ///
    /// Needed for interactive mode to display program state without guessing variable names.
    pub fn entries(&self) -> impl Iterator<Item = (&String, &Value, bool)> {
        self.constants
            .iter()
            .map(|(n, v)| (n, v, true))
            .chain(self.variables.iter().map(|(n, v)| (n, v, false)))
    }

    /// Checks if a name is a constant.
    pub fn is_const(&self, name: &str) -> bool {
        self.constants.contains_key(name)
    }
}

impl Default for Scope {
    fn default() -> Self {
        Self::new()
    }
}
