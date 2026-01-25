//! Математические константы Кумир
//!
//! Содержит все математические константы, доступные в языке.

use std::collections::HashMap;
use once_cell::sync::Lazy;

// ============================================================================
//                    МАТЕМАТИЧЕСКИЕ КОНСТАНТЫ
// ============================================================================

/// Математические константы, доступные в Кумире.
pub mod math_constants {
    // ========================================================================
    //                    ФУНДАМЕНТАЛЬНЫЕ КОНСТАНТЫ
    // ========================================================================
    
    /// Число Пи (π) - отношение длины окружности к диаметру
    pub const PI: f64 = std::f64::consts::PI;
    
    /// Тау (τ = 2π) - полный оборот в радианах
    pub const TAU: f64 = std::f64::consts::TAU;
    
    /// Число Эйлера (e) - основание натурального логарифма
    pub const E: f64 = std::f64::consts::E;
    
    /// Золотое сечение (φ = (1 + √5) / 2)
    pub const PHI: f64 = 1.618033988749895;
    
    /// Серебряное сечение (δₛ = 1 + √2)
    pub const SILVER_RATIO: f64 = 2.414213562373095;
    
    /// Пластическое число (ρ ≈ 1.3247)
    pub const PLASTIC: f64 = 1.324717957244746;
    
    // ========================================================================
    //                    КОРНИ И ЛОГАРИФМЫ
    // ========================================================================
    
    /// Квадратный корень из 2 (√2) - диагональ единичного квадрата
    pub const SQRT2: f64 = std::f64::consts::SQRT_2;
    
    /// Квадратный корень из 3 (√3)
    pub const SQRT3: f64 = 1.7320508075688772;
    
    /// Квадратный корень из 5 (√5)
    pub const SQRT5: f64 = 2.23606797749979;
    
    /// 1 / √2 = √2 / 2
    pub const FRAC_1_SQRT_2: f64 = std::f64::consts::FRAC_1_SQRT_2;
    
    /// Кубический корень из 2 (∛2)
    pub const CBRT2: f64 = 1.2599210498948732;
    
    /// Кубический корень из 3 (∛3)
    pub const CBRT3: f64 = 1.4422495703074083;
    
    /// Натуральный логарифм 2
    pub const LN2: f64 = std::f64::consts::LN_2;
    
    /// Натуральный логарифм 10
    pub const LN10: f64 = std::f64::consts::LN_10;
    
    /// Десятичный логарифм e (log₁₀(e))
    pub const LOG10_E: f64 = std::f64::consts::LOG10_E;
    
    /// Двоичный логарифм e (log₂(e))
    pub const LOG2_E: f64 = std::f64::consts::LOG2_E;
    
    /// Десятичный логарифм 2 (log₁₀(2))
    pub const LOG10_2: f64 = 0.3010299956639812;
    
    // ========================================================================
    //                    ДРОБИ ПИ
    // ========================================================================
    
    /// π / 2 (90°)
    pub const FRAC_PI_2: f64 = std::f64::consts::FRAC_PI_2;
    
    /// π / 3 (60°)
    pub const FRAC_PI_3: f64 = std::f64::consts::FRAC_PI_3;
    
    /// π / 4 (45°)
    pub const FRAC_PI_4: f64 = std::f64::consts::FRAC_PI_4;
    
    /// π / 6 (30°)
    pub const FRAC_PI_6: f64 = std::f64::consts::FRAC_PI_6;
    
    /// π / 8 (22.5°)
    pub const FRAC_PI_8: f64 = std::f64::consts::FRAC_PI_8;
    
    /// 1 / π
    pub const FRAC_1_PI: f64 = std::f64::consts::FRAC_1_PI;
    
    /// 2 / π
    pub const FRAC_2_PI: f64 = std::f64::consts::FRAC_2_PI;
    
    /// 2 / √π
    pub const FRAC_2_SQRT_PI: f64 = std::f64::consts::FRAC_2_SQRT_PI;
    
    // ========================================================================
    //                    КОНВЕРТАЦИЯ УГЛОВ
    // ========================================================================
    
    /// Градусы в радианы (π / 180)
    pub const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;
    
