use shared::strings::StringOperations;
use shared::types::{Expr, Number, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::Builtins;

impl Builtins {
    pub(crate) fn try_call_string(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Option<Value>> {
        let vals = Self::eval_args(args, env)?;
        match name {
            // ===== СТРОКОВЫЕ ФУНКЦИИ =====
            "длина" | "длин" | "len" | "length" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::String(s) => {
                        Ok(Some(Value::Number(Number::I64(s.chars().count() as i64))))
                    }
                    Value::Array(a) => Ok(Some(Value::Number(Number::I64(a.len() as i64)))),
                    Value::Bytes(b) => Ok(Some(Value::Number(Number::I64(b.len() as i64)))),
                    other @ Value::Range { .. } => Ok(Some(Value::Number(Number::I64(
                        other.len().unwrap_or(0) as i64,
                    )))),
                    _ => Err(RuntimeError::type_mismatch(
                        "строка, массив, диапазон или байты",
                        "другой тип",
                    )),
                }
            }

            // ===== БАЙТЫ (KITE 2) =====
            "байты" | "bytes" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::String(s) => Ok(Some(Value::Bytes(s.clone().into_bytes()))),
                    Value::Bytes(b) => Ok(Some(Value::Bytes(b.clone()))),
                    Value::Array(arr) => {
                        let mut out = Vec::with_capacity(arr.len());
                        for v in arr {
                            let n = v.as_int().ok_or_else(|| {
                                RuntimeError::type_mismatch("целое число (байт)", "не целое")
                            })?;
                            if !(0..=255).contains(&n) {
                                return Err(RuntimeError::new(
                                    format!("Значение байта вне диапазона 0..255: {}", n),
                                    RuntimeErrorKind::Other,
                                ));
                            }
                            out.push(n as u8);
                        }
                        Ok(Some(Value::Bytes(out)))
                    }
                    _ => Err(RuntimeError::type_mismatch(
                        "строка или массив целых",
                        "другой тип",
                    )),
                }
            }

            "строка_из_байт" | "байты_в_строку" | "bytes_to_string" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Bytes(b) => {
                        Ok(Some(Value::String(String::from_utf8_lossy(b).into_owned())))
                    }
                    _ => Err(RuntimeError::type_mismatch("байты", "другой тип")),
                }
            }

            "символ" | "chr" => {
                Self::check_args(name, &vals, 1)?;
                let code = vals[0]
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;
                let c = StringOperations::char_from_unicode(code)
                    .map_err(|e| RuntimeError::new(e.msg(), RuntimeErrorKind::Other))?;
                Ok(Some(Value::Char(c)))
            }

            "код" | "ord" => {
                Self::check_args(name, &vals, 1)?;
                match &vals[0] {
                    Value::Char(c) => Ok(Some(Value::Number(Number::I64(
                        StringOperations::code_unicode(*c),
                    )))),
                    Value::String(s) if s.len() == 1 => {
                        let c = s.chars().next().unwrap();
                        Ok(Some(Value::Number(Number::I64(
                            StringOperations::code_unicode(c),
                        ))))
                    }
                    _ => Err(RuntimeError::type_mismatch("символ", "не символ")),
                }
            }

            "подстрока" | "substring" | "substr" | "копировать_строку" => {
                if vals.len() < 2 || vals.len() > 3 {
                    return Err(RuntimeError::argument_count(name, 2, vals.len()));
                }
                let s = match &vals[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let start = vals[1]
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?
                    as usize;

                let chars: Vec<char> = s.chars().collect();
                let len = if vals.len() == 3 {
                    vals[2]
                        .as_int()
                        .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?
                        as usize
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
                Self::check_args(name, &vals, 2)?;
                let haystack = match &vals[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let needle = match &vals[1] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };

                let pos = haystack
                    .find(&needle)
                    .map(|p| {
                        // Возвращаем позицию в символах (не байтах), начиная с 1
                        haystack[..p].chars().count() as i64 + 1
                    })
                    .unwrap_or(0);

                Ok(Some(Value::Number(Number::I64(pos))))
            }

            "верхний_регистр" | "to_upper" | "upper" => {
                Self::check_args(name, &vals, 1)?;
                let s = match &vals[0] {
                    Value::String(s) => s.to_uppercase(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                Ok(Some(Value::String(s)))
            }

            "нижний_регистр" | "to_lower" | "lower" => {
                Self::check_args(name, &vals, 1)?;
                let s = match &vals[0] {
                    Value::String(s) => s.to_lowercase(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                Ok(Some(Value::String(s)))
            }

            "обрезать" | "trim" => {
                Self::check_args(name, &vals, 1)?;
                let s = match &vals[0] {
                    Value::String(s) => s.trim().to_string(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                Ok(Some(Value::String(s)))
            }

            "заменить" | "replace" => {
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
                Self::check_args(name, &vals, 2)?;
                let s = match &vals[0] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let delim = match &vals[1] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let parts: Vec<Value> = s
                    .split(&delim)
                    .map(|p| Value::String(p.to_string()))
                    .collect();
                Ok(Some(Value::Array(parts)))
            }

            "соединить" | "join" => {
                Self::check_args(name, &vals, 2)?;
                let arr = match &vals[0] {
                    Value::Array(a) => a.clone(),
                    _ => return Err(RuntimeError::type_mismatch("массив", "не массив")),
                };
                let delim = match &vals[1] {
                    Value::String(s) => s.clone(),
                    _ => return Err(RuntimeError::type_mismatch("строка", "не строка")),
                };
                let parts: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                Ok(Some(Value::String(parts.join(&delim))))
            }

            // Функция не найдена
            _ => Ok(None),
        }
    }
}
