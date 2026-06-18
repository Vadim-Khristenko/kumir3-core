//! Модуль вычисления выражений для интерпретатора Кумир 3
//!
//! Реализует вычисление всех типов выражений: литералы, переменные,
//! бинарные и унарные операции, вызовы алгоритмов, доступ к массивам,
//! ООП (поля, методы, создание объектов), лямбды и т.д.

use std::collections::BTreeMap;

use shared::math::MathOperators;
use shared::types::{Algorithm, Expr, Number, Pattern, Token, TypeKind, Value};

use super::builtins::Builtins;
use super::environment::Environment;
use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

/// Вычислитель выражений.
pub struct ExprEvaluator;

impl ExprEvaluator {
    /// Вычисляет выражение.
    pub fn evaluate(expr: &Expr, env: &mut Environment) -> RuntimeResult<Value> {
        match expr {
            // Литералы
            Expr::Literal(value) => Ok(value.clone()),

            // Переменные
            Expr::Variable(name) => env.get_variable(name).cloned(),

            // Бинарные операции
            Expr::BinaryOp(left, op, right) => Self::eval_binary_op(left, op, right, env),

            // Унарные операции
            Expr::UnaryOp(op, operand) => Self::eval_unary_op(op, operand, env),

            // Вызов алгоритма
            Expr::Call(name, args) => Self::eval_call(name, args, env),

            // Доступ к элементу массива
            Expr::ArrayAccess(name, indices) => Self::eval_array_access(name, indices, env),

            // ООП: доступ к полю
            Expr::FieldAccess(object, field) => Self::eval_field_access(object, field, env),

            // ООП: вызов метода
            Expr::MethodCall {
                object,
                method,
                args,
            } => Self::eval_method_call(object, method, args, env),

            // ООП: создание экземпляра
            Expr::NewInstance { class_name, args } => {
                Self::eval_new_instance(class_name, args, env)
            }

            // [KITE 2] Диапазон: 1..10 / 1..=10
            Expr::Range {
                start,
                end,
                inclusive,
            } => Self::eval_range(start.as_deref(), end.as_deref(), *inclusive, env),

            // Ссылка на себя (this)
            Expr::SelfRef => env.get_this().cloned().ok_or_else(|| {
                RuntimeError::new(
                    "Ключевое слово 'это' можно использовать только внутри метода",
                    RuntimeErrorKind::Other,
                )
            }),

            // Ссылка на предка (super)
            Expr::SuperRef => Err(RuntimeError::not_implemented("super (предок)")),

            // Приведение типа
            Expr::Cast { expr, target_type } => Self::eval_cast(expr, target_type, env),

            // Проверка типа
            Expr::TypeCheck { expr, check_type } => Self::eval_type_check(expr, check_type, env),

            // Доступ к модулю
            Expr::ModuleAccess(module, name) => Err(RuntimeError::not_implemented(&format!(
                "доступ к модулю {}::{}",
                module, name
            ))),

            // Создание значения перечисления
            Expr::EnumConstruct {
                enum_name,
                variant,
                data,
            } => Self::eval_enum_construct(enum_name, variant, data.as_deref(), env),

            // Получение ссылки
            Expr::Ref(inner) => {
                let value = Self::evaluate(inner, env)?;
                Ok(Value::Pointer(Box::new(value)))
            }

            // Разыменование
            Expr::Deref(inner) => {
                let value = Self::evaluate(inner, env)?;
                match value {
                    Value::Pointer(inner) => Ok(*inner),
                    _ => Err(RuntimeError::type_mismatch("указатель", "не указатель")),
                }
            }

            // Создание указателя
            Expr::New(inner) => {
                let value = Self::evaluate(inner, env)?;
                Ok(Value::Pointer(Box::new(value)))
            }

            // Лямбда-выражение
            Expr::Lambda {
                params, body: _, ..
            } => {
                // Создаём замыкание (пока упрощённая реализация)
                Ok(Value::String(format!("lambda({:?})", params)))
            }

            // Pipe-выражение: x |> f
            Expr::Pipe(value, func) => Self::eval_pipe(value, func, env),

            // Условное выражение
            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond = Self::evaluate(condition, env)?;
                if Self::is_truthy(&cond) {
                    Self::evaluate(then_expr, env)
                } else {
                    Self::evaluate(else_expr, env)
                }
            }

            // Match-выражение
            Expr::MatchExpr { expr, arms } => Self::eval_match_expr(expr, arms, env),

            // Rust-вставка
            Expr::RustExpr(_code) => Err(RuntimeError::not_implemented("Rust-вставки")),

            // Пусто
            Expr::None => Ok(Value::Null),

            // Не реализовано
            Expr::NotImplemented(msg) => {
                let error_msg = msg.as_deref().unwrap_or("не указано");
                Err(RuntimeError::not_implemented(error_msg))
            }

            // Все остальные выражения (не реализованы)
            _ => Err(RuntimeError::not_implemented("данное выражение")),
        }
    }

    // =========================================================================
    //                    БИНАРНЫЕ ОПЕРАЦИИ
    // =========================================================================

    fn eval_binary_op(
        left: &Expr,
        op: &Token,
        right: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Ленивые вычисления для логических операций
        match op {
            Token::And => {
                let left_val = Self::evaluate(left, env)?;
                if !Self::is_truthy(&left_val) {
                    return Ok(Value::Boolean(false));
                }
                let right_val = Self::evaluate(right, env)?;
                return Ok(Value::Boolean(Self::is_truthy(&right_val)));
            }
            Token::Or => {
                let left_val = Self::evaluate(left, env)?;
                if Self::is_truthy(&left_val) {
                    return Ok(Value::Boolean(true));
                }
                let right_val = Self::evaluate(right, env)?;
                return Ok(Value::Boolean(Self::is_truthy(&right_val)));
            }
            _ => {}
        }

        // Вычисляем оба операнда
        let left_val = Self::evaluate(left, env)?;
        let right_val = Self::evaluate(right, env)?;

        match op {
            // Арифметические операции
            Token::Plus => MathOperators::add(left_val, right_val, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),
            Token::Minus => MathOperators::sub(left_val, right_val, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),
            Token::Star => MathOperators::mul(left_val, right_val, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),
            Token::Slash => MathOperators::div(left_val, right_val, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),
            Token::Percent => MathOperators::modulus(left_val, right_val, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),
            Token::Power => MathOperators::pow(left_val, right_val, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),

            // Сравнения
            Token::Equal => Ok(Value::Boolean(Self::values_equal(&left_val, &right_val))),
            Token::NotEqual => Ok(Value::Boolean(!Self::values_equal(&left_val, &right_val))),
            Token::Less => Self::compare_values(&left_val, &right_val, |o| o.is_lt()),
            Token::Greater => Self::compare_values(&left_val, &right_val, |o| o.is_gt()),
            Token::LessEqual => Self::compare_values(&left_val, &right_val, |o| o.is_le()),
            Token::GreaterEqual => Self::compare_values(&left_val, &right_val, |o| o.is_ge()),

            _ => Err(RuntimeError::new(
                format!("Неизвестный бинарный оператор: {:?}", op),
                RuntimeErrorKind::Other,
            )),
        }
    }

    // =========================================================================
    //                    УНАРНЫЕ ОПЕРАЦИИ
    // =========================================================================

    fn eval_unary_op(op: &Token, operand: &Expr, env: &mut Environment) -> RuntimeResult<Value> {
        let value = Self::evaluate(operand, env)?;

        match op {
            Token::Minus => match value {
                Value::Number(n) => {
                    let negated = Self::negate_number(n)?;
                    Ok(Value::Number(negated))
                }
                _ => Err(RuntimeError::type_mismatch("число", "не число")),
            },
            Token::Not => Ok(Value::Boolean(!Self::is_truthy(&value))),
            _ => Err(RuntimeError::new(
                format!("Неизвестный унарный оператор: {:?}", op),
                RuntimeErrorKind::Other,
            )),
        }
    }

    fn negate_number(n: Number) -> RuntimeResult<Number> {
        Ok(match n {
            Number::I8(v) => Number::I8(-v),
            Number::I16(v) => Number::I16(-v),
            Number::I32(v) => Number::I32(-v),
            Number::I64(v) => Number::I64(-v),
            Number::I128(v) => Number::I128(-v),
            Number::F32(v) => Number::F32(-v),
            Number::F64(v) => Number::F64(-v),
            Number::F128(v) => Number::F128(-v),
            // Беззнаковые нельзя отрицать
            _ => {
                return Err(RuntimeError::new(
                    "Нельзя применить унарный минус к беззнаковому числу",
                    RuntimeErrorKind::TypeMismatch,
                ));
            }
        })
    }

    // =========================================================================
    //                    ВЫЗОВ АЛГОРИТМОВ
    // =========================================================================

    pub fn eval_call(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        // Сначала пробуем встроенные функции
        if let Some(result) = Builtins::try_call(name, args, env)? {
            return Ok(result);
        }

        // Проверяем перегруженные алгоритмы
        if let Some(overloaded) = env.get_overloaded_algorithm(name).cloned() {
            // Выбираем подходящую перегрузку (упрощённо - по количеству аргументов)
            for alg in &overloaded.overloads {
                if alg.params.len() == args.len() {
                    return Self::call_algorithm(alg, args, env);
                }
            }
            return Err(RuntimeError::argument_count(
                name,
                overloaded.overloads[0].params.len(),
                args.len(),
            ));
        }

        // Получаем алгоритм
        let algorithm = env.get_algorithm(name)?.clone();

        // Проверяем количество аргументов
        let required_params = algorithm
            .params
            .iter()
            .filter(|p| p.default.is_none())
            .count();

        if args.len() < required_params || args.len() > algorithm.params.len() {
            return Err(RuntimeError::argument_count(
                name,
                algorithm.params.len(),
                args.len(),
            ));
        }

        Self::call_algorithm(&algorithm, args, env)
    }

    fn call_algorithm(
        algorithm: &Algorithm,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // [KITE 4] Аргументы вычисляются в кадре ВЫЗЫВАЮЩЕГО, до создания кадра
        // callee (при лексической видимости callee не видит локали вызывающего).
        let mut bound: Vec<(String, Value)> = Vec::with_capacity(algorithm.params.len());
        for (i, param) in algorithm.params.iter().enumerate() {
            let value = if i < args.len() {
                Self::evaluate(&args[i], env)?
            } else if let Some(default) = &param.default {
                Self::evaluate(default, env)?
            } else {
                return Err(RuntimeError::argument_count(
                    &algorithm.name,
                    algorithm.params.len(),
                    args.len(),
                ));
            };
            bound.push((param.name.to_string(), value));
        }

        // Создаём новый кадр и привязываем параметры.
        env.push_frame(algorithm.name.as_ref())?;
        for (name, value) in bound {
            env.define_local(name, value);
        }

        // Выполняем тело алгоритма
        let result =
            super::executor::Executor::execute_stmts(algorithm.body.as_deref().unwrap_or(&[]), env);

        // Получаем возвращаемое значение
        let return_value = env.get_result_value().cloned();

        // Удаляем кадр
        env.pop_frame();

        // Обрабатываем результат
        match result {
            Ok(super::error::ControlFlow::Return(value)) => Ok(value.unwrap_or(Value::Null)),
            Ok(_) => Ok(return_value.unwrap_or(Value::Null)),
            Err(e) => Err(e),
        }
    }

    /// Вызывает пользовательский алгоритм с уже вычисленными аргументами.
    fn call_user_algorithm(
        name: &str,
        args: &[Value],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Получаем алгоритм
        let algorithm = env.get_algorithm(name)?.clone();

        // Проверяем количество аргументов
        let required_params = algorithm
            .params
            .iter()
            .filter(|p| p.default.is_none())
            .count();

        if args.len() < required_params || args.len() > algorithm.params.len() {
            return Err(RuntimeError::argument_count(
                name,
                algorithm.params.len(),
                args.len(),
            ));
        }

        // Создаём новый кадр
        env.push_frame(algorithm.name.as_ref())?;

        // Привязываем параметры
        for (i, param) in algorithm.params.iter().enumerate() {
            let value = if i < args.len() {
                args[i].clone()
            } else if let Some(default) = &param.default {
                Self::evaluate(default, env)?
            } else {
                return Err(RuntimeError::argument_count(
                    &algorithm.name,
                    algorithm.params.len(),
                    args.len(),
                ));
            };
            env.define_local(param.name.to_string(), value);
        }

        // Выполняем тело алгоритма
        let result =
            super::executor::Executor::execute_stmts(algorithm.body.as_deref().unwrap_or(&[]), env);

        // Получаем возвращаемое значение
        let return_value = env.get_result_value().cloned();

        // Удаляем кадр
        env.pop_frame();

        // Обрабатываем результат
        match result {
            Ok(super::error::ControlFlow::Return(value)) => Ok(value.unwrap_or(Value::Null)),
            Ok(_) => Ok(return_value.unwrap_or(Value::Null)),
            Err(e) => Err(e),
        }
    }

    // =========================================================================
    //                    ДОСТУП К МАССИВАМ
    // =========================================================================

    fn eval_array_access(
        name: &str,
        indices: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let array = env.get_variable(name)?.clone();

        match array {
            Value::Array(elements) => {
                if indices.len() != 1 {
                    return Err(RuntimeError::not_implemented("многомерные массивы"));
                }

                let index = Self::evaluate(&indices[0], env)?;
                let idx = Self::to_index(&index)?;

                if idx < 0 || idx as usize >= elements.len() {
                    return Err(RuntimeError::index_out_of_bounds(idx, elements.len()));
                }

                Ok(elements[idx as usize].clone())
            }
            Value::String(s) => {
                if indices.len() != 1 {
                    return Err(RuntimeError::new(
                        "Строка поддерживает только один индекс",
                        RuntimeErrorKind::Other,
                    ));
                }

                let index = Self::evaluate(&indices[0], env)?;
                let idx = Self::to_index(&index)?;

                let chars: Vec<char> = s.chars().collect();
                if idx < 1 || idx as usize > chars.len() {
                    return Err(RuntimeError::index_out_of_bounds(idx, chars.len()));
                }

                Ok(Value::Char(chars[(idx - 1) as usize]))
            }
            Value::Map(map) => {
                if indices.len() != 1 {
                    return Err(RuntimeError::new(
                        "Словарь поддерживает только один ключ",
                        RuntimeErrorKind::Other,
                    ));
                }

                let key = Self::evaluate(&indices[0], env)?;
                map.get(&key).cloned().ok_or_else(|| {
                    RuntimeError::new(
                        format!("Ключ не найден в словаре: {}", key),
                        RuntimeErrorKind::Other,
                    )
                })
            }
            _ => Err(RuntimeError::type_mismatch(
                "массив, строка или словарь",
                "другой тип",
            )),
        }
    }

    // =========================================================================
    //                    ООП
    // =========================================================================

    fn eval_field_access(
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
                // [KITE 11, шаг 5] Проверка инкапсуляции (видимость поля).
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

    fn eval_method_call(
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
        let result = super::executor::Executor::execute_stmts(body, env);

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
    fn find_class_name_by_type_id(
        type_id: &shared::types::TypeId,
        env: &Environment,
    ) -> Option<String> {
        // [KITE 11, шаг 1] Идентичность объекта через TypeRegistry.
        env.class_name_by_type_id(*type_id)
    }

    /// [KITE 11] Находит класс объекта по наиболее производному совпадению полей:
    /// предпочитается класс с наибольшим числом полей, все из которых есть у объекта.
    fn find_class_by_fields(
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

    /// [KITE 2] Вычисляет диапазон `начало..конец` (или `..=`) в целочисленное значение.
    fn eval_range(
        start: Option<&Expr>,
        end: Option<&Expr>,
        inclusive: bool,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let to_int = |e: &Expr, env: &mut Environment| -> RuntimeResult<i64> {
            Self::evaluate(e, env)?
                .as_int()
                .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))
        };
        let s = match start {
            Some(e) => to_int(e, env)?,
            None => {
                return Err(RuntimeError::new(
                    "Диапазон без начала пока не поддерживается",
                    RuntimeErrorKind::Other,
                ));
            }
        };
        let en = match end {
            Some(e) => to_int(e, env)?,
            None => {
                return Err(RuntimeError::new(
                    "Диапазон без конца пока не поддерживается",
                    RuntimeErrorKind::Other,
                ));
            }
        };
        Ok(Value::Range {
            start: s,
            end: en,
            inclusive,
        })
    }

    /// [KITE 11, шаг 5] Проверяет видимость поля при доступе.
    /// Публичные поля доступны всегда; закрытые — только из класса-владельца;
    /// защищённые — из владельца и его потомков.
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

    /// Является ли `sub` тем же классом, что `sup`, или его потомком.
    fn is_subclass_of(sub: &str, sup: &str, env: &Environment) -> bool {
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

    fn call_string_method(
        s: &str,
        method: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        match method {
            "длина" | "length" | "len" => {
                Ok(Value::Number(Number::I64(s.chars().count() as i64)))
            }
            "верхний_регистр" | "to_upper" | "upper" => {
                Ok(Value::String(s.to_uppercase()))
            }
            "нижний_регистр" | "to_lower" | "lower" => {
                Ok(Value::String(s.to_lowercase()))
            }
            "содержит" | "contains" => {
                if args.len() != 1 {
                    return Err(RuntimeError::argument_count(method, 1, args.len()));
                }
                let substr = Self::evaluate(&args[0], env)?;
                if let Value::String(sub) = substr {
                    Ok(Value::Boolean(s.contains(&sub)))
                } else {
                    Err(RuntimeError::type_mismatch("строка", "не строка"))
                }
            }
            "разделить" | "split" => {
                if args.len() != 1 {
                    return Err(RuntimeError::argument_count(method, 1, args.len()));
                }
                let delim = Self::evaluate(&args[0], env)?;
                if let Value::String(d) = delim {
                    let parts: Vec<Value> =
                        s.split(&d).map(|p| Value::String(p.to_string())).collect();
                    Ok(Value::Array(parts))
                } else {
                    Err(RuntimeError::type_mismatch("строка", "не строка"))
                }
            }
            "обрезать" | "trim" => Ok(Value::String(s.trim().to_string())),
            "заменить" | "replace" => {
                if args.len() != 2 {
                    return Err(RuntimeError::argument_count(method, 2, args.len()));
                }
                let from = Self::evaluate(&args[0], env)?;
                let to = Self::evaluate(&args[1], env)?;
                match (from, to) {
                    (Value::String(f), Value::String(t)) => Ok(Value::String(s.replace(&f, &t))),
                    _ => Err(RuntimeError::type_mismatch("строка, строка", "другое")),
                }
            }
            _ => Err(RuntimeError::new(
                format!("Метод '{}' не найден для строки", method),
                RuntimeErrorKind::Other,
            )),
        }
    }

    fn call_array_method(
        arr: &[Value],
        method: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        match method {
            "длина" | "length" | "len" | "размер" | "size" => {
                Ok(Value::Number(Number::I64(arr.len() as i64)))
            }
            "пусто" | "is_empty" | "empty" => Ok(Value::Boolean(arr.is_empty())),
            "первый" | "first" => arr.first().cloned().ok_or_else(|| {
                RuntimeError::new("Массив пуст", RuntimeErrorKind::IndexOutOfBounds)
            }),
            "последний" | "last" => arr.last().cloned().ok_or_else(|| {
                RuntimeError::new("Массив пуст", RuntimeErrorKind::IndexOutOfBounds)
            }),
            "содержит" | "contains" => {
                if args.len() != 1 {
                    return Err(RuntimeError::argument_count(method, 1, args.len()));
                }
                let value = Self::evaluate(&args[0], env)?;
                Ok(Value::Boolean(arr.contains(&value)))
            }
            "сумма" | "sum" => {
                let mut sum: f64 = 0.0;
                for val in arr {
                    if let Some(n) = val.as_number().and_then(|n| n.to_f64()) {
                        sum += n;
                    } else {
                        return Err(RuntimeError::type_mismatch("числа", "не число"));
                    }
                }
                Ok(Value::Number(Number::F64(sum)))
            }
            _ => Err(RuntimeError::new(
                format!("Метод '{}' не найден для массива", method),
                RuntimeErrorKind::Other,
            )),
        }
    }

    fn eval_new_instance(
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
            let _ = super::executor::Executor::execute_stmts(
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

    fn eval_enum_construct(
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

    // =========================================================================
    //                    ПРИВЕДЕНИЕ И ПРОВЕРКА ТИПОВ
    // =========================================================================

    fn eval_cast(
        expr: &Expr,
        target_type: &TypeKind,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let value = Self::evaluate(expr, env)?;

        match target_type {
            TypeKind::Int64 => {
                let n = value
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_mismatch("цел", "не число"))?;
                Ok(Value::Number(Number::I64(n)))
            }
            TypeKind::Float64 => match &value {
                Value::Number(n) => {
                    let f = n
                        .to_f64()
                        .ok_or_else(|| RuntimeError::type_mismatch("вещ", "не число"))?;
                    Ok(Value::Number(Number::F64(f)))
                }
                Value::String(s) => {
                    let f: f64 = s
                        .parse()
                        .map_err(|_| RuntimeError::type_mismatch("вещ", "не число"))?;
                    Ok(Value::Number(Number::F64(f)))
                }
                _ => Err(RuntimeError::type_mismatch("вещ", "не число")),
            },
            TypeKind::String => Ok(Value::String(value.to_string())),
            TypeKind::Bool => Ok(Value::Boolean(Self::is_truthy(&value))),
            _ => Err(RuntimeError::not_implemented(&format!(
                "приведение к типу {:?}",
                target_type
            ))),
        }
    }

    fn eval_type_check(
        expr: &Expr,
        check_type: &TypeKind,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let value = Self::evaluate(expr, env)?;

        let matches = match (check_type, &value) {
            (TypeKind::Int64, Value::Number(Number::I64(_))) => true,
            (TypeKind::Float64, Value::Number(Number::F64(_))) => true,
            (TypeKind::String, Value::String(_)) => true,
            (TypeKind::Bool, Value::Boolean(_)) => true,
            (TypeKind::Char, Value::Char(_)) => true,
            (TypeKind::Array(_), Value::Array(_)) => true,
            _ => false,
        };

        Ok(Value::Boolean(matches))
    }

    // =========================================================================
    //                    PIPE И MATCH
    // =========================================================================

    fn eval_pipe(value: &Expr, func: &Expr, env: &mut Environment) -> RuntimeResult<Value> {
        let val = Self::evaluate(value, env)?;

        match func {
            Expr::Call(name, args) => {
                let mut new_args = vec![Expr::Literal(val)];
                new_args.extend(args.clone());
                Self::eval_call(name, &new_args, env)
            }
            Expr::Variable(name) => Self::eval_call(name, &[Expr::Literal(val)], env),
            Expr::Pipe(inner_val, inner_func) => {
                let intermediate = Self::eval_pipe(&Expr::Literal(val), inner_val, env)?;
                Self::eval_pipe(&Expr::Literal(intermediate), inner_func, env)
            }
            _ => Err(RuntimeError::new(
                "Правая часть |> должна быть вызовом функции",
                RuntimeErrorKind::Other,
            )),
        }
    }

    fn eval_match_expr(
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

    fn match_pattern(
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

    // =========================================================================
    //                    ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ
    // =========================================================================

    /// Проверяет "истинность" значения.
    pub fn is_truthy(value: &Value) -> bool {
        match value {
            Value::Boolean(b) => *b,
            Value::Number(n) => n.to_f64().map(|f| f != 0.0).unwrap_or(false),
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Null | Value::Undefined => false,
            Value::Option(opt) => opt.is_some(),
            _ => true,
        }
    }

    /// Сравнивает два значения на равенство.
    pub fn values_equal(a: &Value, b: &Value) -> bool {
        a == b
    }

    /// Сравнивает два значения.
    fn compare_values<F>(a: &Value, b: &Value, cmp: F) -> RuntimeResult<Value>
    where
        F: Fn(std::cmp::Ordering) -> bool,
    {
        let result = match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                let fa = na
                    .to_f64()
                    .ok_or_else(|| RuntimeError::type_mismatch("число", "не число"))?;
                let fb = nb
                    .to_f64()
                    .ok_or_else(|| RuntimeError::type_mismatch("число", "не число"))?;
                cmp(fa.partial_cmp(&fb).unwrap_or(std::cmp::Ordering::Equal))
            }
            (Value::String(sa), Value::String(sb)) => cmp(sa.cmp(sb)),
            (Value::Char(ca), Value::Char(cb)) => cmp(ca.cmp(cb)),
            _ => {
                return Err(RuntimeError::type_mismatch(
                    "сравнимые типы",
                    "несравнимые типы",
                ));
            }
        };
        Ok(Value::Boolean(result))
    }

    /// Преобразует значение в индекс.
    fn to_index(value: &Value) -> RuntimeResult<i64> {
        value
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))
    }

    /// Возвращает значение по умолчанию для типа.
    pub fn default_value_for_type(type_spec: &TypeKind) -> Value {
        match type_spec {
            TypeKind::Int8 => Value::Number(Number::I8(0)),
            TypeKind::Int16 => Value::Number(Number::I16(0)),
            TypeKind::Int32 => Value::Number(Number::I32(0)),
            TypeKind::Int64 => Value::Number(Number::I64(0)),
            TypeKind::Int128 => Value::Number(Number::I128(0)),
            TypeKind::UInt8 => Value::Number(Number::U8(0)),
            TypeKind::UInt16 => Value::Number(Number::U16(0)),
            TypeKind::UInt32 => Value::Number(Number::U32(0)),
            TypeKind::UInt64 => Value::Number(Number::U64(0)),
            TypeKind::UInt128 => Value::Number(Number::U128(0)),
            TypeKind::Float32 => Value::Number(Number::F32(0.0)),
            TypeKind::Float64 => Value::Number(Number::F64(0.0)),
            TypeKind::Float128 => Value::Number(Number::F128(shared::f128::F128::from(0.0))),
            TypeKind::String => Value::String(String::new()),
            TypeKind::Bool => Value::Boolean(false),
            TypeKind::Char => Value::Char('\0'),
            TypeKind::Array(_) => Value::Array(Vec::new()),
            TypeKind::Option(_) => Value::Option(Box::new(None)),
            _ => Value::Undefined,
        }
    }
}
