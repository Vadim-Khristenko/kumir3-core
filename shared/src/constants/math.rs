//! Mathematical constants for Kumir 3.
//!
//! All mathematical constants available in the language and associated with types.

use once_cell::sync::Lazy;
use std::collections::HashMap;

// =============================================================================
//         SECTION: MATHEMATICAL CONSTANTS
// =============================================================================

/// Mathematical constants accessible in Kumir programs.
pub mod math_constants {
    // =========================================================================
    //                    FUNDAMENTAL CONSTANTS
    // =========================================================================

    /// Pi (π) — ratio of circle circumference to diameter.
    pub const PI: f64 = std::f64::consts::PI;

    /// Tau (τ = 2π) — full circle rotation in radians.
    pub const TAU: f64 = std::f64::consts::TAU;

    /// Euler's number (e) — base of natural logarithm.
    pub const E: f64 = std::f64::consts::E;

    /// Golden ratio (φ = (1 + √5) / 2).
    pub const PHI: f64 = 1.618033988749895;

    /// Silver ratio (δₛ = 1 + √2).
    pub const SILVER_RATIO: f64 = 2.414213562373095;

    /// Plastic constant (ρ ≈ 1.3247).
    pub const PLASTIC: f64 = 1.324717957244746;

    // =========================================================================
    //                    ROOTS AND LOGARITHMS
    // =========================================================================

    /// Square root of 2 (√2) — diagonal of unit square.
    pub const SQRT2: f64 = std::f64::consts::SQRT_2;

    /// Square root of 3 (√3).
    pub const SQRT3: f64 = 1.7320508075688772;

    /// Square root of 5 (√5).
    pub const SQRT5: f64 = 2.23606797749979;

    /// 1 / √2 = √2 / 2.
    pub const FRAC_1_SQRT_2: f64 = std::f64::consts::FRAC_1_SQRT_2;

    /// Cube root of 2 (∛2).
    pub const CBRT2: f64 = 1.2599210498948732;

    /// Cube root of 3 (∛3).
    pub const CBRT3: f64 = 1.4422495703074083;

    /// Natural logarithm of 2 (ln 2).
    pub const LN2: f64 = std::f64::consts::LN_2;

    /// Natural logarithm of 10 (ln 10).
    pub const LN10: f64 = std::f64::consts::LN_10;

    /// Common logarithm of e (log₁₀(e)).
    pub const LOG10_E: f64 = std::f64::consts::LOG10_E;

    /// Binary logarithm of e (log₂(e)).
    pub const LOG2_E: f64 = std::f64::consts::LOG2_E;

    /// Common logarithm of 2 (log₁₀(2)).
    pub const LOG10_2: f64 = std::f64::consts::LOG10_2;

    // =========================================================================
    //                    FRACTIONS OF PI
    // =========================================================================

    /// π / 2 (90 degrees).
    pub const FRAC_PI_2: f64 = std::f64::consts::FRAC_PI_2;

    /// π / 3 (60 degrees).
    pub const FRAC_PI_3: f64 = std::f64::consts::FRAC_PI_3;

    /// π / 4 (45 degrees).
    pub const FRAC_PI_4: f64 = std::f64::consts::FRAC_PI_4;

    /// π / 6 (30 degrees).
    pub const FRAC_PI_6: f64 = std::f64::consts::FRAC_PI_6;

    /// π / 8 (22.5 degrees).
    pub const FRAC_PI_8: f64 = std::f64::consts::FRAC_PI_8;

    /// 1 / π.
    pub const FRAC_1_PI: f64 = std::f64::consts::FRAC_1_PI;

    /// 2 / π.
    pub const FRAC_2_PI: f64 = std::f64::consts::FRAC_2_PI;

    /// 2 / √π.
    pub const FRAC_2_SQRT_PI: f64 = std::f64::consts::FRAC_2_SQRT_PI;

    // =========================================================================
    //                    ANGLE CONVERSION
    // =========================================================================

    /// Degrees to radians conversion factor (π / 180).
    pub const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

    /// Radians to degrees conversion factor (180 / π).
    pub const RAD_TO_DEG: f64 = 180.0 / std::f64::consts::PI;

    // =========================================================================
    //                    MATHEMATICAL CONSTANTS (SPECIAL)
    // =========================================================================

    /// Euler–Mascheroni constant (γ ≈ 0.5772).
    pub const EULER_MASCHERONI: f64 = 0.5772156649015329;

    /// Apéry's constant (ζ(3) ≈ 1.202) — Riemann zeta function at 3.
    pub const APERY: f64 = 1.2020569031595943;

    /// Catalan's constant (G ≈ 0.9159).
    pub const CATALAN: f64 = 0.915_965_594_177_219;

