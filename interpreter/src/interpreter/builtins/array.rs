use shared::types::{Expr, Number, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::Builtins;

impl Builtins {
    pub(crate) fn try_call_array(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Option<Value>> {
        let vals = Self::eval_args(args, env)?;
        match name {
            // ===== МАССИВЫ =====
            "таб" | "array" | "массив" => Ok(Some(Value::Array(vals))),

            "добавить" | "push" | "append" => {
                Self::check_args(name, &vals, 2)?;
                let mut arr = match &vals[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Err(RuntimeError::type_mismatch("массив", "не массив")),
                };
                arr.push(vals[1].clone());
                Ok(Some(Value::Array(arr)))
            }

            "удалить_последний" | "pop" => {
                Self::check_args(name, &vals, 1)?;
                let mut arr = match &vals[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Err(RuntimeError::type_mismatch("массив", "не массив")),
                };
                Ok(Some(arr.pop().ok_or_else(|| {
                    RuntimeError::new("Массив пуст", RuntimeErrorKind::IndexOutOfBounds)
                })?))
            }

            "первый" | "first" | "head" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Array(a) => Ok(Some(a.first().cloned().ok_or_else(|| {
                        RuntimeError::new("Массив пуст", RuntimeErrorKind::IndexOutOfBounds)
                    })?)),
                    _ => Err(RuntimeError::type_mismatch("массив", "не массив")),
                }
            }

            "последний" | "last" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Array(a) => Ok(Some(a.last().cloned().ok_or_else(|| {
                        RuntimeError::new("Массив пуст", RuntimeErrorKind::IndexOutOfBounds)
                    })?)),
                    _ => Err(RuntimeError::type_mismatch("массив", "не массив")),
                }
            }

            "сумма" | "sum" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Array(a) => {
                        let mut sum = 0.0_f64;
                        for v in a {
                            sum += Self::to_f64(v)?;
                        }
                        Ok(Some(Value::Number(Number::F64(sum))))
                    }
                    _ => Err(RuntimeError::type_mismatch("массив", "не массив")),
                }
            }

            "среднее" | "avg" | "average" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Array(a) => {
                        if a.is_empty() {
                            return Err(RuntimeError::new("Массив пуст", RuntimeErrorKind::Other));
                        }
                        let mut sum = 0.0_f64;
                        for v in a {
                            sum += Self::to_f64(v)?;
                        }
                        Ok(Some(Value::Number(Number::F64(sum / a.len() as f64))))
                    }
                    _ => Err(RuntimeError::type_mismatch("массив", "не массив")),
                }
            }

            "обратить" | "reverse" | "перевернуть" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Array(a) => {
                        let mut reversed = a.clone();
                        reversed.reverse();
                        Ok(Some(Value::Array(reversed)))
                    }
                    Value::String(s) => {
                        let reversed: String = s.chars().rev().collect();
                        Ok(Some(Value::String(reversed)))
                    }
                    _ => Err(RuntimeError::type_mismatch(
                        "массив или строка",
                        "другой тип",
                    )),
                }
            }

            "сортировать" | "sort" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Array(a) => {
                        let mut sorted = a.clone();
                        sorted
                            .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        Ok(Some(Value::Array(sorted)))
                    }
                    _ => Err(RuntimeError::type_mismatch("массив", "не массив")),
                }
            }

            "содержит" | "contains" => {
                Self::check_args(name, &vals, 2)?;
                match &vals[0] {
                    Value::Array(a) => Ok(Some(Value::Boolean(a.contains(&vals[1])))),
                    Value::String(s) => {
                        let needle = match &vals[1] {
                            Value::String(n) => n.clone(),
                            _ => vals[1].to_string(),
                        };
                        Ok(Some(Value::Boolean(s.contains(&needle))))
                    }
                    _ => Err(RuntimeError::type_mismatch(
                        "массив или строка",
                        "другой тип",
                    )),
                }
            }

            "пусто" | "empty" | "is_empty" => {
                Self::check_args(name, &vals, 1)?;
                let empty = match &vals[0] {
                    Value::Array(a) => a.is_empty(),
                    Value::String(s) => s.is_empty(),
                    Value::Null | Value::Undefined => true,
                    _ => false,
                };
                Ok(Some(Value::Boolean(empty)))
            }

            // Функция не найдена
            _ => Ok(None),
        }
    }
}
