// =============================================================================
//         SECTION: IMPORTS
// =============================================================================

use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::num::FpCategory;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};
use std::str::FromStr;

// =============================================================================
//         SECTION: TYPES
// =============================================================================

/// [STABLE] IEEE 754-2008 binary128 (quadruple-precision) floating-point number.
///
/// Layout: 1-bit sign, 15-bit exponent (bias=16383), 112-bit mantissa.
/// Precision: ~34 decimal digits.
#[repr(C)]
#[derive(Clone, Copy, Eq, Hash)]
pub struct F128 {
    pub high: u64,
    pub low: u64,
}

// =============================================================================
//         SECTION: TRAIT IMPLS
// =============================================================================

impl PartialEq for F128 {
    /// [STABLE] Equality comparison.
    fn eq(&self, other: &Self) -> bool {
        if self.is_nan() || other.is_nan() {
            return false;
        }
        if self.is_zero() && other.is_zero() {
            return true;
        }
        self.high == other.high && self.low == other.low
    }
}

// =============================================================================
//         SECTION: CONSTANTS
// =============================================================================

impl F128 {
    const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
    const EXP_MASK: u64 = 0x7FFF_0000_0000_0000;
    const FRAC_HIGH_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;
    const EXP_BITS: u32 = 15;
    const FRAC_BITS: u32 = 112;
    const EXP_BIAS: i32 = 16383;
    const MAX_EXP: i32 = 16383;
    const MIN_EXP: i32 = -16382;

    pub const ZERO: F128 = F128 { high: 0, low: 0 };
    pub const NEG_ZERO: F128 = F128 { high: Self::SIGN_MASK, low: 0 };
    pub const ONE: F128 = F128 { high: 0x3FFF_0000_0000_0000, low: 0 };
    pub const NEG_ONE: F128 = F128 { high: 0xBFFF_0000_0000_0000, low: 0 };

    pub const INFINITY: F128 = F128 { high: Self::EXP_MASK, low: 0 };
    pub const NEG_INFINITY: F128 = F128 { high: Self::SIGN_MASK | Self::EXP_MASK, low: 0 };

    /// Quiet NaN (canonical)
    pub const NAN: F128 = F128 {
        high: Self::EXP_MASK | 0x0000_8000_0000_0000,
        low: 0,
    };

    /// Smallest positive subnormal number
    pub const MIN_POSITIVE_SUBNORMAL: F128 = F128 { high: 0, low: 1 };
    /// Smallest positive normal number (2^-16382)
    pub const MIN_POSITIVE_NORMAL: F128 = F128 { high: 0x0001_0000_0000_0000, low: 0 };
    /// Largest finite number (2^16384 - 2^16372)
    pub const MAX: F128 = F128 {
        high: 0x7FFE_FFFF_FFFF_FFFF,
        low: 0xFFFF_FFFF_FFFF_FFFF,
    };
    /// Machine epsilon (2^-112)
    pub const EPSILON: F128 = F128 { high: 0x3F8F_0000_0000_0000, low: 0 };

    pub const PI: F128 = F128 { high: 0x4000_921F_B544_42D1, low: 0x8469_898C_C517_01B8 };
    pub const TWO_PI: F128 = F128 { high: 0x4001_921F_B544_42D1, low: 0x8469_898C_C517_01B8 };
    pub const FRAC_PI_2: F128 = F128 { high: 0x3FFF_921F_B544_42D1, low: 0x8469_898C_C517_01B8 };
    pub const FRAC_PI_4: F128 = F128 { high: 0x3FFE_921F_B544_42D1, low: 0x8469_898C_C517_01B8 };
    pub const E: F128 = F128 { high: 0x4000_5BF0_A8B1_4576, low: 0x9535_5FB8_AC40_4E7A };
    pub const LN_2: F128 = F128 { high: 0x3FFE_62E4_2FEF_A39E, low: 0xF357_93C7_7FCE_2BBC };
    pub const LOG2_E: F128 = F128 { high: 0x3FFF_B8AA_3B29_5C17, low: 0xF0AB_EA67_0764_8776 };
}

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
        let high = ((sign as u64) << 63) | ((exp as u64) << 48) | (frac_high & Self::FRAC_HIGH_MASK);
        F128 { high, low: frac_low }
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
    fn decompose(self) -> (bool, i32, u128) {
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
    fn compose(sign: bool, mut exp: i32, mut mant: u128) -> Self {
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
                return if sign { Self::NEG_INFINITY } else { Self::INFINITY };
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
            return if sign { Self::NEG_INFINITY } else { Self::INFINITY };
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

        Self::from_raw(
            sign,
            biased as u16,
            (frac >> 64) as u64,
            frac as u64
        )
    }
}