    /// Радианы в градусы (180 / π)
    pub const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;
    
    // ========================================================================
    //                    МАТЕМАТИЧЕСКИЕ КОНСТАНТЫ
    // ========================================================================
    
    /// Постоянная Эйлера-Маскерони (γ ≈ 0.5772)
    pub const EULER_MASCHERONI: f64 = 0.5772156649015329;
    
    /// Постоянная Апери (ζ(3) ≈ 1.202) - дзета-функция Римана от 3
    pub const APERY: f64 = 1.2020569031595943;
    
    /// Постоянная Каталана (G ≈ 0.9159)
    pub const CATALAN: f64 = 0.9159655941772190;
    
    /// Постоянная Хинчина (K ≈ 2.6854)
    pub const KHINCHIN: f64 = 2.6854520010653064;
    
    /// Постоянная Глейшера-Кинкелина (A ≈ 1.2824)
    pub const GLAISHER: f64 = 1.2824271291006226;
    
    /// Омега-константа (Ω) - решение x·eˣ = 1
    pub const OMEGA: f64 = 0.5671432904097838;
    
    /// Постоянная Конвея (λ ≈ 1.3035)
    pub const CONWAY: f64 = 1.3035772690342963;
    
    /// Постоянная Фейгенбаума (δ ≈ 4.6692) - универсальность хаоса
    pub const FEIGENBAUM_DELTA: f64 = 4.669201609102990;
    
    /// Вторая постоянная Фейгенбаума (α ≈ 2.5029)
    pub const FEIGENBAUM_ALPHA: f64 = 2.502907875095892;
    
    /// Постоянная Мейсселя-Мертенса (M ≈ 0.2615)
    pub const MEISSEL_MERTENS: f64 = 0.2614972128476428;
    
    /// Постоянная близнецов простых чисел (C₂ ≈ 0.6601)
    pub const TWIN_PRIME: f64 = 0.6601618158468696;
    
    // ========================================================================
    //                    ФИЗИЧЕСКИЕ КОНСТАНТЫ (БЕЗРАЗМЕРНЫЕ)
    // ========================================================================
    
    /// Постоянная тонкой структуры (α ≈ 1/137)
    pub const FINE_STRUCTURE: f64 = 0.0072973525693;
    
    // ========================================================================
    //                    ПРЕДЕЛЫ ТИПОВ
    // ========================================================================
    
    /// Максимальное целое (i64)
    pub const MAX_INT: i64 = i64::MAX;
    
    /// Минимальное целое (i64)
    pub const MIN_INT: i64 = i64::MIN;
    
    /// Максимальное вещественное (f64)
    pub const MAX_REAL: f64 = f64::MAX;
    
    /// Минимальное положительное вещественное (f64)
    pub const MIN_REAL: f64 = f64::MIN_POSITIVE;
    
    /// Машинный эпсилон (f64) - минимальная разница между 1.0 и следующим числом
    pub const EPSILON: f64 = f64::EPSILON;
    
    /// Бесконечность
    pub const INFINITY: f64 = f64::INFINITY;
    
    /// Минус бесконечность
    pub const NEG_INFINITY: f64 = f64::NEG_INFINITY;
    
    /// Не число (NaN)
    pub const NAN: f64 = f64::NAN;
}

