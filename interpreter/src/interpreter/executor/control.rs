//! Conditionals, error handling, and field assignment.

use super::super::environment::Environment;
use super::super::error::{ControlFlow, RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::evaluator::ExprEvaluator;
use super::Executor;
use shared::types::{Expr, Stmt, Value};

impl Executor {
    // =============================================================================
    //                         IF STATEMENT
    // =============================================================================

    pub(crate) fn execute_if(
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

    // =============================================================================
    //                           ASSERTIONS
    // =============================================================================

    pub(crate) fn execute_assert(expr: &Expr, env: &mut Environment) -> RuntimeResult<ControlFlow> {
        let value = ExprEvaluator::evaluate(expr, env)?;

        if !ExprEvaluator::is_truthy(&value) {
            return Err(RuntimeError::assertion_failed(&format!("{:?}", expr)));
        }

        Ok(ControlFlow::None)
    }

    // =============================================================================
    //                       ERROR HANDLING
    // =============================================================================

    pub(crate) fn execute_try_catch(
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
                // [KITE 4] Block scope: catch sees algorithm locals + error variable.
                env.push_scope();

                if let Some(var) = catch_var {
                    env.define_local(var.to_string(), Value::String(error.message));
                }

                let catch_result = Self::execute_stmts(catch_block, env);
                env.pop_scope();

                catch_result?
            }
        };

        // Execute finally if present.
        if let Some(finally_stmts) = finally_block {
            Self::execute_stmts(finally_stmts, env)?;
        }

        Ok(control_flow)
    }

    // =============================================================================
    //                     OOP: FIELD ASSIGNMENT
    // =============================================================================

    pub(crate) fn execute_field_assignment(
        object: &Expr,
        field: &str,
        value_expr: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<ControlFlow> {
        let value = ExprEvaluator::evaluate(value_expr, env)?;

        // Get the variable name containing the object.
        let var_name = match object {
            Expr::Variable(name) => name.clone(),
            Expr::SelfRef => {
                // Work with this.
                if let Some(this) = env.get_this().cloned()
                    && let Value::Object {
                        type_id,
                        mut fields,
                    } = this
                {
                    fields.insert(field.to_string(), value);
                    // Update this in the current frame.
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

        // Fetch the object.
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
}
