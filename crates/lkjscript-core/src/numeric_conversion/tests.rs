use super::*;

#[test]
fn integer_rounding_uses_nearest_ties_even_without_host_casts() {
    let cases = [
        (0, 0x0000_0000_0000_0000),
        (9_007_199_254_740_991, 0x433f_ffff_ffff_ffff),
        (9_007_199_254_740_992, 0x4340_0000_0000_0000),
        (9_007_199_254_740_993, 0x4340_0000_0000_0000),
        (9_007_199_254_740_994, 0x4340_0000_0000_0001),
        (9_007_199_254_740_995, 0x4340_0000_0000_0002),
        (i64::MIN, 0xc3e0_0000_0000_0000),
        (i64::MAX, 0x43e0_0000_0000_0000),
    ];
    for (value, expected) in cases {
        assert_eq!(f64_from_i64_rounded(value).to_bits(), expected);
    }
}

#[test]
fn exact_integer_conversion_checks_spacing_not_only_two_to_fifty_three() {
    for value in [
        -9_007_199_254_740_992,
        9_007_199_254_740_992,
        9_007_199_254_740_994,
        i64::MIN,
    ] {
        assert!(f64_from_i64_exact(value).is_ok());
    }
    for value in [
        -9_007_199_254_740_993,
        9_007_199_254_740_993,
        9_007_199_254_740_995,
        i64::MAX,
    ] {
        assert_eq!(f64_from_i64_exact(value), Err(NumericError::Inexact));
    }
}

#[test]
fn float_conversion_classifies_signed_zero_subnormal_fraction_and_range() {
    for zero in [0.0, -0.0] {
        assert_eq!(i64_from_f64_exact(zero), Ok(0));
        assert_eq!(i64_from_f64_trunc(zero), Ok(0));
    }
    for value in [f64::from_bits(1), f64::from_bits(1_u64 << 63 | 1)] {
        assert_eq!(i64_from_f64_exact(value), Err(NumericError::Fractional));
        assert_eq!(i64_from_f64_trunc(value), Ok(0));
    }
    assert_eq!(
        i64_from_f64_exact(-9_223_372_036_854_775_808.0),
        Ok(i64::MIN)
    );
    assert_eq!(
        i64_from_f64_exact(9_223_372_036_854_774_784.0),
        Ok(9_223_372_036_854_774_784)
    );
    assert_eq!(
        i64_from_f64_exact(9_223_372_036_854_775_808.0),
        Err(NumericError::OutOfRange)
    );
    for (value, expected) in [(1.9, 1), (-1.9, -1)] {
        assert_eq!(i64_from_f64_exact(value), Err(NumericError::Fractional));
        assert_eq!(i64_from_f64_trunc(value), Ok(expected));
    }
}

#[test]
fn every_nan_class_and_infinity_is_nonfinite_independent_of_payload() {
    let nan_bits = [
        0x7ff0_0000_0000_0001,
        0x7ff7_ffff_ffff_ffff,
        0x7ff8_0000_0000_0000,
        0x7fff_ffff_ffff_ffff,
        0xfff0_0000_0000_0001,
        0xfff8_1234_5678_9abc,
        0xffff_ffff_ffff_ffff,
    ];
    for bits in nan_bits {
        let value = f64::from_bits(bits);
        assert_eq!(i64_from_f64_exact(value), Err(NumericError::NonFinite));
        assert_eq!(i64_from_f64_trunc(value), Err(NumericError::NonFinite));
    }
    for value in [f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(i64_from_f64_exact(value), Err(NumericError::NonFinite));
        assert_eq!(i64_from_f64_trunc(value), Err(NumericError::NonFinite));
    }
}
