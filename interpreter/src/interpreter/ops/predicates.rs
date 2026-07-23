use std::cmp::Ordering;

use shared::f128::F128;
use shared::types::{Number, Value};
use shared::typesys::{TypeOp, default_engine};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

/// Numeric value in form suitable for lossless comparison.
enum Numeric {
    /// Integer representable in `i128` — all language integer types except `u128`
    /// with value exceeding `i128::MAX`.
    Int(i128),
    /// `u128` exceeds `i128::MAX`: signed type cannot hold it.
    Big(u128),
    /// Float; `float32` and `float64` embed in `float128` exactly.
    Real(F128),
}

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
    fn order(a: &Value, b: &Value) -> Option<Ordering> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => Some(Self::order_numbers(na, nb)),
            (Value::String(sa), Value::String(sb)) => Some(sa.cmp(sb)),
            (Value::Char(ca), Value::Char(cb)) => Some(ca.cmp(cb)),
            _ => None,
        }
    }

    /// Order of two numbers without intermediate f64 rounding.
    ///
    /// `NaN` preserves prior behavior: incomparable pair treated as equal
    /// (`partial_cmp` → `None` → [`Ordering::Equal`]).
    fn order_numbers(a: &Number, b: &Number) -> Ordering {
        match (Self::classify(a), Self::classify(b)) {
            (Numeric::Int(x), Numeric::Int(y)) => x.cmp(&y),
            (Numeric::Big(x), Numeric::Big(y)) => x.cmp(&y),
            // `Big` by definition exceeds `i128::MAX`, hence any `Int`.
            (Numeric::Int(_), Numeric::Big(_)) => Ordering::Less,
            (Numeric::Big(_), Numeric::Int(_)) => Ordering::Greater,
            (x, y) => Self::widen(x)
                .partial_cmp(&Self::widen(y))
                .unwrap_or(Ordering::Equal),
        }
    }

    /// Decomposes number to precise form for comparison.
    fn classify(n: &Number) -> Numeric {
        match n {
            Number::I8(v) => Numeric::Int(*v as i128),
            Number::I16(v) => Numeric::Int(*v as i128),
            Number::I32(v) => Numeric::Int(*v as i128),
            Number::I64(v) => Numeric::Int(*v as i128),
            Number::I128(v) => Numeric::Int(*v),
            Number::U8(v) => Numeric::Int(*v as i128),
            Number::U16(v) => Numeric::Int(*v as i128),
            Number::U32(v) => Numeric::Int(*v as i128),
            Number::U64(v) => Numeric::Int(*v as i128),
            Number::U128(v) => match i128::try_from(*v) {
                Ok(i) => Numeric::Int(i),
                Err(_) => Numeric::Big(*v),
            },
            Number::F32(v) => Numeric::Real(F128::from(*v)),
            Number::F64(v) => Numeric::Real(F128::from(*v)),
            Number::F128(v) => Numeric::Real(*v),
        }
    }

    /// Coerces any form to widest representation—`float128`.
    ///
    /// Integers up to 113 significant bits convert exactly; only `int128`/`u128`
    /// beyond this limit round (but then 60 bits more precise than prior `f64` coercion).
    fn widen(n: Numeric) -> F128 {
        match n {
            Numeric::Real(f) => f,
            Numeric::Int(i) => Self::int_to_real(i.unsigned_abs(), i < 0),
            Numeric::Big(u) => Self::int_to_real(u, false),
        }
    }

    /// Constructs `float128` from sign and integer magnitude, decomposing the magnitude
    /// into two 64-bit halves (both convert to `float128` exactly).
    fn int_to_real(magnitude: u128, negative: bool) -> F128 {
        /// 2^64—exactly representable in both `f64` and `float128`.
        const TWO_POW_64: f64 = 18_446_744_073_709_551_616.0;

        let low = F128::from(magnitude as u64);
        let high = (magnitude >> 64) as u64;
        let value = if high == 0 {
            low
        } else {
            F128::from(high) * F128::from(TWO_POW_64) + low
        };
        if negative { -value } else { value }
    }

    /// Converts value to index.
    pub fn to_index(value: &Value) -> RuntimeResult<i64> {
        value
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))
    }
}
