use super::ExprEvaluator;

use shared::types::{Expr, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    /// [KITE 2/0002] Вычисляет диапазон `начало..конец` (или `..=`) в целочисленное значение.
    pub(crate) fn eval_range(
        start: Option<&Expr>,
        end: Option<&Expr>,
        inclusive: bool,
        step: Option<&Expr>,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let to_int = |e: &Expr, env: &mut Environment| -> RuntimeResult<i64> {
            Self::evaluate(e, env)?
                .as_int()
                .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))
        };
        let s = match start {
            Some(e) => to_int(e, env)?,
            None => {
                return Err(RuntimeError::new(
                    "Диапазон без начала пока не поддерживается",
                    RuntimeErrorKind::Other,
                ));
            }
        };
        let en = match end {
            Some(e) => to_int(e, env)?,
            None => {
                return Err(RuntimeError::new(
                    "Диапазон без конца пока не поддерживается",
                    RuntimeErrorKind::Other,
                ));
            }
        };
        let step_i = match step {
            Some(e) => to_int(e, env)?,
            None => 1,
        };
        if step_i == 0 {
            return Err(RuntimeError::new(
                "Шаг диапазона не может быть равен нулю",
                RuntimeErrorKind::Other,
            ));
        }
        Ok(Value::Range {
            start: s,
            end: en,
            inclusive,
            step: step_i,
        })
    }
}