// =============================================================================
//         SECTION: CONVERSIONS
// =============================================================================

impl From<f32> for F128 {
    /// [STABLE] Converts f32 to F128.
    fn from(value: f32) -> Self {
        if value.is_nan() { return Self::NAN; }
        if value.is_infinite() {
            return if value.is_sign_negative() { Self::NEG_INFINITY } else { Self::INFINITY };
        }
        if value == 0.0 {
            return if value.is_sign_negative() { Self::NEG_ZERO } else { Self::ZERO };
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
        if value.is_nan() { return Self::NAN; }
        if value.is_infinite() {
            return if value.is_sign_negative() { Self::NEG_INFINITY } else { Self::INFINITY };
        }
        if value == 0.0 {
            return if value.is_sign_negative() { Self::NEG_ZERO } else { Self::ZERO };
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
        if self.is_nan() { return f64::NAN; }
        if self.is_infinite() {
            return if self.is_sign_negative() { f64::NEG_INFINITY } else { f64::INFINITY };
        }
        if self.is_zero() {
            return if self.is_sign_negative() { -0.0 } else { 0.0 };
        }

        let (sign, exp, mant) = self.decompose();
        let f64_bias = 1023i32;
        let f64_frac_bits = 52u32;

        let biased_exp = exp + f64_bias;

        if biased_exp >= 2047 {
            return if sign { f64::NEG_INFINITY } else { f64::INFINITY };
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

        let bits = ((sign as u64) << 63) 
            | ((biased_exp as u64) << 52) 
            | (final_mant & 0xF_FFFF_FFFF_FFFF);
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
        if value == 0 { return Self::ZERO; }
        let sign = value < 0;
        let abs = if sign { (value as i128).wrapping_neg() as u128 } else { value as u128 };
        Self::from_uint_inner(sign, abs)
    }
}

impl From<u64> for F128 {
    /// [STABLE] Converts u64 to F128.
    fn from(value: u64) -> Self {
        if value == 0 { return Self::ZERO; }
        Self::from_uint_inner(false, value as u128)
    }
}

impl From<i32> for F128 { fn from(v: i32) -> Self { Self::from(v as i64) } }
impl From<u32> for F128 { fn from(v: u32) -> Self { Self::from(v as u64) } }
impl From<i16> for F128 { fn from(v: i16) -> Self { Self::from(v as i64) } }
impl From<u16> for F128 { fn from(v: u16) -> Self { Self::from(v as u64) } }
impl From<i8> for F128 { fn from(v: i8) -> Self { Self::from(v as i64) } }
impl From<u8> for F128 { fn from(v: u8) -> Self { Self::from(v as u64) } }

impl F128 {
    fn from_uint_inner(sign: bool, mut v: u128) -> Self {
        let msb = 127 - v.leading_zeros() as i32;
        let shift = Self::FRAC_BITS as i32 - msb;
        let mant = if shift >= 0 { v << shift } else { v >> (-shift) };
        let exp = msb;
        Self::compose(sign, exp, mant)
    }
}

// =============================================================================
//         SECTION: ARITHMETIC
// =============================================================================

impl Add for F128 {
    type Output = Self;
    
    /// [STABLE] [PERF-SENSITIVE] Addition.
    fn add(self, rhs: Self) -> Self {
        if self.is_nan() || rhs.is_nan() { return Self::NAN; }
        
        if self.is_infinite() || rhs.is_infinite() {
            if self.is_infinite() && rhs.is_infinite() {
                if self.is_sign_negative() != rhs.is_sign_negative() {
                    return Self::NAN; // inf + -inf
                }
                return self;
            }
            return if self.is_infinite() { self } else { rhs };
        }

        let (sa, ea, ma) = self.decompose();
        let (sb, eb, mb) = rhs.decompose();
        
        if ma == 0 { return rhs; }
        if mb == 0 { return self; }

        
        let (mut e_res, mut m_a, mut m_b) = if ea >= eb {
            (ea, ma, mb >> (ea - eb).min(127))
        } else {
            (eb, ma >> (eb - ea).min(127), mb)
        };

        
        let (sign_res, mant_res) = if sa == sb {
            (sa, m_a + m_b)
        } else {
            if m_a >= m_b {
                (sa, m_a - m_b)
            } else {
                (sb, m_b - m_a)
            }
        };

        if mant_res == 0 {
            return Self::ZERO;
        }

        Self::compose(sign_res, e_res, mant_res)
    }
}

impl Sub for F128 {
    type Output = Self;
    
    /// [STABLE] Subtraction.
    fn sub(self, rhs: Self) -> Self {
        self + (-rhs)
    }
}

impl Neg for F128 {
    type Output = Self;
    
    /// [STABLE] Negation.
    fn neg(self) -> Self {
        if self.is_nan() { return self; }
        Self::from_bits(self.high ^ Self::SIGN_MASK, self.low)
    }
}

impl Mul for F128 {
    type Output = Self;
    
    /// [STABLE] [PERF-SENSITIVE] Multiplication.
    fn mul(self, rhs: Self) -> Self {
        if self.is_nan() || rhs.is_nan() { return Self::NAN; }
        
        let s_zero = self.is_zero();
        let r_zero = rhs.is_zero();
        let s_inf = self.is_infinite();
        let r_inf = rhs.is_infinite();
        
        if (s_zero && r_inf) || (r_zero && s_inf) {
            return Self::NAN;
        }
        if s_zero || r_zero {
            let sign = self.is_sign_negative() ^ rhs.is_sign_negative();
            return if sign { Self::NEG_ZERO } else { Self::ZERO };
        }
        if s_inf || r_inf {
            let sign = self.is_sign_negative() ^ rhs.is_sign_negative();
            return if sign { Self::NEG_INFINITY } else { Self::INFINITY };
        }

        let (sa, ea, ma) = self.decompose();
        let (sb, eb, mb) = rhs.decompose();
        
        let prod = U256::mul_u128(ma, mb);

        
        let msb = 255 - prod.leading_zeros() as i32;
        let target = Self::FRAC_BITS as i32;
        let shift = msb - target;
        
        let norm = if shift > 0 {
            prod.shr(shift as u32)
        } else {
            prod.shl((-shift) as u32)
        };
        
        let mant = norm.low_u128() & ((1u128 << (Self::FRAC_BITS + 1)) - 1);
        let exp = ea + eb - Self::FRAC_BITS as i32 + shift;
        
        Self::compose(sa ^ sb, exp, mant)
    }
}

impl Div for F128 {
    type Output = Self;
    
    /// [STABLE] [PERF-SENSITIVE] Division.
    fn div(self, rhs: Self) -> Self {
        if self.is_nan() || rhs.is_nan() { return Self::NAN; }
        
        let s_zero = self.is_zero();
        let r_zero = rhs.is_zero();
        let s_inf = self.is_infinite();
        let r_inf = rhs.is_infinite();
        
        if (s_zero && r_zero) || (s_inf && r_inf) { return Self::NAN; }
        if s_zero || r_inf {
            let sign = self.is_sign_negative() ^ rhs.is_sign_negative();
            return if sign { Self::NEG_ZERO } else { Self::ZERO };
        }
        if r_zero || s_inf {
            let sign = self.is_sign_negative() ^ rhs.is_sign_negative();
            return if sign { Self::NEG_INFINITY } else { Self::INFINITY };
        }

        let (sa, ea, ma) = self.decompose();
        let (sb, eb, mb) = rhs.decompose();
        
        let mut n = U256::from_u128(ma).shl(Self::FRAC_BITS);
        let d = U256::from_u128(mb);
        let mut q: u128 = 0;
        
        for i in (0..=Self::FRAC_BITS).rev() {
            let ds = d.shl(i);
            if n.cmp(&ds) != Ordering::Less {
                n = n.sub(ds);
                q |= 1u128 << i;
            }
        }
        
        let exp = ea - eb;
        Self::compose(sa ^ sb, exp, q)
    }
}

impl Rem for F128 {
    type Output = Self;
    
    /// [STABLE] Remainder.
    fn rem(self, rhs: Self) -> Self {
        if self.is_nan() || rhs.is_nan() { return Self::NAN; }
        if self.is_infinite() || rhs.is_zero() { return Self::NAN; }
        if rhs.is_infinite() || self.is_zero() { return self; }
        
        let div = self / rhs;
        let trunc = div.trunc();
        self - rhs * trunc
    }
}

// =============================================================================
//         SECTION: COMPARISON
// =============================================================================

impl PartialOrd for F128 {
    /// [STABLE] Partial comparison. Returns None if either is NaN.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.is_nan() || other.is_nan() { return None; }
        Some(self.total_cmp(other))
    }
    
    #[inline] fn lt(&self, other: &Self) -> bool { matches!(self.partial_cmp(other), Some(Ordering::Less)) }
    #[inline] fn le(&self, other: &Self) -> bool { matches!(self.partial_cmp(other), Some(Ordering::Less | Ordering::Equal)) }
    #[inline] fn gt(&self, other: &Self) -> bool { matches!(self.partial_cmp(other), Some(Ordering::Greater)) }
    #[inline] fn ge(&self, other: &Self) -> bool { matches!(self.partial_cmp(other), Some(Ordering::Greater | Ordering::Equal)) }
}

impl F128 {
    /// [STABLE] IEEE 754 totalOrder comparison.
    ///
    /// # Behavior
    /// - NaN > all non-NaN (ordered by payload)
    /// - -0.0 == +0.0 (for compatibility; strict IEEE would have -0.0 < +0.0)
    /// - Otherwise normal numeric comparison
    ///
    /// # Parameters
    /// - `other`: the value to compare against
    ///
    /// # Returns
    /// - `Ordering`: Less, Equal, or Greater
    pub fn total_cmp(&self, other: &Self) -> Ordering {
        if self.high == other.high && self.low == other.low {
            return Ordering::Equal;
        }
        
        
        let self_nan = self.is_nan();
        let other_nan = other.is_nan();
        if self_nan || other_nan {
            if self_nan && other_nan {
                return (self.high, self.low).cmp(&(other.high, other.low));
            }
            return if self_nan { Ordering::Greater } else { Ordering::Less };
        }
        if self.is_zero() && other.is_zero() {
            return Ordering::Equal;
        }
        
        let self_neg = self.is_sign_negative();
        let other_neg = other.is_sign_negative();
        
        if self_neg != other_neg {
            return if self_neg { Ordering::Less } else { Ordering::Greater };
        }
        let ord = (self.high, self.low).cmp(&(other.high, other.low));
        if self_neg { ord.reverse() } else { ord }
    }
}

// =============================================================================
//         SECTION: CORE LOGIC
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
            self.low
        )
    }
    
    /// [STABLE] Returns the sign of the number (-1.0, 0.0, or 1.0).
    pub fn signum(self) -> Self {
        if self.is_nan() { Self::NAN }
        else if self.is_zero() { self }
        else { Self::copysign(Self::ONE, self) }
    }

    /// Truncate toward zero (remove fractional part).
    ///
    /// # Returns
    /// - Integer part of `self`, rounded toward zero
    /// - Returns `self` unchanged for NaN, infinite, or zero
    pub fn trunc(self) -> Self {
        if !self.is_finite() || self.is_zero() { return self; }
        
        let (sign, exp, mant) = self.decompose();
        let frac_bits = Self::FRAC_BITS as i32;
        
        if exp >= frac_bits { return self; }
        if exp < 0 { 
            return if sign { Self::NEG_ZERO } else { Self::ZERO };
        }
        
        let shift = (frac_bits - exp) as u32;
        if shift >= 128 {
            return if sign { Self::NEG_ZERO } else { Self::ZERO };
        }
        
        let int_mask = !((1u128 << shift) - 1);
        let new_mant = mant & int_mask;
        
        Self::compose(sign, exp, new_mant)
    }
    
    /// [STABLE] Round down.
    pub fn floor(self) -> Self {
        if !self.is_finite() || self.is_zero() { return self; }
        
        let trunc = self.trunc();
        if self.is_sign_negative() && self != trunc {
            trunc - Self::ONE
        } else {
            trunc
        }
    }
    
    /// [STABLE] Round up.
    pub fn ceil(self) -> Self {
        if !self.is_finite() || self.is_zero() { return self; }
        
        let trunc = self.trunc();
        if !self.is_sign_negative() && self != trunc {
            trunc + Self::ONE
        } else {
            trunc
        }
    }
    
    /// [STABLE] Round to nearest integer (ties to even).
    pub fn round(self) -> Self {
        if !self.is_finite() || self.is_zero() { return self; }
        
        let trunc = self.trunc();
        let frac = self - trunc;
        
        if frac.is_zero() { return self; }
        
        let half = Self::from(0.5);
        let abs_frac = frac.abs();
        
        match abs_frac.partial_cmp(&half) {
            Some(Ordering::Less) => trunc,
            Some(Ordering::Greater) => {
                trunc + Self::copysign(Self::ONE, self)
            }
            Some(Ordering::Equal) => {
                // Round half to even
                let half_trunc = trunc / Self::from(2);
                if half_trunc.trunc() == half_trunc {
                    trunc
                } else {
                    trunc + Self::copysign(Self::ONE, self)
                }
            }
            None => Self::NAN,
        }
    }

    /// [STABLE] [PERF-SENSITIVE] Square root.
    pub fn sqrt(self) -> Self {
        if self.is_nan() || self.is_sign_negative() && !self.is_zero() { 
            return Self::NAN; 
        }
        if self.is_zero() || self.is_infinite() { return self; }

        let guess = Self::from(self.to_f64().sqrt());
        
        
        let half = Self::from(0.5);
        let mut x = guess;
        for _ in 0..8 {
            let div = self / x;
            x = (x + div) * half;
            if (x * x - self).abs() < Self::EPSILON * self.abs() {
                break;
            }
        }
        x
    }
}

