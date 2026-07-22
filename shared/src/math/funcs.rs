// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Математические функции: корни, округление, тригонометрия, модуль.

// =============================================================================
//         IMPORTS
// =============================================================================

use super::MathErr;
use super::MathOperators;
use crate::f128::F128 as tF128;
use crate::types::{Number, Value};

// =============================================================================
//         CORE LOGIC
// =============================================================================

impl MathOperators {
    /// [STABLE] Computes square root.
    ///
    /// # Arguments
    /// * `a` - Value to take square root of (must be non-negative)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Square root result
    /// * `Err(String)` - Error message (negative input, overflow)
    pub fn sqrt(a: Value, fo_e: bool) -> Result<Value, String> {
        match a {
            Value::Number(n) => Self::num_sqrt(n, fo_e),
            _ => Err(MathErr::TypeMismatch("sqrt ожидает число").msg()),
        }
    }

    /// [STABLE] Computes nth root.
    ///
    /// # Arguments
    /// * `a` - Value to take root of
    /// * `n` - Root degree (must be non-zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Nth root result
    /// * `Err(String)` - Error message
    pub fn root(a: Value, n: Value, fo_e: bool) -> Result<Value, String> {
        let n_int = match n {
            Value::Number(Number::I32(v)) => v as i64,
            Value::Number(Number::I64(v)) => v,
            Value::Number(Number::I128(v)) => {
                if v >= i64::MIN as i128 && v <= i64::MAX as i128 {
                    v as i64
                } else {
                    return Err(MathErr::DomainError("слишком большая степень корня").msg());
                }
            }
            _ => return Err(MathErr::TypeMismatch("root ожидает целую степень").msg()),
        };
        if n_int == 0 {
            return Err(MathErr::DomainError("корень нулевой степени не определён").msg());
        }
        match a {
            Value::Number(num) => Self::num_root(num, n_int, fo_e),
            _ => Err(MathErr::TypeMismatch("root ожидает число").msg()),
        }
    }

    /// [STABLE] Rounds a number to specified precision.
    ///
    /// # Arguments
    /// * `a` - Value to round
    /// * `b` - Optional precision (decimal places if positive, significant digits if negative)
    /// * `rf` - Optional rounding factor (1-9, default 5)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Rounded value
    /// * `Err(String)` - Error message
    pub fn round(
        a: Value,
        b: Option<Value>,
        rf: Option<Value>,
        _fo_e: bool,
    ) -> Result<Value, String> {
        let prec: i32 = match b {
            Some(Value::Number(nb)) => match Self::to_i128(&nb) {
                Some(v) => v as i32,
                None => {
                    return Err(
                        MathErr::TypeMismatch("round: точность должна быть целым числом").msg(),
                    );
                }
            },
            Some(_) => {
                return Err(MathErr::TypeMismatch("round: точность должна быть числом").msg());
            }
            None => 0,
        };

        let rf_val: i8 = match rf {
            Some(Value::Number(nr)) => match Self::to_i128(&nr) {
                Some(v) => v as i8,
                None => {
                    return Err(MathErr::TypeMismatch("round: rf должна быть целым числом").msg());
                }
            },
            Some(_) => return Err(MathErr::TypeMismatch("round: rf должна быть числом").msg()),
            None => 5,
        };

        if !(1..=9).contains(&rf_val) {
            return Err(MathErr::DomainError("параметр rf должен быть в диапазоне 1..9").msg());
        }

        match a {
            Value::Number(n) => {
                let res = Self::num_round(n, prec, rf_val)?;
                Ok(Value::Number(res))
            }
            _ => Err(MathErr::TypeMismatch("round ожидает число").msg()),
        }
    }

