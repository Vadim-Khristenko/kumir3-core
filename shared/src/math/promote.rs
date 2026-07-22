// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Числовые предикаты, конвертации и продвижение типов (type promotion).

// =============================================================================
//         IMPORTS
// =============================================================================

use super::MathErr;
use super::MathOperators;
use super::error::warn_auto_widen;
use crate::f128::F128 as tF128;
use crate::types::Number;

// =============================================================================
//         CORE LOGIC
// =============================================================================

impl MathOperators {
    /// Checks if a Number is zero.
    ///
    /// # Arguments
    /// * `n` - The Number to check
    ///
    /// # Returns
    /// * `bool` - True if the number is zero, false otherwise
    pub(super) fn is_zero_num(n: &Number) -> bool {
        use self::Number::*;
        match n {
            I8(v) => *v == 0,
            I16(v) => *v == 0,
            I32(v) => *v == 0,
            I64(v) => *v == 0,
            I128(v) => *v == 0,
            U8(v) => *v == 0,
            U16(v) => *v == 0,
            U32(v) => *v == 0,
            U64(v) => *v == 0,
            U128(v) => *v == 0,
            F32(v) => *v == 0.0,
            F64(v) => *v == 0.0,
            F128(v) => v.is_zero(),
        }
    }

    /// Extracts signedness and rank information from an integer Number.
    ///
    /// # Arguments
    /// * `n` - The Number to analyze
    ///
    /// # Returns
    /// * `Option<(bool, u8)>` - (signed, rank) where rank indicates size (1=i8/u8, 2=i16/u16, etc.), None for non-integers
    pub(super) fn int_info(n: &Number) -> Option<(bool, u8)> {
        use self::Number::*;
        let (signed, rank) = match n {
            I8(_) => (true, 1),
            I16(_) => (true, 2),
            I32(_) => (true, 3),
            I64(_) => (true, 4),
            I128(_) => (true, 5),
            U8(_) => (false, 1),
            U16(_) => (false, 2),
            U32(_) => (false, 3),
            U64(_) => (false, 4),
            U128(_) => (false, 5),
            _ => return None,
        };
        Some((signed, rank))
    }

