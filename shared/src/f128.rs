use std::cmp::Ordering;
use std::ops::{Add, Sub, Mul, Div, Rem, Neg};
use std::panic;
use std::fmt;

// ============================================================================
// Struct Definition & Constants
// ============================================================================

/// Представление 128-битного числа в формате IEEE-754 binary128.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct F128 {
    pub high: u64,
    pub low: u64,
}

impl F128 {
    // 1 бит знака, 15 бит порядка (bias = 16383), 112 бит мантиссы
    const FRAC_BITS: u32 = 112;
    const EXP_BIAS: i32 = 16383;

    const SIGN_MASK: u64 = 0x8000_0000_0000_0000;
    const EXP_MASK: u64 = 0x7FFF_0000_0000_0000;
    const FRAC_HIGH_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;

    pub const ZERO: F128 = F128 { high: 0, low: 0 };
    pub const NEG_ZERO: F128 = F128 { high: Self::SIGN_MASK, low: 0 };

    pub const INFINITY: F128 = F128 { high: Self::EXP_MASK, low: 0 };
    pub const NEG_INFINITY: F128 = F128 { high: Self::SIGN_MASK | Self::EXP_MASK, low: 0 };

    pub const NAN: F128 = F128 { high: Self::EXP_MASK | 0x0000_8000_0000_0000, low: 0 };

    // Mathematical Constants
    // PI = 3.14159265358979323846264338327950288...
    pub const PI: F128 = F128 { high: 0x4000_921F_B544_42D1, low: 0x8469_898C_C517_01B8 };
    pub const TWO_PI: F128 = F128 { high: 0x4001_921F_B544_42D1, low: 0x8469_898C_C517_01B8 };
    pub const PI_2: F128 = F128 { high: 0x3FFF_921F_B544_42D1, low: 0x8469_898C_C517_01B8 };
    pub const PI_4: F128 = F128 { high: 0x3FFE_921F_B544_42D1, low: 0x8469_898C_C517_01B8 };
    
    pub const E: F128 = F128 { high: 0x4000_5BF0_A8B1_4576, low: 0x9535_5FB8_AC40_4E7A };
}

// ============================================================================
// Constructors & Bitwise Operations
// ============================================================================

impl F128 {
    pub fn from_bits(high: u64, low: u64) -> Self { F128 { high, low } }
    pub fn to_bits(self) -> (u64, u64) { (self.high, self.low) }

    pub fn sign_bit(self) -> u8 { ((self.high & Self::SIGN_MASK) != 0) as u8 }
    pub fn raw_exponent(self) -> u16 { ((self.high & Self::EXP_MASK) >> 48) as u16 }
    pub fn raw_fraction(self) -> (u64, u64) { (self.high & Self::FRAC_HIGH_MASK, self.low) }
    
    fn is_one(self) -> bool { self.high == 0x3FFF_0000_0000_0000 && self.low == 0 }
    fn one() -> F128 { F128 { high: 0x3FFF_0000_0000_0000, low: 0 } }
}

// ============================================================================
// Classification
// ============================================================================

impl F128 {
    pub fn is_nan(self) -> bool {
        let e = self.raw_exponent();
        let (fh, fl) = self.raw_fraction();
        e == 0x7FFF && (fh != 0 || fl != 0)
    }

    pub fn is_infinite(self) -> bool {
        let e = self.raw_exponent();
        let (fh, fl) = self.raw_fraction();
        e == 0x7FFF && fh == 0 && fl == 0
    }

    pub fn is_finite(self) -> bool { !self.is_nan() && !self.is_infinite() }

    pub fn is_zero(self) -> bool {
        let e = self.raw_exponent();
        let (fh, fl) = self.raw_fraction();
        e == 0 && fh == 0 && fl == 0
    }

    pub fn is_sign_negative(self) -> bool { self.sign_bit() == 1 }

    pub fn is_subnormal(self) -> bool {
        let e = self.raw_exponent();
        let (fh, fl) = self.raw_fraction();
        e == 0 && (fh != 0 || fl != 0)
    }

    pub fn is_normal(self) -> bool {
        let e = self.raw_exponent();
        e != 0 && e != 0x7FFF
    }
    
    /// Преобразует F128 в f64 с возможной потерей точности.
    pub fn to_f64(self) -> f64 {
        if self.is_nan() { return f64::NAN; }
        if self.is_infinite() { return if self.is_sign_negative() { f64::NEG_INFINITY } else { f64::INFINITY }; }
        if self.is_zero() { return if self.is_sign_negative() { -0.0 } else { 0.0 }; }
        
        let (sign, exp, mant) = self.decompose();
        
        // f64: 11 бит порядка (bias = 1023), 52 бита мантиссы
        let f64_exp_bias: i32 = 1023;
        let f64_frac_bits: u32 = 52;
        
        // Приводим порядок к f64
        let biased_exp = exp + f64_exp_bias;
        
        // Проверка на переполнение/антипереполнение
        if biased_exp >= 2047 {
            // Переполнение -> бесконечность
            return if sign { f64::NEG_INFINITY } else { f64::INFINITY };
        }
        if biased_exp <= 0 {
            // Субнормальное или ноль
            // Упрощённо возвращаем ноль
            return if sign { -0.0 } else { 0.0 };
        }
        
        // Округляем мантиссу до 52 бит
        let shift = Self::FRAC_BITS - f64_frac_bits; // 112 - 52 = 60
        let f64_mant = (mant >> shift) as u64;
        // Убираем неявную единицу (она подразумевается в f64)
        let f64_mant_final = f64_mant & 0xF_FFFF_FFFF_FFFF;
        
        // Собираем биты f64
        let sign_bit = if sign { 1u64 << 63 } else { 0 };
        let exp_bits = ((biased_exp as u64) & 0x7FF) << 52;
        let bits = sign_bit | exp_bits | f64_mant_final;
        
        f64::from_bits(bits)
    }
    