    /// Khinchin's constant (K ≈ 2.6854).
    pub const KHINCHIN: f64 = 2.6854520010653064;

    /// Glaisher–Kinkelin constant (A ≈ 1.2824).
    pub const GLAISHER: f64 = 1.2824271291006226;

    /// Omega constant (Ω) — solution to x·eˣ = 1.
    pub const OMEGA: f64 = 0.5671432904097838;

    /// Conway's constant (λ ≈ 1.3035).
    pub const CONWAY: f64 = 1.3035772690342963;

    /// Feigenbaum constant (δ ≈ 4.6692) — universal chaos scaling.
    pub const FEIGENBAUM_DELTA: f64 = 4.669_201_609_102_99;

    /// Second Feigenbaum constant (α ≈ 2.5029).
    pub const FEIGENBAUM_ALPHA: f64 = 2.502907875095892;

    /// Meissel–Mertens constant (M ≈ 0.2615).
    pub const MEISSEL_MERTENS: f64 = 0.2614972128476428;

    /// Twin prime constant (C₂ ≈ 0.6601).
    pub const TWIN_PRIME: f64 = 0.6601618158468696;

    // =========================================================================
    //                    PHYSICAL CONSTANTS (DIMENSIONLESS)
    // =========================================================================

    /// Fine structure constant (α ≈ 1/137).
    pub const FINE_STRUCTURE: f64 = 0.0072973525693;

    // =========================================================================
    //                    TYPE LIMITS
    // =========================================================================

    /// Maximum 64-bit signed integer.
    pub const MAX_INT: i64 = i64::MAX;

    /// Minimum 64-bit signed integer.
    pub const MIN_INT: i64 = i64::MIN;

    /// Maximum representable 64-bit float.
    pub const MAX_REAL: f64 = f64::MAX;

    /// Smallest positive 64-bit float.
    pub const MIN_REAL: f64 = f64::MIN_POSITIVE;

    /// Machine epsilon for 64-bit float — smallest relative difference from 1.0.
    pub const EPSILON: f64 = f64::EPSILON;

    /// Positive infinity.
    pub const INFINITY: f64 = f64::INFINITY;

    /// Negative infinity.
    pub const NEG_INFINITY: f64 = f64::NEG_INFINITY;

    /// Not-a-number (NaN).
    pub const NAN: f64 = f64::NAN;
}

/// Built-in Kumir constants accessible as variables.
pub static BUILTIN_CONSTANTS: Lazy<HashMap<&'static str, f64>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // Fundamental constants
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

    // Roots
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

    // Logarithms
    m.insert("LN2", math_constants::LN2);
    m.insert("ln2", math_constants::LN2);
    m.insert("ЛН2", math_constants::LN2);

    m.insert("LN10", math_constants::LN10);
    m.insert("ln10", math_constants::LN10);
    m.insert("ЛН10", math_constants::LN10);

    // Fractions of pi
    m.insert("ПИ_2", math_constants::FRAC_PI_2);
    m.insert("пи_2", math_constants::FRAC_PI_2);
    m.insert("PI_2", math_constants::FRAC_PI_2);
    m.insert("ПОЛПИ", math_constants::FRAC_PI_2);

    m.insert("ПИ_4", math_constants::FRAC_PI_4);
    m.insert("пи_4", math_constants::FRAC_PI_4);
    m.insert("PI_4", math_constants::FRAC_PI_4);

    // Angle conversions
    m.insert("ГРАД_РАД", math_constants::DEG_TO_RAD);
    m.insert("град_рад", math_constants::DEG_TO_RAD);
    m.insert("DEG_RAD", math_constants::DEG_TO_RAD);

    m.insert("РАД_ГРАД", math_constants::RAD_TO_DEG);
    m.insert("рад_град", math_constants::RAD_TO_DEG);
    m.insert("RAD_DEG", math_constants::RAD_TO_DEG);

    // Special constants
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

    // Special values
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

/// Integer-valued built-in constants.
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

/// Checks if a string is a built-in constant name.
#[inline]
pub fn is_builtin_constant(s: &str) -> bool {
    BUILTIN_CONSTANTS.contains_key(s) || BUILTIN_INT_CONSTANTS.contains_key(s)
}

/// Returns the value of a built-in floating-point constant.
#[inline]
pub fn get_builtin_constant(s: &str) -> Option<f64> {
    BUILTIN_CONSTANTS.get(s).copied()
}

/// Returns the value of a built-in integer constant.
#[inline]
pub fn get_builtin_int_constant(s: &str) -> Option<i64> {
    BUILTIN_INT_CONSTANTS.get(s).copied()
}

// =============================================================================
//         SECTION: TESTS
// =============================================================================

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
