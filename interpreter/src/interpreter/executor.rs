//! Исполнитель инструкций для интерпретатора Кумир 3
//!
//! Реализует выполнение всех типов инструкций: присваивание, условия,
//! циклы, ввод/вывод, обработка исключений и т.д.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::sync::Arc;

use shared::codegen::rust_block::{RustBlockConfig, RustBlockExecutor, RustExecutionMode};
use shared::types::{EnumVariant, Expr, MatchArm, Number, Pattern, Stmt, TypeKind, Value};

use super::environment::Environment;
use super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::evaluator::ExprEvaluator;

/// Исполнитель инструкций.
pub struct Executor;

impl Executor {
    /// Выполняет список инструкций.
    pub fn execute_stmts(stmts: &[Stmt], env: &mut Environment) -> RuntimeResult<ControlFlow> {
        for stmt in stmts {
            let flow = Self::execute(stmt, env)?;
            match flow {
                ControlFlow::None => continue,
                _ => return Ok(flow),
            }
        }
        Ok(ControlFlow::None)
    }

    /// Выполняет одну инструкцию.
    pub fn execute(stmt: &Stmt, env: &mut Environment) -> RuntimeResult<ControlFlow> {
        match stmt {
            // ===== ПРИСВАИВАНИЕ =====
            Stmt::Assignment(name, expr) => {
                let value = ExprEvaluator::evaluate(expr, env)?;
                env.set_variable(name, value)?;
                Ok(ControlFlow::None)
            }

            Stmt::ArrayAssignment(name, indices, expr) => {
                Self::execute_array_assignment(name, indices, expr, env)
            }

            // ===== УСЛОВИЯ =====
            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => Self::execute_if(condition, then_branch, else_branch.as_deref(), env),

            // ===== ЦИКЛЫ =====
            Stmt::LoopWhile { condition, body } => Self::execute_while(condition, body, env),

            Stmt::LoopForEach {
                variable,
                var_type: _,
                iterable,
                body,
            } => Self::execute_for_each(variable, iterable, body, env),

            Stmt::LoopFor {
                variable,
                from,
                to,
                step,
                body,
            } => Self::execute_for(variable, from, to, step.as_ref(), body, env),

            Stmt::LoopInfinite { body } => Self::execute_infinite_loop(body, env),

            Stmt::LoopDoWhile { body, condition } => Self::execute_do_while(body, condition, env),

            // ===== ВВОД/ВЫВОД =====
            Stmt::Input(vars) => Self::execute_input(vars, env),

            Stmt::Output(exprs) => Self::execute_output(exprs, env),

            // ===== УПРАВЛЕНИЕ ПОТОКОМ =====
            Stmt::Assert(expr) => Self::execute_assert(expr, env),

            Stmt::ExprStmt(expr) => {
                ExprEvaluator::evaluate(expr, env)?;
                Ok(ControlFlow::None)
            }

            Stmt::Return => Ok(ControlFlow::Return(None)),

            Stmt::ReturnValue(expr) => {
                let value = ExprEvaluator::evaluate(expr, env)?;
                Ok(ControlFlow::Return(Some(value)))
            }

            Stmt::ResultAssign(expr) => {
                let value = ExprEvaluator::evaluate(expr, env)?;
                env.set_result_value(value);
                Ok(ControlFlow::None)
            }

            Stmt::Break => Ok(ControlFlow::Break),

            Stmt::Continue => Ok(ControlFlow::Continue),

            // ===== ОБЪЯВЛЕНИЕ ПЕРЕМЕННЫХ =====
            Stmt::AutoVarDecl { name, init, .. } => {
                let value = ExprEvaluator::evaluate(init, env)?;
                env.define_local(name.clone(), value);
                Ok(ControlFlow::None)
            }

            Stmt::VarDecl {
                type_kind,
                names,
                init,
                ..
            } => Self::execute_var_decl(type_kind, names, init.as_ref(), env),

            // ===== МОДУЛИ И ИМПОРТ =====
            Stmt::Import { path, alias, items } => {
                Self::execute_import(path, alias.as_deref(), items.as_deref(), env)
            }

            Stmt::ModuleDecl {
                name,
                body,
                algorithms,
                ..
            } => {
                // Регистрируем алгоритмы модуля с префиксом имени модуля
                for alg in algorithms {
                    let full_name = format!("{}.{}", name, alg.name);
                    // Создаём копию алгоритма с полным именем для вызова через Модуль.алг()
                    let mut prefixed_alg = alg.clone();
                    prefixed_alg.name = Arc::from(full_name.as_str());
                    env.define_algorithm(prefixed_alg);

                    // Также регистрируем оригинальный алгоритм (для вызова без префикса внутри модуля)
                    env.define_algorithm(alg.clone());
                }

                // Выполняем глобальные объявления модуля
                for stmt in body {
                    Self::execute(stmt, env)?;
                }

                Ok(ControlFlow::None)
            }

            Stmt::Export { names: _ } => {
                // TODO: реализация экспорта
                Ok(ControlFlow::None)
            }

            // ===== ПЕРЕЧИСЛЕНИЯ =====
            Stmt::EnumDecl { name, variants, .. } => Self::execute_enum_decl(name, variants, env),

            Stmt::Match { expr, arms, .. } => Self::execute_match(expr, arms, env),

            // ===== УКАЗАТЕЛИ =====
            Stmt::PointerNew { name, value, .. } => {
                let val = ExprEvaluator::evaluate(value, env)?;
                env.define_local(name.clone(), Value::Pointer(Box::new(val)));
                Ok(ControlFlow::None)
            }

            Stmt::PointerDelete { name } => {
                env.set_variable(name, Value::Null)?;
                Ok(ControlFlow::None)
            }

            // ===== ОБРАБОТКА ОШИБОК =====
            Stmt::TryCatch {
                try_block,
                catch_var,
                catch_block,
                finally_block,
                ..
            } => Self::execute_try_catch(
                try_block,
                catch_var.as_deref(),
                catch_block,
                finally_block.as_deref(),
                env,
            ),

            Stmt::Throw(expr) => {
                let value = ExprEvaluator::evaluate(expr, env)?;
                let message = value.as_string().unwrap_or_else(|| value.to_string());
                Err(RuntimeError::user_exception(message))
            }

            // ===== RUST-ВСТАВКИ =====
            Stmt::RustBlock {
                code,
                captured_vars,
                ..
            } => Self::execute_rust_block(code, captured_vars, env),

            // ===== АСИНХРОННОЕ ПРОГРАММИРОВАНИЕ =====
            Stmt::Await(expr) => {
                // В синхронном режиме просто вычисляем выражение
                ExprEvaluator::evaluate(expr, env)?;
                Ok(ControlFlow::None)
            }

            // ===== КЛАССЫ И ООП =====
            Stmt::ClassDecl(class_def) => {
                env.define_class(class_def.clone());
                Ok(ControlFlow::None)
            }

            Stmt::StructDecl(class_def) => {
                // Структура — это ClassDef с kind=Struct, определяем как класс
                env.define_class(class_def.clone());
                Ok(ControlFlow::None)
            }

            Stmt::InterfaceDecl(iface) => {
                env.define_interface(iface.clone());
                Ok(ControlFlow::None)
            }

            Stmt::TraitDecl(trait_def) => {
                env.define_trait(trait_def.clone());
                Ok(ControlFlow::None)
            }

            Stmt::ImplBlock(impl_def) => {
                env.define_impl(impl_def.clone());
                Ok(ControlFlow::None)
            }

            Stmt::FieldAssignment {
                object,
                field,
                value,
            } => Self::execute_field_assignment(object, field, value, env),

            // Все остальные инструкции (не реализованы)
            _ => Err(RuntimeError::not_implemented("данная инструкция")),
        }
    }

