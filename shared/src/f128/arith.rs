//! Arithmetic operator implementations for [`F128`].

use super::F128;
use super::u256::U256;
use std::cmp::Ordering;
use std::ops::{Add, Div, Mul, Neg, Rem, Sub};

// =============================================================================
//         SECTION: ARITHMETIC
// =============================================================================

impl Add for F128 {
    type Output = Self;

    /// [STABLE] [PERF-SENSITIVE] Addition.
    fn add(self, rhs: Self) -> Self {
        if self.is_nan() || rhs.is_nan() {
            return Self::NAN;
        }

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

        if ma == 0 {
            return rhs;
        }
        if mb == 0 {
            return self;
        }

        let (e_res, m_a, m_b) = if ea >= eb {
            (ea, ma, mb >> (ea - eb).min(127))
        } else {
            (eb, ma >> (eb - ea).min(127), mb)
        };

        let (sign_res, mant_res) = if sa == sb {
            (sa, m_a + m_b)
        } else if m_a >= m_b {
            (sa, m_a - m_b)
        } else {
            (sb, m_b - m_a)
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
        if self.is_nan() {
            return self;
        }
        Self::from_bits(self.high ^ Self::SIGN_MASK, self.low)
    }
}

impl Mul for F128 {
    type Output = Self;

    /// [STABLE] [PERF-SENSITIVE] Multiplication.
    fn mul(self, rhs: Self) -> Self {
        if self.is_nan() || rhs.is_nan() {
            return Self::NAN;
        }

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
            return if sign {
                Self::NEG_INFINITY
            } else {
                Self::INFINITY
            };
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
        if self.is_nan() || rhs.is_nan() {
            return Self::NAN;
        }

        let s_zero = self.is_zero();
        let r_zero = rhs.is_zero();
        let s_inf = self.is_infinite();
        let r_inf = rhs.is_infinite();

        if (s_zero && r_zero) || (s_inf && r_inf) {
            return Self::NAN;
        }
        if s_zero || r_inf {
            let sign = self.is_sign_negative() ^ rhs.is_sign_negative();
            return if sign { Self::NEG_ZERO } else { Self::ZERO };
        }
        if r_zero || s_inf {
            let sign = self.is_sign_negative() ^ rhs.is_sign_negative();
            return if sign {
                Self::NEG_INFINITY
            } else {
                Self::INFINITY
            };
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
        if self.is_nan() || rhs.is_nan() {
            return Self::NAN;
        }
        if self.is_infinite() || rhs.is_zero() {
            return Self::NAN;
        }
        if rhs.is_infinite() || self.is_zero() {
            return self;
        }

        let div = self / rhs;
        let trunc = div.trunc();
        self - rhs * trunc
    }
}
