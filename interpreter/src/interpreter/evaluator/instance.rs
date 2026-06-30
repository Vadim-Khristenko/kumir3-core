use std::collections::BTreeMap;

use super::ExprEvaluator;

use shared::types::{Expr, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    // =========================================================================
    //                    ООП: СОЗДАНИЕ ЭКЗЕМПЛЯРА
    // =========================================================================

    pub(crate) fn eval_new_instance(
        class_name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Получаем определение класса
        let class = env.get_class(class_name)?.clone();

        // [KITE 11] Нельзя создавать экземпляр абстрактного класса.
        if class.is_abstract {
            return Err(RuntimeError::new(
                format!(
                    "Нельзя создать экземпляр абстрактного класса '{}'",
                    class_name
                ),
                RuntimeErrorKind::Other,
            ));
        }

        // Создаём поля со значениями по умолчанию
        let mut fields = BTreeMap::new();
        for field in &class.fields {
            let value = if let Some(default) = &field.default {
                Self::evaluate(default, env)?
            } else {
                Self::default_value_for_type(&field.type_kind)
            };
            fields.insert(field.name.to_string(), value);
        }

        // Если есть конструктор, вызываем его
        if !class.constructors.is_empty() {
            // Ищем подходящий конструктор по количеству аргументов
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

            // Создаём объект [KITE 11: стабильный type_id из реестра].
            let obj = Value::Object {
                type_id: env.class_type_id(class_name),
                fields: fields.clone(),
            };

            // [KITE 4] Аргументы — в кадре вызывающего, до создания кадра конструктора.
            let mut bound: Vec<(String, Value)> =
                Vec::with_capacity(constructor.algorithm.params.len());
            for (i, param) in constructor.algorithm.params.iter().enumerate() {
                let value = Self::evaluate(&args[i], env)?;
                bound.push((param.name.to_string(), value));
            }

            // Вызываем конструктор
            env.push_method_frame(format!("{}::конструктор", class_name), obj.clone())?;
            for (name, value) in bound {
                env.define_local(name, value);
            }

            // Выполняем тело конструктора
            let _ = super::super::executor::Executor::execute_stmts(
                constructor.algorithm.body.as_deref().unwrap_or(&[]),
                env,
            );

            // Получаем обновлённый объект
            let updated_obj = env.get_this().cloned().unwrap_or(obj);
            env.pop_frame();

            return Ok(updated_obj);
        }

        // Возвращаем объект без конструктора [KITE 11: стабильный type_id].
        Ok(Value::Object {
            type_id: env.class_type_id(class_name),
            fields,
        })
    }

    // =========================================================================
    //                    ПЕРЕЧИСЛЕНИЯ
    // =========================================================================

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
