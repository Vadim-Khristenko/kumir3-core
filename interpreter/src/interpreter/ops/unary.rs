//! Унарные операции над значениями (хвост `eval_unary_op`).

use shared::types::{Number, Token, Value};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl TypeOps {
    /// Применяет унарный оператор к уже вычисленному операнду.
    pub fn unary(op: &Token, value: Value) -> RuntimeResult<Value> {
        match op {
            Token::Minus => match value {
                Value::Number(n) => {
                    let negated = TypeOps::negate_number(n)?;
                    Ok(Value::Number(negated))
                }
                _ => Err(RuntimeError::type_mismatch("число", "не число")),
            },
            Token::Not => Ok(Value::Boolean(!TypeOps::is_truthy(&value))),
            _ => Err(RuntimeError::new(
                format!("Неизвестный унарный оператор: {:?}", op),
                RuntimeErrorKind::Other,
            )),
        }
    }

    fn negate_number(n: Number) -> RuntimeResult<Number> {
        Ok(match n {
            Number::I8(v) => Number::I8(-v),
            Number::I16(v) => Number::I16(-v),
            Number::I32(v) => Number::I32(-v),
            Number::I64(v) => Number::I64(-v),
            Number::I128(v) => Number::I128(-v),
            Number::F32(v) => Number::F32(-v),
            Number::F64(v) => Number::F64(-v),
            Number::F128(v) => Number::F128(-v),
            // Беззнаковые нельзя отрицать
            _ => {
                return Err(RuntimeError::new(
                    "Нельзя применить унарный минус к беззнаковому числу",
                    RuntimeErrorKind::TypeMismatch,
                ));
            }
        })
    }
}