    /// Converts a Number to i128 if possible.
    ///
    /// # Arguments
    /// * `n` - The Number to convert
    ///
    /// # Returns
    /// * `Option<i128>` - The converted value, or None if out of range or not an integer
    pub(super) fn to_i128(n: &Number) -> Option<i128> {
        use self::Number::*;
        match *n {
            I8(v) => Some(v as i128),
            I16(v) => Some(v as i128),
            I32(v) => Some(v as i128),
            I64(v) => Some(v as i128),
            I128(v) => Some(v),
            U8(v) => Some(v as i128),
            U16(v) => Some(v as i128),
            U32(v) => Some(v as i128),
            U64(v) => {
                if v <= i128::MAX as u64 {
                    Some(v as i128)
                } else {
                    None
                }
            }
            U128(v) => {
                if v <= i128::MAX as u128 {
                    Some(v as i128)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Converts an i128 value back to a Number of specified signedness and rank.
    ///
    /// # Arguments
    /// * `x` - The i128 value to convert
    /// * `signed` - Whether the target type is signed
    /// * `rank` - The size rank (1=i8/u8, 2=i16/u16, etc.)
    ///
    /// # Returns
    /// * `Option<Number>` - The converted Number, or None if out of range
    pub(super) fn from_i128_in_type(x: i128, signed: bool, rank: u8) -> Option<Number> {
        use self::Number::*;
        if signed {
            match rank {
                1 if x >= i8::MIN as i128 && x <= i8::MAX as i128 => Some(I8(x as i8)),
                2 if x >= i16::MIN as i128 && x <= i16::MAX as i128 => Some(I16(x as i16)),
                3 if x >= i32::MIN as i128 && x <= i32::MAX as i128 => Some(I32(x as i32)),
                4 if x >= i64::MIN as i128 && x <= i64::MAX as i128 => Some(I64(x as i64)),
                5 => Some(I128(x)),
                _ => None,
            }
        } else {
            if x < 0 {
                return None;
            }
            let ux = x as u128;
            match rank {
                1 if ux <= u8::MAX as u128 => Some(U8(ux as u8)),
                2 if ux <= u16::MAX as u128 => Some(U16(ux as u16)),
                3 if ux <= u32::MAX as u128 => Some(U32(ux as u32)),
                4 if ux <= u64::MAX as u128 => Some(U64(ux as u64)),
                5 => Some(U128(ux)),
                _ => None,
            }
        }
    }

    /// Converts a Number to F128 with full precision handling.
    ///
    /// # Arguments
    /// * `n` - The Number to convert
    ///
    /// # Returns
    /// * `tF128` - The converted F128 value
    pub(super) fn to_f128_full(n: &Number) -> tF128 {
        use self::Number::*;
        match *n {
            I8(v) => tF128::from(v as i64),
            I16(v) => tF128::from(v as i64),
            I32(v) => tF128::from(v as i64),
            I64(v) => tF128::from(v),
            I128(v) => {
                if v >= i64::MIN as i128 && v <= i64::MAX as i128 {
                    tF128::from(v as i64)
                } else {
                    tF128::from(v as f64)
                }
            }
            U8(v) => tF128::from(v as u64),
            U16(v) => tF128::from(v as u64),
            U32(v) => tF128::from(v as u64),
            U64(v) => tF128::from(v),
            U128(v) => {
                if v <= u64::MAX as u128 {
                    tF128::from(v as u64)
                } else {
                    tF128::from(v as f64)
                }
            }
            F32(v) => tF128::from(v),
            F64(v) => tF128::from(v),
            F128(v) => v,
        }
    }

    /// Performs integer or floating-point operation with overflow handling.
    ///
    /// # Arguments
    /// * `a` - First operand
    /// * `b` - Second operand
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    /// * `int_op` - Integer operation function
    /// * `float_op` - Floating-point operation function
    ///
    /// # Returns
    /// * `Result<Number, MathErr>` - Result of operation or error message
    pub(super) fn int_or_float(
        a: Number,
        b: Number,
        fo_e: bool,
        int_op: fn(i128, i128) -> i128,
        float_op: fn(tF128, tF128) -> tF128,
    ) -> Result<Number, MathErr> {
        if let (Some((sa, ra)), Some((sb, rb))) = (Self::int_info(&a), Self::int_info(&b)) {
            let signed = sa || sb;
            let rank = ra.max(rb);
            if let (Some(x), Some(y)) = (Self::to_i128(&a), Self::to_i128(&b)) {
                let res = int_op(x, y);

                if let Some(n) = Self::from_i128_in_type(res, signed, rank) {
                    return Ok(n);
                }

                if fo_e {
                    return Err(MathErr::Overflow);
                }

                let widened = if signed {
                    if (i128::MIN..=i128::MAX).contains(&res) {
                        Number::I128(res)
                    } else {
                        return Err(MathErr::Overflow);
                    }
                } else if res >= 0 {
                    Number::U128(res as u128)
                } else {
                    return Err(MathErr::Overflow);
                };

                let _ = warn_auto_widen();
                return Ok(widened);
            }
        }

        let fa = Self::to_f128_full(&a);
        let fb = Self::to_f128_full(&b);
        let r = float_op(fa, fb);
        if (r.is_infinite() || r.is_nan()) && fo_e {
            return Err(MathErr::FloatOverflow);
        }
        Ok(Number::F128(r))
    }

    /// Checks if a F128 value is effectively an integer.
    ///
    /// # Arguments
    /// * `x` - F128 value to check
    ///
    /// # Returns
    /// * `bool` - True if the value is an integer, false otherwise
    pub(super) fn is_effectively_integer(x: tF128) -> bool {
        x.is_integer()
    }
}
