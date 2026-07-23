use std::collections::BTreeMap;

use super::ExprEvaluator;

use shared::types::{Expr, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    // =============================================================================
    //         SECTION: OOP OBJECT INSTANTIATION
    // =============================================================================

    pub(crate) fn eval_new_instance(
        class_name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Get class definition.
        let class = env.get_class(class_name)?.clone();

        // [KITE 11] Cannot instantiate abstract class.
        if class.is_abstract {
            return Err(RuntimeError::new(
                format!(
                    "Нельзя создать экземпляр абстрактного класса '{}'",
                    class_name
                ),
                RuntimeErrorKind::Other,
            ));
        }

        // Create fields with default values.
        let mut fields = BTreeMap::new();
        for field in &class.fields {
            let value = if let Some(default) = &field.default {
                Self::evaluate(default, env)?
            } else {
                Self::default_value_for_type(&field.type_kind)
            };
            fields.insert(field.name.to_string(), value);
        }

        // If there is a constructor, call it.
        if !class.constructors.is_empty() {
            // Find suitable constructor by argument count.
            let constructor = class
                .constructors
                .iter()
                .find(|c| c.algorithm.params.len() == args.len())
                .ok_or_else(|| {
                    RuntimeError::argument_count(
                        &format!("{}::конструктор", class_name),
                        class.constructors[0].algorithm.params.len(),
                        args.len(),
                    )
                })?
                .clone();

            // Create object [KITE 11: stable type_id from registry].
            let obj = Value::Object {
                type_id: env.class_type_id(class_name),
                fields: fields.clone(),
            };

            // [KITE 4] Arguments in caller frame, before constructor frame.
            let mut bound: Vec<(String, Value)> =
                Vec::with_capacity(constructor.algorithm.params.len());
            for (i, param) in constructor.algorithm.params.iter().enumerate() {
                let value = Self::evaluate(&args[i], env)?;
                bound.push((param.name.to_string(), value));
            }

            // Call constructor.
            env.push_method_frame(format!("{}::конструктор", class_name), obj.clone())?;
            for (name, value) in bound {
                env.define_local(name, value);
            }

            // Execute constructor body. Constructor body error = `new` error
            // (cannot be silently ignored—otherwise object is "half-constructed").
            // Always pop frame.
            let result = super::super::executor::Executor::execute_stmts(
                constructor.algorithm.body.as_deref().unwrap_or(&[]),
                env,
            );

            // Get updated object (before popping frame—`this` lives in frame).
            let updated_obj = env.get_this().cloned().unwrap_or(obj);
            env.pop_frame();

            match result {
                Ok(_) => {}
                // [KITE-0002] `?` operator in constructor body: early return
                // of the constructor (value becomes the constructed object).
                Err(e) if e.is_propagation() => {}
                Err(e) => return Err(e),
            }

            return Ok(updated_obj);
        }

        // Return object without constructor [KITE 11: stable type_id].
        Ok(Value::Object {
            type_id: env.class_type_id(class_name),
            fields,
        })
    }

    // =============================================================================
    //         SECTION: ENUMERATIONS
    // =============================================================================

    pub(crate) fn eval_enum_construct(
        enum_name: &str,
        variant: &str,
        data: Option<&Expr>,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Проверяем, что вариант существует
        if !env.is_valid_enum_variant(enum_name, variant) {
            return Err(RuntimeError::new(
                format!(
                    "Вариант '{}' не найден в перечислении '{}'",
                    variant, enum_name
                ),
                RuntimeErrorKind::Other,
            ));
        }

        let data_value = if let Some(expr) = data {
            Some(Box::new(Self::evaluate(expr, env)?))
        } else {
            None
        };

        Ok(Value::Enum {
            name: enum_name.to_string(),
            variant: variant.to_string(),
            data: data_value,
        })
    }
}
