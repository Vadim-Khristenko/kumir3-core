use super::ExprEvaluator;

use shared::types::{Expr, Pattern, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    pub(crate) fn eval_match_expr(
        expr: &Expr,
        arms: &[(Pattern, Expr)],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let value = Self::evaluate(expr, env)?;

        for (pattern, result_expr) in arms {
            if let Some(bindings) = Self::match_pattern(pattern, &value)? {
                // [KITE 4] Блочная область: плечо видит локали окружающего алгоритма
                // плюс привязки паттерна; после плеча привязки исчезают.
                env.push_scope();
                for (name, val) in bindings {
                    env.define_local(name, val);
                }
                let result = Self::evaluate(result_expr, env);
                env.pop_scope();
                return result;
            }
        }

        Err(RuntimeError::new(
            "Ни один паттерн не сопоставился",
            RuntimeErrorKind::Other,
        ))
    }

    pub(crate) fn match_pattern(
        pattern: &Pattern,
        value: &Value,
    ) -> RuntimeResult<Option<Vec<(String, Value)>>> {
        match pattern {
            Pattern::Wildcard => Ok(Some(vec![])),

            Pattern::Literal(lit) => {
                if Self::values_equal(lit, value) {
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
                        && bindings.len() == 1
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

            Pattern::Or(patterns) => {
                for p in patterns {
                    if let Some(bindings) = Self::match_pattern(p, value)? {
                        return Ok(Some(bindings));
                    }
                }
                Ok(None)
            }

            _ => Ok(None),
        }
    }
}
