use std::cmp::Ordering;

use shared::f128::F128;
use shared::types::{Number, Value};
use shared::typesys::{TypeOp, default_engine};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl TypeOps {
    /// Checks value truthiness.
    pub fn is_truthy(value: &Value) -> bool {
        match value {
            Value::Boolean(b) => *b,
            Value::Number(n) => n.to_f64().map(|f| f != 0.0).unwrap_or(false),
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Null | Value::Undefined => false,
            Value::Option(opt) => opt.is_some(),
            _ => true,
        }
    }

    /// Compares two values for equality.
    ///
    /// Equality is **total**: engine ([`TypeOp::Eq`]) defines it for any
    /// type pair and never returns error, so verdict is not queried here—there is
    /// nothing to ask. Totality is fixed by test `char_engine_equality_is_total_for_every_type_pair`.
    pub fn values_equal(a: &Value, b: &Value) -> bool {
        a == b
    }

    /// Compares two values (`<`, `<=`, `>`, `>=`).
    pub fn compare<F>(a: &Value, b: &Value, cmp: F) -> RuntimeResult<Value>
    where
        F: Fn(Ordering) -> bool,
    {
        // [typesys-seam: подключён] Engine type verdict—before comparison.
        Self::check_ordered(a, b)?;
        let ordering = Self::order(a, b).ok_or_else(Self::incomparable)?;
        Ok(Value::Boolean(cmp(ordering)))
    }

    /// Asks engine if ordering is defined for operand types.
    ///
    /// All four ordering operators share one typing rule, so it suffices to ask about [`TypeOp::Lt`].
    /// Precise operator name in diagnostics is provided by [`TypeOps::binary`]: it asks engine
    /// earlier with the operator as written in the program; the message from here is seen
    /// only on direct `compare` call.
    fn check_ordered(a: &Value, b: &Value) -> RuntimeResult<()> {
        default_engine()
            .result_of_binop(TypeOp::Lt, &a.type_kind(), &b.type_kind())
            .map(|_| ())
            .map_err(|err| RuntimeError::new(err.to_string(), RuntimeErrorKind::TypeMismatch))
    }

    /// Diagnostic for a pair that engine passed but kernel cannot order.
    /// Unreachable: engine's set of ordered types (number, `string`, `char`)
    /// matches the set of branches in [`TypeOps::order`].
    fn incomparable() -> RuntimeError {
        RuntimeError::type_mismatch("сравнимые типы", "несравнимые типы")
    }

    /// Computation kernel: order of two values already approved by engine.
    ///
    /// Numeric order lives in [`Number::order`] so that the comparison
    /// operators, equality, sorting and hashing cannot drift apart.
    fn order(a: &Value, b: &Value) -> Option<Ordering> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => Some(na.order(nb)),
            (Value::String(sa), Value::String(sb)) => Some(sa.cmp(sb)),
            (Value::Char(ca), Value::Char(cb)) => Some(ca.cmp(cb)),
            _ => None,
        }
    }

    /// Converts value to index.
    pub fn to_index(value: &Value) -> RuntimeResult<i64> {
        value
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))
    }
}
