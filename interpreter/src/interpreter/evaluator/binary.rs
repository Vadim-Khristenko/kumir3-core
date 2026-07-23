use super::ExprEvaluator;

use shared::types::{Expr, Token, Value};

use super::super::environment::Environment;
use super::super::error::RuntimeResult;
use super::super::ops::TypeOps;

impl ExprEvaluator {
    // =============================================================================
    //         SECTION: BINARY OPERATIONS
    // =============================================================================

    pub(crate) fn eval_binary_op(
        left: &Expr,
        op: &Token,
        right: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        // Lazy evaluation for logical operations.
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

        // Evaluate both operands.
        let left_val = Self::evaluate(left, env)?;
        let right_val = Self::evaluate(right, env)?;

        TypeOps::binary(op, left_val, right_val)
    }
}
