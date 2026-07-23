//! Program execution, interactive mode, and algorithm calls.

use super::Interpreter;
use super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::evaluator::ExprEvaluator;
use super::executor::Executor;
use shared::parser::{parse, parse_expression};
use shared::types::{Program, Value};

impl Interpreter {
    // =============================================================================
    //                         PROGRAM EXECUTION
    // =============================================================================

    /// Runs the source code of a program.
    pub fn run(&mut self, source: &str) -> RuntimeResult<Value> {
        let program = parse(source).map_err(|e| {
            RuntimeError::new(format!("Ошибка разбора: {}", e), RuntimeErrorKind::Other)
        })?;

        self.run_program(&program)
    }

    /// Runs a code fragment in interactive mode.
    ///
    /// Key difference from [`Self::run`]: consecutive commands share the same state.
    /// Normal execution wraps free statements in the main algorithm, and its stack
    /// frame is popped after execution — along with all declared variables. In the
    /// REPL, this meant `цел счётчик := 5` typed one line would not be visible the
    /// next line.
    ///
    /// Here, free statements execute directly in global scope; algorithm and class
    /// declarations work as usual (they are global anyway). A program with its own
    /// entry point (`алг главный`) runs fully: the user wrote it expecting full execution.
    pub fn run_interactive(&mut self, source: &str) -> RuntimeResult<Value> {
        let program = parse(source).map_err(|e| {
            RuntimeError::new(format!("Ошибка разбора: {}", e), RuntimeErrorKind::Other)
        })?;

        self.load_program(&program)?;
        self.validate_classes()?;

        for stmt in &program.globals {
            Executor::execute(stmt, &mut self.env)?;
        }

        // Free statements are collected by the parser into an anonymous algorithm.
        // Execute them directly without a frame — so declarations persist.
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

        // Has entry point — user wrote a program and expects it to run.
        if let Some(main) = &program.main {
            return self.call_algorithm(&main.name, &[]);
        }

        // Declarations only. Nothing to run: normal execution would call the first
        // algorithm it finds, making a declaration `алг цел удвоить(цел x)` fail with
        // "expected 1 argument, got 0". In the REPL, a declaration is just that.
        Ok(Value::Null)
    }

    /// Runs a parsed program.
    pub fn run_program(&mut self, program: &Program) -> RuntimeResult<Value> {
        self.load_program(program)?;

        // [KITE 11] OOP checks: final override, abstract methods.
        self.validate_classes()?;

        for stmt in &program.globals {
            Executor::execute(stmt, &mut self.env)?;
        }

        // Look for entry point
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
            // Look for a parameterless algorithm
            for alg in &program.algorithms {
                if alg.params.is_empty() {
                    return self.call_algorithm(&alg.name, &[]);
                }
            }
            // All have parameters; call the first (may fail)
            self.call_algorithm(&program.algorithms[0].name, &[])
        } else {
            Ok(Value::Null)
        }
    }

    /// Loads program definitions into the environment.
    fn load_program(&mut self, program: &Program) -> RuntimeResult<()> {
        for import in &program.imports {
            self.process_import(import)?;
        }

        for alg in &program.algorithms {
            self.env.define_algorithm(alg.clone());
        }

        for overloaded in &program.overloaded_algorithms {
            for alg in &overloaded.overloads {
                self.env.define_algorithm(alg.clone());
            }
        }

        for class in &program.classes {
            self.env.define_class(class.clone());
        }

        if let Some(main) = &program.main {
            self.env.define_algorithm(main.clone());
        }

        Ok(())
    }

    /// Calls an algorithm by name with arguments.
    pub fn call_algorithm(&mut self, name: &str, args: &[Value]) -> RuntimeResult<Value> {
        let algorithm = self.env.get_algorithm(name)?.clone();

        if args.len() != algorithm.params.len() {
            return Err(RuntimeError::argument_count(
                name,
                algorithm.params.len(),
                args.len(),
            ));
        }

        self.env.push_frame(algorithm.name.as_ref())?;

        for (param, value) in algorithm.params.iter().zip(args.iter()) {
            self.env.define_local(param.name.to_string(), value.clone());
        }

        let result =
            Executor::execute_stmts(algorithm.body.as_deref().unwrap_or(&[]), &mut self.env);

        let return_value = self.env.get_result_value().cloned();

        self.env.pop_frame();

        match result {
            Ok(ControlFlow::Return(value)) => Ok(value.unwrap_or(Value::Null)),
            Ok(_) => Ok(return_value.unwrap_or(Value::Null)),
            // [KITE-0002] `?` operator signal: early return of this value.
            Err(e) if e.is_propagation() => Ok(*e.propagate.expect("propagation carries a value")),
            Err(e) => Err(e),
        }
    }

    // =============================================================================
    //                        EXPRESSION EVALUATION
    // =============================================================================

    /// Evaluates an expression from source code.
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
//                        CONVENIENCE FUNCTIONS
// =============================================================================

/// Runs source code and returns the result.
pub fn run(source: &str) -> RuntimeResult<Value> {
    Interpreter::new().run(source)
}

/// Evaluates an expression and returns the result.
pub fn eval(source: &str) -> RuntimeResult<Value> {
    Interpreter::new().eval(source)
}

/// Runs a program and returns its output.
pub fn run_and_get_output(source: &str) -> RuntimeResult<String> {
    let mut interpreter = Interpreter::new();
    interpreter.run(source)?;
    Ok(interpreter.get_output())
}
