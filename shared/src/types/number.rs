//! Numeric types of the Kumir language
//!
//! [STABLE] Supports all numeric types corresponding to Kumir types:
//! - `цел` / `цел_64` → I64 (default)
//! - `малое_цел` / `цел_32` → I32
//! - `большое_цел` / `цел_128` → I128
//! - `вещ` → F64 (default)
//! - `малое_вещ` → F32
//! - `большое_вещ` → F128 (high precision)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      Number (Числа)                             │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  Signed Integers: I8, I16, I32, I64, I128                       │
//! │  Unsigned Integers: U8, U16, U32, U64, U128                     │
//! │  Floating Point: F32, F64, F128                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use std::cmp::Ordering;

use crate::f128::F128;

/// [STABLE] Universal representation of numbers in Value.
#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    // -------------------------------------------------------------------------
    // Signed integers
    // -------------------------------------------------------------------------
    I8(i8),     // цел_8
    I16(i16),   // цел_16
    I32(i32),   // цел_32, малое_цел
    I64(i64),   // цел_64, цел (default)
    I128(i128), // цел_128, большое_цел

    // -------------------------------------------------------------------------
    // Unsigned integers
    // -------------------------------------------------------------------------
    U8(u8),     // нат_8
    U16(u16),   // нат_16
    U32(u32),   // нат_32
    U64(u64),   // нат_64
    U128(u128), // нат_128

    // -------------------------------------------------------------------------
    // Floating point
    // -------------------------------------------------------------------------
    F32(f32),   // вещ_32, малое_вещ
    F64(f64),   // вещ_64, вещ (default)
    F128(F128), // вещ_128, большое_вещ
}

impl Number {
    /// Converts to i64 (if possible without loss of significant digits)
    pub fn to_i64(&self) -> Option<i64> {
        match self {
            Number::I8(v) => Some(*v as i64),
            Number::I16(v) => Some(*v as i64),
            Number::I32(v) => Some(*v as i64),
            Number::I64(v) => Some(*v),
            Number::I128(v) => i64::try_from(*v).ok(),
            Number::U8(v) => Some(*v as i64),
            Number::U16(v) => Some(*v as i64),
            Number::U32(v) => Some(*v as i64),
            Number::U64(v) => i64::try_from(*v).ok(),
            Number::U128(v) => i64::try_from(*v).ok(),
            Number::F32(v) => Some(*v as i64),
            Number::F64(v) => Some(*v as i64),
            Number::F128(v) => Some(v.to_f64() as i64),
        }
    }

