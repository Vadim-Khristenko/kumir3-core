//! Rounding and truncation for [`F128`].

use super::F128;
use std::cmp::Ordering;

// =============================================================================
//         SECTION: ROUNDING
// =============================================================================

impl F128 {
    /// Truncate toward zero (remove fractional part).
    ///
    /// # Returns
    /// - Integer part of `self`, rounded toward zero
    /// - Returns `self` unchanged for NaN, infinite, or zero
    pub fn trunc(self) -> Self {
        if !self.is_finite() || self.is_zero() {
            return self;
        }

        let (sign, exp, mant) = self.decompose();
        let frac_bits = Self::FRAC_BITS as i32;

        if exp >= frac_bits {
            return self;
        }
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
        if !self.is_finite() || self.is_zero() {
            return self;
        }

        let trunc = self.trunc();
        if self.is_sign_negative() && self != trunc {
            trunc - Self::ONE
        } else {
            trunc
        }
    }

    /// [STABLE] Round up.
    pub fn ceil(self) -> Self {
        if !self.is_finite() || self.is_zero() {
            return self;
        }

        let trunc = self.trunc();
        if !self.is_sign_negative() && self != trunc {
            trunc + Self::ONE
        } else {
            trunc
        }
    }

    /// Round to nearest integer (IEEE 754 round-half-to-even).
    pub fn round(self) -> Self {
        self.round_to(0)
    }

    /// Round to a specific number of decimal places.
    ///
    /// # Arguments
    /// * `decimal_places` - Number of decimal places:
    ///   - `positive`: округление до дробной части (1 = десятые, 2 = сотые, 3 = тысячные)
    ///   - `0`: до целого (эквивалентно `round()`)
    ///   - `negative`: округление до разрядов целой части (-1 = до десятков, -2 = до сотен)
    ///
    /// # Examples
    /// ```
    /// use shared::f128::F128;
    /// let x = F128::from(123.456);
    /// // Compare via f64: building F128 from an f64 literal is lossy, so an
    /// // exact F128 bit-comparison against `F128::from(123.46)` would be brittle.
    /// assert_eq!(x.round_to(2).to_f64(), 123.46);  // до сотых
    /// assert_eq!(x.round_to(0).to_f64(), 123.0);   // до целых
    /// assert_eq!(x.round_to(-1).to_f64(), 120.0);  // до десятков
    /// assert_eq!(x.round_to(-2).to_f64(), 100.0);  // до сотен
    /// ```
    pub fn round_to(self, decimal_places: i32) -> Self {
        if !self.is_finite() {
            return self;
        }

        if decimal_places == 0 {
            return self.round_to_integer();
        }

        let abs_places = decimal_places.unsigned_abs();
        let factor = Self::from(10u64).powi(abs_places as i32);

        if !factor.is_finite() || factor.is_zero() {
            return self;
        }

        if decimal_places > 0 {
            let scaled = self * factor;
            if scaled.is_infinite() {
                return self;
            }
            scaled.round_to_integer() / factor
        } else {
            let scaled = self / factor;
            if scaled.is_zero() && !self.is_zero() {
                return if self.is_sign_negative() {
                    Self::NEG_ZERO
                } else {
                    Self::ZERO
                };
            }
            scaled.round_to_integer() * factor
        }
    }

    fn round_to_integer(self) -> Self {
        if !self.is_finite() || self.is_zero() {
            return self;
        }

        let trunc = self.trunc();
        let frac = self - trunc;

        if frac.is_zero() {
            return self;
        }

        let half = Self::from(0.5);
        let abs_frac = frac.abs();

        match abs_frac.partial_cmp(&half) {
            Some(Ordering::Less) => trunc,
            Some(Ordering::Greater) => trunc + Self::copysign(Self::ONE, self),
            Some(Ordering::Equal) => {
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
}
