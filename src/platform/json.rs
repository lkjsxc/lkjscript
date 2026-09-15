//! Strict bounded JSON admission shared by public component boundaries.

use super::binary64::{Binary64, is_decimal};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number};
use std::fmt;

pub const JSON_CONTRACT_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonIntegerPolicy {
    SignedI64,
    SignedOrUnsigned64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JsonNumericPolicy {
    Integers(JsonIntegerPolicy),
    Application,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JsonLimits {
    pub maximum_bytes: usize,
    pub maximum_depth: usize,
    pub maximum_items: usize,
    pub maximum_string_bytes: usize,
}

impl Default for JsonLimits {
    fn default() -> Self {
        Self {
            maximum_bytes: 1_048_576,
            maximum_depth: 128,
            maximum_items: 100_000,
            maximum_string_bytes: 1_048_576,
        }
    }
}

pub fn decode_strict(bytes: &[u8], limits: JsonLimits) -> Result<serde_json::Value, Diagnostic> {
    decode_strict_with_integer_policy(bytes, limits, JsonIntegerPolicy::SignedI64)
}

pub fn decode_strict_with_integer_policy(
    bytes: &[u8],
    limits: JsonLimits,
    integer_policy: JsonIntegerPolicy,
) -> Result<serde_json::Value, Diagnostic> {
    decode_strict_with_numeric_policy(bytes, limits, JsonNumericPolicy::Integers(integer_policy))
}

/// Typed application boundaries admit finite binary64 numbers without changing integer schemas.
pub fn decode_application(
    bytes: &[u8],
    limits: JsonLimits,
) -> Result<serde_json::Value, Diagnostic> {
    decode_strict_with_numeric_policy(bytes, limits, JsonNumericPolicy::Application)
}

pub fn decode_strict_with_numeric_policy(
    bytes: &[u8],
    limits: JsonLimits,
    numeric_policy: JsonNumericPolicy,
) -> Result<serde_json::Value, Diagnostic> {
    if bytes.len() > limits.maximum_bytes {
        return Err(json_error(
            "json_too_large",
            format!(
                "JSON has {} bytes; the limit is {}",
                bytes.len(),
                limits.maximum_bytes
            ),
        ));
    }
    let (masked, numbers) = match numeric_policy {
        JsonNumericPolicy::Application => {
            let (masked, numbers) = prepare_application_numbers(bytes, limits)?;
            (Some(masked), Some(numbers.into_iter()))
        }
        JsonNumericPolicy::Integers(_) => (None, None),
    };
    let mut deserializer = serde_json::Deserializer::from_slice(masked.as_deref().unwrap_or(bytes));
    let mut state = DecodeState {
        limits,
        numeric_policy,
        numbers,
        items: 0,
    };
    let value = StrictSeed {
        state: &mut state,
        depth: 0,
    }
    .deserialize(&mut deserializer)
    .map_err(|error| json_error("json_decode", format!("strict JSON rejected: {error}")))?;
    deserializer.end().map_err(|error| {
        json_error("json_trailing", format!("JSON has trailing input: {error}"))
    })?;
    if state.numbers.is_some_and(|numbers| numbers.len() != 0) {
        return Err(json_error(
            "json_decode",
            "application JSON number inventory was not completely consumed",
        ));
    }
    Ok(value)
}

/// Locate numeric spans outside strings, leaving the complete JSON grammar to serde's visitor.
/// Each validated number becomes an equal-length `0` plus spaces, so the visitor can recover
/// its original scalar representation without serde_json first rounding or erasing its sign.
fn prepare_application_numbers(
    bytes: &[u8],
    limits: JsonLimits,
) -> Result<(Vec<u8>, Vec<Number>), Diagnostic> {
    let mut masked = bytes.to_vec();
    let mut numbers = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                index += 1;
                while index < bytes.len() {
                    let byte = bytes[index];
                    index += 1;
                    if byte == b'"' {
                        break;
                    }
                    if byte == b'\\' && index < bytes.len() {
                        index += 1;
                    }
                }
            }
            b'-' | b'0'..=b'9' => {
                let start = index;
                while index < bytes.len()
                    && matches!(bytes[index], b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
                {
                    index += 1;
                }
                // The root scalar is not an item in the maintained JSON container accounting.
                if numbers.len() > limits.maximum_items {
                    return Err(json_error("json_decode", "JSON item count limit exceeded"));
                }
                let token = std::str::from_utf8(&bytes[start..index]).map_err(|_| {
                    json_error("json_decode", "JSON number is not an ASCII decimal token")
                })?;
                numbers.push(application_number(token)?);
                masked[start] = b'0';
                masked[start + 1..index].fill(b' ');
            }
            _ => index += 1,
        }
    }
    Ok((masked, numbers))
}