    // =========================================================================
    //                    ПРИСВАИВАНИЕ ЭЛЕМЕНТУ МАССИВА
    // =========================================================================

    fn execute_array_assignment(
        name: &str,
        indices: &[Expr],
        value_expr: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let value = ExprEvaluator::evaluate(value_expr, env)?;

        // Получаем массив
        let array = env.get_variable(name)?.clone();

        match array {
            Value::Array(mut elements) => {
                if indices.len() != 1 {
                    return Err(RuntimeError::not_implemented("многомерные массивы"));
                }

                let idx = ExprEvaluator::evaluate(&indices[0], env)?;
                let i = idx
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;

                if i < 0 || i as usize >= elements.len() {
                    return Err(RuntimeError::index_out_of_bounds(i, elements.len()));
                }

                elements[i as usize] = value;
                env.set_variable(name, Value::Array(elements))?;
            }
            Value::Map(mut map) => {
                if indices.len() != 1 {
                    return Err(RuntimeError::new(
                        "Словарь поддерживает только один ключ",
                        RuntimeErrorKind::Other,
                    ));
                }

                let key = ExprEvaluator::evaluate(&indices[0], env)?;
                map.insert(key, value);
                env.set_variable(name, Value::Map(map))?;
            }
            _ => {
                return Err(RuntimeError::type_mismatch(
                    "массив или словарь",
                    "другой тип",
                ));
            }
        }

        Ok(ControlFlow::None)
    }