    /// Converts to f64
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Number::I8(v) => Some(*v as f64),
            Number::I16(v) => Some(*v as f64),
            Number::I32(v) => Some(*v as f64),
            Number::I64(v) => Some(*v as f64),
            Number::I128(v) => Some(*v as f64),
            Number::U8(v) => Some(*v as f64),
            Number::U16(v) => Some(*v as f64),
            Number::U32(v) => Some(*v as f64),
            Number::U64(v) => Some(*v as f64),
            Number::U128(v) => Some(*v as f64),
            Number::F32(v) => Some(*v as f64),
            Number::F64(v) => Some(*v),
            Number::F128(v) => Some(v.to_f64()),
        }
    }

    // =========================================================================
    //         SECTION: SIGNED INTEGER CONVERSIONS
    // =========================================================================

    /// Converts to i8
    pub fn to_i8(&self) -> Option<i8> {
        match self {
            Number::I8(v) => Some(*v),
            Number::I16(v) => i8::try_from(*v).ok(),
            Number::I32(v) => i8::try_from(*v).ok(),
            Number::I64(v) => i8::try_from(*v).ok(),
            Number::I128(v) => i8::try_from(*v).ok(),
            Number::U8(v) => i8::try_from(*v).ok(),
            Number::U16(v) => i8::try_from(*v).ok(),
            Number::U32(v) => i8::try_from(*v).ok(),
            Number::U64(v) => i8::try_from(*v).ok(),
            Number::U128(v) => i8::try_from(*v).ok(),
            Number::F32(v) => Some(*v as i8),
            Number::F64(v) => Some(*v as i8),
            Number::F128(v) => Some(v.to_f64() as i8),
        }
    }

    /// Converts to i16
    pub fn to_i16(&self) -> Option<i16> {
        match self {
            Number::I8(v) => Some(*v as i16),
            Number::I16(v) => Some(*v),
            Number::I32(v) => i16::try_from(*v).ok(),
            Number::I64(v) => i16::try_from(*v).ok(),
            Number::I128(v) => i16::try_from(*v).ok(),
            Number::U8(v) => Some(*v as i16),
            Number::U16(v) => i16::try_from(*v).ok(),
            Number::U32(v) => i16::try_from(*v).ok(),
            Number::U64(v) => i16::try_from(*v).ok(),
            Number::U128(v) => i16::try_from(*v).ok(),
            Number::F32(v) => Some(*v as i16),
            Number::F64(v) => Some(*v as i16),
            Number::F128(v) => Some(v.to_f64() as i16),
        }
    }

    /// Converts to i32
    pub fn to_i32(&self) -> Option<i32> {
        match self {
            Number::I8(v) => Some(*v as i32),
            Number::I16(v) => Some(*v as i32),
            Number::I32(v) => Some(*v),
            Number::I64(v) => i32::try_from(*v).ok(),
            Number::I128(v) => i32::try_from(*v).ok(),
            Number::U8(v) => Some(*v as i32),
            Number::U16(v) => Some(*v as i32),
            Number::U32(v) => i32::try_from(*v).ok(),
            Number::U64(v) => i32::try_from(*v).ok(),
            Number::U128(v) => i32::try_from(*v).ok(),
            Number::F32(v) => Some(*v as i32),
            Number::F64(v) => Some(*v as i32),
            Number::F128(v) => Some(v.to_f64() as i32),
        }
    }

    /// Converts to i128
    pub fn to_i128(&self) -> Option<i128> {
        match self {
            Number::I8(v) => Some(*v as i128),
            Number::I16(v) => Some(*v as i128),
            Number::I32(v) => Some(*v as i128),
            Number::I64(v) => Some(*v as i128),
            Number::I128(v) => Some(*v),
            Number::U8(v) => Some(*v as i128),
            Number::U16(v) => Some(*v as i128),
            Number::U32(v) => Some(*v as i128),
            Number::U64(v) => Some(*v as i128),
            Number::U128(v) => i128::try_from(*v).ok(),
            Number::F32(v) => Some(*v as i128),
            Number::F64(v) => Some(*v as i128),
            Number::F128(v) => Some(v.to_f64() as i128),
        }
    }

    // =========================================================================
    //         SECTION: UNSIGNED INTEGER CONVERSIONS
    // =========================================================================

    /// Converts to u8
    pub fn to_u8(&self) -> Option<u8> {
        match self {
            Number::I8(v) => u8::try_from(*v).ok(),
            Number::I16(v) => u8::try_from(*v).ok(),
            Number::I32(v) => u8::try_from(*v).ok(),
            Number::I64(v) => u8::try_from(*v).ok(),
            Number::I128(v) => u8::try_from(*v).ok(),
            Number::U8(v) => Some(*v),
            Number::U16(v) => u8::try_from(*v).ok(),
            Number::U32(v) => u8::try_from(*v).ok(),
            Number::U64(v) => u8::try_from(*v).ok(),
            Number::U128(v) => u8::try_from(*v).ok(),
            Number::F32(v) => {
                if *v >= 0.0 && *v <= u8::MAX as f32 {
                    Some(*v as u8)
                } else {
                    None
                }
            }
            Number::F64(v) => {
                if *v >= 0.0 && *v <= u8::MAX as f64 {
                    Some(*v as u8)
                } else {
                    None
                }
            }
            Number::F128(v) => {
                let f = v.to_f64();
                if f >= 0.0 && f <= u8::MAX as f64 {
                    Some(f as u8)
                } else {
                    None
                }
            }
        }
    }

    /// Converts to u16
    pub fn to_u16(&self) -> Option<u16> {
        match self {
            Number::I8(v) => u16::try_from(*v).ok(),
            Number::I16(v) => u16::try_from(*v).ok(),
            Number::I32(v) => u16::try_from(*v).ok(),
            Number::I64(v) => u16::try_from(*v).ok(),
            Number::I128(v) => u16::try_from(*v).ok(),
            Number::U8(v) => Some(*v as u16),
            Number::U16(v) => Some(*v),
            Number::U32(v) => u16::try_from(*v).ok(),
            Number::U64(v) => u16::try_from(*v).ok(),
            Number::U128(v) => u16::try_from(*v).ok(),
            Number::F32(v) => {
                if *v >= 0.0 && *v <= u16::MAX as f32 {
                    Some(*v as u16)
                } else {
                    None
                }
            }
            Number::F64(v) => {
                if *v >= 0.0 && *v <= u16::MAX as f64 {
                    Some(*v as u16)
                } else {
                    None
                }
            }
            Number::F128(v) => {
                let f = v.to_f64();
                if f >= 0.0 && f <= u16::MAX as f64 {
                    Some(f as u16)
                } else {
                    None
                }
            }
        }
    }

    /// Converts to u32
    pub fn to_u32(&self) -> Option<u32> {
        match self {
            Number::I8(v) => u32::try_from(*v).ok(),
            Number::I16(v) => u32::try_from(*v).ok(),
            Number::I32(v) => u32::try_from(*v).ok(),
            Number::I64(v) => u32::try_from(*v).ok(),
            Number::I128(v) => u32::try_from(*v).ok(),
            Number::U8(v) => Some(*v as u32),
            Number::U16(v) => Some(*v as u32),
            Number::U32(v) => Some(*v),
            Number::U64(v) => u32::try_from(*v).ok(),
            Number::U128(v) => u32::try_from(*v).ok(),
            Number::F32(v) => {
                if *v >= 0.0 && *v <= u32::MAX as f32 {
                    Some(*v as u32)
                } else {
                    None
                }
            }
            Number::F64(v) => {
                if *v >= 0.0 && *v <= u32::MAX as f64 {
                    Some(*v as u32)
                } else {
                    None
                }
            }
            Number::F128(v) => {
                let f = v.to_f64();
                if f >= 0.0 && f <= u32::MAX as f64 {
                    Some(f as u32)
                } else {
                    None
                }
            }
        }
    }

    /// Converts to u64
    pub fn to_u64(&self) -> Option<u64> {
        match self {
            Number::I8(v) => u64::try_from(*v).ok(),
            Number::I16(v) => u64::try_from(*v).ok(),
            Number::I32(v) => u64::try_from(*v).ok(),
            Number::I64(v) => u64::try_from(*v).ok(),
            Number::I128(v) => u64::try_from(*v).ok(),
            Number::U8(v) => Some(*v as u64),
            Number::U16(v) => Some(*v as u64),
            Number::U32(v) => Some(*v as u64),
            Number::U64(v) => Some(*v),
            Number::U128(v) => u64::try_from(*v).ok(),
            Number::F32(v) => {
                if *v >= 0.0 {
                    Some(*v as u64)
                } else {
                    None
                }
            }
            Number::F64(v) => {
                if *v >= 0.0 {
                    Some(*v as u64)
                } else {
                    None
                }
            }
            Number::F128(v) => {
                let f = v.to_f64();
                if f >= 0.0 { Some(f as u64) } else { None }
            }
        }
    }

    /// Converts to u128
    pub fn to_u128(&self) -> Option<u128> {
        match self {
            Number::I8(v) => u128::try_from(*v).ok(),
            Number::I16(v) => u128::try_from(*v).ok(),
            Number::I32(v) => u128::try_from(*v).ok(),
            Number::I64(v) => u128::try_from(*v).ok(),
            Number::I128(v) => u128::try_from(*v).ok(),
            Number::U8(v) => Some(*v as u128),
            Number::U16(v) => Some(*v as u128),
            Number::U32(v) => Some(*v as u128),
            Number::U64(v) => Some(*v as u128),
            Number::U128(v) => Some(*v),
            Number::F32(v) => {
                if *v >= 0.0 {
                    Some(*v as u128)
                } else {
                    None
                }
            }
            Number::F64(v) => {
                if *v >= 0.0 {
                    Some(*v as u128)
                } else {
                    None
                }
            }
            Number::F128(v) => {
                let f = v.to_f64();
                if f >= 0.0 { Some(f as u128) } else { None }
            }
        }
    }

    // === Floating point ===

    /// Converts to f32
    pub fn to_f32(&self) -> Option<f32> {
        match self {
            Number::I8(v) => Some(*v as f32),
            Number::I16(v) => Some(*v as f32),
            Number::I32(v) => Some(*v as f32),
            Number::I64(v) => Some(*v as f32),
            Number::I128(v) => Some(*v as f32),
            Number::U8(v) => Some(*v as f32),
            Number::U16(v) => Some(*v as f32),
            Number::U32(v) => Some(*v as f32),
            Number::U64(v) => Some(*v as f32),
            Number::U128(v) => Some(*v as f32),
            Number::F32(v) => Some(*v),
            Number::F64(v) => Some(*v as f32),
            Number::F128(v) => Some(v.to_f64() as f32),
        }
    }

    /// Converts to F128 (high precision)
    pub fn to_f128(&self) -> F128 {
        match self {
            Number::I8(v) => F128::from(*v as f64),
            Number::I16(v) => F128::from(*v as f64),
            Number::I32(v) => F128::from(*v as f64),
            Number::I64(v) => F128::from(*v as f64),
            Number::I128(v) => F128::from(*v as f64),
            Number::U8(v) => F128::from(*v as f64),
            Number::U16(v) => F128::from(*v as f64),
            Number::U32(v) => F128::from(*v as f64),
            Number::U64(v) => F128::from(*v as f64),
            Number::U128(v) => F128::from(*v as f64),
            Number::F32(v) => F128::from(*v as f64),
            Number::F64(v) => F128::from(*v),
            Number::F128(v) => *v,
        }
    }

    // === ROUNDING ===

    /// Rounds the number to the specified number of decimal places.
    ///
    /// # Examples
    /// ```
    /// use shared::types::Number;
    ///
    /// let n = Number::F64(3.14159);
    /// assert_eq!(n.round_to(2), Number::F64(3.14));
    /// assert_eq!(n.round_to(0), Number::F64(3.0));
    ///
    /// let n = Number::I64(42);
    /// assert_eq!(n.round_to(2), Number::I64(42)); // Integers remain unchanged
    /// ```
    pub fn round_to(&self, decimals: i32) -> Number {
        match self {
            // Integers do not require rounding
            Number::I8(_)
            | Number::I16(_)
            | Number::I32(_)
            | Number::I64(_)
            | Number::I128(_)
            | Number::U8(_)
            | Number::U16(_)
            | Number::U32(_)
            | Number::U64(_)
            | Number::U128(_) => self.clone(),

            // Floating-point numbers
            Number::F32(v) => {
                let multiplier = 10f32.powi(decimals);
                Number::F32((v * multiplier).round() / multiplier)
            }
            Number::F64(v) => {
                let multiplier = 10f64.powi(decimals);
                Number::F64((v * multiplier).round() / multiplier)
            }
            Number::F128(v) => {
                let rounded = F128::round_to(*v, decimals);
                Number::F128(rounded)
            }
        }
    }

    /// Returns true if the number is an integer
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Number::I8(_)
                | Number::I16(_)
                | Number::I32(_)
                | Number::I64(_)
                | Number::I128(_)
                | Number::U8(_)
                | Number::U16(_)
                | Number::U32(_)
                | Number::U64(_)
                | Number::U128(_)
        )
    }

    /// Returns true if the number is floating-point
    pub fn is_float(&self) -> bool {
        matches!(self, Number::F32(_) | Number::F64(_) | Number::F128(_))
    }

    /// Returns true if the number is signed
    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            Number::I8(_)
                | Number::I16(_)
                | Number::I32(_)
                | Number::I64(_)
                | Number::I128(_)
                | Number::F32(_)
                | Number::F64(_)
                | Number::F128(_)
        )
    }

    /// Returns true if the number is unsigned
    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            Number::U8(_) | Number::U16(_) | Number::U32(_) | Number::U64(_) | Number::U128(_)
        )
    }

    /// Returns the absolute value of the number
    pub fn abs(&self) -> Number {
        match self {
            Number::I8(v) => Number::I8(v.abs()),
            Number::I16(v) => Number::I16(v.abs()),
            Number::I32(v) => Number::I32(v.abs()),
            Number::I64(v) => Number::I64(v.abs()),
            Number::I128(v) => Number::I128(v.abs()),
            Number::U8(v) => Number::U8(*v),
            Number::U16(v) => Number::U16(*v),
            Number::U32(v) => Number::U32(*v),
            Number::U64(v) => Number::U64(*v),
            Number::U128(v) => Number::U128(*v),
            Number::F32(v) => Number::F32(v.abs()),
            Number::F64(v) => Number::F64(v.abs()),
            Number::F128(v) => Number::F128(v.abs()),
        }
    }

    /// Floor (round down)
    pub fn floor(&self) -> Number {
        match self {
            Number::F32(v) => Number::F32(v.floor()),
            Number::F64(v) => Number::F64(v.floor()),
            Number::F128(v) => Number::F128(v.floor()),
            other => other.clone(),
        }
    }

    /// Ceil (round up)
    pub fn ceil(&self) -> Number {
        match self {
            Number::F32(v) => Number::F32(v.ceil()),
            Number::F64(v) => Number::F64(v.ceil()),
            Number::F128(v) => Number::F128(v.ceil()),
            other => other.clone(),
        }
    }

    /// Round to the nearest integer
    pub fn round(&self) -> Number {
        match self {
            Number::F32(v) => Number::F32(v.round()),
            Number::F64(v) => Number::F64(v.round()),
            Number::F128(v) => Number::F128(v.round()),
            other => other.clone(),
        }
    }

    /// Truncate fractional part (trunc)
    pub fn trunc(&self) -> Number {
        match self {
            Number::F32(v) => Number::F32(v.trunc()),
            Number::F64(v) => Number::F64(v.trunc()),
            Number::F128(v) => Number::F128(v.trunc()),
            other => other.clone(),
        }
    }
}

