use super::ExprEvaluator;

use shared::types::{Expr, TypeKind, Value};

use super::super::environment::Environment;
use super::super::error::RuntimeResult;
use super::super::ops::TypeOps;

impl ExprEvaluator {
    // =========================================================================
    //                    ПРИВЕДЕНИЕ И ПРОВЕРКА ТИПОВ
    // =========================================================================

    pub(crate) fn eval_cast(
        expr: &Expr,
        target_type: &TypeKind,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let value = Self::evaluate(expr, env)?;
        TypeOps::cast(value, target_type)
    }

    pub(crate) fn eval_type_check(
        expr: &Expr,
        check_type: &TypeKind,
        env: &mut Environment,
    ) -> RuntimeResult<Value> {
        let value = Self::evaluate(expr, env)?;
        Ok(Value::Boolean(TypeOps::type_check(&value, check_type)))
    }
}
