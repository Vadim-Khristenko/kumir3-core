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
            // ===== УТИЛИТЫ =====
            "печать" | "print" => {
                let output: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                env.print(&output.join(" "));
                Ok(Some(Value::Null))
            }

            "печатьстр" | "println" => {
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

            // Функция не найдена
            _ => Ok(None),
        }
    }
}
