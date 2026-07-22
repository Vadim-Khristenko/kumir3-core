//! [KITE-0015] Доступ к членам модуля через `::`.
//!
//! Парсер превращает `Модуль::член` в [`Expr::ModuleAccess`], а `Модуль::член(…)`
//! — в `Expr::Call("Модуль::член", args)`. Импорт и объявление `модуль` регистрируют
//! алгоритмы под ТОЧЕЧНЫМ полным именем (`"Модуль.член"`), поэтому обе формы
//! приводятся здесь к одному и тому же разрешению имени.

use super::ExprEvaluator;

use shared::types::{Expr, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    /// Полное точечное имя члена модуля: `A::B::c` → `A.B.c`.
    fn dotted_member(module: &str, member: &str) -> String {
        format!("{}.{}", module.replace("::", "."), member)
    }

    /// Ошибка «член модуля не найден» — всегда называет и модуль, и член.
    fn unknown_member(module: &str, member: &str) -> RuntimeError {
        RuntimeError::new(
            format!("Член '{}' не найден в модуле '{}'", member, module),
            RuntimeErrorKind::UndefinedAlgorithm,
        )
    }

    /// [KITE-0015] Вычисляет `Модуль::член` в позиции ЗНАЧЕНИЯ (без вызова).
    ///
    /// Разрешение (в порядке приоритета):
    /// 1. вариант перечисления `Перечисление::Вариант` → [`Value::Enum`];
    /// 2. переменная модуля, зарегистрированная под полным именем `Модуль.член`;
    /// 3. алгоритм `Модуль.член` → ясная ошибка с подсказкой вызвать его;
    /// 4. иначе — ясная ошибка, называющая модуль и член.
    pub(crate) fn eval_module_access(
        module: &str,
        member: &str,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // 1) Перечисление: `Цвет::Красный`.
        if env.is_valid_enum_variant(module, member) {
            return Self::eval_enum_construct(module, member, None, env);
        }

        // 2) Переменная модуля под полным именем.
        let dotted = Self::dotted_member(module, member);
        if let Ok(value) = env.get_variable(&dotted) {
            return Ok(value.clone());
        }

        // 3) Алгоритм модуля: голая ссылка значением не является.
        if env.has_algorithm(&dotted) || env.get_overloaded_algorithm(&dotted).is_some() {
            return Err(RuntimeError::new(
                format!(
                    "'{module}::{member}' — алгоритм модуля, а не значение; \
                     используйте вызов '{module}::{member}(...)'"
                ),
                RuntimeErrorKind::TypeMismatch,
            ));
        }

        // 4) Ничего не нашли.
        Err(Self::unknown_member(module, member))
    }

    /// [KITE-0015] Вычисляет вызов `Модуль::член(args)`.
    ///
    /// Разрешается ровно так же, как точечная форма `Модуль.член(args)`:
    /// сначала алгоритм под полным точечным именем, затем вариант перечисления
    /// с данными, затем функция загруженной библиотеки. Иначе — ясная ошибка,
    /// называющая модуль и член.
    pub(crate) fn eval_qualified_call(
        module: &str,
        member: &str,
        full_name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Алгоритм, объявленный буквально с `::` в имени (на всякий случай).
        if env.has_algorithm(full_name) || env.get_overloaded_algorithm(full_name).is_some() {
            return Self::eval_plain_call(full_name, args, env);
        }

        // Основной путь: то же имя, что и у точечной формы.
        let dotted = Self::dotted_member(module, member);
        if env.has_algorithm(&dotted) || env.get_overloaded_algorithm(&dotted).is_some() {
            return Self::eval_plain_call(&dotted, args, env);
        }

        // Вариант перечисления с данными: `Фигура::Круг(5)`.
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

        // Функция загруженной библиотеки: `Сеть::получить(...)`.
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
