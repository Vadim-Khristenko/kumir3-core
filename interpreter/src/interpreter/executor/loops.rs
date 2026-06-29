//! Циклы.

use super::super::environment::Environment;
use super::super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::evaluator::ExprEvaluator;
use super::Executor;
use shared::types::{Expr, Number, Stmt, Value};

impl Executor {
    // =========================================================================
    //                    ЦИКЛЫ
    // =========================================================================

    pub(crate) fn execute_while(
        condition: &Expr,
        body: &[Stmt],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        loop {
            let cond_value = ExprEvaluator::evaluate(condition, env)?;
            if !ExprEvaluator::is_truthy(&cond_value) {
                break;
            }

            match Self::execute_stmts(body, env)? {
                ControlFlow::Break => break,
                ControlFlow::Continue => continue,
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::None => {}
            }
        }
        Ok(ControlFlow::None)
    }

    pub(crate) fn execute_for(
        variable: &str,
        from: &Expr,
        to: &Expr,
        step: Option<&Expr>,
        body: &[Stmt],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let start = ExprEvaluator::evaluate(from, env)?;
        let end = ExprEvaluator::evaluate(to, env)?;
        let step_val = if let Some(s) = step {
            ExprEvaluator::evaluate(s, env)?
        } else {
            Value::Number(Number::I64(1))
        };

        let start_i = start
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;
        let end_i = end
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;
        let step_i = step_val
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;

        if step_i == 0 {
            return Err(RuntimeError::new(
                "Шаг цикла не может быть равен нулю",
                RuntimeErrorKind::Other,
            ));
        }

        let mut i = start_i;
        loop {
            // Проверяем условие выхода
            if step_i > 0 {
                if i > end_i {
                    break;
                }
            } else if i < end_i {
                break;
            }

            // Устанавливаем переменную цикла
            env.define_local(variable.to_string(), Value::Number(Number::I64(i)));

            // Выполняем тело
            match Self::execute_stmts(body, env)? {
                ControlFlow::Break => break,
                ControlFlow::Continue => {}
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::None => {}
            }

            // Увеличиваем счётчик
            i += step_i;
        }

        Ok(ControlFlow::None)
    }

    /// [KITE 2/4] Цикл по коллекции/диапазону: `нц для x в <итерируемое> … кц`.
    pub(crate) fn execute_for_each(
        variable: &str,
        iterable: &Expr,
        body: &[Stmt],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let value = ExprEvaluator::evaluate(iterable, env)?;

        // Материализуем последовательность элементов для перебора.
        let items: Vec<Value> = match value {
            Value::Range {
                start,
                end,
                inclusive,
            } => {
                let last = if inclusive { end } else { end - 1 };
                let mut v = Vec::new();
                let mut i = start;
                while i <= last {
                    v.push(Value::Number(Number::I64(i)));
                    i += 1;
                }
                v
            }
            Value::Array(a) => a,
            Value::Tuple(t) => t,
            Value::Set(s) => s.into_iter().collect(),
            Value::String(s) => s.chars().map(Value::Char).collect(),
            other => {
                return Err(RuntimeError::type_mismatch(
                    "коллекция или диапазон",
                    &other.type_kind().russian_name(),
                ));
            }
        };

        for item in items {
            env.define_local(variable.to_string(), item);
            match Self::execute_stmts(body, env)? {
                ControlFlow::Break => break,
                ControlFlow::Continue => {}
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::None => {}
            }
        }

        Ok(ControlFlow::None)
    }

    pub(crate) fn execute_infinite_loop(
        body: &[Stmt],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        loop {
            match Self::execute_stmts(body, env)? {
                ControlFlow::Break => break,
                ControlFlow::Continue => continue,
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::None => {}
            }
        }
        Ok(ControlFlow::None)
    }

    pub(crate) fn execute_do_while(
        body: &[Stmt],
        condition: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        loop {
            match Self::execute_stmts(body, env)? {
                ControlFlow::Break => break,
                ControlFlow::Continue => {}
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::None => {}
            }

            let cond_value = ExprEvaluator::evaluate(condition, env)?;
            if !ExprEvaluator::is_truthy(&cond_value) {
                break;
            }
        }
        Ok(ControlFlow::None)
    }
}