    /// Преобразует F128 в f32 с возможной потерей точности.
    pub fn to_f32(self) -> f32 {
        self.to_f64() as f32
    }
}

// ============================================================================
// Mathematical Functions (Public API)
// ============================================================================

impl F128 {
    pub fn abs(self) -> Self {
        let (high, low) = self.to_bits();
        Self::from_bits(high & !Self::SIGN_MASK, low)
    }

    pub fn floor(self) -> F128 {
        if self.is_nan() || self.is_infinite() || self.is_zero() { return self; }
        let (sign, exp, mant) = self.decompose();
        let frac_bits = Self::FRAC_BITS as i32;
        let shift = exp - frac_bits;
        if shift >= 0 { return self; }
        let rshift = (-shift) as u32;
        if rshift == 0 { return self; }
        let int_part = mant >> rshift;
        let frac_mask = (1u128 << rshift) - 1;
        let frac = mant & frac_mask;
        if frac == 0 { return self; }
        if !sign {
            F128::compose(false, frac_bits + shift, int_part)
        } else {
            let adj = int_part + 1;
            F128::compose(true, frac_bits + shift, adj)
        }
    }
    
    pub fn trunc(self) -> Self {
        if self.is_nan() || self.is_infinite() || self.is_zero() { return self; }
        let (sign, exp, mant) = self.decompose();
        let frac_bits = Self::FRAC_BITS as i32;
        let shift = exp - frac_bits;
        if shift >= 0 { return self; }
        let rshift = (-shift) as u32;
        if rshift >= 128 { return if sign { Self::NEG_ZERO } else { Self::ZERO }; }
        let int_part = mant >> rshift;
        let new_mant = int_part << rshift;
        Self::compose(sign, exp, new_mant)
    }
    
    pub fn round(self) -> Self {
        let half = Self::from_bits(0x3FFE_0000_0000_0000, 0); // 0.5
        if self.is_sign_negative() {
            (self - half).trunc()
        } else {
            (self + half).trunc()
        }
    }

    pub fn sqrt(self) -> F128 {
        if self.is_nan() { return F128::NAN; }
        if self.is_zero() { return self; }
        if self.is_sign_negative() { return F128::NAN; }
        if self.is_infinite() { return F128::INFINITY; }

        let (sign, exp, mant) = self.decompose();
        debug_assert!(!sign);

        let mut e = exp;
        let mut m = mant;

        // Normalize exponent to be even
        if e & 1 != 0 {
            e -= 1;
            m <<= 1;
        }
        let half_exp = e / 2;

        // Construct normalized x in range [1, 4)
        let x_norm = F128::compose(false, 0, m);
        
        // Initial guess: y = 1.0
        let mut y = F128::one();
        let half = F128::from_bits(0x3FFE_0000_0000_0000, 0); // 0.5

        // Newton-Raphson: y = 0.5 * (y + x/y)
        for _ in 0..10 {
            let div = x_norm / y;
            let sum = y + div;
            y = sum * half;
        }

        // Re-apply exponent
        let (ys, ye, ym) = y.decompose();
        F128::compose(ys, ye + half_exp, ym)
    }

    pub fn powf(self, exp: F128) -> F128 {
        if self.is_nan() || exp.is_nan() { return F128::NAN; }
        if self.is_one() { return F128::one(); }
        if exp.is_zero() { return F128::one(); }
        if self.is_zero() {
            return if exp.is_sign_negative() { F128::INFINITY } else { F128::ZERO };
        }
        if self.is_infinite() {
            return if exp.is_sign_negative() { F128::ZERO } else { F128::INFINITY };
        }
        if self.is_sign_negative() {
            if !Self::is_integer(exp) { return F128::NAN; }
            let abs_base = -self;
            let y_int = Self::to_i64_saturating(exp);
            let res = Self::powi(abs_base, y_int.abs() as u64);
            if y_int < 0 { return F128::one() / res; }
            return if y_int % 2 != 0 { -res } else { res };
        }
        if Self::is_integer(exp) {
            let y_int = Self::to_i64_saturating(exp);
            if y_int < 0 {
                let res = Self::powi(self, (-y_int) as u64);
                return F128::one() / res;
            } else {
                return Self::powi(self, y_int as u64);
            }
        }
        let ln_x = Self::ln(self);
        if ln_x.is_nan() { return F128::NAN; }
        let y_ln_x = exp * ln_x;
        Self::exp(y_ln_x)
    }
    