/// Встроенные константы Кумира (доступные как переменные).
pub static BUILTIN_CONSTANTS: Lazy<HashMap<&'static str, f64>> = Lazy::new(|| {
    let mut m = HashMap::new();
    
    // Фундаментальные
    m.insert("ПИ", math_constants::PI);
    m.insert("пи", math_constants::PI);
    m.insert("pi", math_constants::PI);
    m.insert("PI", math_constants::PI);
    m.insert("π", math_constants::PI);
    
    m.insert("ТАУ", math_constants::TAU);
    m.insert("тау", math_constants::TAU);
    m.insert("tau", math_constants::TAU);
    m.insert("TAU", math_constants::TAU);
    m.insert("τ", math_constants::TAU);
    
    m.insert("Е", math_constants::E);
    m.insert("е", math_constants::E);
    m.insert("e", math_constants::E);
    m.insert("E", math_constants::E);
    m.insert("ЭЙЛЕР", math_constants::E);
    
    m.insert("ФИ", math_constants::PHI);
    m.insert("фи", math_constants::PHI);
    m.insert("phi", math_constants::PHI);
    m.insert("PHI", math_constants::PHI);
    m.insert("φ", math_constants::PHI);
    m.insert("ЗОЛОТОЕ", math_constants::PHI);
    m.insert("золотое_сечение", math_constants::PHI);
    
    // Корни
    m.insert("КОРЕНЬ2", math_constants::SQRT2);
    m.insert("корень2", math_constants::SQRT2);
    m.insert("sqrt2", math_constants::SQRT2);
    m.insert("SQRT2", math_constants::SQRT2);
    m.insert("√2", math_constants::SQRT2);
    
    m.insert("КОРЕНЬ3", math_constants::SQRT3);
    m.insert("корень3", math_constants::SQRT3);
    m.insert("sqrt3", math_constants::SQRT3);
    m.insert("SQRT3", math_constants::SQRT3);
    m.insert("√3", math_constants::SQRT3);
    
    m.insert("КОРЕНЬ5", math_constants::SQRT5);
    m.insert("корень5", math_constants::SQRT5);
    m.insert("sqrt5", math_constants::SQRT5);
    m.insert("SQRT5", math_constants::SQRT5);
    m.insert("√5", math_constants::SQRT5);
    
    // Логарифмы
    m.insert("LN2", math_constants::LN2);
    m.insert("ln2", math_constants::LN2);
    m.insert("ЛН2", math_constants::LN2);
    
    m.insert("LN10", math_constants::LN10);
    m.insert("ln10", math_constants::LN10);
    m.insert("ЛН10", math_constants::LN10);
    
    // Дроби пи
    m.insert("ПИ_2", math_constants::FRAC_PI_2);
    m.insert("пи_2", math_constants::FRAC_PI_2);
    m.insert("PI_2", math_constants::FRAC_PI_2);
    m.insert("ПОЛПИ", math_constants::FRAC_PI_2);
    
    m.insert("ПИ_4", math_constants::FRAC_PI_4);
    m.insert("пи_4", math_constants::FRAC_PI_4);
    m.insert("PI_4", math_constants::FRAC_PI_4);
    
    // Конвертация углов
    m.insert("ГРАД_РАД", math_constants::DEG_TO_RAD);
    m.insert("град_рад", math_constants::DEG_TO_RAD);
    m.insert("DEG_RAD", math_constants::DEG_TO_RAD);
    
    m.insert("РАД_ГРАД", math_constants::RAD_TO_DEG);
    m.insert("рад_град", math_constants::RAD_TO_DEG);
    m.insert("RAD_DEG", math_constants::RAD_TO_DEG);
    
    // Математические постоянные
    m.insert("ГАММА", math_constants::EULER_MASCHERONI);
    m.insert("гамма", math_constants::EULER_MASCHERONI);
    m.insert("gamma", math_constants::EULER_MASCHERONI);
    m.insert("ЭЙЛЕР_МАСКЕРОНИ", math_constants::EULER_MASCHERONI);
    m.insert("γ", math_constants::EULER_MASCHERONI);
    
    m.insert("АПЕРИ", math_constants::APERY);
    m.insert("апери", math_constants::APERY);
    m.insert("apery", math_constants::APERY);
    m.insert("ДЗЕТА3", math_constants::APERY);
    m.insert("ζ3", math_constants::APERY);
    
    m.insert("КАТАЛАН", math_constants::CATALAN);
    m.insert("каталан", math_constants::CATALAN);
    m.insert("catalan", math_constants::CATALAN);
    
    m.insert("ОМЕГА", math_constants::OMEGA);
    m.insert("омега", math_constants::OMEGA);
    m.insert("omega", math_constants::OMEGA);
    m.insert("Ω", math_constants::OMEGA);
    
    m.insert("ФЕЙГЕНБАУМ", math_constants::FEIGENBAUM_DELTA);
    m.insert("фейгенбаум", math_constants::FEIGENBAUM_DELTA);
    m.insert("feigenbaum", math_constants::FEIGENBAUM_DELTA);
    
    // Спецзначения
    m.insert("БЕСК", math_constants::INFINITY);
    m.insert("беск", math_constants::INFINITY);
    m.insert("inf", math_constants::INFINITY);
    m.insert("INF", math_constants::INFINITY);
    m.insert("БЕСКОНЕЧНОСТЬ", math_constants::INFINITY);
    m.insert("∞", math_constants::INFINITY);
    
    m.insert("ЭПСИЛОН", math_constants::EPSILON);
    m.insert("эпсилон", math_constants::EPSILON);
    m.insert("epsilon", math_constants::EPSILON);
    m.insert("EPSILON", math_constants::EPSILON);
    m.insert("ε", math_constants::EPSILON);
    m.insert("ЕПС", math_constants::EPSILON);
    
    m.insert("НЕЧ", math_constants::NAN);
    m.insert("неч", math_constants::NAN);
    m.insert("nan", math_constants::NAN);
    m.insert("NAN", math_constants::NAN);
    m.insert("НЕ_ЧИСЛО", math_constants::NAN);
    
    m
});

