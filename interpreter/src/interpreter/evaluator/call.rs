use super::ExprEvaluator;

use shared::types::{Algorithm, Expr, LambdaValue, ParamMode, Value};

use super::super::builtins::Builtins;
use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    // =============================================================================
    //         SECTION: ALGORITHM CALLS
    // =============================================================================

    pub(crate) fn eval_call(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // [KITE-0015] Qualified call `Module::member(...)`: the parser joins the name
        // via `::`, and import/`module` register the member as `Module.member`.
        // Resolve it the same way as the dotted form.
        if let Some((module, member)) = name.rsplit_once("::") {
            return Self::eval_qualified_call(module, member, name, args, env);
        }
        Self::eval_plain_call(name, args, env)
    }

    /// Call by simple (unqualified) name: builtins → lambdas → library → algorithms.
    pub(crate) fn eval_plain_call(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Try builtins first.
        if let Some(result) = Builtins::try_call(name, args, env)? {
            return Ok(result);
        }

        // Try lambda value stored in a variable.
        if let Ok(Value::Lambda(lambda)) = env.get_variable(name).cloned() {
            return Self::call_lambda(lambda.as_ref(), args, env);
        }

        // Library function (`use time` → `current_year()`).
        // Searched after value algorithms but before user algorithms:
        // a program may define its own algorithm with the same name and hide
        // the library one, as overloads do.
        if !env.has_algorithm(name)
            && env.is_library_function(name)
            && let Some(result) = Self::call_library(name, args, env)?
        {
            return Ok(result);
        }

        // Check overloaded algorithms.
        if let Some(overloaded) = env.get_overloaded_algorithm(name).cloned() {
            // Select matching overload by argument count.
            for alg in &overloaded.overloads {
                if alg.params.len() == args.len() {
                    return Self::call_algorithm(alg, args, env);
                }
            }
            return Err(RuntimeError::argument_count(
                name,
                overloaded.overloads[0].params.len(),
                args.len(),
            ));
        }

        // Get algorithm.
        let algorithm = match env.get_algorithm(name) {
            Ok(alg) => alg.clone(),
            Err(err) => return Err(Self::explain_unknown_call(name, err)),
        };

        // Check argument count.
        let required_params = algorithm
            .params
            .iter()
            .filter(|p| p.default.is_none())
            .count();

        if args.len() < required_params || args.len() > algorithm.params.len() {
            return Err(RuntimeError::argument_count(
                name,
                algorithm.params.len(),
                args.len(),
            ));
        }

        Self::call_algorithm(&algorithm, args, env)
    }

    /// Enhances "algorithm not defined" error with library suggestion.
    ///
    /// The most common cause of this error is a forgotten `use`: the function
    /// exists but its library is not imported. A bare "algorithm not defined"
    /// then misleads to search for a typo where there is none.
    fn explain_unknown_call(name: &str, err: RuntimeError) -> RuntimeError {
        let Some(library) = shared::libraries::registry::library_providing_function(name) else {
            return err;
        };
        RuntimeError::new(
            format!(
                "Алгоритм не определён: '{name}'. \
                 Такая функция есть в библиотеке «{library}» — \
                 добавьте в начало программы: использовать {library}"
            ),
            RuntimeErrorKind::UndefinedAlgorithm,
        )
    }

    /// Calls a library function.
    ///
    /// Library handlers receive ready values, so arguments are evaluated here;
    /// laziness is not needed—these are regular functions.
    fn call_library(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Option<Value>> {
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(Self::evaluate(arg, env)?);
        }
        env.call_library_function(name, &values)
    }

    pub(crate) fn call_algorithm(
        algorithm: &Algorithm,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // [KITE 4] Arguments are evaluated in the CALLER frame, before creating the callee
        // frame (with lexical scoping, callee does not see caller's locals).
        let mut bound: Vec<(String, Value)> = Vec::with_capacity(algorithm.params.len());
        // Parameters whose final value belongs to the caller: `рез` and `аргрез`.
        let mut outgoing: Vec<(String, &str)> = Vec::new();
        for (i, param) in algorithm.params.iter().enumerate() {
            if writes_back(&param.mode) && i < args.len() {
                let Expr::Variable(target) = &args[i] else {
                    return Err(RuntimeError::new(
                        format!(
                            "Параметру '{}' алгоритма '{}' можно передать только переменную: \
                             он объявлен как {} и возвращает значение вызывающему",
                            param.name,
                            algorithm.name,
                            mode_name(&param.mode)
                        ),
                        RuntimeErrorKind::TypeMismatch,
                    ));
                };
                outgoing.push((param.name.to_string(), target.as_ref()));
            }

            let value = if i < args.len() {
                // A `рез` parameter carries nothing in, and its argument may well
                // be a variable that has not been given a value yet — so a failure
                // to read it is not an error, it is the normal case.
                match Self::evaluate(&args[i], env) {
                    Ok(value) => value,
                    Err(_) if matches!(param.mode, ParamMode::Out) => Value::Null,
                    Err(e) => return Err(e),
                }
            } else if let Some(default) = &param.default {
                Self::evaluate(default, env)?
            } else {
                return Err(RuntimeError::argument_count(
                    &algorithm.name,
                    algorithm.params.len(),
                    args.len(),
                ));
            };
            bound.push((param.name.to_string(), value));
        }

        // Create new frame and bind parameters.
        env.push_frame(algorithm.name.as_ref())?;
        for (name, value) in bound {
            env.define_local(name, value);
        }

        // Execute algorithm body.
        let result = super::super::executor::Executor::execute_stmts(
            algorithm.body.as_deref().unwrap_or(&[]),
            env,
        );

        // Get return value.
        let return_value = env.get_result_value().cloned();

        // Read the outgoing parameters while the callee frame is still alive;
        // they can only be assigned once it is gone and the caller's variables
        // are visible again.
        let written: Vec<(&str, Value)> = outgoing
            .iter()
            .filter_map(|(param, target)| {
                env.get_variable(param).ok().map(|v| (*target, v.clone()))
            })
            .collect();

        // Pop frame.
        env.pop_frame();

        // Handle result.
        match result {
            Ok(super::super::error::ControlFlow::Return(value)) => {
                Self::write_back(written, env)?;
                Ok(value.unwrap_or(Value::Null))
            }
            Ok(_) => {
                Self::write_back(written, env)?;
                Ok(return_value.unwrap_or(Value::Null))
            }
            // [KITE-0002] `?` operator signal: early return of this value.
            Err(e) if e.is_propagation() => {
                Self::write_back(written, env)?;
                Ok(*e.propagate.expect("propagation carries a value"))
            }
            // The call failed, so nothing is written back: the caller's variables
            // keep the values they had before the attempt.
            Err(e) => Err(e),
        }
    }

    /// Assigns `рез` and `аргрез` results into the caller's variables.
    fn write_back(written: Vec<(&str, Value)>, env: &mut Environment) -> RuntimeResult<()> {
        for (target, value) in written {
            // The variable may not exist in the caller yet: `рез` is exactly how
            // an algorithm hands out a value the caller has not computed itself.
            if env.set_variable(target, value.clone()).is_err() {
                env.define_local(target.to_string(), value);
            }
        }
        Ok(())
    }

    /// Calls a user algorithm with already-evaluated arguments.
    pub(crate) fn call_user_algorithm(
        name: &str,
        args: &[Value],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Get algorithm.
        let algorithm = match env.get_algorithm(name) {
            Ok(alg) => alg.clone(),
            Err(err) => return Err(Self::explain_unknown_call(name, err)),
        };

        // Check argument count.
        let required_params = algorithm
            .params
            .iter()
            .filter(|p| p.default.is_none())
            .count();

        if args.len() < required_params || args.len() > algorithm.params.len() {
            return Err(RuntimeError::argument_count(
                name,
                algorithm.params.len(),
                args.len(),
            ));
        }

        // Create new frame.
        env.push_frame(algorithm.name.as_ref())?;

        // Bind parameters.
        for (i, param) in algorithm.params.iter().enumerate() {
            let value = if i < args.len() {
                args[i].clone()
            } else if let Some(default) = &param.default {
                Self::evaluate(default, env)?
            } else {
                return Err(RuntimeError::argument_count(
                    &algorithm.name,
                    algorithm.params.len(),
                    args.len(),
                ));
            };
            env.define_local(param.name.to_string(), value);
        }

        // Execute algorithm body.
        let result = super::super::executor::Executor::execute_stmts(
            algorithm.body.as_deref().unwrap_or(&[]),
            env,
        );

        // Get return value.
        let return_value = env.get_result_value().cloned();

        // Pop frame.
        env.pop_frame();

        // Handle result.
        match result {
            Ok(super::super::error::ControlFlow::Return(value)) => Ok(value.unwrap_or(Value::Null)),
            Ok(_) => Ok(return_value.unwrap_or(Value::Null)),
            // [KITE-0002] `?` operator signal: early return of this value.
            Err(e) if e.is_propagation() => Ok(*e.propagate.expect("propagation carries a value")),
            Err(e) => Err(e),
        }
    }

    /// Calls a lambda value with given arguments.
    pub(crate) fn call_lambda(
        lambda: &LambdaValue,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        if args.len() != lambda.params.len() {
            return Err(RuntimeError::argument_count(
                "lambda",
                lambda.params.len(),
                args.len(),
            ));
        }

        // Arguments are evaluated in the caller's frame.
        let mut arg_values: Vec<Value> = Vec::with_capacity(args.len());
        for arg in args {
            arg_values.push(Self::evaluate(arg, env)?);
        }

        // New frame for lambda.
        env.push_frame("lambda")?;

        // Captured variables.
        for (name, value) in &lambda.captures {
            env.define_local(name.clone(), value.clone());
        }

        // Parameters.
        for (i, param) in lambda.params.iter().enumerate() {
            env.define_local(param.clone(), arg_values[i].clone());
        }

        // Execute lambda body.
        let result = Self::evaluate(&lambda.body, env);

        // Pop frame regardless of result.
        env.pop_frame();

        // [KITE-0002] `?` operator signal inside lambda body—early return.
        match result {
            Err(e) if e.is_propagation() => Ok(*e.propagate.expect("propagation carries a value")),
            other => other,
        }
    }
}

// =============================================================================
//         SECTION: PARAMETER MODES
// =============================================================================

/// Does this mode hand the parameter's final value back to the caller?
///
/// `рез` and `аргрез` are the whole point of КуМир's parameter modes: an
/// algorithm that swaps two values has nowhere else to put its result. `Out` and
/// `InOut` are their direct spelling; `BorrowMut` is the modern form of the same
/// intent.
fn writes_back(mode: &ParamMode) -> bool {
    matches!(
        mode,
        ParamMode::Out | ParamMode::InOut | ParamMode::BorrowMut
    )
}

/// Mode name as it is written in a program — for diagnostics.
fn mode_name(mode: &ParamMode) -> &'static str {
    match mode {
        ParamMode::Out => "рез",
        ParamMode::InOut => "аргрез",
        ParamMode::BorrowMut => "изм",
        ParamMode::In => "арг",
        ParamMode::Borrow => "ссылка",
        ParamMode::Move => "перемещение",
    }
}