    // --- Trigonometry ---
    
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

    pub fn cos(self) -> Self {
        if self.is_nan() || self.is_infinite() { return Self::NAN; }
        if self.is_zero() { return Self::one(); }
        
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

    pub fn tan(self) -> Self {
        let s = self.sin();
        let c = self.cos();
        s / c
    }

    pub fn ctg(self) -> Self {
        let s = self.sin();
        let c = self.cos();
        c / s
    }
    
    pub fn asin(self) -> Self {
        if self.is_nan() || self > Self::one() || self < -Self::one() { return Self::NAN; }
        if self.is_zero() { return self; }
        
        // For |x| > 0.5, use asin(x) = pi/2 - 2*asin(sqrt((1-x)/2))
        if self.abs() > Self::from_bits(0x3FFE_0000_0000_0000, 0) { // 0.5
             let x = self.abs();
             let one = Self::one();
             let two = Self::from_int(2);
             let sub = (one - x) / two;
             let inner = sub.sqrt().asin();
             let res = Self::PI_2 - two * inner;
             return if self.is_sign_negative() { -res } else { res };
        }
        
        // Taylor series: x + x^3/6 + 3x^5/40 + ...
        let x2 = self * self;
        let mut term = self;
        let mut sum = self;
        let mut n = 1;
        for _ in 0..20 {
            let num = Self::from_int((2*n - 1) * (2*n - 1));
            let den = Self::from_int((2*n) * (2*n + 1));
            term = term * x2 * num / den;
            sum = sum + term;
            n += 1;
            if term.is_zero() { break; }
        }
        sum
    }
    
    pub fn acos(self) -> Self {
        Self::PI_2 - self.asin()
    }

    // --- Helpers for Trig ---
    
    fn reduce_pi_2(self) -> (i64, Self) {
        // x = k * pi/2 + r
        let div = self / Self::PI_2;
        let k_float = div.round(); // round to nearest integer
        let k = Self::to_i64_saturating(k_float);
        let r = self - k_float * Self::PI_2;
        (k, r)
    }
    
    fn taylor_sin(x: Self, x2: Self) -> Self {
        let mut term = x;
        let mut sum = x;
        let mut n = 1;
        for _ in 0..15 {
            term = -term * x2 / Self::from_int((2*n) * (2*n+1));
            sum = sum + term;
            n += 1;
            if term.is_zero() { break; }
        }
        sum
    }
    
    fn taylor_cos(_x: Self, x2: Self) -> Self {
        let mut term = Self::one();
        let mut sum = Self::one();
        let mut n = 1;
        for _ in 0..15 {
            term = -term * x2 / Self::from_int((2*n-1) * (2*n));
            sum = sum + term;
            n += 1;
            if term.is_zero() { break; }
        }
        sum
    }
}

// ============================================================================
// Private Helpers & Internal Logic
// ============================================================================

impl F128 {
    fn decompose(self) -> (bool, i32, u128) {
        let sign = self.is_sign_negative();
        let exp = self.raw_exponent();
        let (fh, fl) = self.raw_fraction();
        let frac = ((fh as u128) << 64) | (fl as u128);

        if exp == 0 {
            if frac == 0 { return (sign, 0, 0); }
            let lz = frac.leading_zeros() as i32;
            let mant = (frac << lz) & ((1u128 << Self::FRAC_BITS) - 1);
            let e = 1 - Self::EXP_BIAS - lz;
            (sign, e, mant)
        } else if exp == 0x7FFF {
            (sign, i32::MAX, frac)
        } else {
            let mant = (1u128 << Self::FRAC_BITS) | frac;
            let e = (exp as i32) - Self::EXP_BIAS;
            (sign, e, mant)
        }
    }

    fn compose(sign: bool, exp: i32, mant: u128) -> F128 {
        if mant == 0 {
            return if sign { F128::NEG_ZERO } else { F128::ZERO };
        }

        let mut m = mant;
        let mut e = exp;

        let msb = 127 - m.leading_zeros() as i32;
        let target = Self::FRAC_BITS as i32;
        if msb > target {
            let shift = (msb - target) as u32;
            m >>= shift;
            e += shift as i32;
        } else if msb < target {
            let shift = (target - msb) as u32;
            m <<= shift;
            e -= shift as i32;
        }

        let biased = e + Self::EXP_BIAS;
        if biased >= 0x7FFF {
            return if sign { F128::NEG_INFINITY } else { F128::INFINITY };
        }
        if biased <= 0 {
            return if sign { F128::NEG_ZERO } else { F128::ZERO };
        }

        let frac_mask: u128 = (1u128 << Self::FRAC_BITS) - 1;
        let frac = m & frac_mask;

        let high = ((sign as u64) << 63)
            | (((biased as u64) & 0x7FFF) << 48)
            | ((frac >> 64) as u64 & Self::FRAC_HIGH_MASK);
        let low = frac as u64;

        F128 { high, low }
    }

