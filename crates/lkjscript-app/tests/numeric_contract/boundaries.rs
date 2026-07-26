use super::support::{assert_scalar, edition2, Expected};
use super::*;

#[test]
fn rounded_i64_boundaries_preserve_exact_f64_bits_on_four_engines() {
    let cases = [
        (-9_007_199_254_740_993_i64, 0xc340_0000_0000_0000_u64),
        (-9_007_199_254_740_992, 0xc340_0000_0000_0000),
        (9_007_199_254_740_991, 0x433f_ffff_ffff_ffff),
        (9_007_199_254_740_992, 0x4340_0000_0000_0000),
        (9_007_199_254_740_993, 0x4340_0000_0000_0000),
        (9_007_199_254_740_994, 0x4340_0000_0000_0001),
        (i64::MIN, 0xc3e0_0000_0000_0000),
        (i64::MAX, 0x43e0_0000_0000_0000),
    ];
    for (value, bits) in cases {
        let expression = format!("f64-from-i64-rounded/\n{value}\n/f64-from-i64-rounded");
        assert_scalar(&edition2("F64", &expression), Expected::F64(bits));
    }
}

#[test]
fn exact_i64_boundaries_distinguish_representability() {
    for (value, exact) in [
        (-9_007_199_254_740_993_i64, false),
        (-9_007_199_254_740_992, true),
        (9_007_199_254_740_992, true),
        (9_007_199_254_740_993, false),
        (9_007_199_254_740_994, true),
        (i64::MIN, true),
        (i64::MAX, false),
    ] {
        let expression =
            format!("is-ok/\nf64-from-i64-exact/\n{value}\n/f64-from-i64-exact\n/is-ok");
        assert_scalar(&edition2("Bool", &expression), Expected::Bool(exact));
    }
}

#[test]
fn f64_to_i64_boundaries_cover_zero_fraction_nonfinite_and_range() {
    let exact = [
        ("0.0", true),
        ("-0.0", true),
        ("-9223372036854775808.0", true),
        ("9223372036854774784.0", true),
        ("9223372036854775808.0", false),
        ("1.5", false),
        ("-1.5", false),
        ("div/\n1.0\n0.0\n/div", false),
        ("div/\n-1.0\n0.0\n/div", false),
        ("div/\n0.0\n0.0\n/div", false),
    ];
    for (value, accepted) in exact {
        let expression =
            format!("is-ok/\ni64-from-f64-exact/\n{value}\n/i64-from-f64-exact\n/is-ok");
        assert_scalar(&edition2("Bool", &expression), Expected::Bool(accepted));
    }
    for (value, expected) in [("1.9", 1), ("-1.9", -1), ("0.0", 0), ("-0.0", 0)] {
        let expression =
            format!("unwrap-ok/\ni64-from-f64-trunc/\n{value}\n/i64-from-f64-trunc\n/unwrap-ok");
        assert_scalar(&edition2("I64", &expression), Expected::I64(expected));
    }
    let subnormal = format!("0.{}5", "0".repeat(323));
    let expression =
        format!("unwrap-ok/\ni64-from-f64-trunc/\n{subnormal}\n/i64-from-f64-trunc\n/unwrap-ok");
    assert_scalar(&edition2("I64", &expression), Expected::I64(0));
}

fn error_code(expression: &str) -> String {
    let mut arms = String::new();
    for (name, code) in [
        ("NonFinite", 1),
        ("OutOfRange", 2),
        ("Fractional", 3),
        ("Inexact", 4),
    ] {
        arms.push_str(&format!(
            "arm/\nvariant-pattern/\ntype/\nNumericError/\n/NumericError\n/type\nvariant/\n{name}\n/variant\nfields/\n/fields\n/variant-pattern\n{code}\n/arm\n"
        ));
    }
    edition2(
        "I64",
        &format!("match/\nunwrap-err/\n{expression}\n/unwrap-err\narms/\n{arms}/arms\n/match"),
    )
}

#[test]
fn numeric_error_cases_are_stable_nominal_values_on_four_engines() {
    let cases = [
        (
            "f64-from-i64-exact/\n9007199254740993\n/f64-from-i64-exact",
            4,
        ),
        (
            "i64-from-f64-exact/\ndiv/\n0.0\n0.0\n/div\n/i64-from-f64-exact",
            1,
        ),
        (
            "i64-from-f64-exact/\n9223372036854775808.0\n/i64-from-f64-exact",
            2,
        ),
        ("i64-from-f64-exact/\n-1.5\n/i64-from-f64-exact", 3),
        (
            "i64-from-f64-trunc/\ndiv/\n1.0\n0.0\n/div\n/i64-from-f64-trunc",
            1,
        ),
        (
            "i64-from-f64-trunc/\n9223372036854775808.0\n/i64-from-f64-trunc",
            2,
        ),
    ];
    for (expression, expected) in cases {
        assert_scalar(&error_code(expression), Expected::I64(expected));
    }
}

#[test]
fn edition2_rejects_mixed_numeric_operations_and_edition1_has_no_conversions() {
    let mixed = edition2("F64", "+/\n1\n2.0\n/+");
    assert!(compile_source(&mixed, "mixed.lkjscript", &Limits::default()).is_err());
    let edition1 =
        "main/\nsig/\n->\nF64\n/sig\nf64-from-i64-rounded/\n1\n/f64-from-i64-rounded\n/main\n";
    assert!(compile_source(edition1, "edition1.lkjscript", &Limits::default()).is_err());
}