// =============================================================================
//         SECTION: MATH (TRANSCENDENTAL)
// =============================================================================

impl F128 {
    /// [STABLE] Integer power.
    pub fn powi(self, exp: i32) -> Self {
        let mut base = self;
        let mut exp_abs = exp.abs() as u64;
        let mut acc = Self::ONE;
        
        while exp_abs > 0 {
            if (exp_abs & 1) != 0 { acc = acc * base; }
            base = base * base;
            exp_abs >>= 1;
        }
        
        if exp < 0 { Self::ONE / acc } else { acc }
    }

    /// [STABLE] Floating point power.
    pub fn powf(self, exp: Self) -> Self {
        if self.is_nan() || exp.is_nan() { return Self::NAN; }
        if exp.is_zero() { return Self::ONE; }
        if self.is_one() { return Self::ONE; }
        if self.is_zero() {
            return if exp.is_sign_negative() { 
                Self::INFINITY 
            } else { 
                Self::ZERO 
            };
        }
        
        if self.is_sign_negative() && !exp.is_integer() {
            return Self::NAN;
        }
        
        
        
        if exp.is_integer() {
            if let Some(n) = exp.to_i64_checked() {
                if n >= i32::MIN as i64 && n <= i32::MAX as i64 {
                    return self.powi(n as i32);
                }
            }
        }
        
        // General case: x^y = exp(y * ln(x))
        let ln_x = self.ln();
        if ln_x.is_nan() { return Self::NAN; }
        (exp * ln_x).exp()
    }