    fn add_mant(sign_a: bool, e_a: i32, m_a: u128,
                sign_b: bool, e_b: i32, m_b: u128) -> (bool, i32, u128) {
        if m_a == 0 { return (sign_b, e_b, m_b); }
        if m_b == 0 { return (sign_a, e_a, m_a); }

        let (mut sign_l, mut e_l, mut m_l) = (sign_a, e_a, m_a);
        let (mut sign_s, mut e_s, mut m_s) = (sign_b, e_b, m_b);
        if e_l < e_s {
            core::mem::swap(&mut sign_l, &mut sign_s);
            core::mem::swap(&mut e_l, &mut e_s);
            core::mem::swap(&mut m_l, &mut m_s);
        }

        let de = (e_l - e_s) as u32;
        let m_s_shifted = if de >= 128 { 0 } else { m_s >> de };

        let (sign_res, mant_res) = if sign_l == sign_s {
            (sign_l, m_l.wrapping_add(m_s_shifted))
        } else if m_l >= m_s_shifted {
            (sign_l, m_l - m_s_shifted)
        } else {
            (!sign_l, m_s_shifted - m_l)
        };

        (sign_res, e_l, mant_res)
    }
    
    fn mul_core(self, rhs: F128) -> F128 {
        let (sa, ea, ma) = self.decompose();
        let (sb, eb, mb) = rhs.decompose();
        if ma == 0 || mb == 0 {
            return if sa ^ sb { F128::NEG_ZERO } else { F128::ZERO };
        }

        let prod = U256::mul_u128(ma, mb);
        let lz = prod.leading_zeros();
        if lz == 256 { return if sa ^ sb { F128::NEG_ZERO } else { F128::ZERO }; }
        let msb = 255i32 - lz as i32;

        let target = F128::FRAC_BITS as i32;
        let shift_right = msb - target;
        let norm = if shift_right > 0 {
            prod.shr(shift_right as u32)
        } else if shift_right < 0 {
            prod.shl((-shift_right) as u32)
        } else {
            prod
        };

        let mant_mask: u128 = (1u128 << (F128::FRAC_BITS + 1)) - 1;
        let mant = norm.low_u128() & mant_mask;
        let exp = ea + eb - F128::FRAC_BITS as i32 + shift_right;
        F128::compose(sa ^ sb, exp, mant)
    }

    fn div_core(self, rhs: F128) -> F128 {
        let (sa, ea, ma) = self.decompose();
        let (sb, eb, mb) = rhs.decompose();
        if mb == 0 {
            if ma == 0 { return F128::NAN; }
            return if sa ^ sb { F128::NEG_INFINITY } else { F128::INFINITY };
        }
        if ma == 0 {
            return if sa ^ sb { F128::NEG_ZERO } else { F128::ZERO };
        }

        let mut n = U256::from_u128(ma).shl(F128::FRAC_BITS);
        let d = U256::from_u128(mb);
        let mut q: u128 = 0;
        let mut i = F128::FRAC_BITS as i32;
        while i >= 0 {
            let ds = d.shl(i as u32);
            if n.cmp(&ds) != Ordering::Less {
                n = n.sub(ds);
                q |= 1u128 << (i as u32);
            }
            i -= 1;
        }

        let exp = ea - eb;
        F128::compose(sa ^ sb, exp, q)
    }
    
    fn is_integer(x: F128) -> bool {
        if x.is_nan() || x.is_infinite() { return false; }
        if x.is_zero() { return true; }

        let (_sign, exp, mant) = x.decompose();
        if exp < 0 { return false; }
        let frac_bits = Self::FRAC_BITS as i32;
        if exp >= frac_bits { return true; }
        let frac_mask: u128 = (1u128 << (frac_bits - exp)) - 1;
        (mant & frac_mask) == 0
    }

    pub fn to_i64_saturating(x: F128) -> i64 {
        if x.is_nan() { return 0; }
        if x.is_zero() { return 0; }
        if x.is_infinite() { return if x.is_sign_negative() { i64::MIN } else { i64::MAX }; }

        let (sign, exp, mant) = x.decompose();
        if mant == 0 { return 0; }

        let frac_bits = Self::FRAC_BITS as i32;
        let shift = exp - frac_bits;

        let int_val: i128 = if shift >= 0 {
            if shift as u32 >= 127 { return if sign { i64::MIN } else { i64::MAX }; }
            let val = mant << (shift as u32);
            val as i128
        } else {
            let r = -shift;
            if r >= 127 { 0 } else { (mant >> (r as u32)) as i128 }
        };

        let signed = if sign { -int_val } else { int_val };
        if signed > i64::MAX as i128 { i64::MAX }
        else if signed < i64::MIN as i128 { i64::MIN }
        else { signed as i64 }
    }

    fn powi(mut base: F128, mut exp: u64) -> F128 {
        let mut acc = F128::from_int(1);
        while exp > 0 {
            if (exp & 1) != 0 { acc = acc * base; }
            base = base * base;
            exp >>= 1;
        }
        acc
    }

