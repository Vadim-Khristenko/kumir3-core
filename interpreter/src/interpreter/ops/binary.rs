//! Бинарные операции над значениями (нелизивый хвост `eval_binary_op`).

use shared::math::MathOperators;
use shared::types::{Token, Value};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl TypeOps {
    /// Применяет бинарный оператор к уже вычисленным операндам.
    ///
    /// Ленивые логические операции (`и`/`или`) обрабатываются на уровне
    /// вычислителя выражений; сюда попадают только строгие операции.
    pub fn binary(op: &Token, left: Value, right: Value) -> RuntimeResult<Value> {
        // [typesys-seam] будущее: result_of_binop / coercion через shared::typesys.
        match op {
            // Арифметические операции
            Token::Plus => MathOperators::add(left, right, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),
            Token::Minus => MathOperators::sub(left, right, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),
            Token::Star => MathOperators::mul(left, right, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),
            Token::Slash => MathOperators::div(left, right, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),
            Token::Percent => MathOperators::modulus(left, right, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),
            Token::Power => MathOperators::pow(left, right, false)
                .map_err(|e| RuntimeError::new(e, RuntimeErrorKind::Other)),

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
