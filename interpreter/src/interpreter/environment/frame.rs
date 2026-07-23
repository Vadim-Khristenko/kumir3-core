//! Call frame for algorithm invocation.

use shared::types::Value;

use super::scope::Scope;

// =============================================================================
//                             CALL FRAME
// =============================================================================

/// An algorithm invocation frame.
///
/// [KITE 4] Lexical scoping: a frame contains a **stack of block scopes**
/// (`scopes`), not a single scope. Name lookup proceeds from inner to outer
/// within this frame, then to globals; caller frames are **not** visible
/// (no dynamic scoping).
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// Algorithm name
    pub algorithm_name: String,
    /// Lexical scope stack (scopes[0] = params / top level)
    scopes: Vec<Scope>,
    /// Return value (знач)
    pub result_value: Option<Value>,
    /// Current object (for methods)
    pub this: Option<Value>,
    /// [KITE 11] Class where the executing method is defined (for `предок`).
    pub defining_class: Option<String>,
}

impl CallFrame {
    /// Creates a new call frame with one initial (top-level) scope.
    pub fn new(algorithm_name: impl Into<String>) -> Self {
        Self {
            algorithm_name: algorithm_name.into(),
            scopes: vec![Scope::new()],
            result_value: None,
            this: None,
            defining_class: None,
        }
    }

    /// Creates a call frame for a method with a `this` object.
    pub fn with_this(algorithm_name: impl Into<String>, this: Value) -> Self {
        Self {
            algorithm_name: algorithm_name.into(),
            scopes: vec![Scope::new()],
            result_value: None,
            this: Some(this),
            defining_class: None,
        }
    }

    /// Opens a nested block scope.
    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    /// Closes the current block scope (the top level is never popped).
    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    /// Defines a variable in the innermost (current) scope.
    pub(crate) fn define(&mut self, name: String, value: Value) {
        self.scopes
            .last_mut()
            .expect("frame has at least one scope")
            .define(name, value);
    }

    /// Looks up a value from inner to outer scopes.
    pub(crate) fn get(&self, name: &str) -> Option<&Value> {
        self.scopes.iter().rev().find_map(|s| s.get(name))
    }

    /// Checks if a name exists in any scope of this frame.
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
