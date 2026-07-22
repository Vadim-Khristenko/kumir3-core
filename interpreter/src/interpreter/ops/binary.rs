//! Бинарные операции над значениями (нелизивый хвост `eval_binary_op`).
//!
//! Разделение ответственности:
//! * **движок типов** (`shared::typesys`) — авторитет по *типизации*: определена
//!   ли операция для пары типов операндов и какого типа её результат;
//! * **`MathOperators`** (`shared::math`) — вычислительное ядро: как именно
//!   считать результат.
//!
//! Движок спрашивается ПЕРЕД вычислением, поэтому заведомо бестиповые операции
//! (`1 / "строка"`, `5 < "x"`) отбраковываются сразу и с точным сообщением,
//! называющим оператор и оба типа по-русски.
//!
//! Вердикт движка — ОКОНЧАТЕЛЬНЫЙ: у него есть правила для всех операций,
//! которые определяет ядро, включая строковые и табличные (`таб + таб`,
//! `лит * цел`, `лит / лит`, …) и тотальное равенство. Списка-исключения
//! («ядро умеет больше, чем движок») больше нет.

use shared::math::MathOperators;
use shared::types::{Token, Value};
use shared::typesys::{TypeError, TypeOp, default_engine};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl TypeOps {
    /// Применяет бинарный оператор к уже вычисленным операндам.
    ///
    /// Ленивые логические операции (`и`/`или`) обрабатываются на уровне
    /// вычислителя выражений; сюда попадают только строгие операции.
    pub fn binary(op: &Token, left: Value, right: Value) -> RuntimeResult<Value> {
        // [typesys-seam: подключён] типовой вердикт движка — до вычисления.
        if let Some(type_op) = Self::type_op(op)
            && let Err(err) =
                default_engine().result_of_binop(type_op, &left.type_kind(), &right.type_kind())
        {
            return Err(Self::describe(err));
        }

        Self::compute(op, left, right)
    }

    /// Сопоставляет токен оператора с оператором движка типов.
    ///
    /// `None` — оператор движку неизвестен (тогда типовой вердикт не
    /// запрашивается и работает прежняя диагностика «неизвестный оператор»).
    ///
    /// Вещественное `/` и целочисленное `див` — РАЗНЫЕ операторы движка
    /// ([`TypeOp::Div`] и [`TypeOp::IntDiv`]): у них разные требования к
    /// операндам и разный тип результата (KITE 13 §§ 3.4–3.5).
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

    /// Превращает типовую ошибку движка в диагностику времени выполнения.
    ///
    /// Движок сам называет оператор так, как он записан в программе
    /// ([`TypeOp::symbol`]), включая `див`.
    fn describe(err: TypeError) -> RuntimeError {
        RuntimeError::new(err.to_string(), RuntimeErrorKind::TypeMismatch)
    }

    /// Вычислительное ядро: как считать результат уже разрешённой операции.
    ///
    /// Ошибка ядра ([`shared::math::MathErr`]) несёт свой вид, поэтому
    /// `RuntimeError::from` расставляет `DivisionByZero`/`Overflow`/
    /// `TypeMismatch` (KITE-0014 § 3.2), а не схлопывает всё в `Other`.
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