    /// [STABLE] [PERF-SENSITIVE] Exponential function.
    pub fn exp(self) -> Self {
        if self.is_nan() { return Self::NAN; }
        if self.is_zero() { return Self::ONE; }
        if self > Self::from(11356) { return Self::INFINITY; }
        if self < Self::from(-11356) { return Self::ZERO; }

        
        
        let k = (self / Self::LN_2).round();
        let r = self - k * Self::LN_2;
        let k_int = k.to_i64_checked().unwrap_or(0);
        
        
        let mut term = Self::ONE;
        let mut sum = Self::ONE;
        let mut n = 1;
        loop {
            term = term * r / Self::from(n);
            let new_sum = sum + term;
            if new_sum == sum { break; }
            sum = new_sum;
            n += 1;
            if n > 100 { break; }
        }
        
        if k_int > 0 {
            sum * Self::from(2i64).powi(k_int as i32)
        } else if k_int < 0 {
            sum / Self::from(2i64).powi((-k_int) as i32)
        } else {
            sum
        }
    }

    /// [STABLE] [PERF-SENSITIVE] Natural logarithm.
    pub fn ln(self) -> Self {
        if self.is_nan() || self.is_sign_negative() { return Self::NAN; }
        if self.is_zero() { return Self::NEG_INFINITY; }
        if self.is_infinite() { return Self::INFINITY; }
        if self.is_one() { return Self::ZERO; }

        
        
        let (sign, exp, mant) = self.decompose();
        debug_assert!(!sign);
        
        let e_f = Self::from(exp);
        let m = Self::compose(false, 0, mant); // 1.0 <= m < 2.0
        
        
        
        let one = Self::ONE;
        let z = (m - one) / (m + one);
        let z2 = z * z;
        
        let mut term = z;
        let mut sum = z;
        let mut n = 3u64;
        
        loop {
            term = term * z2;
            let add = term / Self::from(n);
            let new_sum = sum + add;
            if new_sum == sum { break; }
            sum = new_sum;
            n += 2;
            if n > 200 { break; }
        }
        
        e_f * Self::LN_2 + sum * Self::from(2)
    }