// --- Macros for boilerplate implementations ---

macro_rules! impl_from_number {
    ($($t:ty => $v:ident),+ $(,)?) => {
        $(
            impl From<$t> for Number {
                fn from(v: $t) -> Self { Number::$v(v) }
            }
        )+
    };
}

impl_from_number!(
    i8 => I8, i16 => I16, i32 => I32, i64 => I64, i128 => I128,
    u8 => U8, u16 => U16, u32 => U32, u64 => U64, u128 => U128,
    f32 => F32, f64 => F64, F128 => F128
);

// =============================================================================
//         SECTION: DISPLAY IMPLEMENTATION
// =============================================================================

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Number::I8(v) => write!(f, "{}", v),
            Number::I16(v) => write!(f, "{}", v),
            Number::I32(v) => write!(f, "{}", v),
            Number::I64(v) => write!(f, "{}", v),
            Number::I128(v) => write!(f, "{}", v),
            Number::U8(v) => write!(f, "{}", v),
            Number::U16(v) => write!(f, "{}", v),
            Number::U32(v) => write!(f, "{}", v),
            Number::U64(v) => write!(f, "{}", v),
            Number::U128(v) => write!(f, "{}", v),
            Number::F32(v) => write!(f, "{}", v),
            Number::F64(v) => write!(f, "{}", v),
            Number::F128(v) => write!(f, "{}", v),
        }
    }
}

