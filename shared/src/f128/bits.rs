//! Bit layout, raw constructors, classification and decompose/compose for [`F128`].

use super::F128;
use std::num::FpCategory;

// =============================================================================
//         SECTION: CONSTRUCTORS & BIT MANIPULATION
// =============================================================================

impl F128 {
    /// [STABLE] Creates F128 from raw bits.
    #[inline]
    pub const fn from_bits(high: u64, low: u64) -> Self {
        F128 { high, low }
    }

    /// [STABLE] Returns the raw bits of the number.
    #[inline]
    pub const fn to_bits(self) -> (u64, u64) {
        (self.high, self.low)
    }

    /// [STABLE] Returns the sign bit.
    #[inline]
    pub const fn sign_bit(self) -> u8 {
        ((self.high >> 63) & 1) as u8
    }

    /// [STABLE] Returns true if the sign is negative.
    #[inline]
    pub const fn is_sign_negative(self) -> bool {
        (self.high & Self::SIGN_MASK) != 0
    }

    /// [STABLE] Returns true if the sign is positive.
    #[inline]
    pub const fn is_sign_positive(self) -> bool {
        !self.is_sign_negative() && !self.is_nan()
    }

    /// [STABLE] Returns the raw exponent bits.
    #[inline]
    pub const fn raw_exponent(self) -> u16 {
        ((self.high & Self::EXP_MASK) >> 48) as u16
    }

    /// [STABLE] Returns raw fraction bits (high 48 bits, low 64 bits).
    #[inline]
    pub const fn raw_fraction(self) -> (u64, u64) {
        (self.high & Self::FRAC_HIGH_MASK, self.low)
    }

    /// Create F128 from raw sign, exponent, and fraction (no implicit bit).
    /// For internal use by compose().
    #[inline]
    const fn from_raw(sign: bool, exp: u16, frac_high: u64, frac_low: u64) -> Self {
        let high =
            ((sign as u64) << 63) | ((exp as u64) << 48) | (frac_high & Self::FRAC_HIGH_MASK);
        F128 {
            high,
            low: frac_low,
        }
    }
}

// =============================================================================
//         SECTION: CLASSIFICATION
// =============================================================================

impl F128 {
    /// [STABLE] Returns true if the value is NaN.
    #[inline]
    pub const fn is_nan(self) -> bool {
        let exp = self.raw_exponent();
        let (fh, fl) = self.raw_fraction();
        exp == 0x7FFF && (fh != 0 || fl != 0)
    }

    /// [STABLE] Returns true if the value is infinite.
    #[inline]
    pub const fn is_infinite(self) -> bool {
        let exp = self.raw_exponent();
        let (fh, fl) = self.raw_fraction();
        exp == 0x7FFF && fh == 0 && fl == 0
    }

    /// [STABLE] Returns true if the value is finite.
    #[inline]
    pub const fn is_finite(self) -> bool {
        self.raw_exponent() != 0x7FFF
    }

    /// [STABLE] Returns true if the value is zero.
    #[inline]
    pub const fn is_zero(self) -> bool {
        (self.high & !Self::SIGN_MASK) == 0 && self.low == 0
    }

    /// [STABLE] Returns true if the value is exactly 1.0.
    #[inline]
    pub const fn is_one(self) -> bool {
        self.high == 0x3FFF_0000_0000_0000 && self.low == 0
    }

    /// [STABLE] Returns true if the value is exactly -1.0.
    #[inline]
    pub const fn is_neg_one(self) -> bool {
        self.high == 0xBFFF_0000_0000_0000 && self.low == 0
    }

    /// [STABLE] Returns true if the value is subnormal.
    #[inline]
    pub const fn is_subnormal(self) -> bool {
        let exp = self.raw_exponent();
        let (fh, fl) = self.raw_fraction();
        exp == 0 && (fh != 0 || fl != 0)
    }

    /// [STABLE] Returns true if the value is normal.
    #[inline]
    pub const fn is_normal(self) -> bool {
        let exp = self.raw_exponent();
        exp != 0 && exp != 0x7FFF
    }

    /// [STABLE] Classifies the number.
    pub fn classify(self) -> FpCategory {
        if self.is_nan() {
            FpCategory::Nan
        } else if self.is_infinite() {
            FpCategory::Infinite
        } else if self.is_zero() {
            FpCategory::Zero
        } else if self.is_subnormal() {
            FpCategory::Subnormal
        } else {
            FpCategory::Normal
        }
    }