    #[inline] pub fn log2(self) -> Self { self.ln() / Self::LN_2 }
    #[inline] pub fn log10(self) -> Self { self.ln() / Self::from(2.302585092994045684f64) }
}

// =============================================================================
//         SECTION: TRIGONOMETRY
// =============================================================================

impl F128 {
    /// [STABLE] [PERF-SENSITIVE] Sine.
    pub fn sin(self) -> Self {
        if self.is_nan() || self.is_infinite() { return Self::NAN; }
        if self.is_zero() { return self; }
        
        let (k, r) = self.reduce_pi_2();
        let r2 = r * r;
        
        match k.rem_euclid(4) {
            0 => Self::taylor_sin(r, r2),
            1 => Self::taylor_cos(r, r2),
            2 => -Self::taylor_sin(r, r2),
            3 => -Self::taylor_cos(r, r2),
            _ => unreachable!(),
        }
    }

    /// [STABLE] [PERF-SENSITIVE] Cosine.
    pub fn cos(self) -> Self {
        if self.is_nan() || self.is_infinite() { return Self::NAN; }
        if self.is_zero() { return Self::ONE; }
        
        let (k, r) = self.reduce_pi_2();
        let r2 = r * r;
        
        match k.rem_euclid(4) {
            0 => Self::taylor_cos(r, r2),
            1 => -Self::taylor_sin(r, r2),
            2 => -Self::taylor_cos(r, r2),
            3 => Self::taylor_sin(r, r2),
            _ => unreachable!(),
        }
    }

