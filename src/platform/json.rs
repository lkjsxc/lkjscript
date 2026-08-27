//! Strict bounded JSON admission shared by public component boundaries.

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
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let mut state = DecodeState {
        limits,
        integer_policy,
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
    Ok(value)
}

struct DecodeState {
    limits: JsonLimits,
    integer_policy: JsonIntegerPolicy,
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

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(serde_json::Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        match self.state.integer_policy {
            JsonIntegerPolicy::SignedI64 => {
                let value = i64::try_from(value)
                    .map_err(|_| E::custom("JSON integer exceeds signed 64-bit range"))?;
                self.visit_i64(value)
            }
            JsonIntegerPolicy::SignedOrUnsigned64 => {
                Ok(serde_json::Value::Number(Number::from(value)))
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
            self.state.items = self.state.items.saturating_add(1);
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
            self.state.items = self.state.items.saturating_add(1);
            if self.state.items > self.state.limits.maximum_items {
                return Err(A::Error::custom("JSON item count limit exceeded"));
            }
            output.insert(key, value);
        }
        Ok(serde_json::Value::Object(output))
    }
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
}