fn application_number(token: &str) -> Result<Number, Diagnostic> {
    if !is_decimal(token) {
        return Err(json_error(
            "json_decode",
            "JSON number has invalid decimal syntax",
        ));
    }
    // Preserve integer syntax and all integer bits. In particular, neither 1.0 nor 1e0
    // becomes an integer, and -0 remains a signed floating zero for typed F64 admission.
    if token != "-0" && !token.bytes().any(|byte| matches!(byte, b'.' | b'e' | b'E')) {
        if let Ok(value) = token.parse::<i64>() {
            return Ok(Number::from(value));
        }
        if let Ok(value) = token.parse::<u64>() {
            return Ok(Number::from(value));
        }
    }
    Binary64::parse(token)
        .and_then(|value| Number::from_f64(value.to_float()))
        .ok_or_else(|| json_error("json_decode", "JSON decimal exceeds finite binary64 range"))
}

struct DecodeState {
    limits: JsonLimits,
    numeric_policy: JsonNumericPolicy,
    numbers: Option<std::vec::IntoIter<Number>>,
    items: usize,
}

struct StrictSeed<'a> {
    state: &'a mut DecodeState,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for StrictSeed<'_> {
    type Value = serde_json::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        if self.depth > self.state.limits.maximum_depth {
            return Err(D::Error::custom("JSON nesting depth limit exceeded"));
        }
        deserializer.deserialize_any(StrictVisitor {
            state: self.state,
            depth: self.depth,
        })
    }
}

struct StrictVisitor<'a> {
    state: &'a mut DecodeState,
    depth: usize,
}