    fn ln(x: F128) -> F128 {
        if x.is_nan() || x.is_sign_negative() { return F128::NAN; }
        if x.is_zero() { return F128::NEG_INFINITY; }
        if x.is_infinite() { return F128::INFINITY; }

        let (sign, exp, mant) = x.decompose();
        debug_assert!(!sign);
        if mant == 0 { return F128::NEG_INFINITY; }

        let one = F128::from_int(1);
        let m_f = F128::compose(false, 0, mant);
        let t = (m_f - one) / (m_f + one);

        let mut term = t;
        let mut sum = t;
        let max_iter = 20u32;
        let mut n = 3u32;
        while n <= max_iter {
            term = term * t * t;
            let inv_n = F128::from_int_recip(n as u64);
            sum = sum + term * inv_n;
            n += 2;
        }
        let two = F128::from_int(2);
        let ln_m = sum * two;

        let ln2 = F128::ln2_const();
        let e_f = F128::from_exp(exp);
        ln_m + e_f * ln2
    }

    fn exp(x: F128) -> F128 {
        if x.is_nan() { return F128::NAN; }
        if x.is_zero() { return F128::one(); }

        let max_e = F128::from_int(11356);
        let min_e = F128::from_int(-11356);
        if x > max_e { return F128::INFINITY; }
        if x < min_e { return F128::ZERO; }

        let ln2 = F128::ln2_const();
        let k = F128::round_to_int(x / ln2);
        let r = x - k * ln2;

        let one = F128::from_int(1);
        let mut term = one;
        let mut sum = one;
        let max_n = 40u32;
        let mut n = 1u32;
        while n <= max_n {
            term = term * r;
            let inv = F128::from_int_recip(n as u64);
            sum = sum + term * inv;
            n += 1;
        }

        let two = F128::from_int(2);
        let k_i = F128::to_i64_saturating(k);
        if k_i > 0 {
            let pow2 = Self::powi(two, k_i as u64);
            sum * pow2
        } else if k_i < 0 {
            let pow2 = Self::powi(two, (-k_i) as u64);
            sum / pow2
        } else {
            sum
        }
    }
    
    pub fn from_int(value: i64) -> Self {
        if value == 0 { return F128::ZERO; }
        let sign = value < 0;
        let abs = if sign { (value as i128).wrapping_neg() as u128 } else { value as u128 };
        Self::from_uint_inner(sign, abs)
    }

    pub fn from_uint(value: u64) -> Self {
        if value == 0 { return F128::ZERO; }
        Self::from_uint_inner(false, value as u128)
    }

    fn from_uint_inner(sign: bool, v: u128) -> Self {
        let msb = 127 - v.leading_zeros() as i32;
        let shift = Self::FRAC_BITS as i32 - msb;
        let mant = if shift >= 0 { v << shift } else { v >> (-shift) };
        let exp = msb;
        F128::compose(sign, exp, mant)
    }

    fn ln2_const() -> F128 {
        F128::from_bits(0x3FFE_62E4_2FEF_A39E, 0xF357_93C7_7FCE_2BBC)
    }

    fn from_int_recip(n: u64) -> F128 {
        let one = F128::from_int(1);
        one / F128::from_uint(n)
    }

    fn from_exp(e: i32) -> F128 {
        F128::from_int(e as i64)
    }

