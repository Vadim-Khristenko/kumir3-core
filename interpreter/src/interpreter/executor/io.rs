//! Input/output statement execution.

use super::super::environment::Environment;
use super::super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::evaluator::ExprEvaluator;
use super::Executor;
use shared::types::{Expr, Number, Value};
use std::io::{self, BufRead, Write};

impl Executor {
    // =============================================================================
    //                           INPUT/OUTPUT
    // =============================================================================

    pub(crate) fn execute_input(
        vars: &[String],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        for var in vars {
            let mut input = String::new();
            handle
                .read_line(&mut input)
                .map_err(|e| RuntimeError::io_error(format!("Ошибка ввода: {}", e)))?;
            let input = input.trim();

            // Auto-detect type from input.
            let value = if let Ok(i) = input.parse::<i64>() {
                Value::Number(Number::I64(i))
            } else if let Ok(f) = input.parse::<f64>() {
                Value::Number(Number::F64(f))
            } else if input == "да" || input == "true" {
                Value::Boolean(true)
            } else if input == "нет" || input == "false" {
                Value::Boolean(false)
            } else {
                Value::String(input.to_string())
            };

            env.set_variable(var, value)?;
        }

        Ok(ControlFlow::None)
    }

    /// Output statement: values are printed in sequence with no separator.
    ///
    /// The program must insert spaces in its own strings:
    /// `вывод "Answer: ", n` produces `Answer: 5`. When auto-separator was enabled,
    /// this common pattern caused double spaces, and there was no way to suppress it.
    pub(crate) fn execute_output(
        exprs: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let mut output = String::new();

        for expr in exprs {
            let value = ExprEvaluator::evaluate(expr, env)?;
            output.push_str(&Self::format_value(&value));
        }

        env.println(&output);

        // Also print to stdout in debug mode.
        if env.is_debug_mode() {
            println!("{}", output);
        }

        Ok(ControlFlow::None)
    }

    pub(crate) fn execute_pause(_env: &mut Environment) -> RuntimeResult<ControlFlow> {
        let mut stdout = io::stdout();
        write!(stdout, "[Пауза] Нажмите Enter для продолжения...")
            .map_err(|e| RuntimeError::io_error(format!("Ошибка вывода: {}", e)))?;
        stdout
            .flush()
            .map_err(|e| RuntimeError::io_error(format!("Ошибка вывода: {}", e)))?;

        let mut input = String::new();
        io::stdin()
            .lock()
            .read_line(&mut input)
            .map_err(|e| RuntimeError::io_error(format!("Ошибка ввода: {}", e)))?;

        Ok(ControlFlow::None)
    }

    fn format_value(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Char(c) => c.to_string(),
            Value::Boolean(b) => {
                if *b {
                    "да".to_string()
                } else {
                    "нет".to_string()
                }
            }
            Value::Null => "пусто".to_string(),
            Value::Undefined => "неопределено".to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(Self::format_value).collect();
                format!("[{}]", items.join(", "))
            }
            _ => value.to_string(),
        }
    }
}
