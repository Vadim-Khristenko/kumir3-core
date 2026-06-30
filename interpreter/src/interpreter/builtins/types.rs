use shared::types::{Expr, Number, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::evaluator::ExprEvaluator;
use super::Builtins;

impl Builtins {
    pub(crate) fn try_call_type(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Option<Value>> {
        let vals = Self::eval_args(args, env)?;
        match name {
            // ===== ПРЕОБРАЗОВАНИЕ ТИПОВ =====
            "цел" | "int" | "целое" => {
                Self::check_args(name, &vals, 1)?;
                let i = Self::to_int(&vals[0])?;
                Ok(Some(Value::Number(Number::I64(i))))
            }

            "вещ" | "float" | "вещественное" => {
                Self::check_args(name, &vals, 1)?;
                let f = Self::to_f64(&vals[0])?;
                Ok(Some(Value::Number(Number::F64(f))))
            }

            "лит" | "str" | "строка" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::String(vals[0].to_string())))
            }

            "лог" | "bool" | "логическое" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Boolean(ExprEvaluator::is_truthy(&vals[0]))))
            }

            // ===== ПРОВЕРКИ ТИПОВ =====
            "это_число" | "is_number" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Boolean(vals[0].is_number())))
            }

            "это_строка" | "is_string" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Boolean(vals[0].is_string())))
            }

            "это_массив" | "is_array" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Boolean(vals[0].is_array())))
            }

            "это_пусто" | "is_null" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Boolean(
                    vals[0].is_null() || vals[0].is_undefined(),
                )))
            }

            "тип" | "type" | "typeof" => {
                Self::check_args(name, &vals, 1)?;
                let type_name = Self::type_name(&vals[0]);
                Ok(Some(Value::String(type_name)))
            }

            // ===== ПАРЫ И КОРТЕЖИ =====
            "пара" | "pair" => {
                Self::check_args(name, &vals, 2)?;
                Ok(Some(Value::Pair(
                    Box::new(vals[0].clone()),
                    Box::new(vals[1].clone()),
                )))
            }

            "тройка" | "triple" => {
                Self::check_args(name, &vals, 3)?;
                Ok(Some(Value::Triple(
                    Box::new(vals[0].clone()),
                    Box::new(vals[1].clone()),
                    Box::new(vals[2].clone()),
                )))
            }

            "кортеж" | "tuple" => Ok(Some(Value::Tuple(vals))),

            // ===== ОПЦИИ =====
            "некоторое" | "some" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Option(Box::new(Some(vals[0].clone())))))
            }

            "ничего" | "none" => Ok(Some(Value::Option(Box::new(None)))),

            "есть" | "is_some" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Option(opt) => Ok(Some(Value::Boolean(opt.is_some()))),
                    _ => Err(RuntimeError::type_mismatch("опция", "не опция")),
                }
            }

            "извлечь" | "unwrap" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Option(opt) => {
                        let inner = opt.as_ref().as_ref().cloned().ok_or_else(|| {
                            RuntimeError::new(
                                "Попытка извлечь значение из пустой опции",
                                RuntimeErrorKind::Other,
                            )
                        })?;
                        Ok(Some(inner))
                    }
                    Value::Result(res) => match res.as_ref() {
                        Ok(v) => Ok(Some(v.clone())),
                        Err(e) => Err(RuntimeError::new(
                            format!("Попытка извлечь значение из ошибки: {}", e),
                            RuntimeErrorKind::Other,
                        )),
                    },
                    _ => Err(RuntimeError::type_mismatch(
                        "опция или результат",
                        "другой тип",
                    )),
                }
            }

            // Функция не найдена
            _ => Ok(None),
        }
    }
}
