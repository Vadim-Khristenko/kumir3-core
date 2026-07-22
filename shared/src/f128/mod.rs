// =============================================================================
//         SECTION: MODULES
// =============================================================================

mod arith;
mod bits;
mod cmp;
mod convert;
mod fmt;
mod math;
mod round;
mod u256;

pub use self::fmt::ParseF128Error;

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
//         SECTION: CONSTANTS
// =============================================================================

impl F128 {
    pub const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
    pub const EXP_MASK: u64 = 0x7FFF_0000_0000_0000;
    pub const FRAC_HIGH_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;
    pub const EXP_BITS: u32 = 15;
    pub const FRAC_BITS: u32 = 112;
    pub const EXP_BIAS: i32 = 16383;
    pub const MAX_EXP: i32 = 16383;
    pub const MIN_EXP: i32 = -16382;

    pub const ZERO: F128 = F128 { high: 0, low: 0 };
    pub const NEG_ZERO: F128 = F128 {
        high: Self::SIGN_MASK,
        low: 0,
    };
    pub const ONE: F128 = F128 {
        high: 0x3FFF_0000_0000_0000,
        low: 0,
    };
    pub const NEG_ONE: F128 = F128 {
        high: 0xBFFF_0000_0000_0000,
        low: 0,
    };

    pub const INFINITY: F128 = F128 {
        high: Self::EXP_MASK,
        low: 0,
    };
    pub const NEG_INFINITY: F128 = F128 {
        high: Self::SIGN_MASK | Self::EXP_MASK,
        low: 0,
    };

    /// Quiet NaN (canonical)
    pub const NAN: F128 = F128 {
        high: Self::EXP_MASK | 0x0000_8000_0000_0000,
        low: 0,
    };

    /// Smallest positive subnormal number
    pub const MIN_POSITIVE_SUBNORMAL: F128 = F128 { high: 0, low: 1 };
    /// Smallest positive normal number (2^-16382)
    pub const MIN_POSITIVE_NORMAL: F128 = F128 {
        high: 0x0001_0000_0000_0000,
        low: 0,
    };
    /// Largest finite number (2^16384 - 2^16372)
    pub const MAX: F128 = F128 {
        high: 0x7FFE_FFFF_FFFF_FFFF,
        low: 0xFFFF_FFFF_FFFF_FFFF,
    };
    /// Machine epsilon (2^-112)
    pub const EPSILON: F128 = F128 {
        high: 0x3F8F_0000_0000_0000,
        low: 0,
    };

    pub const PI: F128 = F128 {
        high: 0x4000_921F_B544_42D1,
        low: 0x8469_898C_C517_01B8,
    };
    pub const TWO_PI: F128 = F128 {
        high: 0x4001_921F_B544_42D1,
        low: 0x8469_898C_C517_01B8,
    };
    pub const FRAC_PI_2: F128 = F128 {
        high: 0x3FFF_921F_B544_42D1,
        low: 0x8469_898C_C517_01B8,
    };
    pub const FRAC_PI_4: F128 = F128 {
        high: 0x3FFE_921F_B544_42D1,
        low: 0x8469_898C_C517_01B8,
    };
    pub const E: F128 = F128 {
        high: 0x4000_5BF0_A8B1_4576,
        low: 0x9535_5FB8_AC40_4E7A,
    };
    pub const LN_2: F128 = F128 {
        high: 0x3FFE_62E4_2FEF_A39E,
        low: 0xF357_93C7_7FCE_2BBC,
    };
    pub const LOG2_E: F128 = F128 {
        high: 0x3FFF_B8AA_3B29_5C17,
        low: 0xF0AB_EA67_0764_8776,
    };

    const TWO_OVER_PI_BITS: [u64; 4] = [
        0xA2F9836E4E441529,
        0xFC2757D1F534DDC0,
        0xDB6295993C439041,
        0xFE5163ABDEBBC561,
    ];
}

#[cfg(test)]
mod tests;
