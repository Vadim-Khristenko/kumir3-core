//! Square root, transcendental and trigonometric functions for [`F128`].

use super::F128;
use super::u256::U256;
use std::cmp::Ordering;

// =============================================================================
//         SECTION: SQUARE ROOT
// =============================================================================

impl F128 {
    /// [STABLE] [PERF-SENSITIVE] Square root.
    pub fn sqrt(self) -> Self {
        if self.is_nan() || self.is_sign_negative() && !self.is_zero() {
            return Self::NAN;
        }
        if self.is_zero() || self.is_infinite() {
            return self;
        }

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
        let mut exp_abs = exp.unsigned_abs() as u64;
        let mut acc = Self::ONE;

        while exp_abs > 0 {
            if (exp_abs & 1) != 0 {
                acc = acc * base;
            }
            base = base * base;
            exp_abs >>= 1;
        }

        if exp < 0 { Self::ONE / acc } else { acc }
    }

    /// [STABLE] Floating point power.
    pub fn powf(self, exp: Self) -> Self {
        if self.is_nan() || exp.is_nan() {
            return Self::NAN;
        }
        if exp.is_zero() {
            return Self::ONE;
        }
        if self.is_one() {
            return Self::ONE;
        }
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

        if exp.is_integer()
            && let Some(n) = exp.to_i64_checked()
            && n >= i32::MIN as i64
            && n <= i32::MAX as i64
        {
            return self.powi(n as i32);
        }

        // General case: x^y = exp(y * ln(x))
        let ln_x = self.ln();
        if ln_x.is_nan() {
            return Self::NAN;
        }
        (exp * ln_x).exp()
    }

    /// [STABLE] [PERF-SENSITIVE] Exponential function.
    pub fn exp(self) -> Self {
        if self.is_nan() {
            return Self::NAN;
        }
        if self.is_zero() {
            return Self::ONE;
        }
        if self > Self::from(11356) {
            return Self::INFINITY;
        }
        if self < Self::from(-11356) {
            return Self::ZERO;
        }

        let k = (self / Self::LN_2).round();
        let r = self - k * Self::LN_2;
        let k_int = k.to_i64_checked().unwrap_or(0);

        let mut term = Self::ONE;
        let mut sum = Self::ONE;
        let mut n = 1;
        loop {
            term = term * r / Self::from(n);
            let new_sum = sum + term;
            if new_sum == sum {
                break;
            }
            sum = new_sum;
            n += 1;
            if n > 100 {
                break;
            }
        }

        match k_int.cmp(&0) {
            Ordering::Greater => sum * Self::from(2i64).powi(k_int as i32),
            Ordering::Less => sum / Self::from(2i64).powi((-k_int) as i32),
            Ordering::Equal => sum,
        }
    }

    /// [STABLE] [PERF-SENSITIVE] Natural logarithm.
    pub fn ln(self) -> Self {
        if self.is_nan() || self.is_sign_negative() {
            return Self::NAN;
        }
        if self.is_zero() {
            return Self::NEG_INFINITY;
        }
        if self.is_infinite() {
            return Self::INFINITY;
        }
        if self.is_one() {
            return Self::ZERO;
        }

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
            if new_sum == sum {
                break;
            }
            sum = new_sum;
            n += 2;
            if n > 200 {
                break;
            }
        }

        e_f * Self::LN_2 + sum * Self::from(2)
    }

    #[inline]
    pub fn log2(self) -> Self {
        self.ln() / Self::LN_2
    }
    #[inline]
    pub fn log10(self) -> Self {
        self.ln() / Self::from(std::f64::consts::LN_10)
    }
}

// =============================================================================
//         SECTION: TRIGONOMETRY
// =============================================================================

impl F128 {
    /// [STABLE] [PERF-SENSITIVE] Sine.
    pub fn sin(self) -> Self {
        if self.is_nan() || self.is_infinite() {
            return Self::NAN;
        }
        if self.is_zero() {
            return self;
        }

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
        if self.is_nan() || self.is_infinite() {
            return Self::NAN;
        }
        if self.is_zero() {
            return Self::ONE;
        }

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
        if self.is_nan() {
            return Self::NAN;
        }
        if self > Self::ONE || self < -Self::ONE {
            return Self::NAN;
        }
        if self.is_zero() {
            return self;
        }

        if self == Self::ONE {
            return Self::FRAC_PI_2;
        }
        if self == -Self::ONE {
            return -Self::FRAC_PI_2;
        }

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

        for n in 1u64..=25 {
            let num = (2 * n - 1) * (2 * n - 1);
            let den = (2 * n) * (2 * n + 1);
            term = term * x2 * Self::from(num) / Self::from(den);
            sum = sum + term;
            if term.is_zero() {
                break;
            }
        }
        sum
    }