impl<'de> Visitor<'de> for StrictVisitor<'_> {
    type Value = serde_json::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded JSON value with signed 64-bit integers")
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Null)
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if let Some(numbers) = &mut self.state.numbers {
            return take_application_number(value == 0, numbers);
        }
        Ok(serde_json::Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if let Some(numbers) = &mut self.state.numbers {
            return take_application_number(value == 0, numbers);
        }
        match self.state.numeric_policy {
            JsonNumericPolicy::Integers(JsonIntegerPolicy::SignedI64) => {
                let value = i64::try_from(value)
                    .map_err(|_| E::custom("JSON integer exceeds signed 64-bit range"))?;
                self.visit_i64(value)
            }
            JsonNumericPolicy::Integers(JsonIntegerPolicy::SignedOrUnsigned64) => {
                Ok(serde_json::Value::Number(Number::from(value)))
            }
            JsonNumericPolicy::Application => {
                Err(E::custom("application JSON number inventory is absent"))
            }
        }
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Err(E::custom("floating-point JSON numbers are not accepted"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if value.len() > self.state.limits.maximum_string_bytes {
            return Err(E::custom("JSON string byte limit exceeded"));
        }
        Ok(serde_json::Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_str(&value)
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut output = Vec::new();
        while let Some(value) = sequence.next_element_seed(StrictSeed {
            state: self.state,
            depth: self.depth + 1,
        })? {
            self.state.items = self
                .state
                .items
                .checked_add(1)
                .ok_or_else(|| A::Error::custom("JSON item count limit exceeded"))?;
            if self.state.items > self.state.limits.maximum_items {
                return Err(A::Error::custom("JSON item count limit exceeded"));
            }
            output.push(value);
        }
        Ok(serde_json::Value::Array(output))
    }

    fn visit_map<A>(self, mut input: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut output = Map::new();
        while let Some(key) = input.next_key::<String>()? {
            if key.len() > self.state.limits.maximum_string_bytes {
                return Err(A::Error::custom("JSON object key byte limit exceeded"));
            }
            if output.contains_key(&key) {
                return Err(A::Error::custom(format!(
                    "duplicate JSON object field '{key}'"
                )));
            }
            let value = input.next_value_seed(StrictSeed {
                state: self.state,
                depth: self.depth + 1,
            })?;
            self.state.items = self
                .state
                .items
                .checked_add(1)
                .ok_or_else(|| A::Error::custom("JSON item count limit exceeded"))?;
            if self.state.items > self.state.limits.maximum_items {
                return Err(A::Error::custom("JSON item count limit exceeded"));
            }
            output.insert(key, value);
        }
        Ok(serde_json::Value::Object(output))
    }
}

fn take_application_number<E: serde::de::Error>(
    is_placeholder: bool,
    numbers: &mut std::vec::IntoIter<Number>,
) -> Result<serde_json::Value, E> {
    if !is_placeholder {
        return Err(E::custom(
            "application JSON contains an unprepared numeric token",
        ));
    }
    numbers
        .next()
        .map(serde_json::Value::Number)
        .ok_or_else(|| E::custom("application JSON number inventory is exhausted"))
}

fn json_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_float_range_depth_and_trailing_reject() {
        for input in [
            br#"{"a":1,"a":2}"#.as_slice(),
            br#"1.5"#.as_slice(),
            br#"18446744073709551615"#.as_slice(),
            br#"true false"#.as_slice(),
        ] {
            assert!(decode_strict(input, JsonLimits::default()).is_err());
        }
        let error = decode_strict(
            br#"[[[0]]]"#,
            JsonLimits {
                maximum_depth: 1,
                ..JsonLimits::default()
            },
        )
        .expect_err("depth rejects");
        assert_eq!(error.code, "json_decode");
    }

    #[test]
    fn protocol_integer_policy_accepts_the_complete_unsigned_64_bit_domain() {
        let value = decode_strict_with_integer_policy(
            br#"18446744073709551615"#,
            JsonLimits::default(),
            JsonIntegerPolicy::SignedOrUnsigned64,
        )
        .expect("unsigned protocol integer");
        assert_eq!(value.as_u64(), Some(u64::MAX));
        assert!(
            decode_strict(br#"18446744073709551615"#, JsonLimits::default()).is_err(),
            "the signed application boundary remains unchanged"
        );
    }

    #[test]
    fn application_numbers_preserve_integer_syntax_bits_and_zero_signs() {
        for (token, expected) in [
            ("-9223372036854775808", i64::MIN),
            ("9223372036854775807", i64::MAX),
            ("9007199254740993", 9_007_199_254_740_993),
        ] {
            let value = decode_application(token.as_bytes(), JsonLimits::default()).unwrap();
            assert_eq!(value.as_i64(), Some(expected), "{token}");
            assert_eq!(
                decode_strict(token.as_bytes(), JsonLimits::default()).unwrap(),
                value
            );
        }
        for (token, bits) in [
            ("-0", 0x8000_0000_0000_0000u64),
            ("-0.0", 0x8000_0000_0000_0000),
            ("-0e20", 0x8000_0000_0000_0000),
            ("1.0", 0x3ff0_0000_0000_0000),
            ("1e0", 0x3ff0_0000_0000_0000),
            ("18446744073709551616", 0x43f0_0000_0000_0000),
            ("18446744073709553664", 0x43f0_0000_0000_0000),
            ("18446744073709553665", 0x43f0_0000_0000_0001),
            ("5e-324", 1),
            ("-1e-324", 0x8000_0000_0000_0000),
        ] {
            let value = decode_application(token.as_bytes(), JsonLimits::default()).unwrap();
            assert_eq!(value.as_f64().unwrap().to_bits(), bits, "{token}");
            assert_eq!(value.as_i64(), None, "{token}");
            assert!(
                decode_strict(token.as_bytes(), JsonLimits::default()).is_err(),
                "{token}"
            );
            assert!(
                decode_strict_with_integer_policy(
                    token.as_bytes(),
                    JsonLimits::default(),
                    JsonIntegerPolicy::SignedOrUnsigned64,
                )
                .is_err(),
                "{token}"
            );
        }
        let unsigned = decode_application(b"18446744073709551615", JsonLimits::default()).unwrap();
        assert_eq!(unsigned.as_u64(), Some(u64::MAX));
        assert_eq!(unsigned.as_i64(), None);
    }

    #[test]
    fn application_decimal_rounding_retains_long_exact_midpoints() {
        // Exactly 1 + 2^-53: nearest-even selects 1, including a long integer-form spelling.
        let midpoint = "1.00000000000000011102230246251565404236316680908203125";
        let long = format!("{}{}e-853", midpoint.replace('.', ""), "0".repeat(800));
        for token in [midpoint, long.as_str()] {
            let value = decode_application(token.as_bytes(), JsonLimits::default()).unwrap();
            assert_eq!(value.as_f64().unwrap().to_bits(), 0x3ff0_0000_0000_0000);
        }
        // A last-place decimal increment lies above that exact midpoint.
        let above = b"1.00000000000000011102230246251565404236316680908203126";
        let value = decode_application(above, JsonLimits::default()).unwrap();
        assert_eq!(value.as_f64().unwrap().to_bits(), 0x3ff0_0000_0000_0001);
    }

    #[test]
    fn application_number_masking_preserves_strings_structure_and_strict_failures() {
        let value = decode_application(
            br#"{"z":-0,"1e400":"1e400 \\\" -0","a":[1.25,{"v":2e1}]}"#,
            JsonLimits::default(),
        )
        .unwrap();
        assert_eq!(
            value["z"].as_f64().unwrap().to_bits(),
            0x8000_0000_0000_0000
        );
        assert_eq!(value["1e400"].as_str(), Some("1e400 \\\" -0"));
        assert_eq!(value["a"][0].as_f64(), Some(1.25));
        assert_eq!(value["a"][1]["v"].as_f64(), Some(20.0));
        for input in [
            "1e400",
            "-1e400",
            "01",
            "+1",
            "1.",
            ".1",
            "1e",
            "1e+",
            "1_0",
            "0x1",
            "nan",
            "inf",
            "-inf",
            "[1e0 2]",
            "[1e0true]",
            "[true1e0]",
            "[1e0,]",
            "{1e0:2}",
            "{\"a\":1.5,\"a\":2.5}",
            "{\"a\":1.5,\"\\u0061\":2.5}",
            "1.5 false",
            "\"unterminated 1e0",
            "\"invalid \\q 1e0\"",
        ] {
            assert!(
                decode_application(input.as_bytes(), JsonLimits::default()).is_err(),
                "{input}"
            );
        }
        assert!(decode_application(b"[\"\xff\",1.5]", JsonLimits::default()).is_err());
    }

    #[test]
    fn application_numbers_retain_all_existing_admission_limits() {
        for (input, limits) in [
            (
                b"1.5".as_slice(),
                JsonLimits {
                    maximum_bytes: 2,
                    ..JsonLimits::default()
                },
            ),
            (
                b"[[1.5]]".as_slice(),
                JsonLimits {
                    maximum_depth: 1,
                    ..JsonLimits::default()
                },
            ),
            (
                b"[1.5,2.5]".as_slice(),
                JsonLimits {
                    maximum_items: 1,
                    ..JsonLimits::default()
                },
            ),
            (
                b"[1.5,\"abc\"]".as_slice(),
                JsonLimits {
                    maximum_string_bytes: 2,
                    ..JsonLimits::default()
                },
            ),
            (
                b"{\"abc\":1.5}".as_slice(),
                JsonLimits {
                    maximum_string_bytes: 2,
                    ..JsonLimits::default()
                },
            ),
        ] {
            assert!(decode_application(input, limits).is_err());
        }
        assert!(
            decode_application(
                b"1.5",
                JsonLimits {
                    maximum_items: 0,
                    ..JsonLimits::default()
                }
            )
            .is_ok()
        );
        assert!(
            decode_application(
                b"[1.5]",
                JsonLimits {
                    maximum_items: 0,
                    ..JsonLimits::default()
                }
            )
            .is_err()
        );
    }
}
