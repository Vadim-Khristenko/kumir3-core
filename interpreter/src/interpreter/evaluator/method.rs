use super::ExprEvaluator;

use shared::types::{Expr, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl ExprEvaluator {
    // =========================================================================
    //                    ООП: ВЫЗОВ МЕТОДОВ
    // =========================================================================

    pub(crate) fn eval_method_call(
        object: &Expr,
        method: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // [KITE 11] Вызов метода предка: `предок.метод(...)` — диспетчеризация
        // начинается с родителя класса, где определён текущий метод.
        if matches!(object, Expr::SuperRef) {
            return Self::eval_super_method_call(method, args, env);
        }

        // Сначала проверяем, является ли object идентификатором библиотеки или модуля
        // Например: Сеть.http_получить("url") или МояБиблиотека.квадрат(5)
        if let Expr::Variable(lib_name) = object {
            // Проверяем, есть ли алгоритм с полным именем Модуль.функция
            let full_name = format!("{}.{}", lib_name, method);
            if env.has_algorithm(&full_name) {
                // Вычисляем аргументы
                let evaluated_args: Vec<Value> = args
                    .iter()
                    .map(|arg| Self::evaluate(arg, env))
                    .collect::<RuntimeResult<Vec<_>>>()?;

                // Вызываем алгоритм
                return Self::call_user_algorithm(&full_name, &evaluated_args, env);
            }

            // Проверяем загруженную библиотеку
            if env.is_loaded_library(lib_name) {
                // Вычисляем аргументы
                let evaluated_args: Vec<Value> = args
                    .iter()
                    .map(|arg| Self::evaluate(arg, env))
                    .collect::<RuntimeResult<Vec<_>>>()?;

                // Вызываем функцию библиотеки
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

        // Встроенные методы для стандартных типов
        match &obj {
            Value::String(s) => {
                return Self::call_string_method(s, method, args, env);
            }
            Value::Array(arr) => {
                return Self::call_array_method(arr, method, args, env);
            }
            Value::Object { type_id, fields } => {
                // [KITE 11] Определяем класс объекта: сначала по type_id, затем по
                // наиболее производному совпадению набора полей.
                let class_name = Self::find_class_name_by_type_id(type_id, env)
                    .or_else(|| Self::find_class_by_fields(fields, env));

                // [KITE 11] Ищем метод по иерархии наследования (класс → предки).
                if let Some(class_name) = class_name {
                    // 1) Собственные и унаследованные методы класса.
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
                    // 2) [KITE 11, шаг 4] Методы из impl-блоков типажей (по типу и предкам).
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

    /// Вызывает метод класса/impl-блока (владелец передаётся по имени — KITE 11).
    fn call_class_method(
        this: &Value,
        class_name: &str,
        method: &shared::types::Method,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Проверяем количество аргументов
        if args.len() != method.algorithm.params.len() {
            return Err(RuntimeError::argument_count(
                &format!("{}.{}", class_name, method.algorithm.name),
                method.algorithm.params.len(),
                args.len(),
            ));
        }

        // Если метод абстрактный - ошибка
        if method.is_abstract {
            return Err(RuntimeError::new(
                format!(
                    "Метод '{}.{}' абстрактный и не имеет реализации",
                    class_name, method.algorithm.name
                ),
                RuntimeErrorKind::Other,
            ));
        }

        // Получаем тело метода
        let body = method.algorithm.body.as_ref().ok_or_else(|| {
            RuntimeError::new(
                format!(
                    "Метод '{}.{}' не имеет тела",
                    class_name, method.algorithm.name
                ),
                RuntimeErrorKind::Other,
            )
        })?;

        // [KITE 4] Аргументы вычисляются в кадре вызывающего, до создания кадра метода.
        let mut bound: Vec<(String, Value)> = Vec::with_capacity(method.algorithm.params.len());
        for (i, param) in method.algorithm.params.iter().enumerate() {
            let value = Self::evaluate(&args[i], env)?;
            bound.push((param.name.to_string(), value));
        }

        // Создаём кадр вызова метода
        env.push_method_frame(
            format!("{}.{}", class_name, method.algorithm.name),
            this.clone(),
        )?;
        // [KITE 11] Запоминаем класс-владельца метода — для разрешения `предок`.
        env.set_current_defining_class(class_name);
        for (name, value) in bound {
            env.define_local(name, value);
        }

        // Выполняем тело метода
        let result = super::super::executor::Executor::execute_stmts(body, env);

        // Получаем возвращаемое значение
        let return_value = match result {
            Ok(crate::interpreter::ControlFlow::Return(v)) => v,
            Ok(_) => Some(env.get_result_value().cloned().unwrap_or(Value::Null)),
            Err(e) => {
                env.pop_frame();
                return Err(e);
            }
        };

        env.pop_frame();
        Ok(return_value.unwrap_or(Value::Null))
    }

    /// Находит имя класса по TypeId.
    pub(crate) fn find_class_name_by_type_id(
        type_id: &shared::types::TypeId,
        env: &Environment,
    ) -> Option<String> {
        // [KITE 11, шаг 1] Идентичность объекта через TypeRegistry.
        env.class_name_by_type_id(*type_id)
    }

    /// [KITE 11] Находит класс объекта по наиболее производному совпадению полей:
    /// предпочитается класс с наибольшим числом полей, все из которых есть у объекта.
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

    /// [KITE 11] Разрешение метода по иерархии наследования: от класса вверх по предкам.
    /// Первый найденный метод выигрывает (vtable-подобно).
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
            match &class.parent {
                Some(p) => current = p.to_string(),
                None => return None,
            }
        }
    }

    /// Является ли `sub` тем же классом, что `sup`, или его потомком.
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

    /// [KITE 11, шаг 4] Ищет метод в impl-блоках типажей по типу и его предкам.
    /// Возвращает (имя типа-владельца, метод).
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
            match &class.parent {
                Some(p) => current = p.to_string(),
                None => return None,
            }
        }
    }

    /// [KITE 11] Вызов метода предка: `предок.метод(...)`.
    /// Разрешение начинается с родителя класса, в котором определён текущий метод,
    /// поэтому переопределения подкласса намеренно обходятся.
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
