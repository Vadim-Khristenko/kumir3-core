use std::f64::consts::{E, PI};

use shared::math::MathOperators;
use shared::types::{Expr, Number, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::Builtins;

impl Builtins {
    pub(crate) fn try_call_math(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Option<Value>> {
        let vals = Self::eval_args(args, env)?;
        match name {
            // ===== МАТЕМАТИЧЕСКИЕ ФУНКЦИИ =====
            "abs" | "модуль" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::abs(&vals[0])?))
            }

            "sqrt" | "корень" | "квадратный_корень" => {
                Self::check_args(name, &vals, 1)?;
                MathOperators::sqrt(vals[0].clone(), false)
                    .map(Some)
                    .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other))
            }

            "sin" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::sin)?))
            }

            "cos" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::cos)?))
            }

            "tan" | "tg" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::tan)?))
            }

            "asin" | "arcsin" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::asin)?))
            }

            "acos" | "arccos" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::acos)?))
            }

            "atan" | "arctg" | "arctan" => {
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::atan)?))
            }

            "atan2" | "arctg2" => {
                Self::check_args(name, &vals, 2)?;
                let y = Self::to_f64(&vals[0])?;
                let x = Self::to_f64(&vals[1])?;
                Ok(Some(Value::Number(Number::F64(y.atan2(x)))))
            }

            "ln" | "log" => {
                Self::check_args(name, &vals, 1)?;
                let x = Self::to_f64(&vals[0])?;
                if x <= 0.0 {
                    return Err(RuntimeError::new(
                        "Логарифм определён только для положительных чисел",
                        RuntimeErrorKind::Other,
                    ));
                }
                Ok(Some(Value::Number(Number::F64(x.ln()))))
            }

            "log10" | "lg" => {
                Self::check_args(name, &vals, 1)?;
                let x = Self::to_f64(&vals[0])?;
                if x <= 0.0 {
                    return Err(RuntimeError::new(
                        "Логарифм определён только для положительных чисел",
                        RuntimeErrorKind::Other,
                    ));
                }
                Ok(Some(Value::Number(Number::F64(x.log10()))))
            }

            "exp" => {
                Self::check_args(name, &vals, 1)?;
                let x = Self::to_f64(&vals[0])?;
                Ok(Some(Value::Number(Number::F64(x.exp()))))
            }

            "pow" | "степень" => {
                Self::check_args(name, &vals, 2)?;
                MathOperators::pow(vals[0].clone(), vals[1].clone(), false)
                    .map(Some)
                    .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other))
            }

            "floor" | "пол" => {
                Self::check_args(name, &vals, 1)?;
                let x = Self::to_f64(&vals[0])?;
                Ok(Some(Value::Number(Number::I64(x.floor() as i64))))
            }

            "ceil" | "потолок" => {
                Self::check_args(name, &vals, 1)?;
                let x = Self::to_f64(&vals[0])?;
                Ok(Some(Value::Number(Number::I64(x.ceil() as i64))))
            }

            "round" | "округлить" => {
                if vals.is_empty() || vals.len() > 2 {
                    return Err(RuntimeError::argument_count(name, 1, vals.len()));
                }
                let x = Self::to_f64(&vals[0])?;
                if vals.len() == 2 {
                    let digits = vals[1].as_int().unwrap_or(0);
                    let factor = 10_f64.powi(digits as i32);
                    Ok(Some(Value::Number(Number::F64(
                        (x * factor).round() / factor,
                    ))))
                } else {
                    Ok(Some(Value::Number(Number::I64(x.round() as i64))))
                }
            }

            "min" | "минимум" => {
                if vals.is_empty() {
                    return Err(RuntimeError::argument_count(name, 1, 0));
                }
                let mut min = vals[0].clone();
                for v in &vals[1..] {
                    let cmp = Self::compare_values(&min, v)?;
                    if cmp > 0 {
                        min = v.clone();
                    }
                }
                Ok(Some(min))
            }

            "max" | "максимум" => {
                if vals.is_empty() {
                    return Err(RuntimeError::argument_count(name, 1, 0));
                }
                let mut max = vals[0].clone();
                for v in &vals[1..] {
                    let cmp = Self::compare_values(&max, v)?;
                    if cmp < 0 {
                        max = v.clone();
                    }
                }
                Ok(Some(max))
            }

            "sign" | "знак" | "sgn" => {
                Self::check_args(name, &vals, 1)?;
                let x = Self::to_f64(&vals[0])?;
                let sign = if x > 0.0 {
                    1
                } else if x < 0.0 {
                    -1
                } else {
                    0
                };
                Ok(Some(Value::Number(Number::I64(sign))))
            }

            "пи" | "pi" => Ok(Some(Value::Number(Number::F64(PI)))),

            "е" | "e" => Ok(Some(Value::Number(Number::F64(E)))),

            // ===== СЛУЧАЙНЫЕ ЧИСЛА =====
            "случайное" | "random" | "rand" => {
                if vals.is_empty() {
                    // случайное число от 0.0 до 1.0
                    let r = Self::simple_random();
                    Ok(Some(Value::Number(Number::F64(r))))
                } else if vals.len() == 1 {
                    // случайное целое от 0 до n-1
                    let n = vals[0]
                        .as_int()
                        .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;
                    let r = (Self::simple_random() * n as f64) as i64;
                    Ok(Some(Value::Number(Number::I64(r))))
                } else if vals.len() == 2 {
                    // случайное целое от a до b
                    let a = vals[0]
                        .as_int()
                        .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;
                    let b = vals[1]
                        .as_int()
                        .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;
                    let r = a + (Self::simple_random() * (b - a + 1) as f64) as i64;
                    Ok(Some(Value::Number(Number::I64(r))))
                } else {
                    Err(RuntimeError::argument_count(name, 2, vals.len()))
                }
            }

            // Функция не найдена
            _ => Ok(None),
        }
    }
}
