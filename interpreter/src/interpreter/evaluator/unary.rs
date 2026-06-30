use super::ExprEvaluator;

use shared::types::{Expr, Token, Value};

use super::super::environment::Environment;
use super::super::error::RuntimeResult;
use super::super::ops::TypeOps;

impl ExprEvaluator {
    // =========================================================================
    //                    УНАРНЫЕ ОПЕРАЦИИ
    // =========================================================================

    pub(crate) fn eval_unary_op(
        op: &Token,
        operand: &Expr,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let value = Self::evaluate(operand, env)?;
        TypeOps::unary(op, value)
    }
}
