//! Сопоставление с образцом.

use super::super::environment::Environment;
use super::super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::evaluator::ExprEvaluator;
use super::Executor;
use shared::types::{Expr, MatchArm, Pattern, Value};

impl Executor {
    // =========================================================================
    //                    СОПОСТАВЛЕНИЕ С ОБРАЗЦОМ
    // =========================================================================

    pub(crate) fn execute_match(
        expr: &Expr,
        arms: &[MatchArm],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let value = ExprEvaluator::evaluate(expr, env)?;

        for arm in arms {
            if let Some(bindings) = Self::match_pattern(&arm.pattern, &value)? {
                // Проверяем guard если есть
                if let Some(guard) = &arm.guard {
                    // [KITE 4] Блочная область: guard видит локали алгоритма + привязки.
                    env.push_scope();
                    for (name, val) in &bindings {
                        env.define_local(name.clone(), val.clone());
                    }

                    let guard_result = ExprEvaluator::evaluate(guard, env)?;
                    env.pop_scope();

                    if !ExprEvaluator::is_truthy(&guard_result) {
                        continue;
                    }
                }

                // [KITE 4] Блочная область для тела плеча.
                env.push_scope();
                for (name, val) in bindings {
                    env.define_local(name, val);
                }

                let result = Self::execute_stmts(&arm.body, env);
                env.pop_scope();

                return result;
            }
        }

        Err(RuntimeError::new(
            "Ни один паттерн не сопоставился",
            RuntimeErrorKind::Other,
        ))
    }

    fn match_pattern(
        pattern: &Pattern,
        value: &Value,
    ) -> RuntimeResult<Option<Vec<(String, Value)>>> {
        match pattern {
            Pattern::Wildcard => Ok(Some(vec![])),

            Pattern::Literal(lit) => {
                if ExprEvaluator::values_equal(lit, value) {
                    Ok(Some(vec![]))
                } else {
                    Ok(None)
                }
            }

            Pattern::Variable(name) => Ok(Some(vec![(name.clone(), value.clone())])),

            Pattern::EnumVariant {
                enum_name,
                variant,
                bindings,
            } => {
                if let Value::Enum {
                    name,
                    variant: v,
                    data,
                } = value
                    && name == enum_name
                    && v == variant
                {
                    let mut result = vec![];
                    if let Some(data_val) = data
                        && !bindings.is_empty()
                    {
                        // Extract variable name from pattern
                        if let Pattern::Variable(var_name) = &bindings[0] {
                            result.push((var_name.clone(), *data_val.clone()));
                        }
                    }
                    return Ok(Some(result));
                }
                Ok(None)
            }

            Pattern::Range {
                start: _,
                end: _,
                inclusive: _,
            } => {
                // Для чисел проверяем попадание в диапазон
                // TODO: полная реализация
                Ok(None)
            }

            Pattern::Tuple(patterns) => {
                if let Value::Tuple(values) = value {
                    if patterns.len() != values.len() {
                        return Ok(None);
                    }
                    let mut all_bindings = vec![];
                    for (p, v) in patterns.iter().zip(values.iter()) {
                        if let Some(bindings) = Self::match_pattern(p, v)? {
                            all_bindings.extend(bindings);
                        } else {
                            return Ok(None);
                        }
                    }
                    return Ok(Some(all_bindings));
                }
                Ok(None)
            }

            Pattern::Array { elements, rest } => {
                if let Value::Array(values) = value {
                    if elements.len() > values.len() {
                        return Ok(None);
                    }

                    let mut all_bindings = vec![];
                    for (p, v) in elements.iter().zip(values.iter()) {
                        if let Some(bindings) = Self::match_pattern(p, v)? {
                            all_bindings.extend(bindings);
                        } else {
                            return Ok(None);
                        }
                    }

                    // Привязываем остаток если есть
                    if let Some(rest_name) = rest {
                        let rest_values: Vec<Value> = values[elements.len()..].to_vec();
                        all_bindings.push((rest_name.clone(), Value::Array(rest_values)));
                    } else if elements.len() != values.len() {
                        return Ok(None);
                    }

                    return Ok(Some(all_bindings));
                }
                Ok(None)
            }

            Pattern::Or(patterns) => {
                for p in patterns {
                    if let Some(bindings) = Self::match_pattern(p, value)? {
                        return Ok(Some(bindings));
                    }
                }
                Ok(None)
            }

            // Все остальные паттерны (не реализованы)
            _ => Ok(None),
        }
    }
}
