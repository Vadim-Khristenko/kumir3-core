use shared::math::MathOperators;
use shared::types::{Token, Value};
use shared::typesys::{TypeError, TypeOp, default_engine};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl TypeOps {
    /// Applies binary operator to already-evaluated operands.
    ///
    /// Lazy logical operations (`and`/`or`) are handled at the expression evaluator level;
    /// only strict operations reach here.
    pub fn binary(op: &Token, left: Value, right: Value) -> RuntimeResult<Value> {
        // [typesys-seam: подключён] Engine type verdict—before computation.
        if let Some(type_op) = Self::type_op(op)
            && let Err(err) =
                default_engine().result_of_binop(type_op, &left.type_kind(), &right.type_kind())
        {
            return Err(Self::describe(err));
        }

        Self::compute(op, left, right)
    }

    /// Maps operator token to type engine operator.
    ///
    /// `None`—operator unknown to engine (then type verdict is not queried and
    /// earlier "unknown operator" diagnostic applies).
    ///
    /// Float `/` and integer `div` are DIFFERENT engine operators
    /// ([`TypeOp::Div`] and [`TypeOp::IntDiv`]): they have different operand requirements
    /// and different result type (KITE 13 §§ 3.4–3.5).
    fn type_op(op: &Token) -> Option<TypeOp> {
        Some(match op {
            Token::Plus => TypeOp::Add,
            Token::Minus => TypeOp::Sub,
            Token::Star => TypeOp::Mul,
            Token::Slash => TypeOp::Div,
            Token::IntDiv => TypeOp::IntDiv,
            Token::Percent => TypeOp::Mod,
            Token::Power => TypeOp::Pow,
            Token::Equal => TypeOp::Eq,
            Token::NotEqual => TypeOp::Ne,
            Token::Less => TypeOp::Lt,
            Token::LessEqual => TypeOp::Le,
            Token::Greater => TypeOp::Gt,
            Token::GreaterEqual => TypeOp::Ge,
            _ => return None,
        })
    }

    /// Converts engine type error to runtime diagnostics.
    ///
    /// Engine itself names the operator as written in the program
    /// ([`TypeOp::symbol`]), including `div`.
    fn describe(err: TypeError) -> RuntimeError {
        RuntimeError::new(err.to_string(), RuntimeErrorKind::TypeMismatch)
    }

    /// Computation kernel: how to compute result of already-approved operation.
    ///
    /// Kernel error ([`shared::math::MathErr`]) carries its own kind, so
    /// `RuntimeError::from` assigns `DivisionByZero`/`Overflow`/
    /// `TypeMismatch` (KITE-0014 § 3.2), not collapsing everything to `Other`.
    fn compute(op: &Token, left: Value, right: Value) -> RuntimeResult<Value> {
        match op {
            // Арифметические операции
            Token::Plus => MathOperators::add(left, right, false).map_err(RuntimeError::from),
            Token::Minus => MathOperators::sub(left, right, false).map_err(RuntimeError::from),
            Token::Star => MathOperators::mul(left, right, false).map_err(RuntimeError::from),
            Token::Slash => MathOperators::div(left, right, false).map_err(RuntimeError::from),
            Token::IntDiv => MathOperators::int_div(left, right, false).map_err(RuntimeError::from),
            Token::Percent => {
                MathOperators::modulus(left, right, false).map_err(RuntimeError::from)
            }
            Token::Power => MathOperators::pow(left, right, false).map_err(RuntimeError::from),

            // Сравнения
            Token::Equal => Ok(Value::Boolean(TypeOps::values_equal(&left, &right))),
            Token::NotEqual => Ok(Value::Boolean(!TypeOps::values_equal(&left, &right))),
            Token::Less => TypeOps::compare(&left, &right, |o| o.is_lt()),
            Token::Greater => TypeOps::compare(&left, &right, |o| o.is_gt()),
            Token::LessEqual => TypeOps::compare(&left, &right, |o| o.is_le()),
            Token::GreaterEqual => TypeOps::compare(&left, &right, |o| o.is_ge()),

            _ => Err(RuntimeError::new(
                format!("Неизвестный бинарный оператор: {:?}", op),
                RuntimeErrorKind::Other,
            )),
        }
    }
}