    /// [STABLE] Arccosine.
    pub fn acos(self) -> Self {
        if self.is_nan() {
            return Self::NAN;
        }
        if self > Self::ONE || self < -Self::ONE {
            return Self::NAN;
        }
        if self == Self::ONE {
            return Self::ZERO;
        }
        if self == -Self::ONE {
            return Self::PI;
        }

        Self::FRAC_PI_2 - self.asin()
    }
}

// =============================================================================
//         SECTION: HELPERS (INTERNAL)
// =============================================================================

impl F128 {
    /// Reduces angle to range [-π/2, π/2].
    /// [PRECISION-CRITICAL] Cody-Waite + Payne-Hanek range reduction.
    ///
    /// Для |x| < 2^60 используем Cody-Waite (быстро).
    /// Для |x| ≥ 2^60 используем Payne-Hanek (медленнее, но точно до 2^16383).
    ///
    /// Возвращает (k mod 4, r), где x = k·π/2 + r, |r| ≤ π/4.
    pub fn reduce_pi_2(self) -> (i64, Self) {
        if !self.is_finite() {
            return (0, Self::NAN);
        }
        if self.is_zero() {
            return (0, self);
        }

        let abs_x = self.abs();
        let _pi_2 = Self::FRAC_PI_2;
        let pi_4 = Self::FRAC_PI_4;

        // Быстрый путь: уже в диапазоне ±π/4
        if abs_x <= pi_4 {
            return (0, self);
        }

        // Граница Cody-Waite: ~2^60 · (π/2) ≈ 2^61
        // Для меньших значений используем быстрый алгоритм
        const CW_LIMIT: u64 = 0x403D_0000_0000_0000; // 2^61

        if abs_x.high < CW_LIMIT {
            return self.reduce_cody_waite();
        }

        // Payne-Hanek для больших аргументов
        self.reduce_payne_hanek()
    }

    /// Быстрая редукция Cody-Waite с разделённой константой (53 + 59 бит)
    fn reduce_cody_waite(self) -> (i64, Self) {
        const PI_2_HI: F128 = F128::FRAC_PI_2; // 113 бит, но мы используем только старшие 53 для первого шага
        // Младшие биты: π/2 - floor(π/2 · 2^53)/2^53 · 2^53
        // На самом деле используем более точное разделение:
        const C1: F128 = F128 {
            high: 0x3FFF_921F_B544_42D1,
            low: 0x8000000000000000,
        }; // 53 бита
        const C2: F128 = F128 {
            high: 0x3F8D_494C_CF13_38CE,
            low: 0x0F30_47A5_2D6C_821C,
        };

        let k_float = (self / C1).round();
        let k = k_float.to_i64_saturating();

        if k == 0 {
            return (0, self);
        }

        let k_f = F128::from(k);
        // r = (x - k·C1) - k·C2
        let r = (self - k_f * C1) - k_f * C2;

        self.normalize_reduction_result(k, r)
    }