    /// [STABLE] Tangent.
    pub fn tan(self) -> Self {
        let s = self.sin();
        let c = self.cos();
        s / c
    }

    /// [STABLE] Cotangent.
    pub fn ctg(self) -> Self {
        let s = self.sin();
        let c = self.cos();
        c / s
    }

    /// [STABLE] Arcsine.
    pub fn asin(self) -> Self {
        if self.is_nan() { return Self::NAN; }
        if self > Self::ONE || self < -Self::ONE { return Self::NAN; }
        if self.is_zero() { return self; }
        
        
        if self == Self::ONE { return Self::FRAC_PI_2; }
        if self == -Self::ONE { return -Self::FRAC_PI_2; }
        
        
        if self.abs() > Self::from(0.5) {
            let one = Self::ONE;
            let two = Self::from(2);
            let sub = (one - self.abs()) / two;
            let inner = sub.sqrt().asin();
            let res = Self::FRAC_PI_2 - two * inner;
            return if self.is_sign_negative() { -res } else { res };
        }
        
        
        let x2 = self * self;
        let mut term = self;
        let mut sum = self;
        let mut n: u64 = 1;
        
        for _ in 0..25 {
            let num = (2 * n - 1) * (2 * n - 1);
            let den = (2 * n) * (2 * n + 1);
            term = term * x2 * Self::from(num) / Self::from(den);
            sum = sum + term;
            n += 1;
            if term.is_zero() { break; }
        }
        sum
    }

    /// [STABLE] Arccosine.
    pub fn acos(self) -> Self {
        if self.is_nan() { return Self::NAN; }
        if self > Self::ONE || self < -Self::ONE { return Self::NAN; }
        if self == Self::ONE { return Self::ZERO; }
        if self == -Self::ONE { return Self::PI; }
        
        Self::FRAC_PI_2 - self.asin()
    }
}

// =============================================================================
//         SECTION: HELPERS (INTERNAL)
// =============================================================================

impl F128 {
    /// Reduces angle to range [-π/2, π/2].
    fn reduce_pi_2(self) -> (i64, Self) {
        if !self.is_finite() { return (0, Self::NAN); }
        
        let abs_self = self.abs();
        let limit = Self::from_bits(0x403F_0000_0000_0000, 0);
        
        if abs_self > limit { return (0, self); }
        
        let div = self / Self::FRAC_PI_2;
        let k_float = div.round();
        
        let k = if k_float > Self::from(i64::MAX) {
            i64::MAX
        } else if k_float < Self::from(i64::MIN) {
            i64::MIN
        } else {
            k_float.to_i64_checked().unwrap_or(0)
        };
        
        let k_f128 = Self::from(k);
        let r = self - k_f128 * Self::FRAC_PI_2;
        (k, r)
    }

    fn taylor_sin(x: Self, x2: Self) -> Self {
        let mut term = x;
        let mut sum = x;
        let mut n = 1u64;
        for _ in 0..20 {
            term = -term * x2 / Self::from((2 * n) * (2 * n + 1));
            sum = sum + term;
            n += 1;
            if term.is_zero() { break; }
        }
        sum
    }

    fn taylor_cos(x: Self, x2: Self) -> Self {
        let mut term = Self::ONE;
        let mut sum = Self::ONE;
        let mut n = 1u64;
        for _ in 0..20 {
            term = -term * x2 / Self::from((2 * n - 1) * (2 * n));
            sum = sum + term;
            n += 1;
            if term.is_zero() { break; }
        }
        sum
    }

    /// Safely converts F128 to i64 with overflow protection.
    fn to_i64_checked(self) -> Option<i64> {
        if !self.is_finite() { return None; }
        if self.is_zero() { return Some(0); }
        
        let (sign, exp, mant) = self.decompose();
        let frac_bits = Self::FRAC_BITS as i32;
        
        if exp >= 63 && (exp > 63 || mant >= (1u128 << (exp - frac_bits + 63))) {
            return if sign { Some(i64::MIN) } else { Some(i64::MAX) };
        }
        
        if exp < frac_bits - 128 { return Some(0); }
        
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

    /// Checks if the value is an exact integer.
    pub fn is_integer(self) -> bool {
        if self.is_nan() || self.is_infinite() { return false; }
        if self.is_zero() { return true; }

        let (_sign, exp, mant) = self.decompose();
        if exp < 0 { return false; }
        if exp >= Self::FRAC_BITS as i32 { return true; }
        
        let shift = (Self::FRAC_BITS as i32 - exp) as u32;
        if shift >= 128 { return false; }
        
        let frac_mask = (1u128 << shift) - 1;
        (mant & frac_mask) == 0
    }
}

// =============================================================================
//         SECTION: DISPLAY & DEBUG
// =============================================================================

impl Display for F128 {
    /// [STABLE] Formats the value as a string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nan() { return write!(f, "NaN"); }
        if self.is_infinite() { 
            return if self.is_sign_negative() { write!(f, "-inf") } else { write!(f, "inf") }; 
        }
        if self.is_zero() { 
            return if self.is_sign_negative() { write!(f, "-0") } else { write!(f, "0") }; 
        }

