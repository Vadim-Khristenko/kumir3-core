use super::ExprEvaluator;

use shared::types::{Expr, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    // =============================================================================
    //         SECTION: OOP FIELD ACCESS
    // =============================================================================

    pub(crate) fn eval_field_access(
        object: &Expr,
        field: &str,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let obj = Self::evaluate(object, env)?;

        match obj {
            Value::Object { type_id, fields } => {
                let value = fields.get(field).cloned().ok_or_else(|| {
                    RuntimeError::new(
                        format!("Поле '{}' не найдено", field),
                        RuntimeErrorKind::Other,
                    )
                })?;
                // [KITE 11, step 5] Encapsulation check (field visibility).
                let class_name = Self::find_class_name_by_type_id(&type_id, env)
                    .or_else(|| Self::find_class_by_fields(&fields, env));
                if let Some(cn) = class_name {
                    Self::check_field_access(&cn, field, env)?;
                }
                Ok(value)
            }
            Value::Pair(a, b) => match field {
                "первый" | "first" | "a" => Ok(*a),
                "второй" | "second" | "b" => Ok(*b),
                _ => Err(RuntimeError::new(
                    format!("Неизвестное поле пары: {}", field),
                    RuntimeErrorKind::Other,
                )),
            },
            Value::Triple(a, b, c) => match field {
                "первый" | "first" | "a" => Ok(*a),
                "второй" | "second" | "b" => Ok(*b),
                "третий" | "third" | "c" => Ok(*c),
                _ => Err(RuntimeError::new(
                    format!("Неизвестное поле тройки: {}", field),
                    RuntimeErrorKind::Other,
                )),
            },
            _ => Err(RuntimeError::type_mismatch("объект", "не объект")),
        }
    }

    /// [KITE 11, step 5] Checks field visibility on access.
    /// Public fields are always accessible; private fields only from the owning class;
    /// protected fields from the owner and its descendants.
    fn check_field_access(object_class: &str, field: &str, env: &Environment) -> RuntimeResult<()> {
        use shared::types::Visibility;
        let mut current = object_class.to_string();
        loop {
            let class = match env.get_class(&current) {
                Ok(c) => c.clone(),
                Err(_) => return Ok(()),
            };
            if let Some(f) = class.fields.iter().find(|f| f.name.as_ref() == field) {
                let owner = current;
                return match f.visibility {
                    Visibility::Public => Ok(()),
                    Visibility::Private => {
                        if env.current_defining_class().as_deref() == Some(owner.as_str()) {
                            Ok(())
                        } else {
                            Err(RuntimeError::new(
                                format!("Поле '{}' закрыто (класс '{}')", field, owner),
                                RuntimeErrorKind::Other,
                            ))
                        }
                    }
                    Visibility::Protected => {
                        let ok = env
                            .current_defining_class()
                            .is_some_and(|c| c == owner || Self::is_subclass_of(&c, &owner, env));
                        if ok {
                            Ok(())
                        } else {
                            Err(RuntimeError::new(
                                format!("Поле '{}' защищено (класс '{}')", field, owner),
                                RuntimeErrorKind::Other,
                            ))
                        }
                    }
                };
            }
            match &class.parent {
                Some(p) => current = p.to_string(),
                None => return Ok(()),
            }
        }
    }
}
