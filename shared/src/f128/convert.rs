//! Conversions between [`F128`] and the primitive numeric types.

use super::F128;

// =============================================================================
//         SECTION: CONVERSIONS
// =============================================================================

impl From<f32> for F128 {
    /// [STABLE] Converts f32 to F128.
    fn from(value: f32) -> Self {
        if value.is_nan() {
            return Self::NAN;
        }
        if value.is_infinite() {
            return if value.is_sign_negative() {
                Self::NEG_INFINITY
            } else {
                Self::INFINITY
            };
        }
        if value == 0.0 {
            return if value.is_sign_negative() {
                Self::NEG_ZERO
            } else {
                Self::ZERO
            };
        }

        let bits = value.to_bits();
        let sign = (bits >> 31) != 0;
        let exp = ((bits >> 23) & 0xFF) as i32;
        let mant = (bits & 0x7FFFFF) as u128;

        let (unbiased_exp, full_mant) = if exp == 0 {
            (1 - 127 - 23, mant << (Self::FRAC_BITS - 23))
        } else {
            let e = exp - 127;
            let m = (1u128 << 23) | mant;
            (e, m << (Self::FRAC_BITS - 23))
        };

        Self::compose(sign, unbiased_exp, full_mant)
    }
}

impl From<f64> for F128 {
    /// [STABLE] Converts f64 to F128.
    fn from(value: f64) -> Self {
        if value.is_nan() {
            return Self::NAN;
        }
        if value.is_infinite() {
            return if value.is_sign_negative() {
                Self::NEG_INFINITY
            } else {
                Self::INFINITY
            };
        }
        if value == 0.0 {
            return if value.is_sign_negative() {
                Self::NEG_ZERO
            } else {
                Self::ZERO
            };
        }

        let bits = value.to_bits();
        let sign = (bits >> 63) != 0;
        let exp = ((bits >> 52) & 0x7FF) as i32;
        let mant = (bits & 0xF_FFFF_FFFF_FFFF) as u128;

        let (unbiased_exp, full_mant) = if exp == 0 {
            (1 - 1023 - 52, mant << (Self::FRAC_BITS - 52))
        } else {
            let e = exp - 1023;
            let m = (1u128 << 52) | mant;
            (e, m << (Self::FRAC_BITS - 52))
        };

        Self::compose(sign, unbiased_exp, full_mant)
    }
}

impl F128 {
    /// [STABLE] Converts F128 to f64 (lossy).
    pub fn to_f64(self) -> f64 {
        if self.is_nan() {
            return f64::NAN;
        }
        if self.is_infinite() {
            return if self.is_sign_negative() {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
        if self.is_zero() {
            return if self.is_sign_negative() { -0.0 } else { 0.0 };
        }

        let (sign, exp, mant) = self.decompose();
        let f64_bias = 1023i32;
        let f64_frac_bits = 52u32;

        let biased_exp = exp + f64_bias;

        if biased_exp >= 2047 {
            return if sign {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
        if biased_exp <= 0 {
            if biased_exp < -52 {
                return if sign { -0.0 } else { 0.0 };
            }
            let shift = 1 - biased_exp as u32;
            let sub_mant = mant >> shift;
            let bits = ((sign as u64) << 63) | (sub_mant as u64 & 0xF_FFFF_FFFF_FFFF);
            return f64::from_bits(bits);
        }

        let shift = Self::FRAC_BITS - f64_frac_bits;
        let rounded_mant = (mant >> shift) as u64;
        let rem = mant & ((1u128 << shift) - 1);
        let half = 1u128 << (shift - 1);

        let final_mant = if rem > half || (rem == half && (rounded_mant & 1) != 0) {
            rounded_mant + 1
        } else {
            rounded_mant
        };

        let bits =
            ((sign as u64) << 63) | ((biased_exp as u64) << 52) | (final_mant & 0xF_FFFF_FFFF_FFFF);
        f64::from_bits(bits)
    }

    /// [STABLE] Converts F128 to f32 (lossy).
    pub fn to_f32(self) -> f32 {
        self.to_f64() as f32
    }
}

impl From<i64> for F128 {
    /// [STABLE] Converts i64 to F128.
    fn from(value: i64) -> Self {
        if value == 0 {
            return Self::ZERO;
        }
        let sign = value < 0;
        let abs = if sign {
            (value as i128).wrapping_neg() as u128
        } else {
            value as u128
        };
        Self::from_uint_inner(sign, abs)
    }
}

impl From<u64> for F128 {
    /// [STABLE] Converts u64 to F128.
    fn from(value: u64) -> Self {
        if value == 0 {
            return Self::ZERO;
        }
        Self::from_uint_inner(false, value as u128)
    }
}

impl From<i32> for F128 {
    fn from(v: i32) -> Self {
        Self::from(v as i64)
    }
}
impl From<u32> for F128 {
    fn from(v: u32) -> Self {
        Self::from(v as u64)
    }
}
impl From<i16> for F128 {
    fn from(v: i16) -> Self {
        Self::from(v as i64)
    }
}
impl From<u16> for F128 {
    fn from(v: u16) -> Self {
        Self::from(v as u64)
    }
}
impl From<i8> for F128 {
    fn from(v: i8) -> Self {
        Self::from(v as i64)
    }
}
impl From<u8> for F128 {
    fn from(v: u8) -> Self {
        Self::from(v as u64)
    }
}

impl F128 {
    fn from_uint_inner(sign: bool, v: u128) -> Self {
        let msb = 127 - v.leading_zeros() as i32;
        let shift = Self::FRAC_BITS as i32 - msb;
        let mant = if shift >= 0 {
            v << shift
        } else {
            v >> (-shift)
        };
        let exp = msb;
        Self::compose(sign, exp, mant)
    }
}

// =============================================================================
//         SECTION: INTEGER EXTRACTION
// =============================================================================

impl F128 {
    /// Вспомогательный метод для to_i64 с насыщением
    pub(super) fn to_i64_saturating(self) -> i64 {
        if !self.is_finite() {
            return 0;
        }
        if self >= Self::from(i64::MAX) {
            return i64::MAX;
        }
        if self <= Self::from(i64::MIN) {
            return i64::MIN;
        }
        self.to_i64_checked().unwrap_or(0)
    }

    /// Safely converts F128 to i64 with overflow protection.
    pub(super) fn to_i64_checked(self) -> Option<i64> {
        if !self.is_finite() {
            return None;
        }
        if self.is_zero() {
            return Some(0);
        }

        let (sign, exp, mant) = self.decompose();
        let frac_bits = Self::FRAC_BITS as i32;

        if exp >= 63 && (exp > 63 || mant >= (1u128 << (exp - frac_bits + 63))) {
            return if sign { Some(i64::MIN) } else { Some(i64::MAX) };
        }

        if exp < frac_bits - 128 {
            return Some(0);
        }

        let shift = exp - frac_bits;
        let val: i128 = if shift >= 0 {
            (mant as i128) << shift
        } else {
            (mant as i128) >> (-shift).min(127)
        };

        let signed = if sign { -val } else { val };
        if signed < i64::MIN as i128 || signed > i64::MAX as i128 {
            None
        } else {
            Some(signed as i64)
        }
    }
}