    /// [STABLE] Computes sine of an angle.
    ///
    /// # Arguments
    /// * `a` - Angle in radians
    ///
    /// # Returns
    /// * `Ok(Value)` - Sine value
    /// * `Err(String)` - Error message
    pub fn sin(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_sin(n))),
            _ => Err(MathErr::TypeMismatch("sin ожидает число").msg()),
        }
    }

    /// [STABLE] Computes cosine of an angle.
    ///
    /// # Arguments
    /// * `a` - Angle in radians
    ///
    /// # Returns
    /// * `Ok(Value)` - Cosine value
    /// * `Err(String)` - Error message
    pub fn cos(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_cos(n))),
            _ => Err(MathErr::TypeMismatch("cos ожидает число").msg()),
        }
    }

    /// [STABLE] Computes tangent of an angle.
    ///
    /// # Arguments
    /// * `a` - Angle in radians
    ///
    /// # Returns
    /// * `Ok(Value)` - Tangent value
    /// * `Err(String)` - Error message
    pub fn tg(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_tan(n))),
            _ => Err(MathErr::TypeMismatch("tg ожидает число").msg()),
        }
    }

    /// [STABLE] Computes cotangent of an angle.
    ///
    /// # Arguments
    /// * `a` - Angle in radians (must not be multiple of π)
    ///
    /// # Returns
    /// * `Ok(Value)` - Cotangent value
    /// * `Err(String)` - Error message (division by zero)
    pub fn ctg(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Self::num_ctg(n),
            _ => Err(MathErr::TypeMismatch("ctg ожидает число").msg()),
        }
    }

    /// [STABLE] Computes absolute value.
    ///
    /// # Arguments
    /// * `a` - Numeric value
    ///
    /// # Returns
    /// * `Ok(Value)` - Absolute value
    /// * `Err(String)` - Error message
    pub fn abs(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_abs(n))),
            _ => Err(MathErr::TypeMismatch("abs ожидает число").msg()),
        }
    }

    /// Internal square root computation.
    ///
    /// # Arguments
    /// * `n` - Value to take square root of (must be non-negative)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Value, String>` - Square root result or error message
    pub(super) fn num_sqrt(n: Number, fo_e: bool) -> Result<Value, String> {
        let x = Self::to_f128_full(&n);
        // Easter egg: calc sqrt(-1) -> return specific NotRealOneSqrt error
        if x == tF128::from(-1.0_f64) {
            return Err(MathErr::NotRealOneSqrt.msg());
        }
        if x.is_sign_negative() {
            return Err(MathErr::NegativeSqrt.msg());
        }
        let r = x.sqrt();
        if (r.is_infinite() || r.is_nan()) && fo_e {
            return Err(MathErr::FloatOverflow.msg());
        }
        Ok(Value::Number(Number::F128(r)))
    }

    /// Internal nth root computation.
    ///
    /// # Arguments
    /// * `n` - Value to take root of
    /// * `k` - Root degree (must be non-zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Value, String>` - Nth root result or error message
    pub(super) fn num_root(n: Number, k: i64, fo_e: bool) -> Result<Value, String> {
        let x = Self::to_f128_full(&n);
        if k == 0 {
            return Err(MathErr::DomainError("root(x,0) не определён").msg());
        }
        if x.is_sign_negative() && k % 2 == 0 {
            return Err(MathErr::NegativeRoot.msg());
        }
        let kf = k as f64;
        let r = x.powf(tF128::from(1.0_f64 / kf));
        if (r.is_infinite() || r.is_nan()) && fo_e {
            return Err(MathErr::FloatOverflow.msg());
        }
        Ok(Value::Number(Number::F128(r)))
    }

    /// Internal number rounding to specified precision.
    ///
    /// # Arguments
    /// * `n` - Number to round
    /// * `prec` - Precision (decimal places if positive, significant digits if negative)
    /// * `rf` - Rounding factor (1-9, default 5)
    ///
    /// # Returns
    /// * `Result<Number, String>` - Rounded number or error message
    pub(super) fn num_round(n: Number, prec: i32, rf: i8) -> Result<Number, String> {
        match n {
            Number::F128(v) => {
                let rounded = Self::round_decimal_f128(v, prec, rf);
                Ok(Number::F128(rounded))
            }
            Number::F64(v) => {
                let rounded = Self::round_decimal_f64(v, prec, rf);
                Ok(Number::F64(rounded))
            }
            Number::F32(v) => {
                let rounded = Self::round_decimal_f64(v as f64, prec, rf) as f32;
                Ok(Number::F32(rounded))
            }
            _ => {
                if prec >= 0 {
                    Ok(n)
                } else {
                    Self::round_integer(n, prec, rf)
                }
            }
        }
    }

    /// Rounds a F128 decimal number to specified precision.
    ///
    /// # Arguments
    /// * `v` - F128 value to round
    /// * `prec` - Decimal precision
    /// * `rf` - Rounding factor (1-9)
    ///
    /// # Returns
    /// * `tF128` - Rounded F128 value
    pub(super) fn round_decimal_f128(v: tF128, prec: i32, rf: i8) -> tF128 {
        if !v.is_finite() || prec < 0 {
            return if prec < 0 {
                Self::round_f128_to_power_of_10(v, -prec, rf)
            } else {
                v
            };
        }

        if rf == 5 {
            return v.round_to(prec);
        }

        let factor = tF128::from(10u64).powi(prec);
        if !factor.is_finite() || factor.is_zero() {
            return v;
        }

        let scaled = v * factor;
        let truncated = scaled.trunc();
        let frac = scaled - truncated;
        let abs_frac = frac.abs();

        let threshold = tF128::from(rf as i64) / tF128::from(10);

        let rounded_scaled = if abs_frac >= threshold {
            if v.is_sign_negative() {
                truncated - tF128::ONE
            } else {
                truncated + tF128::ONE
            }
        } else {
            truncated
        };

        rounded_scaled / factor
    }

    /// Rounds a f64 decimal number to specified precision.
    ///
    /// # Arguments
    /// * `v` - f64 value to round
    /// * `prec` - Decimal precision
    /// * `rf` - Rounding factor (1-9)
    ///
    /// # Returns
    /// * `f64` - Rounded f64 value
    pub(super) fn round_decimal_f64(v: f64, prec: i32, rf: i8) -> f64 {
        if !v.is_finite() {
            return v;
        }

        if rf == 5 {
            let multiplier = 10f64.powi(prec);
            return (v * multiplier).round() / multiplier;
        }

        let multiplier = 10f64.powi(prec);
        let scaled = v * multiplier;
        let truncated = scaled.trunc();
        let frac = (scaled - truncated).abs();

        let threshold = (rf as f64) / 10.0;

        if frac >= threshold {
            if v >= 0.0 {
                (truncated + 1.0) / multiplier
            } else {
                (truncated - 1.0) / multiplier
            }
        } else {
            truncated / multiplier
        }
    }

    /// Rounds F128 to nearest power of 10.
    ///
    /// # Arguments
    /// * `v` - F128 value to round
    /// * `power` - Power of 10 to round to
    /// * `rf` - Rounding factor (1-9)
    ///
    /// # Returns
    /// * `tF128` - Rounded F128 value
    pub(super) fn round_f128_to_power_of_10(v: tF128, power: i32, rf: i8) -> tF128 {
        let divisor = tF128::from(10u64).powi(power);
        let scaled = v / divisor;
        let truncated = scaled.trunc();
        let frac = (scaled - truncated).abs();

        let half = tF128::from(5) / tF128::from(10);
        let threshold = tF128::from(rf as i64) / tF128::from(10);
        let epsilon = tF128::EPSILON * tF128::from(10);
        let is_half = (frac - half).abs() < epsilon;

        let rounded_scaled = if is_half || frac >= threshold {
            if v.is_sign_negative() {
                truncated - tF128::ONE
            } else {
                truncated + tF128::ONE
            }
        } else {
            truncated
        };

        rounded_scaled * divisor
    }

    /// Rounds an integer number to specified precision.
    ///
    /// # Arguments
    /// * `n` - Integer number to round
    /// * `prec` - Precision (negative for significant digits)
    /// * `rf` - Rounding factor (1-9)
    ///
    /// # Returns
    /// * `Result<Number, String>` - Rounded integer or error message
    pub(super) fn round_integer(n: Number, prec: i32, rf: i8) -> Result<Number, String> {
        let exp = (-prec) as u32;
        let divisor = 10i128.pow(exp);

        let val = Self::to_i128(&n).ok_or_else(|| MathErr::Overflow.msg())?;
        let sign = if val < 0 { -1 } else { 1 };
        let abs_val = val.abs();

        let remainder = abs_val % divisor;
        let base = abs_val - remainder;

        let half = divisor / 2;
        let _threshold = (exp * rf as u32) as i128; // rf/10 * 10^exp = rf * 10^(exp-1)

        let rounded = if remainder >= half {
            base + divisor
        } else {
            base
        };

        let result = rounded * sign;

        Self::from_i128_in_type(
            result,
            val < 0,
            Self::int_info(&n).map(|(_, r)| r).unwrap_or(5),
        )
        .ok_or_else(|| MathErr::Overflow.msg())
    }

    /// Computes sine of a number.
    ///
    /// # Arguments
    /// * `n` - Angle in radians
    ///
    /// Computes cosine of a number.
    ///
    /// # Arguments
    /// * `n` - Angle in radians
    ///
    /// # Returns
    /// * `Number` - Cosine value
    /// # Returns
    /// * `Number` - Sine value
    pub(super) fn num_sin(n: Number) -> Number {
        let f = Self::to_f128_full(&n);
        Number::F128(f.sin())
    }

    /// Computes cosine of a number.
    ///
    /// # Arguments
    /// * `n` - Angle in radians
    ///
    /// # Returns
    /// * `Number` - Cosine value
    pub(super) fn num_cos(n: Number) -> Number {
        let f = Self::to_f128_full(&n);
        Number::F128(f.cos())
    }

    /// Computes tangent of a number.
    ///
    /// # Arguments
    /// * `n` - Angle in radians
    ///
    /// # Returns
    /// * `Number` - Tangent value
    pub(super) fn num_tan(n: Number) -> Number {
        let f = Self::to_f128_full(&n);
        Number::F128(f.tan())
    }

    /// Computes cotangent of a number.
    ///
    /// # Arguments
    /// * `n` - Angle in radians (must not be multiple of π)
    ///
    /// # Returns
    /// * `Result<Value, String>` - Cotangent value or division by zero error
    pub(super) fn num_ctg(n: Number) -> Result<Value, String> {
        let f = Self::to_f128_full(&n);
        let s = f.sin();
        if s.is_zero() {
            return Err(MathErr::DivisionByZero.msg());
        }
        Ok(Value::Number(Number::F128(f.ctg())))
    }

    /// Computes absolute value of a number.
    ///
    /// # Arguments
    /// * `n` - Number to take absolute value of
    ///
    /// # Returns
    /// * `Number` - Absolute value
    pub(super) fn num_abs(n: Number) -> Number {
        use self::Number::*;
        match n {
            I8(v) => I8(v.abs()),
            I16(v) => I16(v.abs()),
            I32(v) => I32(v.abs()),
            I64(v) => I64(v.abs()),
            I128(v) => I128(v.abs()),
            U8(v) => U8(v),
            U16(v) => U16(v),
            U32(v) => U32(v),
            U64(v) => U64(v),
            U128(v) => U128(v),
            F32(v) => F32(v.abs()),
            F64(v) => F64(v.abs()),
            F128(v) => Number::F128(v.abs()),
        }
    }
}
