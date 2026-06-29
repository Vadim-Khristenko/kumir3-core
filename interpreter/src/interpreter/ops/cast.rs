//! Приведение и проверка типов значений (хвосты `eval_cast` / `eval_type_check`).

use shared::types::{Number, TypeKind, Value};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeResult};

impl TypeOps {
    /// Приводит значение к целевому типу.
    pub fn cast(value: Value, target: &TypeKind) -> RuntimeResult<Value> {
        // [typesys-seam] будущее: приведение/coercion через shared::typesys.
        match target {
            TypeKind::Int64 => {
                let n = value
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_mismatch("цел", "не число"))?;
                Ok(Value::Number(Number::I64(n)))
            }
            TypeKind::Float64 => match &value {
                Value::Number(n) => {
                    let f = n
                        .to_f64()
                        .ok_or_else(|| RuntimeError::type_mismatch("вещ", "не число"))?;
                    Ok(Value::Number(Number::F64(f)))
                }
                Value::String(s) => {
                    let f: f64 = s
                        .parse()
                        .map_err(|_| RuntimeError::type_mismatch("вещ", "не число"))?;
                    Ok(Value::Number(Number::F64(f)))
                }
                _ => Err(RuntimeError::type_mismatch("вещ", "не число")),
            },
            TypeKind::String => Ok(Value::String(value.to_string())),
            TypeKind::Bool => Ok(Value::Boolean(TypeOps::is_truthy(&value))),
            _ => Err(RuntimeError::not_implemented(&format!(
                "приведение к типу {:?}",
                target
            ))),
        }
    }

    /// Проверяет, соответствует ли значение указанному типу.
    pub fn type_check(value: &Value, check: &TypeKind) -> bool {
        // [typesys-seam] будущее: conformance через shared::typesys.
        matches!(
            (check, value),
            (TypeKind::Int64, Value::Number(Number::I64(_)))
                | (TypeKind::Float64, Value::Number(Number::F64(_)))
                | (TypeKind::String, Value::String(_))
                | (TypeKind::Bool, Value::Boolean(_))
                | (TypeKind::Char, Value::Char(_))
                | (TypeKind::Array(_), Value::Array(_))
        )
    }
}
