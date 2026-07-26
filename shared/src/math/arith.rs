// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Basic arithmetic: addition, subtraction, multiplication, division, integer division, modulus, and exponentiation.

// =============================================================================
//         SECTION: IMPORTS
// =============================================================================

use super::MathErr;
use super::MathOperators;
use crate::types::{Number, Value};
use std::collections::HashMap;

// =============================================================================
//         SECTION: CORE LOGIC
// =============================================================================

impl MathOperators {
    /// [STABLE] Performs addition with overflow control.
    ///
    /// # Arguments
    /// * `a` - First value
    /// * `b` - Second value
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of addition
    /// * `Err(MathErr)` - Error message
    pub fn add(a: Value, b: Value, fo_e: bool) -> Result<Value, MathErr> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                Self::num_add(na, nb, fo_e).map(Value::Number)
            }
            (Value::String(sa), Value::String(sb)) => Ok(Value::String(sa + &sb)),
            (Value::Array(mut va), Value::Array(vb)) => {
                va.extend(vb);
                Ok(Value::Array(va))
            }
            _ => Err(MathErr::TypeMismatch("операция сложения")),
        }
    }

    /// [STABLE] Performs subtraction with overflow control.
    ///
    /// # Arguments
    /// * `a` - Minuend value
    /// * `b` - Subtrahend value
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of subtraction
    /// * `Err(MathErr)` - Error message
    pub fn sub(a: Value, b: Value, fo_e: bool) -> Result<Value, MathErr> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                Self::num_sub(na, nb, fo_e).map(Value::Number)
            }
            (Value::String(sa), Value::String(sb)) => {
                if sb.is_empty() {
                    Ok(Value::String(sa))
                } else {
                    let res = Self::remove_all_substring_bytes(sa, &sb);
                    Ok(Value::String(res))
                }
            }
            (Value::Array(mut va), Value::Array(vb)) => {
                if vb.len() <= 8 {
                    for item in vb {
                        if let Some(pos) = va.iter().position(|x| x == &item) {
                            va.remove(pos);
                        }
                    }
                    return Ok(Value::Array(va));
                }
                let mut counts: HashMap<String, usize> = HashMap::new();
                for item in &vb {
                    let key = Self::value_key(item);
                    *counts.entry(key).or_insert(0) += 1;
                }
                let mut out: Vec<Value> = Vec::with_capacity(va.len());
                for v in va.into_iter() {
                    let key = Self::value_key(&v);
                    if let Some(cnt) = counts.get_mut(&key)
                        && *cnt > 0
                    {
                        *cnt -= 1;
                        continue;
                    }
                    out.push(v);
                }
                Ok(Value::Array(out))
            }
            _ => Err(MathErr::TypeMismatch("операция вычитания")),
        }
    }

    /// [STABLE] Performs multiplication with overflow control.
    ///
    /// # Arguments
    /// * `a` - First factor
    /// * `b` - Second factor
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of multiplication
    /// * `Err(MathErr)` - Error message
    pub fn mul(a: Value, b: Value, fo_e: bool) -> Result<Value, MathErr> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                Self::num_mul(na, nb, fo_e).map(Value::Number)
            }
            (Value::String(sa), Value::Number(nb)) => Self::str_mul_string_number(sa, nb, fo_e),
            (Value::Number(na), Value::String(sb)) => Self::str_mul_string_number(sb, na, fo_e),
            _ => Err(MathErr::TypeMismatch("операция умножения")),
        }
    }

    /// [STABLE] Performs division with overflow control.
    ///
    /// # Arguments
    /// * `a` - Dividend
    /// * `b` - Divisor (must not be zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of division
    /// * `Err(MathErr)` - Error message (division by zero, overflow)
    pub fn div(a: Value, b: Value, fo_e: bool) -> Result<Value, MathErr> {
        match (a, b) {
            (Value::Number(_), Value::Number(nb)) if Self::is_zero_num(&nb) => {
                Err(MathErr::DivisionByZero)
            }
            (Value::Number(na), Value::Number(nb)) => {
                Self::num_div(na, nb, fo_e).map(Value::Number)
            }
            (Value::String(sa), Value::Number(nb)) => Self::str_div_string_number(sa, nb, fo_e),
            (Value::String(sa), Value::String(sb)) => Self::str_div_string_delim(sa, sb, fo_e),
            _ => Err(MathErr::TypeMismatch("операция деления")),
        }
    }

    /// [STABLE] Performs modulus operation.
    ///
    /// # Arguments
    /// * `a` - Dividend
    /// * `b` - Divisor (must not be zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Remainder of division
    /// * `Err(MathErr)` - Error message
    pub fn modulus(a: Value, b: Value, fo_e: bool) -> Result<Value, MathErr> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                Self::num_mod(na, nb, fo_e).map(Value::Number)
            }
            _ => Err(MathErr::TypeMismatch("операция взятия остатка")),
        }
    }

    /// [STABLE] Performs integer division (integer quotient).
    ///
    /// The counterpart to [`modulus`](Self::modulus): both operate on integers
    /// only and truncate toward zero, so the identity
    /// `a == (a int_div b) * b + (a modulus b)` holds.
    ///
    /// # Arguments
    /// * `a` - Dividend
    /// * `b` - Divisor (must not be zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Integer quotient
    /// * `Err(MathErr)` - Error message (division by zero, type mismatch)
    pub fn int_div(a: Value, b: Value, fo_e: bool) -> Result<Value, MathErr> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                Self::num_int_div(na, nb, fo_e).map(Value::Number)
            }
            _ => Err(MathErr::TypeMismatch("операция целочисленного деления")),
        }
    }

    /// [STABLE] Performs exponentiation.
    ///
    /// # Arguments
    /// * `a` - Base
    /// * `b` - Exponent
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of exponentiation
    /// * `Err(MathErr)` - Error message
    pub fn pow(a: Value, b: Value, fo_e: bool) -> Result<Value, MathErr> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => Self::num_pow(na, nb, fo_e),
            _ => Err(MathErr::TypeMismatch("операция возведения в степень")),
        }
    }

    /// Internal numeric addition with overflow control.
    ///
    /// # Arguments
    /// * `a` - First number
    /// * `b` - Second number
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Number, MathErr>` - Result of addition or error message
    pub(super) fn num_add(a: Number, b: Number, fo_e: bool) -> Result<Number, MathErr> {
        Self::int_or_float(a, b, fo_e, |x, y| x.wrapping_add(y), |x, y| x + y)
    }

    /// Internal numeric subtraction with overflow control.
    ///
    /// # Arguments
    /// * `a` - Minuend
    /// * `b` - Subtrahend
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Number, MathErr>` - Result of subtraction or error message
    pub(super) fn num_sub(a: Number, b: Number, fo_e: bool) -> Result<Number, MathErr> {
        Self::int_or_float(a, b, fo_e, |x, y| x.wrapping_sub(y), |x, y| x - y)
    }

    /// Internal numeric multiplication with overflow control.
    ///
    /// # Arguments
    /// * `a` - First factor
    /// * `b` - Second factor
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Number, MathErr>` - Result of multiplication or error message
    pub(super) fn num_mul(a: Number, b: Number, fo_e: bool) -> Result<Number, MathErr> {
        Self::int_or_float(a, b, fo_e, |x, y| x.wrapping_mul(y), |x, y| x * y)
    }

    /// Internal numeric division with overflow control.
    ///
    /// # Arguments
    /// * `a` - Dividend
    /// * `b` - Divisor (must not be zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Number, MathErr>` - Result of division or error message
    pub(super) fn num_div(a: Number, b: Number, fo_e: bool) -> Result<Number, MathErr> {
        if Self::is_zero_num(&b) {
            return Err(MathErr::DivisionByZero);
        }
        let fa = Self::to_f128_full(&a);
        let fb = Self::to_f128_full(&b);
        let r = fa / fb;
        if (r.is_infinite() || r.is_nan()) && fo_e {
            return Err(MathErr::FloatOverflow);
        }
        Ok(Number::F128(r))
    }

    /// Internal numeric modulus operation.
    ///
    /// # Arguments
    /// * `a` - Dividend
    /// * `b` - Divisor (must not be zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Number, MathErr>` - Remainder or error
    pub(super) fn num_mod(a: Number, b: Number, fo_e: bool) -> Result<Number, MathErr> {
        use self::Number::*;
        if Self::is_zero_num(&b) {
            return Err(MathErr::DivisionByZero);
        }
        // `wrapping_rem` вместо `%`: делитель уже проверен на ноль, и остаётся
        // единственная пара, на которой `%` паникует, — MIN и -1. Остаток там
        // представим и равен нулю (непредставимо только частное, см.
        // [`num_int_div`]), а `wrapping_rem` этот ноль и возвращает. Взять
        // `checked_rem` было бы неверно: он на той же паре отдаёт `None`,
        // и правильный ответ превратился бы в ошибку переполнения.
        match (a, b) {
            (I8(x), I8(y)) => Ok(I8(x.wrapping_rem(y))),
            (I16(x), I16(y)) => Ok(I16(x.wrapping_rem(y))),
            (I32(x), I32(y)) => Ok(I32(x.wrapping_rem(y))),
            (I64(x), I64(y)) => Ok(I64(x.wrapping_rem(y))),
            (I128(x), I128(y)) => Ok(I128(x.wrapping_rem(y))),
            (U8(x), U8(y)) => Ok(U8(x % y)),
            (U16(x), U16(y)) => Ok(U16(x % y)),
            (U32(x), U32(y)) => Ok(U32(x % y)),
            (U64(x), U64(y)) => Ok(U64(x % y)),
            (U128(x), U128(y)) => Ok(U128(x % y)),
            _ => Err(MathErr::TypeMismatch("остаток только для целых")),
        }
    }

    /// Internal integer division (integer quotient).
    ///
    /// Mirrors [`num_mod`](Self::num_mod): integer-only, truncating toward zero
    /// (Rust `/`), so `a == (a / b) * b + (a % b)`.
    ///
    /// # Arguments
    /// * `a` - Dividend
    /// * `b` - Divisor (must not be zero)
    /// * `_fo_e` - Overflow flag (unused; kept for signature parity with siblings)
    ///
    /// # Returns
    /// * `Result<Number, MathErr>` - Integer quotient or error
    pub(super) fn num_int_div(a: Number, b: Number, _fo_e: bool) -> Result<Number, MathErr> {
        use self::Number::*;
        if Self::is_zero_num(&b) {
            return Err(MathErr::DivisionByZero);
        }
        // `checked_div` — по той же причине, что и в [`num_mod`]: частное
        // MIN / -1 не представимо в знаковом типе, и без проверки это паника.
        match (a, b) {
            (I8(x), I8(y)) => x.checked_div(y).map(I8).ok_or(MathErr::Overflow),
            (I16(x), I16(y)) => x.checked_div(y).map(I16).ok_or(MathErr::Overflow),
            (I32(x), I32(y)) => x.checked_div(y).map(I32).ok_or(MathErr::Overflow),
            (I64(x), I64(y)) => x.checked_div(y).map(I64).ok_or(MathErr::Overflow),
            (I128(x), I128(y)) => x.checked_div(y).map(I128).ok_or(MathErr::Overflow),
            (U8(x), U8(y)) => Ok(U8(x / y)),
            (U16(x), U16(y)) => Ok(U16(x / y)),
            (U32(x), U32(y)) => Ok(U32(x / y)),
            (U64(x), U64(y)) => Ok(U64(x / y)),
            (U128(x), U128(y)) => Ok(U128(x / y)),
            _ => Err(MathErr::TypeMismatch(
                "целочисленное деление только для целых",
            )),
        }
    }

    /// Internal numeric exponentiation.
    ///
    /// # Arguments
    /// * `a` - Base
    /// * `b` - Exponent
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Value, MathErr>` - Result of exponentiation or error message
    pub(super) fn num_pow(a: Number, b: Number, fo_e: bool) -> Result<Value, MathErr> {
        let fa = Self::to_f128_full(&a);
        let fb = Self::to_f128_full(&b);

        if fa.is_sign_negative() && !Self::is_effectively_integer(fb) && fo_e {
            return Err(MathErr::NegativePowNonInteger);
        }

        let r = fa.powf(fb);
        if (r.is_infinite() || r.is_nan()) && fo_e {
            return Err(MathErr::FloatOverflow);
        }
        Ok(Value::Number(Number::F128(r)))
    }
}
