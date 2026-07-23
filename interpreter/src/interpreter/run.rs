use super::Interpreter;
use super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::evaluator::ExprEvaluator;
use super::executor::Executor;
use shared::parser::{parse, parse_expression};
use shared::types::{Program, Value};

impl Interpreter {
    // =========================================================================
    //                    ВЫПОЛНЕНИЕ ПРОГРАММ
    // =========================================================================

    /// Выполняет исходный код программы.
    pub fn run(&mut self, source: &str) -> RuntimeResult<Value> {
        // Парсим программу
        let program = parse(source).map_err(|e| {
            RuntimeError::new(format!("Ошибка разбора: {}", e), RuntimeErrorKind::Other)
        })?;

        self.run_program(&program)
    }

    /// Выполняет фрагмент так, как этого ждут в интерактивном режиме.
    ///
    /// Отличие от [`Self::run`] одно, но важное: набранные подряд команды
    /// работают с общим состоянием. Обычный запуск оборачивает свободные
    /// инструкции в главный алгоритм, а его кадр после выполнения снимается —
    /// вместе со всеми объявленными переменными. В консоли это означало, что
    /// `цел счётчик := 5`, набранное строкой выше, следующей строке уже не
    /// видно.
    ///
    /// Поэтому свободные инструкции выполняются прямо в глобальной области, а
    /// объявления алгоритмов и классов — как обычно, они и так глобальны.
    /// Программа с собственной точкой входа (`алг главный`) запускается
    /// целиком: раз человек её написал, он ждёт именно запуска.
    pub fn run_interactive(&mut self, source: &str) -> RuntimeResult<Value> {
        let program = parse(source).map_err(|e| {
            RuntimeError::new(format!("Ошибка разбора: {}", e), RuntimeErrorKind::Other)
        })?;

        self.load_program(&program)?;
        self.validate_classes()?;

        for stmt in &program.globals {
            Executor::execute(stmt, &mut self.env)?;
        }

        // Свободные инструкции разбор собирает в анонимный алгоритм.
        // Выполняем их телом, без кадра, — тогда объявленное остаётся.
        if program.auto_wrapped
            && let Some(main) = &program.main
        {
            let body = main.body.clone().unwrap_or_default();
            for stmt in &body {
                if let ControlFlow::Return(value) = Executor::execute(stmt, &mut self.env)? {
                    return Ok(value.unwrap_or(Value::Null));
                }
            }
            return Ok(Value::Null);
        }

        // Программа написана как программа — запускаем её целиком.
        if program.main.is_some() || !program.algorithms.is_empty() {
            return self.run_program(&program);
        }

        // Объявлены только классы или импорты — выполнять нечего.
        Ok(Value::Null)
    }

    /// Выполняет распаршенную программу.
    pub fn run_program(&mut self, program: &Program) -> RuntimeResult<Value> {
        // Загружаем определения в среду
        self.load_program(program)?;

        // [KITE 11] Проверки ООП: финал-переопределение, абстрактные методы.
        self.validate_classes()?;

        // Выполняем глобальные инструкции (объявления перечислений и т.д.)
        for stmt in &program.globals {
            Executor::execute(stmt, &mut self.env)?;
        }

        // Ищем главный алгоритм
        if let Some(main) = &program.main {
            self.call_algorithm(&main.name, &[])
        } else if self.env.has_algorithm("Главный") {
            self.call_algorithm("Главный", &[])
        } else if self.env.has_algorithm("главный") {
            self.call_algorithm("главный", &[])
        } else if self.env.has_algorithm("Тест") {
            self.call_algorithm("Тест", &[])
        } else if self.env.has_algorithm("Main") {
            self.call_algorithm("Main", &[])
        } else if self.env.has_algorithm("main") {
            self.call_algorithm("main", &[])
        } else if !program.algorithms.is_empty() {
            // Ищем алгоритм без параметров
            for alg in &program.algorithms {
                if alg.params.is_empty() {
                    return self.call_algorithm(&alg.name, &[]);
                }
            }
            // Если все с параметрами - вызываем первый (возможно будет ошибка)
            self.call_algorithm(&program.algorithms[0].name, &[])
        } else {
            Ok(Value::Null)
        }
    }

    /// Загружает определения программы в среду.
    fn load_program(&mut self, program: &Program) -> RuntimeResult<()> {
        // Обрабатываем импорты
        for import in &program.imports {
            self.process_import(import)?;
        }

        // Загружаем алгоритмы
        for alg in &program.algorithms {
            self.env.define_algorithm(alg.clone());
        }

        // Загружаем перегруженные алгоритмы
        for overloaded in &program.overloaded_algorithms {
            for alg in &overloaded.overloads {
                self.env.define_algorithm(alg.clone());
            }
        }

        // Загружаем классы
        for class in &program.classes {
            self.env.define_class(class.clone());
        }

        // Загружаем главный алгоритм
        if let Some(main) = &program.main {
            self.env.define_algorithm(main.clone());
        }

        Ok(())
    }

    /// Вызывает алгоритм по имени.
    pub fn call_algorithm(&mut self, name: &str, args: &[Value]) -> RuntimeResult<Value> {
        let algorithm = self.env.get_algorithm(name)?.clone();

        // Проверяем количество аргументов
        if args.len() != algorithm.params.len() {
            return Err(RuntimeError::argument_count(
                name,
                algorithm.params.len(),
                args.len(),
            ));
        }

        // Создаём кадр вызова
        self.env.push_frame(algorithm.name.as_ref())?;

        // Привязываем параметры
        for (param, value) in algorithm.params.iter().zip(args.iter()) {
            self.env.define_local(param.name.to_string(), value.clone());
        }

        // Выполняем тело
        let result =
            Executor::execute_stmts(algorithm.body.as_deref().unwrap_or(&[]), &mut self.env);

        // Получаем возвращаемое значение
        let return_value = self.env.get_result_value().cloned();

        // Удаляем кадр
        self.env.pop_frame();

        // Обрабатываем результат
        match result {
            Ok(ControlFlow::Return(value)) => Ok(value.unwrap_or(Value::Null)),
            Ok(_) => Ok(return_value.unwrap_or(Value::Null)),
            // [KITE-0002] Сигнал оператора `?`: ранний возврат этого значения.
            Err(e) if e.is_propagation() => Ok(*e.propagate.expect("propagation carries a value")),
            Err(e) => Err(e),
        }
    }

    // =========================================================================
    //                    ВЫЧИСЛЕНИЕ ВЫРАЖЕНИЙ
    // =========================================================================

    /// Вычисляет выражение из строки.
    pub fn eval(&mut self, source: &str) -> RuntimeResult<Value> {
        let expr = parse_expression(source).map_err(|e| {
            RuntimeError::new(
                format!("Ошибка разбора выражения: {}", e),
                RuntimeErrorKind::Other,
            )
        })?;

        ExprEvaluator::evaluate(&expr, &mut self.env)
    }
}

// =============================================================================
//                           УДОБНЫЕ ФУНКЦИИ
// =============================================================================

/// Выполняет исходный код и возвращает результат.
pub fn run(source: &str) -> RuntimeResult<Value> {
    Interpreter::new().run(source)
}

/// Вычисляет выражение и возвращает результат.
pub fn eval(source: &str) -> RuntimeResult<Value> {
    Interpreter::new().eval(source)
}

/// Выполняет программу и возвращает вывод.
pub fn run_and_get_output(source: &str) -> RuntimeResult<String> {
    let mut interpreter = Interpreter::new();
    interpreter.run(source)?;
    Ok(interpreter.get_output())
}
