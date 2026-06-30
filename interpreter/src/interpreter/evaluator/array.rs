use super::ExprEvaluator;

use shared::types::{Expr, Number, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::ops::TypeOps;

impl ExprEvaluator {
    // =========================================================================
    //                    ДОСТУП К МАССИВАМ
    // =========================================================================

    pub(crate) fn eval_array_access(
        name: &str,
        indices: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let array = env.get_variable(name)?.clone();

        match array {
            Value::Array(elements) => {
                if indices.len() != 1 {
                    return Err(RuntimeError::not_implemented("многомерные массивы"));
                }

                let index = Self::evaluate(&indices[0], env)?;
                let idx = TypeOps::to_index(&index)?;

                if idx < 0 || idx as usize >= elements.len() {
                    return Err(RuntimeError::index_out_of_bounds(idx, elements.len()));
                }

                Ok(elements[idx as usize].clone())
            }
            Value::String(s) => {
                if indices.len() != 1 {
                    return Err(RuntimeError::new(
                        "Строка поддерживает только один индекс",
                        RuntimeErrorKind::Other,
                    ));
                }

                let index = Self::evaluate(&indices[0], env)?;
                let idx = TypeOps::to_index(&index)?;

                let chars: Vec<char> = s.chars().collect();
                if idx < 1 || idx as usize > chars.len() {
                    return Err(RuntimeError::index_out_of_bounds(idx, chars.len()));
                }

                Ok(Value::Char(chars[(idx - 1) as usize]))
            }
            Value::Map(map) => {
                if indices.len() != 1 {
                    return Err(RuntimeError::new(
                        "Словарь поддерживает только один ключ",
                        RuntimeErrorKind::Other,
                    ));
                }

                let key = Self::evaluate(&indices[0], env)?;
                map.get(&key).cloned().ok_or_else(|| {
                    RuntimeError::new(
                        format!("Ключ не найден в словаре: {}", key),
                        RuntimeErrorKind::Other,
                    )
                })
            }
            _ => Err(RuntimeError::type_mismatch(
                "массив, строка или словарь",
                "другой тип",
            )),
        }
    }

    pub(crate) fn call_array_method(
        arr: &[Value],
        method: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        match method {
            "длина" | "length" | "len" | "размер" | "size" => {
                Ok(Value::Number(Number::I64(arr.len() as i64)))
            }
            "пусто" | "is_empty" | "empty" => Ok(Value::Boolean(arr.is_empty())),
            "первый" | "first" => arr.first().cloned().ok_or_else(|| {
                RuntimeError::new("Массив пуст", RuntimeErrorKind::IndexOutOfBounds)
            }),
            "последний" | "last" => arr.last().cloned().ok_or_else(|| {
                RuntimeError::new("Массив пуст", RuntimeErrorKind::IndexOutOfBounds)
            }),
            "содержит" | "contains" => {
                if args.len() != 1 {
                    return Err(RuntimeError::argument_count(method, 1, args.len()));
                }
                let value = Self::evaluate(&args[0], env)?;
                Ok(Value::Boolean(arr.contains(&value)))
            }
            "сумма" | "sum" => {
                let mut sum: f64 = 0.0;
                for val in arr {
                    if let Some(n) = val.as_number().and_then(|n| n.to_f64()) {
                        sum += n;
                    } else {
                        return Err(RuntimeError::type_mismatch("числа", "не число"));
                    }
                }
                Ok(Value::Number(Number::F64(sum)))
            }
            _ => Err(RuntimeError::new(
                format!("Метод '{}' не найден для массива", method),
                RuntimeErrorKind::Other,
            )),
        }
    }
}
