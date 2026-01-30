use shared::f128::F128;

#[test]
fn nan_classification() {
    let n = F128::NAN;
    assert!(n.is_nan());
    assert!(!n.is_infinite());
}

#[test]
fn zero_and_sign() {
    let z = F128::ZERO;
    let nz = F128::NEG_ZERO;
    assert!(z.is_zero());
    assert!(nz.is_zero());
    assert!(z == z);
    assert!(z.total_cmp(&nz) == core::cmp::Ordering::Equal);
}

#[test]
fn inf_ordering() {
    let p = F128::INFINITY;
    let n = F128::NEG_INFINITY;
    assert!(p.is_infinite());
    assert!(n.is_infinite());
    assert!(n < p);
}

#[test]
fn nan_ordering() {
    let n = F128::NAN;
    assert!(n.is_nan());
    assert!(n.partial_cmp(&n).is_none());
    assert_eq!(n.total_cmp(&n), core::cmp::Ordering::Equal);
}

#[test]
fn zero_total_cmp_equivalence() {
    let z = F128::ZERO;
    let nz = F128::NEG_ZERO;
    assert_eq!(z.total_cmp(&nz), core::cmp::Ordering::Equal);
}

#[test]
fn nan_vs_normal_total_order() {
    let n = F128::NAN;
    let x = F128::INFINITY;
    assert_eq!(x.total_cmp(&n), core::cmp::Ordering::Less);
    assert_eq!(n.total_cmp(&x), core::cmp::Ordering::Greater);
}

#[test]
fn add_basic() {
    let one = F128::from_bits(0x3fff_0000_0000_0000, 0);
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);
    let sum = one + one;
    assert!(!sum.is_nan());
    assert!(!sum.is_infinite());
    assert!(!sum.is_zero());
    assert!(sum == two);

    let diff = two - one;
    assert!(!diff.is_nan());
    assert!(!diff.is_infinite());
    assert!(!diff.is_zero());
    assert!(diff == one);
}

#[test]
fn sub_basic() {
    let three = F128::from_bits(0x4000_8000_0000_0000, 0);
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);
    let one = F128::from_bits(0x3fff_0000_0000_0000, 0);
    let result = three - one;
    assert!(!result.is_nan());
    assert!(!result.is_infinite());
    assert!(!result.is_zero());
    assert!(result == two);
}

#[test]
fn sub_inf_minus_inf_is_nan() {
    let p_inf = F128::INFINITY;
    let result = p_inf - p_inf;
    assert!(result.is_nan());
}

#[test]
fn sub_pos_inf_minus_neg_inf_is_inf() {
    let p_inf = F128::INFINITY;
    let n_inf = F128::NEG_INFINITY;
    let result = p_inf - n_inf;
    assert!(result.is_infinite());
    assert!(!result.is_sign_negative());
}

#[test]
fn sub_neg_inf_minus_pos_inf_is_neg_inf() {
    let p_inf = F128::INFINITY;
    let n_inf = F128::NEG_INFINITY;
    let result = n_inf - p_inf;
    assert!(result.is_infinite());
    assert!(result.is_sign_negative());
}

#[test]
fn add_inf_and_inf() {
    let p_inf = F128::INFINITY;
    let result = p_inf + p_inf;
    assert!(result.is_infinite());
    assert!(!result.is_sign_negative());
}

#[test]
fn mul_zero_and_inf() {
    let z = F128::ZERO;
    let inf = F128::INFINITY;
    let r1 = z * inf;
    let r2 = inf * z;
    assert!(r1.is_nan());
    assert!(r2.is_nan());
}

#[test]
fn mul_basic() {
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);
    let three = F128::from_bits(0x4000_8000_0000_0000, 0);
    let six = F128::from_bits(0x4001_8000_0000_0000, 0);

    let result = two * three;
    assert!(!result.is_nan());
    assert!(!result.is_infinite());
    assert!(!result.is_sign_negative());
    assert!(!result.is_zero());
    assert!(result == six);
}

#[test]
fn div_zero_cases() {
    let z = F128::ZERO;
    let one = F128::from_bits(0x3fff_0000_0000_0000, 0);
    let r1 = z / one;
    assert!(r1.is_zero());

    let r2 = one / z;
    assert!(r2.is_infinite());
}

