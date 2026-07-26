//! I/O and system time functions.

use std::time::{SystemTime, UNIX_EPOCH};

use shared::types::{Expr, Number, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::Builtins;

impl Builtins {
    pub(crate) fn try_call_io(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Option<Value>> {
        let vals = Self::eval_args(args, env)?;
        match name {
            // Utilities
            "печать" | "print" => {
                let output: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                env.print(&output.join(" "));
                Ok(Some(Value::Null))
            }

            "печатьстр" | "println" | "вывод_строки" => {
                let output: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                env.println(&output.join(" "));
                Ok(Some(Value::Null))
            }

            // Перевод строки — это значение, а не действие. Прежде `нс()`
            // печатала пустую строку сама и возвращала `Пусто`, поэтому внутри
            // списка аргументов — `вывод "а", нс(), "б"` — она успевала выдать
            // лишнюю строку до самого вывода, а `Пусто` затем печаталось словом
            // «пусто» посреди текста. Возвращая символ перевода строки, она
            // склеивается с остальными аргументами там, где её и поставили.
            //
            // Отдельную пустую строку теперь печатают `вывод нс()` или
            // `вывод ""`.
            "нс" | "newline" | "nl" | "новая_строка" => {
                Self::check_args(name, &vals, 0)?;
                Ok(Some(Value::String("\n".to_string())))
            }

            "время" | "time" => {
                let duration = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default();
                Ok(Some(Value::Number(Number::F64(duration.as_secs_f64()))))
            }

            "пауза" | "sleep" | "ждать" => {
                Self::check_args(name, &vals, 1)?;
                let ms = vals[0]
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;
                std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                Ok(Some(Value::Null))
            }

            _ => Ok(None),
        }
    }
}
