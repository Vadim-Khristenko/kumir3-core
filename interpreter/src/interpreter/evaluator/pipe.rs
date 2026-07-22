use super::ExprEvaluator;

use std::collections::BTreeMap;

use shared::types::{Expr, LambdaValue, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

/// Имя параметра синтетической лямбды-композиции (не пересекается с именами
/// исходного кода: идентификаторы Кумира не начинаются с `__`).
const COMPOSE_ARG: &str = "__аргумент_композиции";
/// Имена захваченных лямбд-операндов композиции.
const COMPOSE_LHS: &str = "__композиция_слева";
const COMPOSE_RHS: &str = "__композиция_справа";

/// Как вызывать операнд композиции: по имени алгоритма/встроенной функции
/// или по захваченному значению-лямбде.
enum Callable {
    /// Имя, разрешаемое обычным вызовом (алгоритм, перегрузка, builtin).
    Named(String),
    /// Значение-лямбда, которое нужно захватить под служебным именем.
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

    /// [KITE-0013] Композиция функций `f >> g`.
    ///
    /// Результат — значение-лямбда одного аргумента, эквивалентное
    /// `лямбда(x) -> g(f(x))`: сначала применяется левый операнд, затем правый.
    /// Оба операнда обязаны быть функциями (именем алгоритма/встроенной функции
    /// либо значением-лямбдой); иначе — ясная ошибка выполнения.
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

        // Тело: rhs(lhs(x)).
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

    /// Приводит операнд `>>` к вызываемой форме или сообщает ясную ошибку.
    fn resolve_composable(expr: &Expr, env: &mut Environment) -> RuntimeResult<Callable> {
        if let Expr::Variable(name) = expr {
            // Переменная со значением-лямбдой — захватываем значение.
            if let Ok(value) = env.get_variable(name) {
                return match value {
                    Value::Lambda(_) => Ok(Callable::Captured(value.clone())),
                    other => Err(Self::compose_error(other)),
                };
            }
            // Иначе это должно быть имя алгоритма или встроенной функции.
            return Ok(Callable::Named(name.clone()));
        }

        // Произвольное выражение: должно вычисляться в лямбду
        // (в том числе вложенная композиция `f >> g >> h`).
        match Self::evaluate(expr, env)? {
            value @ Value::Lambda(_) => Ok(Callable::Captured(value)),
            other => Err(Self::compose_error(&other)),
        }
    }

    /// Регистрирует операнд в захватах (если нужно) и возвращает имя для вызова.
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

    /// Единая формулировка ошибки для неподходящего операнда `>>`.
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