#[test]
fn div_basic() {
    let six = F128::from_bits(0x4001_8000_0000_0000, 0);
    let three = F128::from_bits(0x4000_8000_0000_0000, 0);
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);

    let result = six / three;
    assert!(!result.is_nan());
    assert!(!result.is_infinite());
    assert!(!result.is_zero());
    assert!(result == two);
}

#[test]
fn mul_commutativity_two_three() {
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);
    let three = F128::from_bits(0x4000_8000_0000_0000, 0);
    let six = F128::from_bits(0x4001_8000_0000_0000, 0);

    let a = two * three;
    let b = three * two;
    assert_eq!(a, b, "commutativity failed: two*three != three*two");
    assert_eq!(a, six, "expected 2*3 == 6");
}

#[test]
fn mul_power_of_two_doubling() {
    let two = F128::from_bits(0x4000_0000_0000_0000, 0); // 2.0
    let four = F128::from_bits(0x4001_0000_0000_0000, 0); // 4.0
    let res = two * two;
    assert_eq!(res, four, "2 * 2 should be 4");
}

#[test]
fn div_inverse_basic_cases() {
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);
    let three = F128::from_bits(0x4000_8000_0000_0000, 0);
    let six = F128::from_bits(0x4001_8000_0000_0000, 0);

    assert_eq!(six / three, two, "6 / 3 should be 2");
    assert_eq!(six / two, three, "6 / 2 should be 3");
}

#[test]
fn mul_identity_and_div_identity() {
    let one = F128::from_bits(0x3fff_0000_0000_0000, 0);
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);

    assert_eq!(one * two, two, "1 * x == x");
    assert_eq!(two / one, two, "x / 1 == x");
}

#[test]
fn div_by_self_returns_one() {
    let three = F128::from_bits(0x4000_8000_0000_0000, 0);
    let one = F128::from_bits(0x3fff_0000_0000_0000, 0);
    assert_eq!(three / three, one, "x / x == 1 (for normal non-zero x)");
}

#[test]
fn neg_flips_sign_bit() {
    let x = F128::from_bits(0x3fff_0000_0000_0000, 0);
    let y = -x;
    assert!(y.is_sign_negative());
    assert!(!y.is_zero());
}

#[test]
fn neg_nan_is_nan() {
    let n = F128::NAN;
    let m = -n;
    assert!(m.is_nan());
}

#[test]
fn from_u64_basic() {
    let one = F128::from(1u64);
    let two = F128::from(2u64);
    assert!(one < two);
    assert!(one + one == two);
}

#[test]
fn from_i64_negative() {
    let minus_one = F128::from(-1i64);
    let one = F128::from(1i64);
    assert!(minus_one.is_sign_negative());
    let sum = minus_one + one;
    assert!(sum.is_zero());
}

#[test]
fn sqrt_basic_perfect_square() {
    let four = F128::from_bits(0x4001_0000_0000_0000, 0);
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);
    let r = four.sqrt();
    assert!(!r.is_nan());
    assert_eq!(r, two);
}

#[test]
fn sqrt_negative_is_nan() {
    let minus_one = -F128::from_bits(0x3fff_0000_0000_0000, 0);
    let r = minus_one.sqrt();
    assert!(r.is_nan());
}

#[test]
fn powf_one_anything() {
    let one = F128::from_bits(0x3fff_0000_0000_0000, 0);
    let three = F128::from_bits(0x4000_8000_0000_0000, 0);
    let r = one.powf(three);
    assert_eq!(r, one);
}

#[test]
fn powf_anything_zero() {
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);
    let zero = F128::ZERO;
    let one = F128::from_bits(0x3fff_0000_0000_0000, 0);
    let r = two.powf(zero);
    assert_eq!(r, one);
}

#[test]
fn powf_zero_positive() {
    let zero = F128::ZERO;
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);
    let r = zero.powf(two);
    assert!(r.is_zero());
}

#[test]
fn powf_zero_negative_inf() {
    let zero = F128::ZERO;
    let minus_two = -F128::from_bits(0x4000_0000_0000_0000, 0);
    let r = zero.powf(minus_two);
    assert!(r.is_infinite());
    assert!(!r.is_sign_negative());
}