/// Целочисленные константы
pub static BUILTIN_INT_CONSTANTS: Lazy<HashMap<&'static str, i64>> = Lazy::new(|| {
    let mut m = HashMap::new();
    
    m.insert("МАКС_ЦЕЛ", math_constants::MAX_INT);
    m.insert("макс_цел", math_constants::MAX_INT);
    m.insert("MAX_INT", math_constants::MAX_INT);
    m.insert("INT_MAX", math_constants::MAX_INT);
    
    m.insert("МИН_ЦЕЛ", math_constants::MIN_INT);
    m.insert("мин_цел", math_constants::MIN_INT);
    m.insert("MIN_INT", math_constants::MIN_INT);
    m.insert("INT_MIN", math_constants::MIN_INT);
    
    m
});

/// Проверяет, является ли строка встроенной константой.
#[inline]
pub fn is_builtin_constant(s: &str) -> bool {
    BUILTIN_CONSTANTS.contains_key(s) || BUILTIN_INT_CONSTANTS.contains_key(s)
}

/// Возвращает значение встроенной константы (f64).
#[inline]
pub fn get_builtin_constant(s: &str) -> Option<f64> {
    BUILTIN_CONSTANTS.get(s).copied()
}

/// Возвращает значение целочисленной константы.
#[inline]
pub fn get_builtin_int_constant(s: &str) -> Option<i64> {
    BUILTIN_INT_CONSTANTS.get(s).copied()
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pi_variants() {
        assert_eq!(get_builtin_constant("ПИ"), Some(std::f64::consts::PI));
        assert_eq!(get_builtin_constant("пи"), Some(std::f64::consts::PI));
        assert_eq!(get_builtin_constant("pi"), Some(std::f64::consts::PI));
        assert_eq!(get_builtin_constant("π"), Some(std::f64::consts::PI));
    }
    
    #[test]
    fn test_tau() {
        let tau = get_builtin_constant("τ").unwrap();
        assert!((tau - 2.0 * std::f64::consts::PI).abs() < 1e-10);
    }
    
    #[test]
    fn test_golden_ratio() {
        let phi = get_builtin_constant("φ").unwrap();
        // φ² = φ + 1
        assert!((phi * phi - phi - 1.0).abs() < 1e-10);
    }
    
    #[test]
    fn test_euler_mascheroni() {
        let gamma = get_builtin_constant("γ").unwrap();
        assert!((gamma - 0.5772156649015329).abs() < 1e-10);
    }
    
    #[test]
    fn test_int_constants() {
        assert_eq!(get_builtin_int_constant("МАКС_ЦЕЛ"), Some(i64::MAX));
        assert_eq!(get_builtin_int_constant("МИН_ЦЕЛ"), Some(i64::MIN));
    }
    
    #[test]
    fn test_special_values() {
        assert!(get_builtin_constant("∞").unwrap().is_infinite());
        assert!(get_builtin_constant("НЕЧ").unwrap().is_nan());
    }
}