    fn round_to_int(x: F128) -> F128 {
        if x.is_nan() || x.is_infinite() || x.is_zero() { return x; }
        let (sign, exp, mant) = x.decompose();
        let frac_bits = Self::FRAC_BITS as i32;
        let shift = exp - frac_bits;
        if shift >= 0 { return x; }
        let rshift = (-shift) as u32;
        if rshift == 0 { return x; }
        let int_part = mant >> rshift;
        let frac_mask = (1u128 << rshift) - 1;
        let frac = mant & frac_mask;
        let half = 1u128 << (rshift - 1);
        let rounded = if frac > half || (frac == half && (int_part & 1) == 1) {
            int_part + 1
        } else {
            int_part
        };
        F128::compose(sign, frac_bits + shift, rounded)
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl Add for F128 {
    type Output = F128;
    fn add(self, rhs: F128) -> F128 {
        if self.is_nan() || rhs.is_nan() { return F128::NAN; }
        if self.is_infinite() || rhs.is_infinite() {
            match (self, rhs) {
                (a, b) if a.is_infinite() && b.is_infinite() => {
                    if a.is_sign_negative() != b.is_sign_negative() { return F128::NAN; }
                    return a;
                }
                (a, _) if a.is_infinite() => return a,
                (_, b) if b.is_infinite() => return b,
                _ => {}
            }
        }
        let (sa, ea, ma) = self.decompose();
        let (sb, eb, mb) = rhs.decompose();
        let (sr, er, mr) = F128::add_mant(sa, ea, ma, sb, eb, mb);
        F128::compose(sr, er, mr)
    }
}

impl Sub for F128 {
    type Output = F128;
    fn sub(self, rhs: F128) -> F128 {
        let (high, low) = rhs.to_bits();
        let neg_rhs = F128 { high: high ^ F128::SIGN_MASK, low };
        self + neg_rhs
    }
}

impl Mul for F128 {
    type Output = F128;
    fn mul(self, rhs: F128) -> F128 {
        if self.is_nan() || rhs.is_nan() { return F128::NAN; }
        if self.is_infinite() || rhs.is_infinite() {
            let zero_left = self.is_zero();
            let zero_right = rhs.is_zero();
            if (zero_left && rhs.is_infinite()) || (zero_right && self.is_infinite()) {
                return F128::NAN;
            }
            let sign = self.is_sign_negative() ^ rhs.is_sign_negative();
            return if sign { F128::NEG_INFINITY } else { F128::INFINITY };
        }
        self.mul_core(rhs)
    }
}

impl Div for F128 {
    type Output = F128;
    fn div(self, rhs: F128) -> F128 {
        if self.is_nan() || rhs.is_nan() { return F128::NAN; }
        if (self.is_zero() && rhs.is_zero()) || (self.is_infinite() && rhs.is_infinite()) {
            return F128::NAN;
        }
        if self.is_infinite() && !rhs.is_zero() && !rhs.is_nan() {
            let sign = self.is_sign_negative() ^ rhs.is_sign_negative();
            return if sign { F128::NEG_INFINITY } else { F128::INFINITY };
        }
        if rhs.is_zero() {
            let sign = self.is_sign_negative() ^ rhs.is_sign_negative();
            return if sign { F128::NEG_INFINITY } else { F128::INFINITY };
        }
        if rhs.is_infinite() && !self.is_infinite() {
            let sign = self.is_sign_negative() ^ rhs.is_sign_negative();
            return if sign { F128::NEG_ZERO } else { F128::ZERO };
        }
        self.div_core(rhs)
    }
}

impl Rem for F128 {
    type Output = F128;
    fn rem(self, rhs: F128) -> F128 {
        if self.is_nan() || rhs.is_nan() { return F128::NAN; }
        if self.is_infinite() { return F128::NAN; }
        if rhs.is_zero() { return F128::NAN; }
        if rhs.is_infinite() { return self; }
        
        let div = self / rhs;
        let trunc = div.trunc();
        self - rhs * trunc
    }
}

impl Neg for F128 {
    type Output = Self;
    fn neg(self) -> Self::Output {
        if self.is_nan() { return F128::NAN; }
        let (high, low) = self.to_bits();
        F128 { high: high ^ Self::SIGN_MASK, low }
    }
}

impl PartialOrd for F128 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.is_nan() || other.is_nan() { return None; }
        Some(self.total_cmp(other))
    }
}

impl Ord for F128 {
    fn cmp(&self, other: &Self) -> Ordering { self.total_cmp(other) }
}

impl F128 {
    pub fn total_cmp(&self, other: &Self) -> Ordering {
        if self.high == other.high && self.low == other.low { return Ordering::Equal; }
        if self.is_zero() && other.is_zero() { return Ordering::Equal; }

        let a_nan = self.is_nan();
        let b_nan = other.is_nan();
        if a_nan || b_nan {
            return match (a_nan, b_nan) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => unreachable!(),
            };
        }

        let a_sign = self.is_sign_negative();
        let b_sign = other.is_sign_negative();
        if a_sign != b_sign {
            return if a_sign { Ordering::Less } else { Ordering::Greater };
        }

        let a = (self.high, self.low);
        let b = (other.high, other.low);
        let ord = if a < b { Ordering::Less } else { Ordering::Greater };
        if a_sign { ord.reverse() } else { ord }
    }
}

impl fmt::Display for F128 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_decimal_string())
    }
}

// ============================================================================
// Conversions (From)
// ============================================================================

impl From<f32> for F128 {
    fn from(value: f32) -> Self {
        if value.is_nan() { return F128::NAN; }
        if value.is_infinite() { return if value.is_sign_negative() { F128::NEG_INFINITY } else { F128::INFINITY }; }
        if value == 0.0 { return if value.is_sign_negative() { F128::NEG_ZERO } else { F128::ZERO }; }

        let bits = value.to_bits();
        let sign = (bits >> 31) != 0;
        let exp = ((bits >> 23) & 0xFF) as i32;
        let mant = bits & 0x7FFFFF;

        let unbiased_exp = if exp == 0 { if mant == 0 { 0 } else { 1 - 127 } } else { exp as i32 - 127 };
        let mantissa = if exp == 0 { mant } else { mant | 0x800000 };
        let extended_mant = (mantissa as u128) << (F128::FRAC_BITS - 23);
        F128::compose(sign, unbiased_exp, extended_mant)
    }
}

impl From<f64> for F128 {
    fn from(value: f64) -> Self {
        if value.is_nan() { return F128::NAN; }
        if value.is_infinite() { return if value.is_sign_negative() { F128::NEG_INFINITY } else { F128::INFINITY }; }
        if value == 0.0 { return if value.is_sign_negative() { F128::NEG_ZERO } else { F128::ZERO }; }

        let bits = value.to_bits();
        let sign = (bits >> 63) != 0;
        let exp = ((bits >> 52) & 0x7FF) as i32;
        let mant = bits & 0xF_FFFF_FFFF_FFFF;

        let unbiased_exp = if exp == 0 { if mant == 0 { 0 } else { 1 - 1023 } } else { exp as i32 - 1023 };
        let mantissa = if exp == 0 { mant } else { mant | 0x10_0000_0000_0000 };
        let extended_mant = (mantissa as u128) << (F128::FRAC_BITS - 52);
        F128::compose(sign, unbiased_exp, extended_mant)
    }
}

