//! Исполнитель инструкций для интерпретатора Кумир 3
//!
//! Реализует выполнение всех типов инструкций: присваивание, условия,
//! циклы, ввод/вывод, обработка исключений и т.д.

use shared::types::{Stmt, Value};

use super::environment::Environment;
use super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::evaluator::ExprEvaluator;

mod control;
mod import;
mod io;
mod loops;
mod r#match;
mod rust_block;
mod statements;

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

            Stmt::Pause => Self::execute_pause(env),

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
            } => Self::execute_module_decl(name, body, algorithms, env),

            Stmt::Export { names } => Self::execute_export(names, env),

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

            // ===== МЕТА / ОТЛАДКА =====
            Stmt::TypeAlias { .. } => {
                // Псевдонимы типов — чисто compile-time конструкция.
                Ok(ControlFlow::None)
            }

            // Все остальные инструкции (не реализованы)
            _ => Err(RuntimeError::not_implemented("данная инструкция")),
        }
    }
}
