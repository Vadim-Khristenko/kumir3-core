//! Сопоставление с образцом.

use super::super::environment::Environment;
use super::super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::evaluator::ExprEvaluator;
use super::Executor;
use shared::types::{Expr, MatchArm, Number, Pattern, Value};

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
            if let Some(bindings) = Self::match_pattern(&arm.pattern, &value, env)? {
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
        env: &mut Environment,
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
                start,
                end,
                inclusive,
                step,
            } => {
                // Для чисел проверяем попадание в диапазон с учётом шага.
                let n = match value {
                    Value::Number(Number::I64(v)) => *v,
                    Value::Number(Number::I32(v)) => *v as i64,
                    Value::Number(Number::I16(v)) => *v as i64,
                    Value::Number(Number::I8(v)) => *v as i64,
                    Value::Number(Number::U64(v)) => *v as i64,
                    Value::Number(Number::U32(v)) => *v as i64,
                    Value::Number(Number::U16(v)) => *v as i64,
                    Value::Number(Number::U8(v)) => *v as i64,
                    _ => return Ok(None),
                };

                let mut to_int = |e: &Expr| -> RuntimeResult<i64> {
                    ExprEvaluator::evaluate(e, env)?
                        .as_int()
                        .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))
                };

                let (start_i, has_start) = match start {
                    Some(e) => (to_int(e)?, true),
                    None => (0, false),
                };
                let (end_i, has_end) = match end {
                    Some(e) => (to_int(e)?, true),
                    None => (0, false),
                };
                let step_i = match step {
                    Some(e) => to_int(e)?,
                    None => 1,
                };

                if step_i == 0 {
                    return Ok(None);
                }

                let inside = if has_start && has_end {
                    if step_i > 0 {
                        n >= start_i && n <= end_i
                    } else {
                        n <= start_i && n >= end_i
                    }
                } else if has_start {
                    if step_i > 0 {
                        n >= start_i
                    } else {
                        n <= start_i
                    }
                } else if has_end {
                    if step_i > 0 { n <= end_i } else { n >= end_i }
                } else {
                    true
                };

                if !inside {
                    return Ok(None);
                }

                if has_start {
                    let diff = n - start_i;
                    if diff % step_i != 0 || diff / step_i < 0 {
                        return Ok(None);
                    }
                } else if has_end {
                    let diff = end_i - n;
                    if diff % step_i != 0 || diff / step_i < 0 {
                        return Ok(None);
                    }
                }

                Ok(Some(vec![]))
            }

            Pattern::Tuple(patterns) => {
                if let Value::Tuple(values) = value {
                    if patterns.len() != values.len() {
                        return Ok(None);
                    }
                    let mut all_bindings = vec![];
                    for (p, v) in patterns.iter().zip(values.iter()) {
                        if let Some(bindings) = Self::match_pattern(p, v, env)? {
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
                        if let Some(bindings) = Self::match_pattern(p, v, env)? {
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
                    if let Some(bindings) = Self::match_pattern(p, value, env)? {
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
