//! Formatting and parsing for [`F128`].

use super::F128;
use std::cmp::Ordering;
use std::fmt::{self, Debug, Display};
use std::str::FromStr;

// =============================================================================
//         SECTION: DISPLAY & DEBUG
// =============================================================================

impl Display for F128 {
    /// [STABLE] Formats the value as a string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_nan() {
            return write!(f, "NaN");
        }
        if self.is_infinite() {
            return if self.is_sign_negative() {
                write!(f, "-inf")
            } else {
                write!(f, "inf")
            };
        }
        if self.is_zero() {
            return if self.is_sign_negative() {
                write!(f, "-0")
            } else {
                write!(f, "0")
            };
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
            Some('-') => {
                chars.next();
                true
            }
            Some('+') => {
                chars.next();
                false
            }
            _ => false,
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
            if !c.is_ascii_digit() {
                break;
            }
            has_digits = true;
            chars.next();

            let digit = (c as u8 - b'0') as u128;
            if mantissa_len < 38 {
                mantissa = mantissa.wrapping_mul(10).wrapping_add(digit);
            }
            mantissa_len += 1;
        }

        if !has_digits {
            return Err(ParseF128Error(()));
        }

        let frac_digits = if let Some(pos) = dot_pos {
            mantissa_len - pos - 1
        } else {
            0
        };

        let mut exp10: i32 = 0;
        if let Some(&c) = chars.peek()
            && (c == 'e' || c == 'E')
        {
            chars.next();

            let exp_neg = match chars.peek() {
                Some('-') => {
                    chars.next();
                    true
                }
                Some('+') => {
                    chars.next();
                    false
                }
                _ => false,
            };

            let mut exp_digits: u32 = 0;
            let mut has_exp_digits = false;

            while let Some(&c) = chars.peek() {
                if !c.is_ascii_digit() {
                    break;
                }
                has_exp_digits = true;
                chars.next();

                let d = (c as u8 - b'0') as i32;
                exp10 = exp10.saturating_mul(10).saturating_add(d);
                exp_digits += 1;
            }

            if !has_exp_digits || exp_digits > 10 {
                return Err(ParseF128Error(()));
            }
            if exp_neg {
                exp10 = -exp10;
            }
        }

        if chars.peek().is_some() {
            return Err(ParseF128Error(()));
        }

        exp10 = exp10.saturating_sub(frac_digits as i32);

        if mantissa == 0 {
            return Ok(if negative { F128::NEG_ZERO } else { F128::ZERO });
        }

        let mut result = Self::from_u128(mantissa);
        if negative {
            result = -result;
        }

        if exp10 != 0 {
            result = Self::scale_by_power_of_10(result, exp10);
        }

        Ok(result)
    }
}

impl F128 {
    fn scale_by_power_of_10(mut val: F128, mut n: i32) -> F128 {
        if n == 0 || val.is_zero() || !val.is_finite() {
            return val;
        }

        const TEN: F128 = F128 {
            high: 0x4002_8000_0000_0000,
            low: 0,
        };
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

                if val.is_infinite() {
                    break;
                }
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

                if val.is_zero() {
                    break;
                }
            }
        }

        val
    }

    fn from_u128(v: u128) -> Self {
        if v == 0 {
            return Self::ZERO;
        }

        let msb = 127 - v.leading_zeros() as i32;
        let target = Self::FRAC_BITS as i32;

        let (exp, mant) = match msb.cmp(&target) {
            Ordering::Greater => {
                let shift = (msb - target) as u32;
                (msb, v >> shift)
            }
            Ordering::Less => {
                let shift = (target - msb) as u32;
                (msb, v << shift)
            }
            Ordering::Equal => (msb, v),
        };

        Self::compose(false, exp, mant)
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
