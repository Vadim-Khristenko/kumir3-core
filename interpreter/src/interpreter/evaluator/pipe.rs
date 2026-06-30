use super::ExprEvaluator;

use shared::types::{Expr, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    pub(crate) fn eval_pipe(
        value: &Expr,
        func: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let val = Self::evaluate(value, env)?;

        match func {
            Expr::Call(name, args) => {
                let mut new_args = vec![Expr::Literal(val)];
                new_args.extend(args.clone());
                Self::eval_call(name, &new_args, env)
            }
            Expr::Variable(name) => Self::eval_call(name, &[Expr::Literal(val)], env),
            Expr::Pipe(inner_val, inner_func) => {
                let intermediate = Self::eval_pipe(&Expr::Literal(val), inner_val, env)?;
                Self::eval_pipe(&Expr::Literal(intermediate), inner_func, env)
            }
            _ => Err(RuntimeError::new(
                "Правая часть |> должна быть вызовом функции",
                RuntimeErrorKind::Other,
            )),
        }
    }
}