impl From<i32> for F128 {
    fn from(value: i32) -> Self { F128::from_int(value as i64) }
}

impl From<u32> for F128 {
    fn from(value: u32) -> Self { F128::from_uint(value as u64) }
}

impl From<i64> for F128 {
    fn from(value: i64) -> Self { F128::from_int(value) }
}

impl From<u64> for F128 {
    fn from(value: u64) -> Self { F128::from_uint(value) }
}

// ============================================================================
// String Conversion (BigUint10)
// ============================================================================

impl F128 {
    fn to_decimal_string(self) -> String {
        if self.is_nan() { return "nan128".to_string(); }
        if self.is_infinite() { return if self.is_sign_negative() { "-inf128" } else { "+inf128" }.to_string(); }
        if self.is_zero() { return if self.is_sign_negative() { "-0.0" } else { "0.0" }.to_string(); }

        let (sign, exp, mant) = self.decompose();
        if mant == 0 { return if sign { "-0.0" } else { "0.0" }.to_string(); }

        let scale = exp - Self::FRAC_BITS as i32;
        let mut big = BigUint10::from_u128(mant);
        let mut dec_exp: i32 = 0;

        if scale > 0 {
            big.shl_mul2(scale as u32);
        } else if scale < 0 {
            let k = (-scale) as u32;
            big.mul_pow5(k);
            dec_exp -= k as i32;
        }

        let s = big.to_string();
        let mut out = String::new();
        if sign { out.push('-'); }

        if dec_exp == 0 {
            out.push_str(&s);
            out.push_str(".0");
            return out;
        }

        if dec_exp < 0 {
            let shift = (-dec_exp) as usize;
            if shift >= s.len() {
                out.push('0');
                out.push('.');
                for _ in 0..(shift - s.len()) { out.push('0'); }
                out.push_str(&s);
            } else {
                let split = s.len() - shift;
                out.push_str(&s[..split]);
                out.push('.');
                let frac = &s[split..];
                let frac_trimmed = frac.trim_end_matches('0');
                if frac_trimmed.is_empty() { out.push('0'); } else { out.push_str(frac_trimmed); }
            }
            return out;
        }

        out.push_str(&s);
        for _ in 0..dec_exp { out.push('0'); }
        out.push_str(".0");
        out
    }
}

#[derive(Clone, Debug)]
struct BigUint10 {
    data: Vec<u32>,
}

impl BigUint10 {
    const BASE: u64 = 1_000_000_000;

    fn is_zero(&self) -> bool { self.data.is_empty() }

    fn from_u128(mut x: u128) -> Self {
        let mut v = Vec::new();
        while x > 0 {
            let rem = (x % Self::BASE as u128) as u32;
            v.push(rem);
            x /= Self::BASE as u128;
        }
        Self { data: v }
    }

    fn mul_u32(&mut self, m: u32) {
        if m == 0 || self.is_zero() {
            self.data.clear();
            return;
        }
        let mut carry: u64 = 0;
        for d in &mut self.data {
            let cur = (*d as u64) * (m as u64) + carry;
            *d = (cur % Self::BASE) as u32;
            carry = cur / Self::BASE;
        }
        if carry != 0 { self.data.push(carry as u32); }
    }

    fn shl_mul2(&mut self, k: u32) {
        for _ in 0..k { self.mul_u32(2); }
    }

    fn mul_pow5(&mut self, mut k: u32) {
        while k >= 9 {
            self.mul_u32(1_953_125); // 5^9
            k -= 9;
        }
        let m = match k {
            0 => 1, 1 => 5, 2 => 25, 3 => 125, 4 => 625,
            5 => 3_125, 6 => 15_625, 7 => 78_125, 8 => 390_625,
            _ => unreachable!(),
        };
        self.mul_u32(m);
    }

    fn div_mod_10(&mut self) -> u32 {
        if self.is_zero() { return 0; }
        let mut carry: u64 = 0;
        for d in self.data.iter_mut().rev() {
            let cur = carry * Self::BASE + (*d as u64);
            *d = (cur / 10) as u32;
            carry = cur % 10;
        }
        while self.data.last().map_or(false, |&x| x == 0) { self.data.pop(); }
        carry as u32
    }

    fn to_string(&self) -> String {
        if self.is_zero() { return "0".to_string(); }
        let mut tmp = self.clone();
        let mut digits = Vec::new();
        while !tmp.is_zero() {
            digits.push((tmp.div_mod_10() as u8) + b'0');
        }
        digits.reverse();
        String::from_utf8(digits).unwrap_or_else(|_| "0".to_string())
    }
}

// ============================================================================
// Helper Struct U256
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct U256 {
    hi_hi: u64,
    hi_lo: u64,
    lo_hi: u64,
    lo_lo: u64,
}

impl U256 {
    const ZERO: U256 = U256 { hi_hi: 0, hi_lo: 0, lo_hi: 0, lo_lo: 0 };