#[test]
fn powf_negative_integer_exponent() {
    let minus_two = -F128::from_bits(0x4000_0000_0000_0000, 0);
    let three = F128::from_bits(0x4000_8000_0000_0000, 0);
    let r = minus_two.powf(three);
    assert!(r.is_sign_negative());
}

#[test]
fn powf_negative_non_integer_nan() {
    let minus_two = -F128::from_bits(0x4000_0000_0000_0000, 0);
    let half = F128::from_bits(0x3ffe_8000_0000_0000, 0);
    let r = minus_two.powf(half);
    assert!(r.is_nan());
}

#[test]
fn powf_two_squared_via_internal_ln_exp() {
    let two = F128::from_bits(0x4000_0000_0000_0000, 0);
    let two_exp = F128::from_bits(0x4000_0000_0000_0000, 0);
    let four = F128::from_bits(0x4001_0000_0000_0000, 0);
    let r = two.powf(two_exp);

    assert!(!r.is_nan());
    assert!(!r.is_infinite());
    assert!(!r.is_zero());

    let three = F128::from(3u64);
    let five = F128::from(5u64);
    println!("r = {}", r);
    assert!(r > three && r < five);
    assert!(r == four);
}

#[test]
fn demo_f128_add() {
    let a = F128::from(3u64);
    let b = F128::from(2u64);
    let c = a + b;

    println!("{} + {} = {}", a, b, c);
}

#[test]
fn trig_sin_cos_values() {
    let zero = F128::ZERO;
    let pi = F128::PI;
    let pi_2 = F128::PI_2;
    let epsilon = F128::from_bits(0x3FEE_0000_0000_0000, 0); // ~1e-5

    // sin(0) = 0
    assert!(zero.sin().is_zero());
    // cos(0) = 1
    assert_eq!(zero.cos(), F128::from(1));

    // sin(pi/2) = 1
    let s_pi2 = pi_2.sin();
    assert!((s_pi2 - F128::from(1)).abs() < epsilon);

    // cos(pi) = -1
    let c_pi = pi.cos();
    assert!((c_pi - F128::from(-1)).abs() < epsilon);
}

#[test]
fn trig_tan_values() {
    let zero = F128::ZERO;
    let pi_4 = F128::PI_4;
    let epsilon = F128::from_bits(0x3FEE_0000_0000_0000, 0);

    // tan(0) = 0
    assert!(zero.tan().is_zero());

    // tan(pi/4) = 1
    let t_pi4 = pi_4.tan();
    assert!((t_pi4 - F128::from(1)).abs() < epsilon);
}

#[test]
fn trig_pythagorean_identity_and_symmetry() {
    let pi = F128::PI;
    let _pi_2 = F128::PI_2;
    let _pi_4 = F128::PI_4;
    let eps = F128::from_bits(0x3FEE_0000_0000_0000, 0); // ~1e-5

    // angles to test: pi/6, pi/4, pi/3
    let a = pi / F128::from_int(6);
    let b = pi / F128::from_int(4);
    let c = pi / F128::from_int(3);

    for &x in &[a, b, c] {
        let s = x.sin();
        let coss = x.cos();
        // sin^2 + cos^2 == 1
        let lhs = s * s + coss * coss;
        assert!((lhs - F128::from_int(1)).abs() < eps);

        // odd/even symmetry
        assert!(((-x).sin() + s).abs() < eps, "sin(-x) == -sin(x)");
        assert!(((-x).cos() - coss).abs() < eps, "cos(-x) == cos(x)");
        assert!(((-x).tan() + x.tan()).abs() < eps, "tan(-x) == -tan(x)");
    }
}

#[test]
fn trig_periodicity_and_shift_identities() {
    let pi = F128::PI;
    let two_pi = F128::TWO_PI;
    let eps = F128::from_bits(0x3FEE_0000_0000_0000, 0);

    let x = pi / F128::from_int(6);
    assert!(((x + two_pi).sin() - x.sin()).abs() < eps);
    assert!(((x + two_pi).cos() - x.cos()).abs() < eps);

    // sin(x + pi) = -sin(x), cos(x + pi) = -cos(x)
    assert!(((x + pi).sin() + x.sin()).abs() < eps);
    assert!(((x + pi).cos() + x.cos()).abs() < eps);
}