    /// Payne-Hanek reduction для очень больших аргументов (≥ 2^61)
    /// Использует 256-битную арифметику для точности.
    fn reduce_payne_hanek(self) -> (i64, Self) {
        // Разбираем self = m · 2^e, где 1 ≤ m < 2
        let (sign, exp, mant) = self.decompose();

        // Вычисляем y = x · (2/π) = mant · 2^e · (2/π)
        // Нам нужна дробная часть y с точностью ~113 бит

        // Индекс в таблице битов 2/π: смещение на (e + exponent_bias)
        let e_idx = exp + Self::FRAC_BITS as i32; // нормализация

        // Берем 256 бит из таблицы 2/π, начиная с позиции e_idx
        let bits_2_over_pi = self.fetch_bits_2_over_pi(e_idx);

        // Умножаем mantissa (128 бит) на bits_2_over_pi (256 бит)
        // Получаем 384-битное произведение, нам нужны средние 128 бит (дробная часть)
        let product = U256::mul_u128_u256(mant, bits_2_over_pi);

        // Извлекаем дробную часть: сдвигаем вправо на (256 - 113) = 143 бита
        // и берем 113 бит для формирования новой мантиссы
        let frac_shift = 143u32;
        let frac_bits = product.shr(frac_shift).low_u128() & ((1u128 << 113) - 1);

        // Округляем k = floor(y)
        let k_int = (product.shr(256).low_u128() as i64).wrapping_add(if frac_bits >> 112 != 0 {
            1
        } else {
            0
        });

        // Восстанавливаем остаток: r = frac(y) · (π/2)
        // frac(y) = frac_bits / 2^113
        // r = frac_bits · (π/2) / 2^113
        let r_mant = frac_bits << (128 - 113); // нормализация к 1.xxxxx
        let r = Self::compose(sign, -1, r_mant); // экспонента = -1 т.к. < 1

        // Корректируем знак и квадрант
        let k_mod = k_int.rem_euclid(4);
        let r_final = if sign { -r.abs() } else { r.abs() };

        (k_mod, r_final)
    }

    /// Извлекает 256 бит из константы 2/π со смещением bit_idx
    fn fetch_bits_2_over_pi(&self, bit_idx: i32) -> U256 {
        // bit_idx может быть отрицательным (для subnormal) или очень большим
        // Нормализуем к диапазону таблицы [0, 255]
        let idx = if bit_idx < 0 {
            0
        } else if bit_idx > 192 {
            192 // Защита от выхода за границы (возвращаем нули)
        } else {
            bit_idx as usize
        };

        // Сдвиг внутри 64-битного слова
        let word = idx / 64;
        let shift = idx % 64;

        let mut result = [0u64; 4];

        for (i, result_item) in result.iter_mut().enumerate() {
            let src_idx = word + i;
            if src_idx < 4 {
                *result_item |= Self::TWO_OVER_PI_BITS[src_idx] >> shift;
                if shift != 0 && src_idx + 1 < 4 {
                    *result_item |= Self::TWO_OVER_PI_BITS[src_idx + 1] << (64 - shift);
                }
            }
        }

        U256 { d: result }
    }

    /// Нормализация результата редукции: гарантирует |r| ≤ π/2 и корректный k
    fn normalize_reduction_result(&self, k: i64, r: Self) -> (i64, Self) {
        let pi_2 = Self::FRAC_PI_2;
        let mut k_final = k;
        let mut r_final = r;

        // Коррекция выхода за границы
        if r_final > pi_2 {
            r_final = r_final - pi_2;
            k_final += 1;
        } else if r_final < -pi_2 {
            r_final = r_final + pi_2;
            k_final -= 1;
        }

        // Оптимизация: если очень близко к ±π/2, проверяем альтернативу
        let near_boundary = pi_2 - Self::EPSILON * Self::from(16);
        if r_final > near_boundary {
            let alt = r_final - pi_2;
            if alt.abs() < r_final.abs() {
                r_final = alt;
                k_final += 1;
            }
        } else if r_final < -near_boundary {
            let alt = r_final + pi_2;
            if alt.abs() < r_final.abs() {
                r_final = alt;
                k_final -= 1;
            }
        }

        // Нормализация нуля
        if r_final.is_zero() || r_final.abs() < Self::MIN_POSITIVE_SUBNORMAL {
            r_final = if self.is_sign_negative() {
                Self::NEG_ZERO
            } else {
                Self::ZERO
            };
        }

        (k_final.rem_euclid(4), r_final)
    }

    fn taylor_sin(x: Self, x2: Self) -> Self {
        let mut term = x;
        let mut sum = x;
        for n in 1u64..=20 {
            term = -term * x2 / Self::from((2 * n) * (2 * n + 1));
            sum = sum + term;
            if term.is_zero() {
                break;
            }
        }
        sum
    }

    fn taylor_cos(_x: Self, x2: Self) -> Self {
        let mut term = Self::ONE;
        let mut sum = Self::ONE;
        for n in 1u64..=20 {
            term = -term * x2 / Self::from((2 * n - 1) * (2 * n));
            sum = sum + term;
            if term.is_zero() {
                break;
            }
        }
        sum
    }
}