    fn from_u128(x: u128) -> Self {
        U256 { hi_hi: 0, hi_lo: 0, lo_hi: (x >> 64) as u64, lo_lo: x as u64 }
    }

    fn shl(self, shift: u32) -> Self {
        if shift == 0 { return self; }
        if shift >= 256 { return U256::ZERO; }
        let limb_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let limbs = [self.lo_lo, self.lo_hi, self.hi_lo, self.hi_hi];
        let mut out = [0u64; 4];
        for i in 0..4 {
            if i + limb_shift < 4 {
                out[i + limb_shift] |= limbs[i] << bit_shift;
                if bit_shift > 0 && i + limb_shift + 1 < 4 {
                    out[i + limb_shift + 1] |= limbs[i] >> (64 - bit_shift);
                }
            }
        }
        U256 { lo_lo: out[0], lo_hi: out[1], hi_lo: out[2], hi_hi: out[3] }
    }

    fn mul_u128(a: u128, b: u128) -> Self {
        let a_lo = (a & 0xFFFF_FFFF_FFFF_FFFFu128) as u128;
        let a_hi = (a >> 64) as u128;
        let b_lo = (b & 0xFFFF_FFFF_FFFF_FFFFu128) as u128;
        let b_hi = (b >> 64) as u128;

        let p0 = a_lo * b_lo;
        let p1 = a_lo * b_hi;
        let p2 = a_hi * b_lo;
        let p3 = a_hi * b_hi;

        let mask = 0xFFFF_FFFF_FFFF_FFFFu128;

        let r0 = p0 & mask;
        let t1 = (p0 >> 64) + (p1 & mask) + (p2 & mask);
        let r1 = t1 & mask;
        let t2 = (p1 >> 64) + (p2 >> 64) + (p3 & mask) + (t1 >> 64);
        let r2 = t2 & mask;
        let t3 = (p3 >> 64) + (t2 >> 64);
        let r3 = t3 & mask;

        U256 { lo_lo: r0 as u64, lo_hi: r1 as u64, hi_lo: r2 as u64, hi_hi: r3 as u64 }
    }

    fn leading_zeros(self) -> u32 {
        if self.hi_hi != 0 { return self.hi_hi.leading_zeros(); }
        if self.hi_lo != 0 { return 64 + self.hi_lo.leading_zeros(); }
        if self.lo_hi != 0 { return 128 + self.lo_hi.leading_zeros(); }
        if self.lo_lo != 0 { return 192 + self.lo_lo.leading_zeros(); }
        256
    }

    fn shr(self, shift: u32) -> Self {
        if shift == 0 { return self; }
        if shift >= 256 { return U256::ZERO; }
        let limb_shift = (shift / 64) as usize;
        let bit_shift = shift % 64;
        let limbs = [self.lo_lo, self.lo_hi, self.hi_lo, self.hi_hi];
        let mut out = [0u64; 4];
        for i in limb_shift..4 {
            let j = i - limb_shift;
            let upper = limbs[i];
            out[j] = if bit_shift == 0 { upper } else { upper >> bit_shift };
            if bit_shift > 0 && i + 1 < 4 {
                out[j] |= limbs[i + 1] << (64 - bit_shift);
            }
        }
        U256 { lo_lo: out[0], lo_hi: out[1], hi_lo: out[2], hi_hi: out[3] }
    }

    fn low_u128(self) -> u128 {
        ((self.lo_hi as u128) << 64) | (self.lo_lo as u128)
    }

    fn cmp(&self, other: &Self) -> Ordering {
        if self.hi_hi != other.hi_hi { return self.hi_hi.cmp(&other.hi_hi); }
        if self.hi_lo != other.hi_lo { return self.hi_lo.cmp(&other.hi_lo); }
        if self.lo_hi != other.lo_hi { return self.lo_hi.cmp(&other.lo_hi); }
        self.lo_lo.cmp(&other.lo_lo)
    }

    fn sub(self, other: U256) -> U256 {
        let (lo_lo, b0) = self.lo_lo.overflowing_sub(other.lo_lo);
        let (mut lo_hi, b1) = self.lo_hi.overflowing_sub(other.lo_hi);
        let (mut hi_lo, b2) = self.hi_lo.overflowing_sub(other.hi_lo);
        let (mut hi_hi, b3) = self.hi_hi.overflowing_sub(other.hi_hi);

        if b0 {
            let (v, c) = lo_hi.overflowing_sub(1);
            lo_hi = v;
            if c {
                let (v2, c2) = hi_lo.overflowing_sub(1);
                hi_lo = v2;
                if c2 { hi_hi = hi_hi.wrapping_sub(1); }
            }
        }
        if b1 {
            let (v, c) = hi_lo.overflowing_sub(1);
            hi_lo = v;
            if c { hi_hi = hi_hi.wrapping_sub(1); }
        }
        if b2 { hi_hi = hi_hi.wrapping_sub(1); }
        if b3 { panic!("U256 underflow. Actually you did something wrong please review your code."); }

        U256 { lo_lo, lo_hi, hi_lo, hi_hi }
    }
}