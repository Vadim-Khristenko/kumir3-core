use super::ExprEvaluator;

use shared::types::{Expr, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    // =============================================================================
    //         SECTION: OOP METHOD CALLS
    // =============================================================================

    pub(crate) fn eval_method_call(
        object: &Expr,
        method: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // [KITE 11] Super method call: `super.method(...)` — dispatch starts
        // from the parent class where the current method is defined.
        if matches!(object, Expr::SuperRef) {
            return Self::eval_super_method_call(method, args, env);
        }

        // [KITE-0002] Early return operator `?`.
        //
        // Parser desugars `expr?` to `expr.__propagate__()`. Semantics:
        //   * Ok(v) / Some(v) / non-null-value → unwrap to `v`;
        //   * Err(e) / None / null → propagate
        //     (early return of enclosing algorithm with this value);
        //   * other → clear error.
        if method == "__propagate__" && args.is_empty() {
            let value = Self::evaluate(object, env)?;
            return Self::eval_propagate(value);
        }

        // Indexing value that is not a variable:
        // `parse_iso(d)["year"]`, `[1, 2, 3][0]`. Parser reduces such syntax
        // to `__index__` call because the array access node stores a variable name,
        // not an expression. Semantics are exactly the same as `a[i]`.
        if method == "__index__" {
            let value = Self::evaluate(object, env)?;
            return Self::index_value(value, args, env);
        }

        // First check if object is a library or module identifier.
        // Example: Net.http_get("url") or MyLibrary.square(5)
        if let Expr::Variable(lib_name) = object {
            // Check if algorithm with full name Module.function exists.
            let full_name = format!("{}.{}", lib_name, method);
            if env.has_algorithm(&full_name) {
                // Evaluate arguments.
                let evaluated_args: Vec<Value> = args
                    .iter()
                    .map(|arg| Self::evaluate(arg, env))
                    .collect::<RuntimeResult<Vec<_>>>()?;

                // Call algorithm.
                return Self::call_user_algorithm(&full_name, &evaluated_args, env);
            }

            // Check loaded library.
            if env.is_loaded_library(lib_name) {
                // Evaluate arguments.
                let evaluated_args: Vec<Value> = args
                    .iter()
                    .map(|arg| Self::evaluate(arg, env))
                    .collect::<RuntimeResult<Vec<_>>>()?;

                // Call library function.
                return env
                    .call_library_qualified(lib_name, method, &evaluated_args)?
                    .ok_or_else(|| {
                        RuntimeError::new(
                            format!("Функция '{}.{}' не найдена", lib_name, method),
                            RuntimeErrorKind::UndefinedAlgorithm,
                        )
                    });
            }
        }

        let obj = Self::evaluate(object, env)?;

        // Built-in methods for standard types.
        match &obj {
            Value::String(s) => {
                return Self::call_string_method(s, method, args, env);
            }
            Value::Array(arr) => {
                return Self::call_array_method(arr, method, args, env);
            }
            Value::Object { type_id, fields } => {
                // [KITE 11] Determine object's class: first by type_id, then by
                // most specific field set match.
                let class_name = Self::find_class_name_by_type_id(type_id, env)
                    .or_else(|| Self::find_class_by_fields(fields, env));

                // [KITE 11] Look up method in inheritance hierarchy (class → parents).
                if let Some(class_name) = class_name {
                    // 1) Own and inherited class methods.
                    if let Some((owner, method_def)) =
                        Self::find_method_in_hierarchy(&class_name, method, env)
                    {
                        return Self::call_class_method(
                            &obj,
                            owner.name.as_ref(),
                            &method_def,
                            args,
                            env,
                        );
                    }
                    // 2) [KITE 11, step 4] Methods from trait impl blocks (by type and parents).
                    if let Some((owner_name, method_def)) =
                        Self::find_impl_method_in_hierarchy(&class_name, method, env)
                    {
                        return Self::call_class_method(&obj, &owner_name, &method_def, args, env);
                    }
                }
            }
            _ => {}
        }

        Err(RuntimeError::new(
            format!("Метод '{}' не найден", method),
            RuntimeErrorKind::Other,
        ))
    }

    /// Calls class/impl-block method (owner passed by name—KITE 11).
    fn call_class_method(
        this: &Value,
        class_name: &str,
        method: &shared::types::Method,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Check argument count.
        if args.len() != method.algorithm.params.len() {
            return Err(RuntimeError::argument_count(
                &format!("{}.{}", class_name, method.algorithm.name),
                method.algorithm.params.len(),
                args.len(),
            ));
        }

        // If method is abstract—error.
        if method.is_abstract {
            return Err(RuntimeError::new(
                format!(
                    "Метод '{}.{}' абстрактный и не имеет реализации",
                    class_name, method.algorithm.name
                ),
                RuntimeErrorKind::Other,
            ));
        }

        // Get method body.
        let body = method.algorithm.body.as_ref().ok_or_else(|| {
            RuntimeError::new(
                format!(
                    "Метод '{}.{}' не имеет тела",
                    class_name, method.algorithm.name
                ),
                RuntimeErrorKind::Other,
            )
        })?;

        // [KITE 4] Arguments evaluated in caller frame, before method frame created.
        let mut bound: Vec<(String, Value)> = Vec::with_capacity(method.algorithm.params.len());
        for (i, param) in method.algorithm.params.iter().enumerate() {
            let value = Self::evaluate(&args[i], env)?;
            bound.push((param.name.to_string(), value));
        }

        // Create method call frame.
        env.push_method_frame(
            format!("{}.{}", class_name, method.algorithm.name),
            this.clone(),
        )?;
        // [KITE 11] Remember the method's owning class—for super resolution.
        env.set_current_defining_class(class_name);
        for (name, value) in bound {
            env.define_local(name, value);
        }

        // Execute method body.
        let result = super::super::executor::Executor::execute_stmts(body, env);

        // Get return value.
        let return_value = match result {
            Ok(crate::interpreter::ControlFlow::Return(v)) => v,
            Ok(_) => Some(env.get_result_value().cloned().unwrap_or(Value::Null)),
            // [KITE-0002] `?` operator signal: early return of this value.
            Err(e) if e.is_propagation() => {
                Some(*e.propagate.expect("propagation carries a value"))
            }
            Err(e) => {
                env.pop_frame();
                return Err(e);
            }
        };

        env.pop_frame();
        Ok(return_value.unwrap_or(Value::Null))
    }

    /// [KITE-0002] Implements `?` operator semantics (early error return).
    ///
    /// * `Result::Ok(v)` / `Option::Some(v)` → unwrap to `v`.
    /// * `Result::Err(e)` → propagate `Err(e)` (early return).
    /// * `Option::None` / `Null` → propagate `Nothing` (early return).
    /// * `Value::Error{..}` → propagate the error value itself.
    /// * any other value—not propagatable, not unwrappable →
    ///   clear runtime error (not panic).
    pub(crate) fn eval_propagate(value: Value) -> RuntimeResult<Value> {
        match value {
            Value::Result(res) => match *res {
                Ok(v) => Ok(v),
                Err(e) => Err(RuntimeError::propagation(e)),
            },
            Value::Option(opt) => match *opt {
                Some(v) => Ok(v),
                None => Err(RuntimeError::propagation(Value::Null)),
            },
            Value::Null => Err(RuntimeError::propagation(Value::Null)),
            err @ Value::Error { .. } => Err(RuntimeError::propagation(err)),
            other => Err(RuntimeError::new(
                format!(
                    "Оператор '?' применим только к результату (рез), \
                     необязательному значению или ошибке, а не к {}",
                    other.type_name_ru()
                ),
                RuntimeErrorKind::TypeMismatch,
            )),
        }
    }

    /// Finds class name by TypeId.
    pub(crate) fn find_class_name_by_type_id(
        type_id: &shared::types::TypeId,
        env: &Environment,
    ) -> Option<String> {
        // [KITE 11, step 1] Object identity via TypeRegistry.
        env.class_name_by_type_id(*type_id)
    }

    /// [KITE 11] Finds object's class by most specific field set match:
    /// prefers the class with most fields that all exist in the object.
    pub(crate) fn find_class_by_fields(
        fields: &std::collections::BTreeMap<String, Value>,
        env: &Environment,
    ) -> Option<String> {
        let obj_fields: std::collections::HashSet<&str> =
            fields.keys().map(|k| k.as_str()).collect();
        let mut best: Option<(String, usize)> = None;
        for (name, class) in env.all_classes() {
            let class_fields: std::collections::HashSet<&str> =
                class.fields.iter().map(|f| f.name.as_ref()).collect();
            if class_fields.iter().all(|f| obj_fields.contains(f)) {
                let count = class_fields.len();
                if best.as_ref().is_none_or(|(_, c)| count > *c) {
                    best = Some((name.clone(), count));
                }
            }
        }
        best.map(|(n, _)| n)
    }

    /// [KITE 11] Method resolution in inheritance hierarchy: from class up through parents.
    /// First found method wins (vtable-like).
    fn find_method_in_hierarchy(
        start_class: &str,
        method: &str,
        env: &Environment,
    ) -> Option<(shared::types::ClassDef, shared::types::Method)> {
        let mut current = start_class.to_string();
        loop {
            let class = env.get_class(&current).ok()?.clone();
            if let Some(m) = class
                .methods
                .iter()
                .find(|m| m.algorithm.name.as_ref() == method)
            {
                return Some((class.clone(), m.clone()));
            }
            current = class.parent.as_ref()?.to_string();
        }
    }

    /// Is `sub` the same class as `sup`, or its descendant?
    pub(crate) fn is_subclass_of(sub: &str, sup: &str, env: &Environment) -> bool {
        let mut current = sub.to_string();
        loop {
            if current == sup {
                return true;
            }
            match env
                .get_class(&current)
                .ok()
                .and_then(|c| c.parent.as_ref().map(|p| p.to_string()))
            {
                Some(p) => current = p,
                None => return false,
            }
        }
    }

    /// [KITE 11, step 4] Look up method in trait impl blocks by type and its parents.
    /// Returns (owner type name, method).
    fn find_impl_method_in_hierarchy(
        start_class: &str,
        method: &str,
        env: &Environment,
    ) -> Option<(String, shared::types::Method)> {
        let mut current = start_class.to_string();
        loop {
            if let Some(m) = env.find_impl_method(&current, method) {
                return Some((current, m));
            }
            let class = env.get_class(&current).ok()?.clone();
            current = class.parent.as_ref()?.to_string();
        }
    }

    /// [KITE 11] Super method call: `super.method(...)`.
    /// Resolution starts from the parent class where the current method is defined,
    /// so subclass overrides are intentionally skipped.
    fn eval_super_method_call(
        method: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let this = env.get_this().cloned().ok_or_else(|| {
            RuntimeError::new(
                "'предок' можно использовать только внутри метода",
                RuntimeErrorKind::Other,
            )
        })?;
        let defining = env.current_defining_class().ok_or_else(|| {
            RuntimeError::new(
                "Не удалось определить текущий класс для 'предок'",
                RuntimeErrorKind::Other,
            )
        })?;
        let parent = env
            .get_class(&defining)
            .ok()
            .and_then(|c| c.parent.as_ref().map(|p| p.to_string()))
            .ok_or_else(|| {
                RuntimeError::new(
                    format!("У класса '{}' нет предка", defining),
                    RuntimeErrorKind::Other,
                )
            })?;
        let (owner, method_def) =
            Self::find_method_in_hierarchy(&parent, method, env).ok_or_else(|| {
                RuntimeError::new(
                    format!("Метод '{}' не найден у предка '{}'", method, parent),
                    RuntimeErrorKind::Other,
                )
            })?;
        Self::call_class_method(&this, owner.name.as_ref(), &method_def, args, env)
    }
}