// =============================================================================
//         SECTION: ZERO / SIGN CHECKS
// =============================================================================

impl Number {
    /// Returns true if the number equals zero
    pub fn is_zero(&self) -> bool {
        match self {
            Number::I8(v) => *v == 0,
            Number::I16(v) => *v == 0,
            Number::I32(v) => *v == 0,
            Number::I64(v) => *v == 0,
            Number::I128(v) => *v == 0,
            Number::U8(v) => *v == 0,
            Number::U16(v) => *v == 0,
            Number::U32(v) => *v == 0,
            Number::U64(v) => *v == 0,
            Number::U128(v) => *v == 0,
            Number::F32(v) => *v == 0.0,
            Number::F64(v) => *v == 0.0,
            Number::F128(v) => v.is_zero(),
        }
    }

    /// Returns true if the number is negative
    pub fn is_negative(&self) -> bool {
        match self {
            Number::I8(v) => *v < 0,
            Number::I16(v) => *v < 0,
            Number::I32(v) => *v < 0,
            Number::I64(v) => *v < 0,
            Number::I128(v) => *v < 0,
            Number::U8(_) | Number::U16(_) | Number::U32(_) | Number::U64(_) | Number::U128(_) => {
                false
            }
            Number::F32(v) => *v < 0.0,
            Number::F64(v) => *v < 0.0,
            Number::F128(v) => v.is_sign_negative(),
        }
    }