    /// [STABLE] Returns the number of radix-2 digits in the mantissa.
    pub fn mantissa_digits(self) -> u32 {
        if self.is_subnormal() {
            112 - (self.raw_fraction().0.leading_zeros() + self.raw_fraction().1.leading_zeros())
        } else {
            113 // implicit + explicit
        }
    }
}

// =============================================================================
//         SECTION: DECOMPOSITION & COMPOSITION
// =============================================================================

impl F128 {
    /// Decompose into (sign, unbiased exponent, mantissa with implicit bit).
    pub fn decompose(self) -> (bool, i32, u128) {
        let sign = self.is_sign_negative();
        let exp = self.raw_exponent();
        let (fh, fl) = self.raw_fraction();
        let frac = ((fh as u128) << 64) | (fl as u128);

        match exp {
            0 => {
                if frac == 0 {
                    (sign, Self::MIN_EXP - 1, 0)
                } else {
                    let lz = frac.leading_zeros() as i32 - (128 - Self::FRAC_BITS as i32);
                    (sign, Self::MIN_EXP - lz, frac << lz)
                }
            }
            0x7FFF => (sign, i32::MAX, frac),
            _ => {
                let mant = (1u128 << Self::FRAC_BITS) | frac;
                (sign, exp as i32 - Self::EXP_BIAS, mant)
            }
        }
    }

    /// Compose from (sign, unbiased exponent, mantissa).
    /// Normalizes the mantissa and handles overflow/underflow.
    pub fn compose(sign: bool, mut exp: i32, mut mant: u128) -> Self {
        if mant == 0 {
            return if sign { Self::NEG_ZERO } else { Self::ZERO };
        }

        let lz = mant.leading_zeros();
        if lz >= 128 {
            return if sign { Self::NEG_ZERO } else { Self::ZERO };
        }
        let msb = 127i32.saturating_sub(lz as i32);
        let target = Self::FRAC_BITS as i32;

        if msb > target {
            let shift = (msb - target) as u32;
            if shift >= 128 {
                return if sign {
                    Self::NEG_INFINITY
                } else {
                    Self::INFINITY
                };
            }
            mant >>= shift;
            exp = exp.saturating_add(shift as i32);
        } else if msb < target {
            let shift = (target - msb) as u32;
            if shift >= 128 {
                return if sign { Self::NEG_ZERO } else { Self::ZERO };
            }
            mant <<= shift;
            exp = exp.saturating_sub(shift as i32);
        }

        let biased = exp + Self::EXP_BIAS;

        if biased >= 0x7FFF {
            return if sign {
                Self::NEG_INFINITY
            } else {
                Self::INFINITY
            };
        }
        if biased <= 0 {
            let shift = (1i32 - biased) as u32;
            if shift >= 128 {
                return if sign { Self::NEG_ZERO } else { Self::ZERO };
            }
            mant >>= shift;
            return Self::from_raw(sign, 0, (mant >> 64) as u64, mant as u64);
        }

        let frac_mask = (1u128 << Self::FRAC_BITS) - 1;
        let frac = mant & frac_mask;

        Self::from_raw(sign, biased as u16, (frac >> 64) as u64, frac as u64)
    }
}

// =============================================================================
//         SECTION: SIGN & INTEGER PREDICATES
// =============================================================================

impl F128 {
    /// [STABLE] Absolute value.
    pub fn abs(self) -> Self {
        Self::from_bits(self.high & !Self::SIGN_MASK, self.low)
    }

    /// [STABLE] Copy sign from another value.
    pub fn copysign(self, sign_from: Self) -> Self {
        Self::from_bits(
            (self.high & !Self::SIGN_MASK) | (sign_from.high & Self::SIGN_MASK),
            self.low,
        )
    }

    /// [STABLE] Returns the sign of the number (-1.0, 0.0, or 1.0).
    pub fn signum(self) -> Self {
        if self.is_nan() {
            Self::NAN
        } else if self.is_zero() {
            self
        } else {
            Self::copysign(Self::ONE, self)
        }
    }

    /// Checks if the value is an exact integer.
    pub fn is_integer(self) -> bool {
        if self.is_nan() || self.is_infinite() {
            return false;
        }
        if self.is_zero() {
            return true;
        }

        let (_sign, exp, mant) = self.decompose();
        if exp < 0 {
            return false;
        }
        if exp >= Self::FRAC_BITS as i32 {
            return true;
        }

        let shift = (Self::FRAC_BITS as i32 - exp) as u32;
        if shift >= 128 {
            return false;
        }

        let frac_mask = (1u128 << shift) - 1;
        (mant & frac_mask) == 0
    }
}
