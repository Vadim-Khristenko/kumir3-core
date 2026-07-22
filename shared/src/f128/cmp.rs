//! Equality and ordering for [`F128`].

use super::F128;
use std::cmp::Ordering;

// =============================================================================
//         SECTION: COMPARISON
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

impl PartialOrd for F128 {
    /// [STABLE] Partial comparison. Returns None if either is NaN.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.is_nan() || other.is_nan() {
            return None;
        }
        Some(self.total_cmp(other))
    }

    #[inline]
    fn lt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Less))
    }
    #[inline]
    fn le(&self, other: &Self) -> bool {
        matches!(
            self.partial_cmp(other),
            Some(Ordering::Less | Ordering::Equal)
        )
    }
    #[inline]
    fn gt(&self, other: &Self) -> bool {
        matches!(self.partial_cmp(other), Some(Ordering::Greater))
    }
    #[inline]
    fn ge(&self, other: &Self) -> bool {
        matches!(
            self.partial_cmp(other),
            Some(Ordering::Greater | Ordering::Equal)
        )
    }
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
            return if self_nan {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }
        if self.is_zero() && other.is_zero() {
            return Ordering::Equal;
        }

        let self_neg = self.is_sign_negative();
        let other_neg = other.is_sign_negative();

        if self_neg != other_neg {
            return if self_neg {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let ord = (self.high, self.low).cmp(&(other.high, other.low));
        if self_neg { ord.reverse() } else { ord }
    }
}
