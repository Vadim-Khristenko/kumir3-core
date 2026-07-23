//! Access to execution state: environment, variables, output, warnings, and runtime.

use super::{Environment, Interpreter, KumirRuntime, LibraryManager, RuntimeResult};
use shared::types::Value;

impl Interpreter {
    /// [W0] Returns warnings collected during execution (e.g., undeclared variables).
    /// These do NOT appear in program output (`вывод`/stdout).
    pub fn warnings(&self) -> &[String] {
        self.env.warnings()
    }

    /// Returns a reference to the execution environment.
    pub fn environment(&self) -> &Environment {
        &self.env
    }

    /// Returns a mutable reference to the execution environment.
    pub fn environment_mut(&mut self) -> &mut Environment {
        &mut self.env
    }

    // =============================================================================
    //                           VARIABLES
    // =============================================================================

    /// Sets a global variable.
    pub fn set_global(&mut self, name: impl Into<String>, value: Value) {
        self.env.define_global(name.into(), value);
    }

    /// Gets the value of a variable.
    pub fn get_variable(&self, name: &str) -> RuntimeResult<&Value> {
        self.env.get_variable(name)
    }

    // =============================================================================
    //                            OUTPUT
    // =============================================================================

    /// Gets the program output.
    pub fn get_output(&self) -> String {
        self.env.get_output()
    }

    /// Clears the output buffer.
    pub fn clear_output(&mut self) {
        self.env.clear_output();
    }

    // =============================================================================
    //                      RUNTIME AND LIBRARIES
    // =============================================================================

    /// Gets the library manager.
    pub fn libraries(&self) -> &std::sync::Arc<std::sync::RwLock<LibraryManager>> {
        &self.libraries
    }

    /// Gets the runtime (if initialized).
    pub fn runtime(&self) -> Option<&KumirRuntime> {
        self.runtime.as_ref()
    }
}
