use super::support::{assert_allocation_free_scalar, assert_scalar, program, Expected};
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
        let expression =
            format!("convert-i64-to-f64-rounded/\n{value}\n/convert-i64-to-f64-rounded");
        assert_allocation_free_scalar(&program("f64", &expression), Expected::F64(bits));
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
        let expression = format!(
            "is-ok/\nconvert-i64-to-f64-exact/\n{value}\n/convert-i64-to-f64-exact\n/is-ok"
        );
        assert_scalar(&program("bool", &expression), Expected::Bool(exact));
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
        ("divide/\n1.0\n0.0\n/divide", false),
        ("divide/\n-1.0\n0.0\n/divide", false),
        ("divide/\n0.0\n0.0\n/divide", false),
    ];
    for (value, accepted) in exact {
        let expression = format!(
            "is-ok/\nconvert-f64-to-i64-exact/\n{value}\n/convert-f64-to-i64-exact\n/is-ok"
        );
        assert_scalar(&program("bool", &expression), Expected::Bool(accepted));
    }
    for (value, expected) in [("1.9", 1), ("-1.9", -1), ("0.0", 0), ("-0.0", 0)] {
        let expression =
            format!("unwrap-ok/\nconvert-f64-to-i64-truncating/\n{value}\n/convert-f64-to-i64-truncating\n/unwrap-ok");
        assert_scalar(&program("i64", &expression), Expected::I64(expected));
    }
    let subnormal = format!("0.{}5", "0".repeat(323));
    let expression =
        format!("unwrap-ok/\nconvert-f64-to-i64-truncating/\n{subnormal}\n/convert-f64-to-i64-truncating\n/unwrap-ok");
    assert_scalar(&program("i64", &expression), Expected::I64(0));
}

fn error_code(expression: &str) -> String {
    let mut arms = String::new();
    for (name, code) in [
        ("non-finite", 1),
        ("out-of-range", 2),
        ("fractional", 3),
        ("inexact", 4),
    ] {
        arms.push_str(&format!(
            "arm/\nvariant-pattern/\ntype/\nnumeric-error/\n/numeric-error\n/type\nvariant/\n{name}\n/variant\nfields/\n/fields\n/variant-pattern\n{code}\n/arm\n"
        ));
    }
    program(
        "i64",
        &format!("match/\nunwrap-err/\n{expression}\n/unwrap-err\narms/\n{arms}/arms\n/match"),
    )
}

#[test]
fn numeric_error_cases_are_stable_nominal_values_on_four_engines() {
    let cases = [
        (
            "convert-i64-to-f64-exact/\n9007199254740993\n/convert-i64-to-f64-exact",
            4,
        ),
        (
            "convert-f64-to-i64-exact/\ndivide/\n0.0\n0.0\n/divide\n/convert-f64-to-i64-exact",
            1,
        ),
        (
            "convert-f64-to-i64-exact/\n9223372036854775808.0\n/convert-f64-to-i64-exact",
            2,
        ),
        ("convert-f64-to-i64-exact/\n-1.5\n/convert-f64-to-i64-exact", 3),
        (
            "convert-f64-to-i64-truncating/\ndivide/\n1.0\n0.0\n/divide\n/convert-f64-to-i64-truncating",
            1,
        ),
        (
            "convert-f64-to-i64-truncating/\n9223372036854775808.0\n/convert-f64-to-i64-truncating",
            2,
        ),
    ];
    for (expression, expected) in cases {
        assert_scalar(&error_code(expression), Expected::I64(expected));
    }
}

#[test]
fn canonical_numeric_operations_require_exact_types_and_explicit_conversions() {
    let mixed = program("f64", "add/\n1\n2.0\n/add");
    assert!(compile_source(&mixed, "mixed.lkjscript", &Limits::default()).is_err());
    let explicit =
        "main/\nsig/\ninputs/\n/inputs\noutput/\nf64\n/output\n/sig\nconvert-i64-to-f64-rounded/\n1\n/convert-i64-to-f64-rounded\n/main\n";
    assert!(compile_source(explicit, "explicit.lkjscript", &Limits::default()).is_ok());
}
