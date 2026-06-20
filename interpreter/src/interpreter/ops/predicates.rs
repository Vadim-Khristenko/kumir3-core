//! Предикаты и сравнения над значениями (истинность, равенство, упорядочение, индексация).

use shared::types::Value;

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeResult};

impl TypeOps {
    /// Проверяет "истинность" значения.
    pub fn is_truthy(value: &Value) -> bool {
        match value {
            Value::Boolean(b) => *b,
            Value::Number(n) => n.to_f64().map(|f| f != 0.0).unwrap_or(false),
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Null | Value::Undefined => false,
            Value::Option(opt) => opt.is_some(),
            _ => true,
        }
    }

    /// Сравнивает два значения на равенство.
    pub fn values_equal(a: &Value, b: &Value) -> bool {
        a == b
    }

    /// Сравнивает два значения.
    pub fn compare<F>(a: &Value, b: &Value, cmp: F) -> RuntimeResult<Value>
    where
        F: Fn(std::cmp::Ordering) -> bool,
    {
        // [typesys-seam] будущее: lossless mixed-numeric сравнение через shared::typesys.
        let result = match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                let fa = na
                    .to_f64()
                    .ok_or_else(|| RuntimeError::type_mismatch("число", "не число"))?;
                let fb = nb
                    .to_f64()
                    .ok_or_else(|| RuntimeError::type_mismatch("число", "не число"))?;
                cmp(fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal))
            }
            (Value::String(sa), Value::String(sb)) => cmp(sa.cmp(sb)),
            (Value::Char(ca), Value::Char(cb)) => cmp(ca.cmp(cb)),
            _ => {
                return Err(RuntimeError::type_mismatch(
                    "сравнимые типы",
                    "несравнимые типы",
                ));
            }
        };
        Ok(Value::Boolean(result))
    }

    /// Преобразует значение в индекс.
    pub fn to_index(value: &Value) -> RuntimeResult<i64> {
        value
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))
    }
}
