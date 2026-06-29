//! Ввод/вывод.

use super::super::environment::Environment;
use super::super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::evaluator::ExprEvaluator;
use super::Executor;
use shared::types::{Expr, Number, Value};
use std::io::{self, BufRead, Write};

impl Executor {
    // =========================================================================
    //                    ВВОД/ВЫВОД
    // =========================================================================

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

            // Пытаемся определить тип автоматически
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

    pub(crate) fn execute_output(
        exprs: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let mut output_parts = Vec::new();

        for expr in exprs {
            let value = ExprEvaluator::evaluate(expr, env)?;
            output_parts.push(Self::format_value(&value));
        }

        let output = output_parts.join(" ");
        env.println(&output);

        // Также выводим в stdout если не в режиме тестирования
        if env.is_debug_mode() {
            println!("{}", output);
        }

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