        if let Some(prec) = f.precision() {
            write!(f, "{:.*}", prec, self.to_f64())
        } else {
            write!(f, "{}", self.to_f64())
        }
    }
}

impl Debug for F128 {
    /// [STABLE] Debug representation (hex bits).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "F128(0x{:016X}_{:016X})", self.high, self.low)
    }
}

impl FromStr for F128 {
    type Err = ParseF128Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();

        if s.is_empty() {
            return Err(ParseF128Error(()));
        }

        let lower = s.to_ascii_lowercase();
        match lower.as_str() {
            "nan" | "-nan" | "+nan" => return Ok(F128::NAN),
            "inf" | "+inf" | "infinity" | "+infinity" => return Ok(F128::INFINITY),
            "-inf" | "-infinity" => return Ok(F128::NEG_INFINITY),
            _ => {}
        }

        let mut chars = s.chars().peekable();

        let negative = match chars.peek() {
            Some('-') => { chars.next(); true }
            Some('+') => { chars.next(); false }
            _ => false
        };

        let mut mantissa: u128 = 0;
        let mut mantissa_len: u32 = 0;
        let mut dot_pos: Option<u32> = None;
        let mut has_digits = false;

        while let Some(&c) = chars.peek() {
            if c == '.' {
                if dot_pos.is_some() {
                    return Err(ParseF128Error(()));
                }
                dot_pos = Some(mantissa_len);
                chars.next();
                continue;
            }
            if !c.is_ascii_digit() { break; }
            has_digits = true;
            chars.next();

            let digit = (c as u8 - b'0') as u128;
            if mantissa_len < 38 {
                mantissa = mantissa.wrapping_mul(10).wrapping_add(digit);
            }
            mantissa_len += 1;
        }

        if !has_digits { return Err(ParseF128Error(())); }

        let frac_digits = if let Some(pos) = dot_pos { mantissa_len - pos - 1 } else { 0 };

        let mut exp10: i32 = 0;
        if let Some(&c) = chars.peek() {
            if c == 'e' || c == 'E' {
                chars.next();

                let exp_neg = match chars.peek() {
                    Some('-') => { chars.next(); true }
                    Some('+') => { chars.next(); false }
                    _ => false
                };

                let mut exp_digits: u32 = 0;
                let mut has_exp_digits = false;

                while let Some(&c) = chars.peek() {
                    if !c.is_ascii_digit() { break; }
                    has_exp_digits = true;
                    chars.next();

                    let d = (c as u8 - b'0') as i32;
                    exp10 = exp10.saturating_mul(10).saturating_add(d);
                    exp_digits += 1;
                }

                if !has_exp_digits || exp_digits > 10 { return Err(ParseF128Error(())); }
                if exp_neg { exp10 = -exp10; }
            }
        }

        if chars.peek().is_some() { return Err(ParseF128Error(())); }

        exp10 = exp10.saturating_sub(frac_digits as i32);

        if mantissa == 0 { return Ok(if negative { F128::NEG_ZERO } else { F128::ZERO }); }

        let mut result = Self::from_u128(mantissa);
        if negative { result = -result; }

        if exp10 != 0 { result = Self::scale_by_power_of_10(result, exp10); }

        Ok(result)
    }
}

impl F128 {
    fn scale_by_power_of_10(mut val: F128, mut n: i32) -> F128 {
        if n == 0 || val.is_zero() || !val.is_finite() { return val; }
        
        const TEN: F128 = F128 { high: 0x4002_8000_0000_0000, low: 0 };
        if n > 0 {
            while n > 0 {
                if n >= 100 {
                    let factor = TEN.powi(100.min(n));
                    val = val * factor;
                    n -= 100;
                } else if n >= 10 {
                    let factor = TEN.powi(10.min(n));
                    val = val * factor;
                    n -= 10;
                } else {
                    val = val * TEN;
                    n -= 1;
                }
                
                if val.is_infinite() { break; }
            }
        } else {
            n = -n;
            while n > 0 {
                if n >= 100 {
                    let factor = TEN.powi(100.min(n));
                    val = val / factor;
                    n -= 100;
                } else if n >= 10 {
                    let factor = TEN.powi(10.min(n));
                    val = val / factor;
                    n -= 10;
                } else {
                    val = val / TEN;
                    n -= 1;
                }
                
                if val.is_zero() { break; }
            }
        }
        
        val
    }
    