    /// Returns true if the number is positive
    pub fn is_positive(&self) -> bool {
        match self {
            Number::I8(v) => *v > 0,
            Number::I16(v) => *v > 0,
            Number::I32(v) => *v > 0,
            Number::I64(v) => *v > 0,
            Number::I128(v) => *v > 0,
            Number::U8(v) => *v > 0,
            Number::U16(v) => *v > 0,
            Number::U32(v) => *v > 0,
            Number::U64(v) => *v > 0,
            Number::U128(v) => *v > 0,
            Number::F32(v) => *v > 0.0,
            Number::F64(v) => *v > 0.0,
            Number::F128(v) => v.is_sign_positive(),
        }
    }
}

// =============================================================================
//         SECTION: ORDERING BY NUMERIC VALUE
// =============================================================================

/// Precise decomposition of a number for comparison.
///
/// Kept separate from [`Number`] so that comparison never has to look at which
/// of the thirteen variants a value happens to be stored in: `цел` and `вещ`
/// holding the same quantity must compare equal, and a `u128` above
/// `i128::MAX` must not silently wrap.
enum Numeric {
    /// Integer representable in `i128` — every integer type except a `u128`
    /// whose value exceeds `i128::MAX`.
    Int(i128),
    /// `u128` beyond `i128::MAX`: no signed type can hold it.
    Big(u128),
    /// Float; `float32` and `float64` embed in `float128` exactly.
    Real(F128),
}

