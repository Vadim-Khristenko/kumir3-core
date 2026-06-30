use super::ExprEvaluator;

use shared::types::{Expr, Number, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    pub(crate) fn call_string_method(
        s: &str,
        method: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        match method {
            "длина" | "length" | "len" => {
                Ok(Value::Number(Number::I64(s.chars().count() as i64)))
            }
            "верхний_регистр" | "to_upper" | "upper" => {
                Ok(Value::String(s.to_uppercase()))
            }
            "нижний_регистр" | "to_lower" | "lower" => {
                Ok(Value::String(s.to_lowercase()))
            }
            "содержит" | "contains" => {
                if args.len() != 1 {
                    return Err(RuntimeError::argument_count(method, 1, args.len()));
                }
                let substr = Self::evaluate(&args[0], env)?;
                if let Value::String(sub) = substr {
                    Ok(Value::Boolean(s.contains(&sub)))
                } else {
                    Err(RuntimeError::type_mismatch("строка", "не строка"))
                }
            }
            "разделить" | "split" => {
                if args.len() != 1 {
                    return Err(RuntimeError::argument_count(method, 1, args.len()));
                }
                let delim = Self::evaluate(&args[0], env)?;
                if let Value::String(d) = delim {
                    let parts: Vec<Value> =
                        s.split(&d).map(|p| Value::String(p.to_string())).collect();
                    Ok(Value::Array(parts))
                } else {
                    Err(RuntimeError::type_mismatch("строка", "не строка"))
                }
            }
            "обрезать" | "trim" => Ok(Value::String(s.trim().to_string())),
            "заменить" | "replace" => {
                if args.len() != 2 {
                    return Err(RuntimeError::argument_count(method, 2, args.len()));
                }
                let from = Self::evaluate(&args[0], env)?;
                let to = Self::evaluate(&args[1], env)?;
                match (from, to) {
                    (Value::String(f), Value::String(t)) => Ok(Value::String(s.replace(&f, &t))),
                    _ => Err(RuntimeError::type_mismatch("строка, строка", "другое")),
                }
            }
            _ => Err(RuntimeError::new(
                format!("Метод '{}' не найден для строки", method),
                RuntimeErrorKind::Other,
            )),
        }
    }
}
