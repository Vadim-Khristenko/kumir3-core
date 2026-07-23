//! Rust block execution.

use super::super::environment::Environment;
use super::super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::Executor;
use shared::codegen::rust_block::{RustBlockConfig, RustBlockExecutor, RustExecutionMode};
use std::collections::HashMap;

impl Executor {
    // =============================================================================
    //                         RUST BLOCKS
    // =============================================================================

    /// Executes a Rust block with captured variables.
    pub(crate) fn execute_rust_block(
        code: &str,
        captured_vars: &[String],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        // Collect captured variables from the environment.
        let mut vars = HashMap::new();
        for var_name in captured_vars {
            if let Ok(value) = env.get_variable(var_name) {
                vars.insert(var_name.clone(), value.clone());
            }
        }

        // Create a Rust block executor (interpret if rustc unavailable).
        let config = RustBlockConfig {
            execution_mode: RustExecutionMode::Interpret,
            ..Default::default()
        };
        let mut executor = RustBlockExecutor::with_config(config);

        // Execute the code.
        let result = executor.execute(code, &vars)?;

        // Print stdout if present.
        if !result.stdout.is_empty() {
            env.print(&result.stdout);
            if env.is_debug_mode() {
                print!("{}", result.stdout);
            }
        }

        // Print stderr if present.
        if !result.stderr.is_empty() {
            env.print(&format!("[stderr] {}", result.stderr));
            if env.is_debug_mode() {
                eprint!("{}", result.stderr);
            }
        }

        // Check exit code.
        if let Some(code) = result.exit_code
            && code != 0
        {
            return Err(RuntimeError::new(
                format!("Rust-блок завершился с кодом {}", code),
                RuntimeErrorKind::Other,
            ));
        }

        Ok(ControlFlow::None)
    }
}
