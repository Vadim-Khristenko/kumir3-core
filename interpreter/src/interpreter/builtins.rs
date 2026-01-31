//! Встроенные функции интерпретатора Кумир 3
//!
//! Реализация стандартных функций: математика, строки, массивы,
//! ввод/вывод, работа с типами и т.д.

use std::f64::consts::{PI, E};

use shared::types::{Value, Number, Expr};
use shared::math::MathOperators;
use shared::strings::StringOperations;
use shared::f128::F128;

use super::environment::Environment;
use super::evaluator::ExprEvaluator;
use super::error::{RuntimeError, RuntimeResult, RuntimeErrorKind};

/// Встроенные функции.
pub struct Builtins;

impl Builtins {
    /// Пытается вызвать встроенную функцию.
    /// Возвращает Some(value) если функция найдена, None если нет.
    pub fn try_call(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Option<Value>> {
        // Вычисляем аргументы
        let eval_args = |env: &mut Environment| -> RuntimeResult<Vec<Value>> {
            args.iter().map(|e| ExprEvaluator::evaluate(e, env)).collect()
        };

        match name {
            // ===== МАТЕМАТИЧЕСКИЕ ФУНКЦИИ =====
            "abs" | "модуль" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::abs(&vals[0])?))
            }

            "sqrt" | "корень" | "квадратный_корень" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                MathOperators::sqrt(vals[0].clone(), false)
                    .map(Some)
                    .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other))
            }

            "sin" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::sin)?))
            }

            "cos" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::cos)?))
            }

            "tan" | "tg" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::tan)?))
            }

            "asin" | "arcsin" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::asin)?))
            }

            "acos" | "arccos" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::acos)?))
            }

            "atan" | "arctg" | "arctan" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Self::trig_func(&vals[0], f64::atan)?))
            }

            "atan2" | "arctg2" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 2)?;
                let y = Self::to_f64(&vals[0])?;
                let x = Self::to_f64(&vals[1])?;
                Ok(Some(Value::Number(Number::F64(y.atan2(x)))))
            }

            "ln" | "log" => {
                let vals = eval_args(env)?;
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
                let vals = eval_args(env)?;
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
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let x = Self::to_f64(&vals[0])?;
                Ok(Some(Value::Number(Number::F64(x.exp()))))
            }

            "pow" | "степень" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 2)?;
                MathOperators::pow(vals[0].clone(), vals[1].clone(), false)
                    .map(Some)
                    .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other))
            }

            "floor" | "пол" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let x = Self::to_f64(&vals[0])?;
                Ok(Some(Value::Number(Number::I64(x.floor() as i64))))
            }

            "ceil" | "потолок" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let x = Self::to_f64(&vals[0])?;
                Ok(Some(Value::Number(Number::I64(x.ceil() as i64))))
            }

            "round" | "округлить" => {
                let vals = eval_args(env)?;
                if vals.is_empty() || vals.len() > 2 {
                    return Err(RuntimeError::argument_count(name, 1, vals.len()));
                }
                let x = Self::to_f64(&vals[0])?;
                if vals.len() == 2 {
                    let digits = vals[1].as_int().unwrap_or(0);
                    let factor = 10_f64.powi(digits as i32);
                    Ok(Some(Value::Number(Number::F64((x * factor).round() / factor))))
                } else {
                    Ok(Some(Value::Number(Number::I64(x.round() as i64))))
                }
            }

            "min" | "минимум" => {
                let vals = eval_args(env)?;
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
                let vals = eval_args(env)?;
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
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let x = Self::to_f64(&vals[0])?;
                let sign = if x > 0.0 { 1 } else if x < 0.0 { -1 } else { 0 };
                Ok(Some(Value::Number(Number::I64(sign))))
            }

            "пи" | "pi" => {
                Ok(Some(Value::Number(Number::F64(PI))))
            }

            "е" | "e" => {
                Ok(Some(Value::Number(Number::F64(E))))
            }

            "случайное" | "random" | "rand" => {
                let vals = eval_args(env)?;
                if vals.is_empty() {
                    // случайное число от 0.0 до 1.0
                    let r = Self::simple_random();
                    Ok(Some(Value::Number(Number::F64(r))))
                } else if vals.len() == 1 {
                    // случайное целое от 0 до n-1
                    let n = vals[0].as_int().ok_or_else(|| {
                        RuntimeError::type_mismatch("целое число", "не целое")
                    })?;
                    let r = (Self::simple_random() * n as f64) as i64;
                    Ok(Some(Value::Number(Number::I64(r)))
)
                } else if vals.len() == 2 {
                    // случайное целое от a до b
                    let a = vals[0].as_int().ok_or_else(|| {
                        RuntimeError::type_mismatch("целое число", "не целое")
                    })?;
                    let b = vals[1].as_int().ok_or_else(|| {
                        RuntimeError::type_mismatch("целое число", "не целое")
                    })?;
                    let r = a + (Self::simple_random() * (b - a + 1) as f64) as i64;
                    Ok(Some(Value::Number(Number::I64(r))))
                } else {
                    Err(RuntimeError::argument_count(name, 2, vals.len()))
                }
            }

            // ===== СТРОКОВЫЕ ФУНКЦИИ =====
            "длина" | "длин" | "len" | "length" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::String(s) => Ok(Some(Value::Number(Number::I64(s.chars().count() as i64)))),
                    Value::Array(a) => Ok(Some(Value::Number(Number::I64(a.len() as i64)))),
                    _ => Err(RuntimeError::type_mismatch("строка или массив", "другой тип")),
                }
            }

            "символ" | "chr" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let code = vals[0].as_int().ok_or_else(|| {
                    RuntimeError::type_mismatch("целое число", "не целое")
                })?;
                let c = StringOperations::char_from_unicode(code)
                    .map_err(|e| RuntimeError::new(e.msg(), RuntimeErrorKind::Other))?;
                Ok(Some(Value::Char(c)))
            }

            "код" | "ord" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Char(c) => Ok(Some(Value::Number(Number::I64(StringOperations::code_unicode(*c))))),
                    Value::String(s) if s.len() == 1 => {
                        let c = s.chars().next().unwrap();
                        Ok(Some(Value::Number(Number::I64(StringOperations::code_unicode(c)))))
                    }
                    _ => Err(RuntimeError::type_mismatch("символ", "не символ")),
                }
            }

            "подстрока" | "substring" | "substr" | "копировать_строку" => {
                let vals = eval_args(env)?;
                if vals.len() < 2 || vals.len() > 3 {
                    return Err(RuntimeError::argument_count(name, 2, vals.len()));
                }
                let s = match &vals[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let start = vals[1].as_int().ok_or_else(|| {
                    RuntimeError::type_mismatch("целое число", "не целое")
                })? as usize;
                
                let chars: Vec<char> = s.chars().collect();
                let len = if vals.len() == 3 {
                    vals[2].as_int().ok_or_else(|| {
                        RuntimeError::type_mismatch("целое число", "не целое")
                    })? as usize
                } else {
                    chars.len() - start + 1
                };
                
                // Индексы в Кумире начинаются с 1
                let start_idx = start.saturating_sub(1);
                let end_idx = (start_idx + len).min(chars.len());
                
                let result: String = chars[start_idx..end_idx].iter().collect();
                Ok(Some(Value::String(result)))
            }

            "позиция" | "position" | "pos" | "найти" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 2)?;
                let haystack = match &vals[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let needle = match &vals[1] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                
                let pos = haystack.find(&needle).map(|p| {
                    // Возвращаем позицию в символах (не байтах), начиная с 1
                    haystack[..p].chars().count() as i64 + 1
                }).unwrap_or(0);
                
                Ok(Some(Value::Number(Number::I64(pos))))
            }

            "верхний_регистр" | "to_upper" | "upper" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let s = match &vals[0] {
                    Value::String(s) => s.to_uppercase(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                Ok(Some(Value::String(s)))
            }

            "нижний_регистр" | "to_lower" | "lower" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let s = match &vals[0] {
                    Value::String(s) => s.to_lowercase(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                Ok(Some(Value::String(s)))
            }

            "обрезать" | "trim" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let s = match &vals[0] {
                    Value::String(s) => s.trim().to_string(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                Ok(Some(Value::String(s)))
            }

            "заменить" | "replace" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 3)?;
                let s = match &vals[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let from = match &vals[1] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let to = match &vals[2] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                Ok(Some(Value::String(s.replace(&from, &to))))
            }

            "разделить" | "split" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 2)?;
                let s = match &vals[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let delim = match &vals[1] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let parts: Vec<Value> = s.split(&delim)
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                Ok(Some(Value::Array(parts)))
            }

            "соединить" | "join" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 2)?;
                let arr = match &vals[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Err(RuntimeError::type_mismatch("массив", "не массив")),
                };
                let delim = match &vals[1] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let parts: Vec<String> = arr.iter()
                    .map(|v| v.to_string())
                    .collect();
                Ok(Some(Value::String(parts.join(&delim))))
            }

            // ===== ПРЕОБРАЗОВАНИЕ ТИПОВ =====
            "цел" | "int" | "целое" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let i = Self::to_int(&vals[0])?;
                Ok(Some(Value::Number(Number::I64(i))))
            }

            "вещ" | "float" | "вещественное" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let f = Self::to_f64(&vals[0])?;
                Ok(Some(Value::Number(Number::F64(f))))
            }

            "лит" | "str" | "строка" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::String(vals[0].to_string())))
            }

            "лог" | "bool" | "логическое" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Boolean(ExprEvaluator::is_truthy(&vals[0]))))
            }

            // ===== МАССИВЫ =====
            "таб" | "array" | "массив" => {
                let vals = eval_args(env)?;
                Ok(Some(Value::Array(vals)))
            }

            "добавить" | "push" | "append" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 2)?;
                let mut arr = match &vals[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Err(RuntimeError::type_mismatch("массив", "не массив")),
                };
                arr.push(vals[1].clone());
                Ok(Some(Value::Array(arr)))
            }

            "удалить_последний" | "pop" => {
                let vals = eval_args(env)?;
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
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Array(a) => Ok(Some(a.first().cloned().ok_or_else(|| {
                        RuntimeError::new("Массив пуст", RuntimeErrorKind::IndexOutOfBounds)
                    })?)),
                    _ => Err(RuntimeError::type_mismatch("массив", "не массив")),
                }
            }

            "последний" | "last" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Array(a) => Ok(Some(a.last().cloned().ok_or_else(|| {
                        RuntimeError::new("Массив пуст", RuntimeErrorKind::IndexOutOfBounds)
                    })?)),
                    _ => Err(RuntimeError::type_mismatch("массив", "не массив")),
                }
            }

            "сумма" | "sum" => {
                let vals = eval_args(env)?;
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
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Array(a) => {
                        if a.is_empty() {
                            return Err(RuntimeError::new(
                                "Массив пуст",
                                RuntimeErrorKind::Other,
                            ));
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
                let vals = eval_args(env)?;
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
                    _ => Err(RuntimeError::type_mismatch("массив или строка", "другой тип")),
                }
            }

            "сортировать" | "sort" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Array(a) => {
                        let mut sorted = a.clone();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        Ok(Some(Value::Array(sorted)))
                    }
                    _ => Err(RuntimeError::type_mismatch("массив", "не массив")),
                }
            }

            "содержит" | "contains" => {
                let vals = eval_args(env)?;
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
                    _ => Err(RuntimeError::type_mismatch("массив или строка", "другой тип")),
                }
            }

            "пусто" | "empty" | "is_empty" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let empty = match &vals[0] {
                    Value::Array(a) => a.is_empty(),
                    Value::String(s) => s.is_empty(),
                    Value::Null | Value::Undefined => true,
                    _ => false,
                };
                Ok(Some(Value::Boolean(empty)))
            }

            // ===== ПРОВЕРКИ ТИПОВ =====
            "это_число" | "is_number" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Boolean(vals[0].is_number())))
            }

            "это_строка" | "is_string" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Boolean(vals[0].is_string())))
            }

            "это_массив" | "is_array" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Boolean(vals[0].is_array())))
            }

            "это_пусто" | "is_null" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Boolean(vals[0].is_null() || vals[0].is_undefined())))
            }

            "тип" | "type" | "typeof" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let type_name = Self::type_name(&vals[0]);
                Ok(Some(Value::String(type_name.to_string())))
            }

            // ===== ПАРЫ И КОРТЕЖИ =====
            "пара" | "pair" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 2)?;
                Ok(Some(Value::Pair(
                    Box::new(vals[0].clone()),
                    Box::new(vals[1].clone()),
                )))
            }

            "тройка" | "triple" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 3)?;
                Ok(Some(Value::Triple(
                    Box::new(vals[0].clone()),
                    Box::new(vals[1].clone()),
                    Box::new(vals[2].clone()),
                )))
            }

            "кортеж" | "tuple" => {
                let vals = eval_args(env)?;
                Ok(Some(Value::Tuple(vals)))
            }

            // ===== ОПЦИИ =====
            "некоторое" | "some" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                Ok(Some(Value::Option(Box::new(Some(vals[0].clone())))))
            }

            "ничего" | "none" => {
                Ok(Some(Value::Option(Box::new(None))))
            }

            "есть" | "is_some" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Option(opt) => Ok(Some(Value::Boolean(opt.is_some()))),
                    _ => Err(RuntimeError::type_mismatch("опция", "не опция")),
                }
            }

            "извлечь" | "unwrap" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Option(opt) => {
                        let inner = opt.as_ref().as_ref().cloned().ok_or_else(|| {
                            RuntimeError::new("Попытка извлечь значение из пустой опции", RuntimeErrorKind::Other)
                        })?;
                        Ok(Some(inner))
                    },
                    Value::Result(res) => match res.as_ref() {
                        Ok(v) => Ok(Some(v.clone())),
                        Err(e) => Err(RuntimeError::new(
                            format!("Попытка извлечь значение из ошибки: {}", e),
                            RuntimeErrorKind::Other,
                        )),
                    },
                    _ => Err(RuntimeError::type_mismatch("опция или результат", "другой тип")),
                }
            }

            // ===== УТИЛИТЫ =====
            "печать" | "print" => {
                let vals = eval_args(env)?;
                let output: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                env.print(&output.join(" "));
                Ok(Some(Value::Null))
            }

            "печатьстр" | "println" => {
                let vals = eval_args(env)?;
                let output: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                env.println(&output.join(" "));
                Ok(Some(Value::Null))
            }

            "нс" | "newline" | "nl" => {
                // Вывод новой строки
                env.println("");
                Ok(Some(Value::Null))
            }

            "время" | "time" => {
                use std::time::{SystemTime, UNIX_EPOCH};
                let duration = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default();
                Ok(Some(Value::Number(Number::F64(
                    duration.as_secs_f64()
                ))))
            }

            "пауза" | "sleep" | "ждать" => {
                let vals = eval_args(env)?;
                Self::check_args(name, &vals, 1)?;
                let ms = vals[0].as_int().ok_or_else(|| {
                    RuntimeError::type_mismatch("целое число", "не целое")
                })?;
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                Ok(Some(Value::Null))
            }

            // Функция не найдена
            _ => Ok(None),
        }
    }

    // =========================================================================
    //                    ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ
    // =========================================================================

    fn check_args(name: &str, args: &[Value], expected: usize) -> RuntimeResult<()> {
        if args.len() != expected {
            Err(RuntimeError::argument_count(name, expected, args.len()))
        } else {
            Ok(())
        }
    }

    fn to_f64(value: &Value) -> RuntimeResult<f64> {
        match value {
            Value::Number(n) => n.to_f64().ok_or_else(|| {
                RuntimeError::type_mismatch("число", "не число")
            }),
            Value::String(s) => s.parse::<f64>().map_err(|_| {
                RuntimeError::type_mismatch("число", "строка")
            }),
            _ => Err(RuntimeError::type_mismatch("число", "другой тип")),
        }
    }

    fn to_int(value: &Value) -> RuntimeResult<i64> {
        match value {
            Value::Number(n) => n.to_i64().ok_or_else(|| {
                RuntimeError::type_mismatch("целое", "не целое")
            }),
            Value::String(s) => s.parse::<i64>().map_err(|_| {
                RuntimeError::type_mismatch("целое", "строка")
            }),
            Value::Boolean(b) => Ok(if *b { 1 } else { 0 }),
            _ => Err(RuntimeError::type_mismatch("целое", "другой тип")),
        }
    }

    fn abs(value: &Value) -> RuntimeResult<Value> {
        match value {
            Value::Number(n) => {
                let result = match n {
                    Number::I8(v) => Number::I8(v.abs()),
                    Number::I16(v) => Number::I16(v.abs()),
                    Number::I32(v) => Number::I32(v.abs()),
                    Number::I64(v) => Number::I64(v.abs()),
                    Number::I128(v) => Number::I128(v.abs()),
                    Number::U8(v) => Number::U8(*v),
                    Number::U16(v) => Number::U16(*v),
                    Number::U32(v) => Number::U32(*v),
                    Number::U64(v) => Number::U64(*v),
                    Number::U128(v) => Number::U128(*v),
                    Number::F32(v) => Number::F32(v.abs()),
                    Number::F64(v) => Number::F64(v.abs()),
                    Number::F128(v) => Number::F128(v.abs()),
                };
                Ok(Value::Number(result))
            }
            _ => Err(RuntimeError::type_mismatch("число", "не число")),
        }
    }

    fn trig_func<F>(value: &Value, f: F) -> RuntimeResult<Value>
    where
        F: Fn(f64) -> f64,
    {
        let x = Self::to_f64(value)?;
        Ok(Value::Number(Number::F64(f(x))))
    }

    fn compare_values(a: &Value, b: &Value) -> RuntimeResult<i32> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                let fa = na.to_f64().unwrap_or(0.0);
                let fb = nb.to_f64().unwrap_or(0.0);
                Ok(fa.partial_cmp(&fb).map(|o| o as i32).unwrap_or(0))
            }
            (Value::String(sa), Value::String(sb)) => {
                Ok(sa.cmp(sb) as i32)
            }
            _ => Err(RuntimeError::type_mismatch("сравнимые типы", "несравнимые")),
        }
    }

    fn type_name(value: &Value) -> String {
        match value {
            Value::Number(Number::I64(_)) => "цел".to_string(),
            Value::Number(Number::F64(_)) => "вещ".to_string(),
            Value::Number(_) => "число".to_string(),
            Value::String(_) => "лит".to_string(),
            Value::Boolean(_) => "лог".to_string(),
            Value::Char(_) => "сим".to_string(),
            Value::Array(_) => "таб".to_string(),
            Value::Pair(_, _) => "пара".to_string(),
            Value::Triple(_, _, _) => "тройка".to_string(),
            Value::Tuple(_) => "кортеж".to_string(),
            Value::Set(_) => "множество".to_string(),
            Value::Map(_) => "словарь".to_string(),
            Value::Option(_) => "опция".to_string(),
            Value::Result(_) => "результат".to_string(),
            Value::Pointer(_) => "указатель".to_string(),
            Value::Enum { .. } => "перечисление".to_string(),
            Value::Object { .. } => "объект".to_string(),
            Value::NativeObject { type_name, .. } => type_name.clone(),
            Value::Promise { .. } => "промис".to_string(),
            Value::Null => "пусто".to_string(),
            Value::Undefined => "неопределено".to_string(),
        }
    }

    // Простой генератор псевдослучайных чисел
    fn simple_random() -> f64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        use std::cell::Cell;
        
        thread_local! {
            static SEED: Cell<u64> = Cell::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64
            );
        }
        
        SEED.with(|seed| {
            let mut s = seed.get();
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            seed.set(s);
            (s as f64) / (u64::MAX as f64)
        })
    }
}
