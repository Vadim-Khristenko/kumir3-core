//! Приведение и проверка типов значений (хвосты `eval_cast` / `eval_type_check`).

use shared::types::{Number, TypeKind, Value};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeResult};

impl TypeOps {
    /// Приводит значение к целевому типу.
    pub fn cast(value: Value, target: &TypeKind) -> RuntimeResult<Value> {
        // [typesys-seam] будущее: приведение/coercion через shared::typesys.
        match target {
            // `любой` (top type): приведение к нему — тождество; принимает любое
            // значение и никогда не ошибается. Всё является подтипом `любой`.
            TypeKind::Any => Ok(value),
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
            // [W0] `как сим`: символ ← целое (код) | строка длины 1 | символ.
            // Всё остальное — ясная ошибка (раньше был not_implemented).
            TypeKind::Char => Self::cast_to_char(value),
            _ => Err(RuntimeError::not_implemented(&format!(
                "приведение к типу {:?}",
                target
            ))),
        }
    }

    /// [W0] Реализация `значение как сим`.
    ///
    /// * `Value::Char(c)` → сам символ (тождество);
    /// * целое число → символ с этим кодом Unicode (скалярным значением);
    /// * строка ровно из одного символа → этот символ;
    /// * всё остальное (в т.ч. строка другой длины, вещественное, логическое,
    ///   массив, объект) → ясная ошибка выполнения.
    fn cast_to_char(value: Value) -> RuntimeResult<Value> {
        match &value {
            Value::Char(c) => Ok(Value::Char(*c)),
            Value::Number(n) if n.is_integer() => {
                let code = n
                    .to_i64()
                    .ok_or_else(|| Self::char_cast_error("код символа вне диапазона"))?;
                let ch = u32::try_from(code)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| {
                        Self::char_cast_error(&format!(
                            "{} не является кодом символа Unicode",
                            code
                        ))
                    })?;
                Ok(Value::Char(ch))
            }
            Value::String(s) => {
                let mut it = s.chars();
                match (it.next(), it.next()) {
                    (Some(c), None) => Ok(Value::Char(c)),
                    _ => Err(Self::char_cast_error(&format!(
                        "строка \"{}\" должна состоять ровно из одного символа",
                        s
                    ))),
                }
            }
            other => Err(Self::char_cast_error(&format!(
                "значение типа {} нельзя привести к символу",
                other.type_name_ru()
            ))),
        }
    }

    /// Единая формулировка ошибки приведения к `сим`.
    fn char_cast_error(details: &str) -> RuntimeError {
        RuntimeError::type_mismatch("сим (символ: целое-код или строка длины 1)", details)
    }

    /// Проверяет, соответствует ли значение указанному типу.
    pub fn type_check(value: &Value, check: &TypeKind) -> bool {
        // [typesys-seam] будущее: conformance через shared::typesys.
        // `любой` (top type): любое значение является его подтипом, поэтому
        // `значение это любой` всегда истинно.
        if matches!(check, TypeKind::Any) {
            return true;
        }
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