#[test]
fn trig_addition_double_and_tan_identities() {
    let pi = F128::PI;
    let eps = F128::from_bits(0x3FEE_0000_0000_0000, 0);

    let a = pi / F128::from_int(6); // 30deg
    let b = pi / F128::from_int(4); // 45deg

    let sin_a = a.sin();
    let cos_a = a.cos();
    let sin_b = b.sin();
    let cos_b = b.cos();

    // sin(a+b) = sin a cos b + cos a sin b
    let lhs = (a + b).sin();
    let rhs = sin_a * cos_b + cos_a * sin_b;
    assert!((lhs - rhs).abs() < eps);

    // cos(a+b) = cos a cos b - sin a sin b
    let lhs = (a + b).cos();
    let rhs = cos_a * cos_b - sin_a * sin_b;
    assert!((lhs - rhs).abs() < eps);

    // sin(2a) = 2 sin a cos a
    let lhs2 = (a + a).sin();
    let rhs2 = F128::from_int(2) * sin_a * cos_a;
    assert!((lhs2 - rhs2).abs() < eps);

    // tan(a+b) = (tan a + tan b) / (1 - tan a tan b)
    let tan_a = sin_a / cos_a;
    let tan_b = sin_b / cos_b;
    let lhs = (a + b).tan();
    let rhs = (tan_a + tan_b) / (F128::from_int(1) - tan_a * tan_b);
    assert!((lhs - rhs).abs() < eps);
}

#[test]
fn trig_tan_asymptote_small_angle_and_inverses() {
    let pi = F128::PI;
    let pi_2 = F128::PI_2;
    let eps = F128::from_bits(0x3FEE_0000_0000_0000, 0);

    // cos(pi/2) == 0, tan(pi/2) should be infinite
    assert!(pi_2.cos().is_zero());
    assert!(pi_2.tan().is_infinite());

    // small angle approximations
    let small = F128::from_bits(0x3F5A_2C0E_6FB3_7A6, 0); // ~1e-3 (approx)
    assert!((small.sin() - small).abs() < F128::from_bits(0x3FCB_0000_0000_0000, 0));
    let one_minus_cos = F128::from_int(1) - small.cos();
    let approx = small * small / F128::from_int(2);
    assert!((one_minus_cos - approx).abs() < F128::from_bits(0x3FCB_0000_0000_0000, 0));

    // asin(sin(x)) ~= x for x in [-pi/2, pi/2]
    let x = pi / F128::from_int(6);
    let r = x.sin().asin();
    assert!((r - x).abs() < eps);

    // acos(cos(x)) ~= x for x in [0, pi]
    let x2 = pi / F128::from_int(3);
    let r2 = x2.cos().acos();
    assert!((r2 - x2).abs() < eps);
}

#[test]
fn trig_asin_acos_values() {
    let zero = F128::ZERO;
    let one = F128::from(1);
    let neg_one = F128::from(-1);
    let epsilon = F128::from_bits(0x3FEE_0000_0000_0000, 0);

    // asin(0) = 0
    assert!(zero.asin().is_zero());

    // acos(1) = 0
    assert!(one.acos().is_zero());

    // asin(1) = pi/2
    let as_1 = one.asin();
    assert!((as_1 - F128::PI_2).abs() < epsilon);

    // acos(-1) = pi
    let ac_neg1 = neg_one.acos();
    assert!((ac_neg1 - F128::PI).abs() < epsilon);
}

#[test]
fn trig_domain_errors() {
    let two = F128::from(2);
    let inf = F128::INFINITY;
    let nan = F128::NAN;

    // asin(2) -> NaN
    assert!(two.asin().is_nan());
    // acos(2) -> NaN
    assert!(two.acos().is_nan());

    // sin(Inf) -> NaN
    assert!(inf.sin().is_nan());
    // cos(Inf) -> NaN
    assert!(inf.cos().is_nan());

    // Propagate NaN
    assert!(nan.sin().is_nan());
    assert!(nan.cos().is_nan());
    assert!(nan.tan().is_nan());
    assert!(nan.asin().is_nan());
    assert!(nan.acos().is_nan());
}