    // =========================================================================
    //                    УСЛОВНЫЙ ОПЕРАТОР
    // =========================================================================

    fn execute_if(
        condition: &Expr,
        then_branch: &[Stmt],
        else_branch: Option<&[Stmt]>,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let cond_value = ExprEvaluator::evaluate(condition, env)?;

        if ExprEvaluator::is_truthy(&cond_value) {
            Self::execute_stmts(then_branch, env)
        } else if let Some(else_stmts) = else_branch {
            Self::execute_stmts(else_stmts, env)
        } else {
            Ok(ControlFlow::None)
        }
    }

    // =========================================================================
    //                    ЦИКЛЫ
    // =========================================================================

    fn execute_while(
        condition: &Expr,
        body: &[Stmt],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        loop {
            let cond_value = ExprEvaluator::evaluate(condition, env)?;
            if !ExprEvaluator::is_truthy(&cond_value) {
                break;
            }

            match Self::execute_stmts(body, env)? {
                ControlFlow::Break => break,
                ControlFlow::Continue => continue,
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::None => {}
            }
        }
        Ok(ControlFlow::None)
    }

    fn execute_for(
        variable: &str,
        from: &Expr,
        to: &Expr,
        step: Option<&Expr>,
        body: &[Stmt],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let start = ExprEvaluator::evaluate(from, env)?;
        let end = ExprEvaluator::evaluate(to, env)?;
        let step_val = if let Some(s) = step {
            ExprEvaluator::evaluate(s, env)?
        } else {
            Value::Number(Number::I64(1))
        };

        let start_i = start
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;
        let end_i = end
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;
        let step_i = step_val
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))?;

        if step_i == 0 {
            return Err(RuntimeError::new(
                "Шаг цикла не может быть равен нулю",
                RuntimeErrorKind::Other,
            ));
        }

        let mut i = start_i;
        loop {
            // Проверяем условие выхода
            if step_i > 0 {
                if i > end_i {
                    break;
                }
            } else if i < end_i {
                break;
            }

            // Устанавливаем переменную цикла
            env.define_local(variable.to_string(), Value::Number(Number::I64(i)));

            // Выполняем тело
            match Self::execute_stmts(body, env)? {
                ControlFlow::Break => break,
                ControlFlow::Continue => {}
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::None => {}
            }

            // Увеличиваем счётчик
            i += step_i;
        }

        Ok(ControlFlow::None)
    }

    /// [KITE 2/4] Цикл по коллекции/диапазону: `нц для x в <итерируемое> … кц`.
    fn execute_for_each(
        variable: &str,
        iterable: &Expr,
        body: &[Stmt],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let value = ExprEvaluator::evaluate(iterable, env)?;

        // Материализуем последовательность элементов для перебора.
        let items: Vec<Value> = match value {
            Value::Range {
                start,
                end,
                inclusive,
            } => {
                let last = if inclusive { end } else { end - 1 };
                let mut v = Vec::new();
                let mut i = start;
                while i <= last {
                    v.push(Value::Number(Number::I64(i)));
                    i += 1;
                }
                v
            }
            Value::Array(a) => a,
            Value::Tuple(t) => t,
            Value::Set(s) => s.into_iter().collect(),
            Value::String(s) => s.chars().map(Value::Char).collect(),
            other => {
                return Err(RuntimeError::type_mismatch(
                    "коллекция или диапазон",
                    &other.type_kind().russian_name(),
                ));
            }
        };

        for item in items {
            env.define_local(variable.to_string(), item);
            match Self::execute_stmts(body, env)? {
                ControlFlow::Break => break,
                ControlFlow::Continue => {}
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::None => {}
            }
        }

        Ok(ControlFlow::None)
    }

    fn execute_infinite_loop(body: &[Stmt], env: &mut Environment) -> RuntimeResult<ControlFlow> {
        loop {
            match Self::execute_stmts(body, env)? {
                ControlFlow::Break => break,
                ControlFlow::Continue => continue,
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::None => {}
            }
        }
        Ok(ControlFlow::None)
    }

    fn execute_do_while(
        body: &[Stmt],
        condition: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        loop {
            match Self::execute_stmts(body, env)? {
                ControlFlow::Break => break,
                ControlFlow::Continue => {}
                ControlFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                ControlFlow::None => {}
            }

            let cond_value = ExprEvaluator::evaluate(condition, env)?;
            if !ExprEvaluator::is_truthy(&cond_value) {
                break;
            }
        }
        Ok(ControlFlow::None)
    }

    // =========================================================================
    //                    ВВОД/ВЫВОД
    // =========================================================================

    fn execute_input(vars: &[String], env: &mut Environment) -> RuntimeResult<ControlFlow> {
        let stdin = io::stdin();
        let mut handle = stdin.lock();

        for var in vars {
            let mut input = String::new();
            handle
                .read_line(&mut input)
                .map_err(|e| RuntimeError::io_error(format!("Ошибка ввода: {}", e)))?;
            let input = input.trim();

            // Пытаемся определить тип автоматически
            let value = if let Ok(i) = input.parse::<i64>() {
                Value::Number(Number::I64(i))
            } else if let Ok(f) = input.parse::<f64>() {
                Value::Number(Number::F64(f))
            } else if input == "да" || input == "true" {
                Value::Boolean(true)
            } else if input == "нет" || input == "false" {
                Value::Boolean(false)
            } else {
                Value::String(input.to_string())
            };

            env.set_variable(var, value)?;
        }

        Ok(ControlFlow::None)
    }

    fn execute_output(exprs: &[Expr], env: &mut Environment) -> RuntimeResult<ControlFlow> {
        let mut output_parts = Vec::new();

        for expr in exprs {
            let value = ExprEvaluator::evaluate(expr, env)?;
            output_parts.push(Self::format_value(&value));
        }

        let output = output_parts.join(" ");
        env.println(&output);

        // Также выводим в stdout если не в режиме тестирования
        if env.is_debug_mode() {
            println!("{}", output);
        }

        Ok(ControlFlow::None)
    }

    fn format_value(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Char(c) => c.to_string(),
            Value::Boolean(b) => {
                if *b {
                    "да".to_string()
                } else {
                    "нет".to_string()
                }
            }
            Value::Null => "пусто".to_string(),
            Value::Undefined => "неопределено".to_string(),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(Self::format_value).collect();
                format!("[{}]", items.join(", "))
            }
            _ => value.to_string(),
        }
    }

    // =========================================================================
    //                    УТВЕРЖДЕНИЕ
    // =========================================================================

    fn execute_assert(expr: &Expr, env: &mut Environment) -> RuntimeResult<ControlFlow> {
        let value = ExprEvaluator::evaluate(expr, env)?;

        if !ExprEvaluator::is_truthy(&value) {
            return Err(RuntimeError::assertion_failed(&format!("{:?}", expr)));
        }

        Ok(ControlFlow::None)
    }

    // =========================================================================
    //                    RUST-ВСТАВКИ
    // =========================================================================

    /// Выполняет Rust-блок с захваченными переменными
    fn execute_rust_block(
        code: &str,
        captured_vars: &[String],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        // Собираем захваченные переменные из окружения
        let mut vars = HashMap::new();
        for var_name in captured_vars {
            if let Ok(value) = env.get_variable(var_name) {
                vars.insert(var_name.clone(), value.clone());
            }
        }

        // Создаём исполнитель Rust-блоков
        // По умолчанию используем интерпретацию, если rustc недоступен
        let config = RustBlockConfig {
            execution_mode: RustExecutionMode::Interpret,
            ..Default::default()
        };
        let mut executor = RustBlockExecutor::with_config(config);

        // Выполняем код
        let result = executor.execute(code, &vars)?;

        // Выводим stdout если есть
        if !result.stdout.is_empty() {
            env.print(&result.stdout);
            if env.is_debug_mode() {
                print!("{}", result.stdout);
            }
        }

        // Выводим stderr если есть
        if !result.stderr.is_empty() {
            env.print(&format!("[stderr] {}", result.stderr));
            if env.is_debug_mode() {
                eprint!("{}", result.stderr);
            }
        }

        // Проверяем код возврата
        if let Some(code) = result.exit_code
            && code != 0
        {
            return Err(RuntimeError::new(
                format!("Rust-блок завершился с кодом {}", code),
                RuntimeErrorKind::Other,
            ));
        }

        Ok(ControlFlow::None)
    }

    // =========================================================================
    //                    ОБЪЯВЛЕНИЕ ПЕРЕМЕННЫХ
    // =========================================================================

    fn execute_var_decl(
        type_spec: &TypeKind,
        names: &[String],
        init: Option<&Expr>,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let initial_value = if let Some(expr) = init {
            ExprEvaluator::evaluate(expr, env)?
        } else {
            ExprEvaluator::default_value_for_type(type_spec)
        };

        // Если инициализация есть и только одна переменная
        if init.is_some() && names.len() == 1 {
            env.define_local(names[0].clone(), initial_value);
        } else {
            // Для нескольких переменных используем значение по умолчанию
            let default = ExprEvaluator::default_value_for_type(type_spec);
            for name in names {
                env.define_local(name.clone(), default.clone());
            }
        }

        Ok(ControlFlow::None)
    }

    // =========================================================================
    //                    ПЕРЕЧИСЛЕНИЯ
    // =========================================================================

    fn execute_enum_decl(
        name: &str,
        variants: &[EnumVariant],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let variant_names: Vec<String> = variants.iter().map(|v| v.name.clone()).collect();
        env.define_enum(name.to_string(), variant_names);
        Ok(ControlFlow::None)
    }

    fn execute_match(
        expr: &Expr,
        arms: &[MatchArm],
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let value = ExprEvaluator::evaluate(expr, env)?;

        for arm in arms {
            if let Some(bindings) = Self::match_pattern(&arm.pattern, &value)? {
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
                start: _,
                end: _,
                inclusive: _,
            } => {
                // Для чисел проверяем попадание в диапазон
                // TODO: полная реализация
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

            Pattern::Array { elements, rest } => {
                if let Value::Array(values) = value {
                    if elements.len() > values.len() {
                        return Ok(None);
                    }

                    let mut all_bindings = vec![];
                    for (p, v) in elements.iter().zip(values.iter()) {
                        if let Some(bindings) = Self::match_pattern(p, v)? {
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
                    if let Some(bindings) = Self::match_pattern(p, value)? {
                        return Ok(Some(bindings));
                    }
                }
                Ok(None)
            }

            // Все остальные паттерны (не реализованы)
            _ => Ok(None),
        }
    }

    // =========================================================================
    //                    ОБРАБОТКА ИСКЛЮЧЕНИЙ
    // =========================================================================

    fn execute_try_catch(
        try_block: &[Stmt],
        catch_var: Option<&str>,
        catch_block: &[Stmt],
        finally_block: Option<&[Stmt]>,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let result = Self::execute_stmts(try_block, env);

        let control_flow = match result {
            Ok(flow) => flow,
            Err(error) => {
                // [KITE 4] Блочная область: catch видит локали алгоритма + переменную ошибки.
                env.push_scope();

                if let Some(var) = catch_var {
                    env.define_local(var.to_string(), Value::String(error.message.clone()));
                }

                let catch_result = Self::execute_stmts(catch_block, env);
                env.pop_scope();

                catch_result?
            }
        };

        // Выполняем finally если есть
        if let Some(finally_stmts) = finally_block {
            Self::execute_stmts(finally_stmts, env)?;
        }

        Ok(control_flow)
    }

    // =========================================================================
    //                    ООП: ПРИСВАИВАНИЕ ПОЛЮ
    // =========================================================================

    fn execute_field_assignment(
        object: &Expr,
        field: &str,
        value_expr: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let value = ExprEvaluator::evaluate(value_expr, env)?;

        // Получаем имя переменной с объектом
        let var_name = match object {
            Expr::Variable(name) => name.clone(),
            Expr::SelfRef => {
                // Работаем с this
                if let Some(this) = env.get_this().cloned()
                    && let Value::Object {
                        type_id,
                        mut fields,
                    } = this
                {
                    fields.insert(field.to_string(), value);
                    // Обновляем this в текущем кадре
                    if let Some(frame) = env.current_frame_mut() {
                        frame.this = Some(Value::Object { type_id, fields });
                    }
                }
                return Ok(ControlFlow::None);
            }
            _ => {
                return Err(RuntimeError::new(
                    "Ожидалась переменная для присваивания полю",
                    RuntimeErrorKind::Other,
                ));
            }
        };

        // Получаем объект
        let obj = env.get_variable(&var_name)?.clone();

        match obj {
            Value::Object {
                type_id,
                mut fields,
            } => {
                fields.insert(field.to_string(), value);
                env.set_variable(&var_name, Value::Object { type_id, fields })?;
            }
            _ => {
                return Err(RuntimeError::type_mismatch("объект", "не объект"));
            }
        }

        Ok(ControlFlow::None)
    }

    // =========================================================================
    //                    ИМПОРТ БИБЛИОТЕК И МОДУЛЕЙ
    // =========================================================================

    /// Выполняет импорт библиотеки или файла.
    ///
    /// Поддерживает:
    /// - Библиотеки: `использовать время`, `использовать время@^2.0`
    /// - Файлы: `использовать "./модуль.kum"`, `использовать ../utils`
    /// - Алиасы: `использовать время как т`
    /// - Выборочный импорт: `использовать время { now_ms, sleep }`
    fn execute_import(
        path: &str,
        alias: Option<&str>,
        items: Option<&[String]>,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        use super::file_importer::FileImporter;

        // Проверяем, является ли это файлом или библиотекой
        let is_file = FileImporter::is_kum_file(path);

        if is_file {
            // Импорт .kum файла
            Self::execute_file_import(path, alias, items, env)
        } else {
            // Импорт библиотеки
            Self::execute_library_import(path, alias, items, env)
        }
    }

    /// Импортирует библиотеку.
    fn execute_library_import(
        path: &str,
        alias: Option<&str>,
        items: Option<&[String]>,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        // Парсим имя библиотеки и версию (например, "время@^2.0")
        let (lib_name, version_spec) = if let Some(at_pos) = path.find('@') {
            let name = &path[..at_pos];
            let version = &path[at_pos + 1..];
            (name, Some(version))
        } else {
            (path, None)
        };

        // Получаем менеджер библиотек
        let lib_manager = env.library_manager().ok_or_else(|| {
            RuntimeError::new(
                "Менеджер библиотек не инициализирован",
                RuntimeErrorKind::Other,
            )
        })?;

        // Импортируем библиотеку
        if let Some(version) = version_spec {
            lib_manager
                .write()
                .unwrap()
                .import_versioned(lib_name, version, alias)?;
        } else {
            lib_manager.write().unwrap().import(lib_name, alias)?;
        }

        // Если указан выборочный импорт - проверяем доступность функций
        if let Some(item_names) = items {
            let manager = lib_manager.read().unwrap();
            for item in item_names {
                if !manager.is_library_function(item) {
                    return Err(RuntimeError::new(
                        format!("Функция '{}' не найдена в библиотеке '{}'", item, lib_name),
                        RuntimeErrorKind::UndefinedAlgorithm,
                    ));
                }
            }
        }

        Ok(ControlFlow::None)
    }

    /// Импортирует .kum файл.
    fn execute_file_import(
        path: &str,
        alias: Option<&str>,
        items: Option<&[String]>,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        // Получаем импортер файлов
        let file_importer = env.file_importer().ok_or_else(|| {
            RuntimeError::new(
                "Импортер файлов не инициализирован",
                RuntimeErrorKind::Other,
            )
        })?;

        // Импортируем модуль
        let module = {
            let mut importer = file_importer.write().unwrap();
            importer.import(path, alias)?
        };

        // Регистрируем алгоритмы модуля в среде
        let module_prefix = alias.unwrap_or_else(|| module.name.as_str());

        if let Some(item_names) = items {
            // Выборочный импорт
            for item in item_names {
                if let Some(alg) = module.get_algorithm(item) {
                    // Регистрируем с префиксом модуля
                    let full_name = format!("{}.{}", module_prefix, item);
                    let mut prefixed_alg = alg.clone();
                    prefixed_alg.name = std::sync::Arc::from(full_name.as_str());
                    env.define_algorithm(prefixed_alg);
                } else if let Some(class) = module.get_class(item) {
                    // Регистрируем класс
                    env.define_class(class.clone());
                } else {
                    return Err(RuntimeError::new(
                        format!("Элемент '{}' не найден в модуле '{}'", item, path),
                        RuntimeErrorKind::UndefinedAlgorithm,
                    ));
                }
            }
        } else {
            // Импортируем все публичные элементы
            for (name, alg) in module.public_algorithms() {
                let full_name = format!("{}.{}", module_prefix, name);
                let mut prefixed_alg = alg.clone();
                prefixed_alg.name = std::sync::Arc::from(full_name.as_str());
                env.define_algorithm(prefixed_alg);
            }

            for (_, class) in module.public_classes() {
                env.define_class(class.clone());
            }
        }

        Ok(ControlFlow::None)
    }
}