impl Number {
    /// Orders two numbers by value, without an intermediate `f64` rounding.
    ///
    /// This is the single source of truth for numeric order in the language:
    /// the `<`, `<=`, `>`, `>=` operators, equality, sorting and hashing all
    /// route here. Comparing the stored representation instead — or the printed
    /// text — is what made `сортировать` order `[100, 9]` as `[100, 9]`.
    ///
    /// `NaN` is incomparable; it is reported as [`Ordering::Equal`] so that
    /// sorting stays total and cannot panic.
    pub fn order(&self, other: &Number) -> Ordering {
        match (Self::classify(self), Self::classify(other)) {
            (Numeric::Int(x), Numeric::Int(y)) => x.cmp(&y),
            (Numeric::Big(x), Numeric::Big(y)) => x.cmp(&y),
            // `Big` exceeds `i128::MAX` by definition, hence any `Int`.
            (Numeric::Int(_), Numeric::Big(_)) => Ordering::Less,
            (Numeric::Big(_), Numeric::Int(_)) => Ordering::Greater,
            (x, y) => Self::widen(x)
                .partial_cmp(&Self::widen(y))
                .unwrap_or(Ordering::Equal),
        }
    }

    /// Equality by value: `1` and `1.0` denote the same number.
    pub fn equals(&self, other: &Number) -> bool {
        // A `NaN` must not equal itself, but `order` reports incomparable pairs
        // as equal, so the check is made explicitly.
        if self.is_nan() || other.is_nan() {
            return false;
        }
        self.order(other) == Ordering::Equal
    }

    /// Is this a float holding `NaN`?
    pub fn is_nan(&self) -> bool {
        match self {
            Number::F32(v) => v.is_nan(),
            Number::F64(v) => v.is_nan(),
            Number::F128(v) => v.is_nan(),
            _ => false,
        }
    }

    /// Canonical key for hashing, consistent with [`Number::equals`].
    ///
    /// Equal values must hash equally, so a float with no fractional part
    /// hashes as the integer it denotes.
    pub fn hash_key(&self) -> (u8, i128, u64) {
        match Self::classify(self) {
            Numeric::Int(i) => (0, i, 0),
            Numeric::Big(u) => (1, 0, u as u64),
            Numeric::Real(f) => {
                let as_f64 = f.to_f64();
                if as_f64.fract() == 0.0 && as_f64.abs() < i128::MAX as f64 {
                    (0, as_f64 as i128, 0)
                } else {
                    (2, 0, as_f64.to_bits())
                }
            }
        }
    }

