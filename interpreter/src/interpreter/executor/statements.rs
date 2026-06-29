//! Остальные вспомогательные инструкции.

use super::super::environment::Environment;
use super::super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::evaluator::ExprEvaluator;
use super::Executor;
use shared::types::{Algorithm, EnumVariant, Expr, Stmt, TypeKind, Value};
use std::sync::Arc;

impl Executor {
    // =========================================================================
    //                    ПРИСВАИВАНИЕ ЭЛЕМЕНТУ МАССИВА
    // =========================================================================

    pub(crate) fn execute_array_assignment(
        name: &str,
        indices: &[Expr],
        value_expr: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let value = ExprEvaluator::evaluate(value_expr, env)?;

        // Получаем массив
        let array = env.get_variable(name)?.clone();

        match array {
            Value::Array(mut elements) => {
                if indices.len() != 1 {
                    return Err(RuntimeError::not_implemented("многомерные массивы"));
                }

                let idx = ExprEvaluator::evaluate(&indices[0], env)?;
                let i = idx
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;

                if i < 0 || i as usize >= elements.len() {
                    return Err(RuntimeError::index_out_of_bounds(i, elements.len()));
                }

                elements[i as usize] = value;
                env.set_variable(name, Value::Array(elements))?;
            }
            Value::Map(mut map) => {
                if indices.len() != 1 {
                    return Err(RuntimeError::new(
                        "Словарь поддерживает только один ключ",
                        RuntimeErrorKind::Other,
                    ));
                }

                let key = ExprEvaluator::evaluate(&indices[0], env)?;
                map.insert(key, value);
                env.set_variable(name, Value::Map(map))?;
            }
            _ => {
                return Err(RuntimeError::type_mismatch(
                    "массив или словарь",
                    "другой тип",
                ));
            }
        }

        Ok(ControlFlow::None)
    }

    // =========================================================================
    //                    ОБЪЯВЛЕНИЕ ПЕРЕМЕННЫХ
    // =========================================================================

    pub(crate) fn execute_var_decl(
        type_spec: &TypeKind,
        names: &[String],
        init: Option<&Expr>,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let initial_value = if let Some(expr) = init {
            ExprEvaluator::evaluate(expr, env)?
        } else {
            ExprEvaluator::default_value_for_type(type_spec)
        };

        // Если инициализация есть и только одна переменная
        if init.is_some() && names.len() == 1 {
            env.define_local(names[0].clone(), initial_value);
        } else {
            // Для нескольких переменных используем значение по умолчанию
            let default = ExprEvaluator::default_value_for_type(type_spec);
            for name in names {
                env.define_local(name.clone(), default.clone());
            }
        }

        Ok(ControlFlow::None)
    }

    // =========================================================================
    //                    ПЕРЕЧИСЛЕНИЯ
    // =========================================================================

    pub(crate) fn execute_enum_decl(
        name: &str,
        variants: &[EnumVariant],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let variant_names: Vec<String> = variants.iter().map(|v| v.name.clone()).collect();
        env.define_enum(name.to_string(), variant_names);
        Ok(ControlFlow::None)
    }

    // =========================================================================
    //                    МОДУЛИ И ЭКСПОРТ
    // =========================================================================

    pub(crate) fn execute_module_decl(
        name: &str,
        body: &[Stmt],
        algorithms: &[Algorithm],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        // Регистрируем алгоритмы модуля с префиксом имени модуля
        for alg in algorithms {
            let full_name = format!("{}.{}", name, alg.name);
            // Создаём копию алгоритма с полным именем для вызова через Модуль.алг()
            let mut prefixed_alg = alg.clone();
            prefixed_alg.name = Arc::from(full_name.as_str());
            env.define_algorithm(prefixed_alg);

            // Также регистрируем оригинальный алгоритм (для вызова без префикса внутри модуля)
            env.define_algorithm(alg.clone());
        }

        // Выполняем глобальные объявления модуля
        for stmt in body {
            Self::execute(stmt, env)?;
        }

        Ok(ControlFlow::None)
    }

    pub(crate) fn execute_export(
        _names: &[String],
        _env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        // TODO: реализация экспорта
        Ok(ControlFlow::None)
    }
}