    fn from_u128(mut v: u128) -> Self {
        if v == 0 { return Self::ZERO; }
        
        let msb = 127 - v.leading_zeros() as i32;
        let target = Self::FRAC_BITS as i32;
        
        let (exp, mant) = if msb > target {
            let shift = (msb - target) as u32;
            (msb, v >> shift)
        } else if msb < target {
            let shift = (target - msb) as u32;
            (msb, v << shift)
        } else {
            (msb, v)
        };
        
        Self::compose(false, exp, mant as u128)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseF128Error(());

impl Display for ParseF128Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid F128 literal")
    }
}

impl std::error::Error for ParseF128Error {}

// =============================================================================
//         SECTION: INTERNALS (U256)
// =============================================================================

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct U256 {
    d: [u64; 4],
}

impl U256 {
    const ZERO: Self = Self { d: [0, 0, 0, 0] };
    
    fn from_u128(x: u128) -> Self {
        Self { d: [x as u64, (x >> 64) as u64, 0, 0] }
    }
    
    fn mul_u128(a: u128, b: u128) -> Self {
        let a_lo = a & 0xFFFFFFFFFFFFFFFFu128;
        let a_hi = a >> 64;
        let b_lo = b & 0xFFFFFFFFFFFFFFFFu128;
        let b_hi = b >> 64;
        
        let p0 = a_lo * b_lo;
        let p1 = a_lo * b_hi;
        let p2 = a_hi * b_lo;
        let p3 = a_hi * b_hi;
        
        let mask = 0xFFFFFFFFFFFFFFFFu128;
        let r0 = p0 & mask;
        let t1 = (p0 >> 64) + (p1 & mask) + (p2 & mask);
        let r1 = t1 & mask;
        let t2 = (p1 >> 64) + (p2 >> 64) + (p3 & mask) + (t1 >> 64);
        let r2 = t2 & mask;
        let r3 = (p3 >> 64) + (t2 >> 64);
        
        Self { d: [r0 as u64, r1 as u64, r2 as u64, r3 as u64] }
    }
    
    fn shl(self, shift: u32) -> Self {
        if shift == 0 || self.is_zero() { return self; }
        if shift >= 256 { return Self::ZERO; }
        
        let limb_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let mut out = [0u64; 4];
        
        for i in 0..4 {
            let dest = i + limb_shift;
            if dest < 4 {
                out[dest] |= self.d[i] << bit_shift;
                if bit_shift > 0 && dest + 1 < 4 {
                    out[dest + 1] |= self.d[i] >> (64 - bit_shift);
                }
            }
        }
        Self { d: out }
    }
    
    fn shr(self, shift: u32) -> Self {
        if shift == 0 || self.is_zero() { return self; }
        if shift >= 256 { return Self::ZERO; }
        
        let limb_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let mut out = [0u64; 4];
        
        for i in limb_shift..4 {
            let src = i;
            let dest = i - limb_shift;
            out[dest] = if bit_shift == 0 {
                self.d[src]
            } else {
                (self.d[src] >> bit_shift) | 
                (if src + 1 < 4 { self.d[src + 1] << (64 - bit_shift) } else { 0 })
            };
        }
        Self { d: out }
    }
    
    fn sub(self, other: Self) -> Self {
        let mut out = [0u64; 4];
        let mut borrow = false;
        for i in 0..4 {
            let (diff, b1) = self.d[i].overflowing_sub(other.d[i]);
            let (res, b2) = diff.overflowing_sub(borrow as u64);
            out[i] = res;
            borrow = b1 || b2;
        }
        debug_assert!(!borrow, "U256 underflow");
        Self { d: out }
    }
    
    fn cmp(&self, other: &Self) -> Ordering {
        for i in (0..4).rev() {
            match self.d[i].cmp(&other.d[i]) {
                Ordering::Equal => continue,
                ord => return ord,
            }
        }
        Ordering::Equal
    }
    
    fn is_zero(&self) -> bool { self.d.iter().all(|&x| x == 0) }
    
    fn leading_zeros(&self) -> u32 {
        for i in (0..4).rev() {
            if self.d[i] != 0 {
                return self.d[i].leading_zeros() + (3 - i) as u32 * 64;
            }
        }
        256
    }
    
    fn low_u128(&self) -> u128 { ((self.d[1] as u128) << 64) | (self.d[0] as u128) }
}