    /// Decomposes a number into the precise form used for comparison.
    fn classify(n: &Number) -> Numeric {
        match n {
            Number::I8(v) => Numeric::Int(*v as i128),
            Number::I16(v) => Numeric::Int(*v as i128),
            Number::I32(v) => Numeric::Int(*v as i128),
            Number::I64(v) => Numeric::Int(*v as i128),
            Number::I128(v) => Numeric::Int(*v),
            Number::U8(v) => Numeric::Int(*v as i128),
            Number::U16(v) => Numeric::Int(*v as i128),
            Number::U32(v) => Numeric::Int(*v as i128),
            Number::U64(v) => Numeric::Int(*v as i128),
            Number::U128(v) => match i128::try_from(*v) {
                Ok(i) => Numeric::Int(i),
                Err(_) => Numeric::Big(*v),
            },
            Number::F32(v) => Numeric::Real(F128::from(*v)),
            Number::F64(v) => Numeric::Real(F128::from(*v)),
            Number::F128(v) => Numeric::Real(*v),
        }
    }

    /// Coerces any form to the widest representation — `float128`.
    ///
    /// Integers up to 113 significant bits convert exactly; only `int128` and
    /// `u128` beyond that round, and even then 60 bits more precisely than an
    /// `f64` coercion would.
    fn widen(n: Numeric) -> F128 {
        match n {
            Numeric::Real(f) => f,
            Numeric::Int(i) => Self::int_to_real(i.unsigned_abs(), i < 0),
            Numeric::Big(u) => Self::int_to_real(u, false),
        }
    }

    /// Builds a `float128` from a sign and an integer magnitude, splitting the
    /// magnitude into two 64-bit halves — both convert exactly.
    fn int_to_real(magnitude: u128, negative: bool) -> F128 {
        /// 2^64 — exactly representable in both `f64` and `float128`.
        const TWO_POW_64: f64 = 18_446_744_073_709_551_616.0;

        let low = F128::from(magnitude as u64);
        let high = (magnitude >> 64) as u64;
        let value = if high == 0 {
            low
        } else {
            F128::from(high) * F128::from(TWO_POW_64) + low
        };
        if negative { -value } else { value }
    }
}

// =============================================================================
//         SECTION: TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_and_float_of_the_same_quantity_are_equal() {
        assert!(Number::I64(1).equals(&Number::F64(1.0)));
        assert!(Number::F32(-3.0).equals(&Number::I8(-3)));
        assert!(!Number::I64(1).equals(&Number::F64(1.5)));
    }

    #[test]
    fn order_follows_value_not_printed_text() {
        // The defect this guards: comparing `to_string()` puts "100" before "9".
        assert_eq!(Number::I64(9).order(&Number::I64(100)), Ordering::Less);
        assert_eq!(Number::I64(100).order(&Number::I64(9)), Ordering::Greater);
        assert_eq!(Number::I64(21).order(&Number::F64(21.0)), Ordering::Equal);
    }

    #[test]
    fn sorting_a_mixed_slice_yields_ascending_values() {
        let mut ns = [
            Number::I64(100),
            Number::F64(9.5),
            Number::I64(21),
            Number::I64(3),
        ];
        ns.sort_by(Number::order);
        let as_f64: Vec<f64> = ns.iter().map(|n| n.to_f64().unwrap()).collect();
        assert_eq!(as_f64, vec![3.0, 9.5, 21.0, 100.0]);
    }

    #[test]
    fn unsigned_beyond_signed_range_stays_above_every_signed_value() {
        let big = Number::U128(u128::MAX);
        assert_eq!(big.order(&Number::I128(i128::MAX)), Ordering::Greater);
        assert_eq!(Number::I128(i128::MAX).order(&big), Ordering::Less);
    }

    #[test]
    fn not_a_number_equals_nothing_including_itself() {
        let nan = Number::F64(f64::NAN);
        assert!(!nan.equals(&nan));
        assert!(!nan.equals(&Number::I64(0)));
    }

    #[test]
    fn equal_values_share_a_hash_key() {
        // Required by the `Hash`/`Eq` contract: `1` and `1.0` are one key.
        assert_eq!(Number::I64(1).hash_key(), Number::F64(1.0).hash_key());
        assert_ne!(Number::I64(1).hash_key(), Number::F64(1.5).hash_key());
    }
}
