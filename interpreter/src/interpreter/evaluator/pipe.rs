use super::ExprEvaluator;

use std::collections::BTreeMap;

use shared::types::{Expr, LambdaValue, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

/// Synthetic lambda composition parameter name (does not conflict with source code:
/// Kumir identifiers do not start with `__`).
const COMPOSE_ARG: &str = "__аргумент_композиции";
/// Names of captured composition operand lambdas.
const COMPOSE_LHS: &str = "__композиция_слева";
const COMPOSE_RHS: &str = "__композиция_справа";

/// How to call a composition operand: by name or as captured lambda value.
enum Callable {
    /// Name resolved via normal call (algorithm, overload, builtin).
    Named(String),
    /// Lambda value to be captured under a service name.
    Captured(Value),
}

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

    /// [KITE-0013] Function composition `f >> g`.
    ///
    /// Result is a single-argument lambda value equivalent to
    /// `lambda(x) -> g(f(x))`: left operand is applied first, then right.
    /// Both operands must be functions (algorithm/builtin name or lambda value);
    /// otherwise—clear runtime error.
    pub(crate) fn eval_compose(
        left: &Expr,
        right: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let lhs = Self::resolve_composable(left, env)?;
        let rhs = Self::resolve_composable(right, env)?;

        let mut captures: BTreeMap<String, Value> = BTreeMap::new();
        let lhs_name = Self::bind_composable(lhs, COMPOSE_LHS, &mut captures);
        let rhs_name = Self::bind_composable(rhs, COMPOSE_RHS, &mut captures);

        // Body: rhs(lhs(x)).
        let inner = Expr::Call(lhs_name, vec![Expr::Variable(COMPOSE_ARG.to_string())]);
        let body = Expr::Call(rhs_name, vec![inner]);

        Ok(Value::Lambda(Box::new(LambdaValue {
            params: vec![COMPOSE_ARG.to_string()],
            param_types: vec![None],
            return_type: None,
            body: Box::new(body),
            captures,
        })))
    }

    /// Converts `>>` operand to callable form or reports clear error.
    fn resolve_composable(expr: &Expr, env: &mut Environment) -> RuntimeResult<Callable> {
        if let Expr::Variable(name) = expr {
            // Variable with lambda value—capture it.
            if let Ok(value) = env.get_variable(name) {
                return match value {
                    Value::Lambda(_) => Ok(Callable::Captured(value.clone())),
                    other => Err(Self::compose_error(other)),
                };
            }
            // Otherwise, must be algorithm or builtin name.
            return Ok(Callable::Named(name.clone()));
        }

        // Arbitrary expression: must evaluate to lambda
        // (including nested composition `f >> g >> h`).
        match Self::evaluate(expr, env)? {
            value @ Value::Lambda(_) => Ok(Callable::Captured(value)),
            other => Err(Self::compose_error(&other)),
        }
    }

    /// Registers operand in captures (if needed) and returns name for call.
    fn bind_composable(
        callable: Callable,
        slot: &str,
        captures: &mut BTreeMap<String, Value>,
    ) -> String {
        match callable {
            Callable::Named(name) => name,
            Callable::Captured(value) => {
                captures.insert(slot.to_string(), value);
                slot.to_string()
            }
        }
    }

    /// Unified error message for unsuitable `>>` operand.
    fn compose_error(value: &Value) -> RuntimeError {
        RuntimeError::new(
            format!(
                "Операнды композиции '>>' должны быть функциями, получено значение типа {}",
                value.type_name_ru()
            ),
            RuntimeErrorKind::TypeMismatch,
        )
    }
}
