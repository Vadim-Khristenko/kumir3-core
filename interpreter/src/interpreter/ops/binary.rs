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
            && !Self::kernel_extension(op, &left, &right)
        {
            return Err(Self::describe(op, err));
        }

        Self::compute(op, left, right)
    }

    /// Сопоставляет токен оператора с оператором движка типов.
    ///
    /// `None` — оператор движку неизвестен (тогда типовой вердикт не
    /// запрашивается и работает прежняя диагностика «неизвестный оператор»).
    ///
    /// Замечание о делении: язык различает вещественное `/` и целочисленное
    /// `див`, а движок — нет (у него один [`TypeOp::Div`]). Для вопроса
    /// «определена ли операция» этого достаточно: обе требуют числовых
    /// операндов. Тип результата у движка НЕ берётся — он различается (`/`
    /// всегда вещественное, `див` всегда целое) и обеспечивается ядром.
    fn type_op(op: &Token) -> Option<TypeOp> {
        Some(match op {
            Token::Plus => TypeOp::Add,
            Token::Minus => TypeOp::Sub,
            Token::Star => TypeOp::Mul,
            Token::Slash | Token::IntDiv => TypeOp::Div,
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

    /// Операции, которые вычислительное ядро определяет ШИРЕ структурных
    /// правил движка. Для них отрицательный вердикт движка не является вето.
    ///
    /// Список исчерпывающе повторяет «нечисловые» ветви `shared::math`:
    /// * `+` / `-` над массивами — конкатенация и удаление элементов;
    /// * `-` строки из строки — удаление всех вхождений подстроки;
    /// * `*` строки на число — повторение строки;
    /// * `/` строки на число — разрезание на части, `/` строки на строку —
    ///   разбиение по разделителю;
    /// * `=` / `<>` — структурное равенство тотально и определено для любой
    ///   пары значений (разнотипные значения просто не равны).
    ///
    /// Целочисленное `див` сюда НЕ входит: у ядра нет строковых/массивных
    /// ветвей для него.
    fn kernel_extension(op: &Token, left: &Value, right: &Value) -> bool {
        matches!(
            (op, left, right),
            (Token::Plus | Token::Minus, Value::Array(_), Value::Array(_))
                | (Token::Minus, Value::String(_), Value::String(_))
                | (Token::Star, Value::String(_), Value::Number(_))
                | (Token::Star, Value::Number(_), Value::String(_))
                | (Token::Slash, Value::String(_), Value::Number(_))
                | (Token::Slash, Value::String(_), Value::String(_))
                | (Token::Equal | Token::NotEqual, _, _)
        )
    }

    /// Превращает типовую ошибку движка в диагностику времени выполнения.
    ///
    /// Оператор называется так, как он записан в программе: движок знает
    /// только [`TypeOp::symbol`], где целочисленное деление неотличимо от `/`.
    fn describe(op: &Token, err: TypeError) -> RuntimeError {
        let message = match &err {
            TypeError::BinaryOpUnsupported {
                op: symbol,
                left,
                right,
            } => {
                let symbol = if matches!(op, Token::IntDiv) {
                    "див"
                } else {
                    symbol
                };
                format!(
                    "Операция '{}' не определена для типов '{}' и '{}'",
                    symbol, left, right
                )
            }
            other => other.to_string(),
        };
        RuntimeError::new(message, RuntimeErrorKind::TypeMismatch)
    }

    /// Вычислительное ядро: как считать результат уже разрешённой операции.
    fn compute(op: &Token, left: Value, right: Value) -> RuntimeResult<Value> {
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
            Token::IntDiv => MathOperators::int_div(left, right, false)
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
