use super::ExprEvaluator;

use shared::types::{Expr, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    /// Full dotted name of module member: `A::B::c` → `A.B.c`.
    fn dotted_member(module: &str, member: &str) -> String {
        format!("{}.{}", module.replace("::", "."), member)
    }

    /// "Member not found in module" error—always names both module and member.
    fn unknown_member(module: &str, member: &str) -> RuntimeError {
        RuntimeError::new(
            format!("Член '{}' не найден в модуле '{}'", member, module),
            RuntimeErrorKind::UndefinedAlgorithm,
        )
    }

    /// [KITE-0015] Evaluates `Module::member` in VALUE position (no call).
    ///
    /// Resolution (in priority order):
    /// 1. enumeration variant `Enum::Variant` → [`Value::Enum`];
    /// 2. module variable registered under full name `Module.member`;
    /// 3. algorithm `Module.member` → clear error with suggestion to call it;
    /// 4. otherwise—clear error naming both module and member.
    pub(crate) fn eval_module_access(
        module: &str,
        member: &str,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // 1) Enumeration: `Color::Red`.
        if env.is_valid_enum_variant(module, member) {
            return Self::eval_enum_construct(module, member, None, env);
        }

        // 2) Module variable under full name.
        let dotted = Self::dotted_member(module, member);
        if let Ok(value) = env.get_variable(&dotted) {
            return Ok(value.clone());
        }

        // 3) Module algorithm: bare reference is not a value.
        if env.has_algorithm(&dotted) || env.get_overloaded_algorithm(&dotted).is_some() {
            return Err(RuntimeError::new(
                format!(
                    "'{module}::{member}' — алгоритм модуля, а не значение; \
                     используйте вызов '{module}::{member}(...)'"
                ),
                RuntimeErrorKind::TypeMismatch,
            ));
        }

        // 4) Nothing found.
        Err(Self::unknown_member(module, member))
    }

    /// [KITE-0015] Evaluates `Module::member(args)` call.
    ///
    /// Resolved exactly like dotted form `Module.member(args)`:
    /// first algorithm under full dotted name, then enumeration variant
    /// with data, then loaded library function. Otherwise—clear error
    /// naming both module and member.
    pub(crate) fn eval_qualified_call(
        module: &str,
        member: &str,
        full_name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Algorithm literally declared with `::` in name (just in case).
        if env.has_algorithm(full_name) || env.get_overloaded_algorithm(full_name).is_some() {
            return Self::eval_plain_call(full_name, args, env);
        }

        // Main path: same name as dotted form.
        let dotted = Self::dotted_member(module, member);
        if env.has_algorithm(&dotted) || env.get_overloaded_algorithm(&dotted).is_some() {
            return Self::eval_plain_call(&dotted, args, env);
        }

        // Enumeration variant with data: `Shape::Circle(5)`.
        if env.is_valid_enum_variant(module, member) {
            return match args {
                [] => Self::eval_enum_construct(module, member, None, env),
                [single] => Self::eval_enum_construct(module, member, Some(single), env),
                _ => Err(RuntimeError::new(
                    format!(
                        "Вариант '{}::{}' принимает не более одного значения, передано {}",
                        module,
                        member,
                        args.len()
                    ),
                    RuntimeErrorKind::Other,
                )),
            };
        }

        // Loaded library function: `Net::get(...)`.
        if env.is_loaded_library(module) {
            let evaluated: Vec<Value> = args
                .iter()
                .map(|arg| Self::evaluate(arg, env))
                .collect::<RuntimeResult<Vec<_>>>()?;
            if let Some(value) = env.call_library_qualified(module, member, &evaluated)? {
                return Ok(value);
            }
        }

        Err(Self::unknown_member(module, member))
    }
}
