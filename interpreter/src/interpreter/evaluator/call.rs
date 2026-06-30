use super::ExprEvaluator;

use shared::types::{Algorithm, Expr, Value};

use super::super::builtins::Builtins;
use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    // =========================================================================
    //                    ВЫЗОВ АЛГОРИТМОВ
    // =========================================================================

    pub(crate) fn eval_call(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Сначала пробуем встроенные функции
        if let Some(result) = Builtins::try_call(name, args, env)? {
            return Ok(result);
        }

        // Проверяем перегруженные алгоритмы
        if let Some(overloaded) = env.get_overloaded_algorithm(name).cloned() {
            // Выбираем подходящую перегрузку (упрощённо - по количеству аргументов)
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

        // Получаем алгоритм
        let algorithm = env.get_algorithm(name)?.clone();

        // Проверяем количество аргументов
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

    pub(crate) fn call_algorithm(
        algorithm: &Algorithm,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // [KITE 4] Аргументы вычисляются в кадре ВЫЗЫВАЮЩЕГО, до создания кадра
        // callee (при лексической видимости callee не видит локали вызывающего).
        let mut bound: Vec<(String, Value)> = Vec::with_capacity(algorithm.params.len());
        for (i, param) in algorithm.params.iter().enumerate() {
            let value = if i < args.len() {
                Self::evaluate(&args[i], env)?
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

        // Создаём новый кадр и привязываем параметры.
        env.push_frame(algorithm.name.as_ref())?;
        for (name, value) in bound {
            env.define_local(name, value);
        }

        // Выполняем тело алгоритма
        let result = super::super::executor::Executor::execute_stmts(
            algorithm.body.as_deref().unwrap_or(&[]),
            env,
        );

        // Получаем возвращаемое значение
        let return_value = env.get_result_value().cloned();

        // Удаляем кадр
        env.pop_frame();

        // Обрабатываем результат
        match result {
            Ok(super::super::error::ControlFlow::Return(value)) => Ok(value.unwrap_or(Value::Null)),
            Ok(_) => Ok(return_value.unwrap_or(Value::Null)),
            Err(e) => Err(e),
        }
    }

    /// Вызывает пользовательский алгоритм с уже вычисленными аргументами.
    pub(crate) fn call_user_algorithm(
        name: &str,
        args: &[Value],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Получаем алгоритм
        let algorithm = env.get_algorithm(name)?.clone();

        // Проверяем количество аргументов
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

        // Создаём новый кадр
        env.push_frame(algorithm.name.as_ref())?;

        // Привязываем параметры
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

        // Выполняем тело алгоритма
        let result = super::super::executor::Executor::execute_stmts(
            algorithm.body.as_deref().unwrap_or(&[]),
            env,
        );

        // Получаем возвращаемое значение
        let return_value = env.get_result_value().cloned();

        // Удаляем кадр
        env.pop_frame();

        // Обрабатываем результат
        match result {
            Ok(super::super::error::ControlFlow::Return(value)) => Ok(value.unwrap_or(Value::Null)),
            Ok(_) => Ok(return_value.unwrap_or(Value::Null)),
            Err(e) => Err(e),
        }
    }
}
