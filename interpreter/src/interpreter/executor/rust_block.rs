//! Rust-вставки.

use super::super::environment::Environment;
use super::super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::Executor;
use shared::codegen::rust_block::{RustBlockConfig, RustBlockExecutor, RustExecutionMode};
use std::collections::HashMap;

impl Executor {
    // =========================================================================
    //                    RUST-ВСТАВКИ
    // =========================================================================

    /// Выполняет Rust-блок с захваченными переменными
    pub(crate) fn execute_rust_block(
        code: &str,
        captured_vars: &[String],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        // Собираем захваченные переменные из окружения
        let mut vars = HashMap::new();
        for var_name in captured_vars {
            if let Ok(value) = env.get_variable(var_name) {
                vars.insert(var_name.clone(), value.clone());
            }
        }

        // Создаём исполнитель Rust-блоков
        // По умолчанию используем интерпретацию, если rustc недоступен
        let config = RustBlockConfig {
            execution_mode: RustExecutionMode::Interpret,
            ..Default::default()
        };
        let mut executor = RustBlockExecutor::with_config(config);

        // Выполняем код
        let result = executor.execute(code, &vars)?;

        // Выводим stdout если есть
        if !result.stdout.is_empty() {
            env.print(&result.stdout);
            if env.is_debug_mode() {
                print!("{}", result.stdout);
            }
        }

        // Выводим stderr если есть
        if !result.stderr.is_empty() {
            env.print(&format!("[stderr] {}", result.stderr));
            if env.is_debug_mode() {
                eprint!("{}", result.stderr);
            }
        }

        // Проверяем код возврата
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
