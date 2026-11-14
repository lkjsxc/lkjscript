//! Exact decimal conversion is checked against Rust and then run as source on both routes.
#[path = "../bridge/library.rs"]
mod bootstrap_library;
#[path = "../src/elaborate/floats.rs"]
mod floats;
#[path = "support/native.rs"]
mod native_test;

use lkjscript::source::{SourceBundle, SourceInput};
use lkjscript::{compiler, interpreter};

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Finite(u64),
    Overflow,
    Invalid,
}
fn rust_outcome(text: &str) -> Outcome {
    match text.parse::<f64>() {
        Ok(value) if value.is_finite() => Outcome::Finite(value.to_bits()),
        Ok(_) => Outcome::Overflow,
        Err(_) => Outcome::Invalid,
    }
}
fn compare(text: &str) {
    let actual = match floats::decimal_bits(text) {
        floats::Parsed::Finite(bits) => Outcome::Finite(bits),
        floats::Parsed::Overflow => Outcome::Overflow,
        floats::Parsed::Invalid => Outcome::Invalid,
    };
    assert_eq!(actual, rust_outcome(text), "decimal {text:?}");
}

#[test]
fn exact_midpoints_and_neighbours_match_rust_bits() {
    // Generated independently with integer arithmetic: (2q+1)*2^(e-53),
    // then one exact decimal unit below/above. The fixture records positive
    // cases; testing both signs also checks negative zero and underflow.
    for line in include_str!("fixtures/float_rounding.tsv").lines() {
        let (_, text) = line.split_once('\t').unwrap();
        compare(text);
        compare(&format!("-{text}"));
    }
}

#[test]
fn decimal_grammar_and_unbounded_exponent_cancellation_match_rust() {
    for text in [
        "0",
        "-0",
        "+0",
        "0.",
        "-.0",
        ".5",
        "1.",
        "1.e2",
        "-1e-999999999999999999999999999999",
        "0e999999999999999999999999999999",
        "1e999999999999999999999999999999",
        "1e-999999999999999999999999999999",
        "1e309",
        "1e-325",
        "1e-324",
        "2.4703282292062327e-324",
        "2.4703282292062328e-324",
        "2.2250738585072011e-308",
        "2.2250738585072012e-308",
        "1.7976931348623157e308",
        "1.7976931348623158e308",
        "1.7976931348623159e308",
        "9007199254740993",
        "9007199254740995",
        "",
        ".",
        "1e",
        "1e+",
        "--1",
        "1..2",
        " 1",
        "1 ",
        "0e999x",
    ] {
        compare(text);
    }
    compare(&format!("0.{}1e4097", "0".repeat(4096)));
    compare(&format!("1{}e-4096", "0".repeat(4096)));
    compare(&format!("-0.{}1e-4097", "0".repeat(4096)));
}

fn next(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}
#[test]
fn seeded_structured_decimals_match_rust_bits() {
    let mut state = 0x6c6b6a73u32;
    for _ in 0..512 {
        let length = 1 + next(&mut state) as usize % 200;
        let mut digits = String::new();
        for _ in 0..length {
            digits.push(char::from(b'0' + (next(&mut state) % 10) as u8));
        }
        let point = next(&mut state) as usize % (length + 1);
        let sign = ["", "+", "-"][next(&mut state) as usize % 3];
        let exponent = (next(&mut state) % 901) as i32 - 450;
        compare(&format!(
            "{sign}{}.{}e{exponent:+}",
            &digits[..point],
            &digits[point..]
        ));
    }
}

#[test]
fn production_decimal_conversion_executes_on_both_routes() {
    let cases = [
        "0",
        "-0",
        ".5",
        "1.e2",
        "1.00000000000000011102230246251565404236316680908203125",
        "1.00000000000000033306690738754696212708950042724609375",
        "2.4703282292062327e-324",
        "2.4703282292062328e-324",
        "2.2250738585072012e-308",
        "1.7976931348623157e308",
        "1.7976931348623159e308",
        "1e999999999999999999999999999999",
        "-1e-999999999999999999999999999999",
        "0e999x",
    ];
    let mut root = "mod floats; use floats::{decimal_bits, Parsed};\n".to_string();
    for (index, text) in cases.iter().enumerate() {
        let body = match rust_outcome(text) {
            Outcome::Finite(bits) => format!(
                "match decimal_bits({text:?}) {{ Parsed::Finite(bits) => if bits=={bits}u64 {{0i64}} else {{1i64}}, _=>2i64 }}"
            ),
            Outcome::Overflow => {
                format!("match decimal_bits({text:?}) {{ Parsed::Overflow=>0i64, _=>3i64 }}")
            }
            Outcome::Invalid => {
                format!("match decimal_bits({text:?}) {{ Parsed::Invalid=>0i64, _=>4i64 }}")
            }
        };
        root.push_str(&format!("pub fn compare_{index}()->i64 {{ {body} }}\n"));
    }
    let mut files = bootstrap_library::standard_library();
    files.push(SourceInput {
        path: "float_component.rs".to_string(),
        text: root,
    });
    files.push(SourceInput {
        path: "floats.rs".to_string(),
        text: include_str!("../src/elaborate/floats.rs").to_string(),
    });
    let checked = compiler::check_bundle(&SourceBundle {
        root: "float_component.rs".to_string(),
        library: Some("$std/lib.rs".to_string()),
        files,
    })
    .unwrap();
    let names: Vec<_> = (0..cases.len())
        .map(|index| format!("compare_{index}"))
        .collect();
    let expected: Vec<_> = names.iter().map(|name| (name.as_str(), 0i64)).collect();
    native_test::assert_values(&checked, &expected);
    for name in names {
        let index = checked
            .program()
            .functions
            .iter()
            .position(|function| function.name == name)
            .unwrap();
        let result = interpreter::run(checked.program(), index, &[], &mut Vec::new())
            .unwrap_or_else(|error| {
                panic!("{name}: {}:{}: {}", error.file, error.line, error.message)
            });
        match result.value {
            interpreter::Scalar::Int {
                value: 0,
                bits: 64,
                signed: true,
            } => {}
            _ => panic!("{name}: production decimal conversion disagreed with Rust"),
        }
    }
}
