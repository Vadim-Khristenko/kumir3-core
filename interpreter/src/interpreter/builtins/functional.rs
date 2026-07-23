//! Higher-order functions over collections.
//!
//! Functions here take other functions as parameters. Unlike other builtins,
//! function arguments remain unevaluated expressions: algorithm names like
//! `удвоить` are not variables and would fail if evaluated. Only data arguments
//! are evaluated; the function argument is passed as an expression to the caller.

use shared::types::{Expr, Number, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::evaluator::ExprEvaluator;
use super::Builtins;

impl Builtins {
    /// Attempts to dispatch a higher-order function call.
    ///
    /// Must be checked before other categories because they evaluate all arguments,
    /// which is invalid for function parameters.
    pub(crate) fn try_call_functional(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Option<Value>> {
        match name {
            "отобразить" | "map" => Self::hof_map(name, args, env).map(Some),
            "отобрать" | "отфильтровать" | "filter" => {
                Self::hof_filter(name, args, env).map(Some)
            }
            "свернуть" | "fold" | "reduce" => Self::hof_fold(name, args, env).map(Some),
            "любой_из" | "any" => {
                Self::hof_quantifier(name, args, env, Quantifier::Any).map(Some)
            }
            "все_из" | "all" => {
                Self::hof_quantifier(name, args, env, Quantifier::All).map(Some)
            }
            "найти_первый" | "find_first" => {
                Self::hof_find_first(name, args, env).map(Some)
            }
            "количество" | "count" => Self::hof_count(name, args, env).map(Some),
            "сортировать_по" | "sort_by" => {
                Self::hof_sort_by(name, args, env).map(Some)
            }
            _ => Ok(None),
        }
    }

    // ---------------------------------------------------------------------------
    //                      FUNCTION ARGUMENT INVOCATION
    // ---------------------------------------------------------------------------

    /// Invokes a function argument with pre-evaluated values.
    ///
    /// A bare name (`удвоить`) is delegated to the normal call dispatcher, which
    /// decides if it is a lambda variable, user algorithm, or builtin.
    /// Any other expression (lambda literal, composition result) is evaluated and
    /// must produce a lambda.
    ///
    /// Values are wrapped in `Expr::Literal`, so existing call machinery is reused
    /// (parameter arity checks, `?` operator in function body, etc.).
    fn apply(
        callee: &Expr,
        arg_values: Vec<Value>,
        env: &mut Environment,
        context: &str,
    ) -> RuntimeResult<Value> {
        let arg_exprs: Vec<Expr> = arg_values.into_iter().map(Expr::Literal).collect();

        if let Expr::Variable(name) = callee {
            return ExprEvaluator::evaluate(&Expr::Call(name.clone(), arg_exprs), env);
        }

        match ExprEvaluator::evaluate(callee, env)? {
            Value::Lambda(lambda) => ExprEvaluator::call_lambda(&lambda, &arg_exprs, env),
            other => Err(RuntimeError::new(
                format!(
                    "{context}: вторым аргументом ожидалась функция, получено значение типа {}",
                    other.type_name_ru()
                ),
                RuntimeErrorKind::TypeMismatch,
            )),
        }
    }

    /// Parses calls of the form `функция(table, function)`.
    fn split_table_and_callee<'a>(
        name: &str,
        args: &'a [Expr],
        env: &mut Environment,
    ) -> RuntimeResult<(Vec<Value>, &'a Expr)> {
        if args.len() != 2 {
            return Err(RuntimeError::argument_count(name, 2, args.len()));
        }
        let table = Self::eval_table(name, &args[0], env)?;
        Ok((table, &args[1]))
    }

    /// Evaluates an argument and requires it to be an array.
    fn eval_table(name: &str, expr: &Expr, env: &mut Environment) -> RuntimeResult<Vec<Value>> {
        match ExprEvaluator::evaluate(expr, env)? {
            Value::Array(items) => Ok(items),
            other => Err(RuntimeError::new(
                format!(
                    "{name}: первым аргументом ожидалась таблица, получено значение типа {}",
                    other.type_name_ru()
                ),
                RuntimeErrorKind::TypeMismatch,
            )),
        }
    }

    /// Coerces a filter/predicate result to a boolean.
    ///
    /// Requires exactly `лог` (boolean): silent coercion of other types (per [KITE 13 § 3.20])
    /// would hide typos like `отобрать(a, лямбда(x) -> x)`.
    fn expect_bool(name: &str, value: Value) -> RuntimeResult<bool> {
        match value {
            Value::Boolean(b) => Ok(b),
            other => Err(RuntimeError::new(
                format!(
                    "{name}: функция-условие должна возвращать лог, получено значение типа {}",
                    other.type_name_ru()
                ),
                RuntimeErrorKind::TypeMismatch,
            )),
        }
    }

    // ---------------------------------------------------------------------------
    //                           IMPLEMENTATIONS
    // ---------------------------------------------------------------------------

    /// Map: `отобразить(table, func)` returns the array of function results.
    fn hof_map(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(Self::apply(callee, vec![item], env, name)?);
        }
        Ok(Value::Array(out))
    }

    /// Filter: `отобрать(table, pred)` returns elements where predicate is true.
    fn hof_filter(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;
        let mut out = Vec::new();
        for item in items {
            let verdict = Self::apply(callee, vec![item.clone()], env, name)?;
            if Self::expect_bool(name, verdict)? {
                out.push(item);
            }
        }
        Ok(Value::Array(out))
    }

    /// Fold: `свернуть(table, init, func)` performs left-associative reduction.
    ///
    /// The function receives the accumulator and current element in order.
    /// An initial value is mandatory: fold of an empty array is undefined without it.
    fn hof_fold(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        if args.len() != 3 {
            return Err(RuntimeError::argument_count(name, 3, args.len()));
        }
        let items = Self::eval_table(name, &args[0], env)?;
        let mut acc = ExprEvaluator::evaluate(&args[1], env)?;
        let callee = &args[2];
        for item in items {
            acc = Self::apply(callee, vec![acc, item], env, name)?;
        }
        Ok(acc)
    }

    /// Quantifiers: `любой_из` (any) and `все_из` (all).
    ///
    /// Traversal short-circuits on the first decisive element: true for `any`, false for `all`.
    fn hof_quantifier(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        kind: Quantifier,
    ) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;
        for item in items {
            let verdict = Self::apply(callee, vec![item], env, name)?;
            let verdict = Self::expect_bool(name, verdict)?;
            match kind {
                Quantifier::Any if verdict => return Ok(Value::Boolean(true)),
                Quantifier::All if !verdict => return Ok(Value::Boolean(false)),
                _ => {}
            }
        }
        // Empty array: `any` → false, `all` → true (mathematical convention).
        Ok(Value::Boolean(matches!(kind, Quantifier::All)))
    }

    /// Find: `найти_первый(table, pred)` returns an optional value.
    ///
    /// Returns `некоторое(element)` or `ничего`, not the element itself:
    /// this way "not found" is distinguishable from finding `пусто`.
    fn hof_find_first(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;
        for item in items {
            let verdict = Self::apply(callee, vec![item.clone()], env, name)?;
            if Self::expect_bool(name, verdict)? {
                return Ok(Value::Option(Box::new(Some(item))));
            }
        }
        Ok(Value::Option(Box::new(None)))
    }

    /// Count: `количество(table, pred)` counts elements satisfying the predicate.
    fn hof_count(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;
        let mut n: i64 = 0;
        for item in items {
            let verdict = Self::apply(callee, vec![item], env, name)?;
            if Self::expect_bool(name, verdict)? {
                n += 1;
            }
        }
        Ok(Value::Number(Number::I64(n)))
    }

    /// Sort by key: `сортировать_по(table, key_func)` sorts by a computed key.
    ///
    /// Keys are computed once per element before sorting: calling the function
    /// from the comparator would invoke it O(n log n) times instead of O(n).
    /// Sort is stable: elements with equal keys preserve their original order.
    fn hof_sort_by(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;

        let mut keyed: Vec<(Value, Value)> = Vec::with_capacity(items.len());
        for item in items {
            let key = Self::apply(callee, vec![item.clone()], env, name)?;
            keyed.push((key, item));
        }

        // Key comparison uses the same rules as the `<` operator ([KITE 13 § 3.8]):
        // incomparable keys are an error, not an arbitrary ordering.
        let mut failure = None;
        keyed.sort_by(|(a, _), (b, _)| {
            match super::super::ops::TypeOps::compare(a, b, |o| o == std::cmp::Ordering::Less) {
                Ok(Value::Boolean(true)) => std::cmp::Ordering::Less,
                Ok(_) => match super::super::ops::TypeOps::compare(b, a, |o| {
                    o == std::cmp::Ordering::Less
                }) {
                    Ok(Value::Boolean(true)) => std::cmp::Ordering::Greater,
                    Ok(_) => std::cmp::Ordering::Equal,
                    Err(e) => {
                        failure.get_or_insert(e);
                        std::cmp::Ordering::Equal
                    }
                },
                Err(e) => {
                    failure.get_or_insert(e);
                    std::cmp::Ordering::Equal
                }
            }
        });
        if let Some(e) = failure {
            return Err(e);
        }

        Ok(Value::Array(keyed.into_iter().map(|(_, v)| v).collect()))
    }
}

/// Which quantifier `hof_quantifier` implements.
#[derive(Clone, Copy)]
enum Quantifier {
    Any,
    All,
}
