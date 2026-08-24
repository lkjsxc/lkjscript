//! Strict bounded JSON and exact conversion at typed component boundaries.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::package::PackageId;
use super::semantic::{NominalShape, OwnerId, ResolvedType, ValidatedPackage};
use super::value::{MapKey, Value};
use base64::Engine;
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::sync::Arc;

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

pub fn decode_typed(
    bytes: &[u8],
    ty: &ResolvedType,
    packages: &BTreeMap<PackageId, ValidatedPackage>,
    limits: JsonLimits,
) -> Result<Value, Diagnostic> {
    let value = decode_strict(bytes, limits)?;
    from_json(&value, ty, packages, "$", 0)
}

pub fn encode_typed(
    value: &Value,
    ty: &ResolvedType,
    packages: &BTreeMap<PackageId, ValidatedPackage>,
    limits: JsonLimits,
) -> Result<Vec<u8>, Diagnostic> {
    let json = to_json(value, ty, packages, "$", 0)?;
    let bytes = serde_json::to_vec(&json)
        .map_err(|error| json_error("json_encode", format!("JSON encoding failed: {error}")))?;
    if bytes.len() > limits.maximum_bytes {
        return Err(json_error(
            "json_output_too_large",
            format!(
                "encoded JSON has {} bytes; the limit is {}",
                bytes.len(),
                limits.maximum_bytes
            ),
        ));
    }
    Ok(bytes)
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

fn from_json(
    value: &serde_json::Value,
    ty: &ResolvedType,
    packages: &BTreeMap<PackageId, ValidatedPackage>,
    path: &str,
    depth: usize,
) -> Result<Value, Diagnostic> {
    if depth > 256 {
        return Err(typed_error(path, "typed JSON value exceeds depth 256"));
    }
    match ty {
        ResolvedType::Unit if value.is_null() => Ok(Value::Unit),
        ResolvedType::Bool => value
            .as_bool()
            .map(Value::Bool)
            .ok_or_else(|| typed_error(path, "expected boolean")),
        ResolvedType::I64 => value
            .as_i64()
            .map(Value::I64)
            .ok_or_else(|| typed_error(path, "expected signed 64-bit integer")),
        ResolvedType::Text => value
            .as_str()
            .map(|value| Value::Text(Arc::from(value)))
            .ok_or_else(|| typed_error(path, "expected text string")),
        ResolvedType::StaticText => Err(typed_error(
            path,
            "static text must originate in accepted source and cannot be decoded from input JSON",
        )),
        ResolvedType::Bytes => decode_bytes(value, path),
        ResolvedType::Record(fields) => decode_record(value, None, fields, packages, path, depth),
        ResolvedType::Nominal(owner) => match nominal_shape(packages, owner)? {
            NominalShape::Record(fields) => {
                decode_record(value, Some(owner.clone()), fields, packages, path, depth)
            }
            NominalShape::Variant(cases) => {
                decode_variant(value, owner, cases, packages, path, depth)
            }
        },
        ResolvedType::List(item) => {
            let items = value
                .as_array()
                .ok_or_else(|| typed_error(path, "expected JSON array"))?;
            let values = items
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    from_json(
                        value,
                        item,
                        packages,
                        &format!("{path}[{index}]"),
                        depth + 1,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::List(Arc::new(values)))
        }
        ResolvedType::Map(key, item) => {
            let entries = value
                .as_array()
                .ok_or_else(|| typed_error(path, "expected map entry array"))?;
            let mut output = BTreeMap::new();
            for (index, entry) in entries.iter().enumerate() {
                let pair = entry
                    .as_array()
                    .filter(|pair| pair.len() == 2)
                    .ok_or_else(|| typed_error(path, "map entry must be [key, value]"))?;
                let key_value = from_json(
                    &pair[0],
                    key,
                    packages,
                    &format!("{path}[{index}][0]"),
                    depth + 1,
                )?;
                let key = MapKey::from_value(key_value)
                    .map_err(|error| typed_error(path, error.message))?;
                let item = from_json(
                    &pair[1],
                    item,
                    packages,
                    &format!("{path}[{index}][1]"),
                    depth + 1,
                )?;
                if output.insert(key, item).is_some() {
                    return Err(typed_error(path, "map contains a duplicate key"));
                }
            }
            Ok(Value::Map(Arc::new(output)))
        }
        ResolvedType::Secret | ResolvedType::Stream(_) | ResolvedType::Function(_, _) => Err(
            typed_error(path, "live or callable values cannot be decoded from JSON"),
        ),
        ResolvedType::Option(_) | ResolvedType::Result(_, _) => Err(typed_error(
            path,
            "built-in Option and Result need a nominal boundary codec in contract 1",
        )),
        _ => Err(typed_error(
            path,
            format!("JSON value does not match {ty:?}"),
        )),
    }
}

fn to_json(
    value: &Value,
    ty: &ResolvedType,
    packages: &BTreeMap<PackageId, ValidatedPackage>,
    path: &str,
    depth: usize,
) -> Result<serde_json::Value, Diagnostic> {
    if depth > 256 {
        return Err(typed_error(path, "typed value exceeds depth 256"));
    }
    match (value, ty) {
        (Value::Unit, ResolvedType::Unit) => Ok(serde_json::Value::Null),
        (Value::Bool(value), ResolvedType::Bool) => Ok(serde_json::Value::Bool(*value)),
        (Value::I64(value), ResolvedType::I64) => Ok(serde_json::Value::from(*value)),
        (Value::Text(value), ResolvedType::Text) => {
            Ok(serde_json::Value::String(value.to_string()))
        }
        (Value::StaticText(value), ResolvedType::StaticText) => {
            Ok(serde_json::Value::String(value.to_string()))
        }
        (Value::Bytes(value), ResolvedType::Bytes) => Ok(serde_json::json!({
            "$bytes": base64::engine::general_purpose::STANDARD.encode(value),
        })),
        (Value::Record { owner, fields }, ResolvedType::Record(expected)) if owner.is_none() => {
            encode_record(fields, expected, packages, path, depth)
        }
        (
            Value::Record {
                owner: Some(actual),
                fields,
            },
            ResolvedType::Nominal(expected),
        ) if actual == expected => {
            let NominalShape::Record(shape) = nominal_shape(packages, expected)? else {
                return Err(typed_error(path, "nominal variant represented as record"));
            };
            encode_record(fields, shape, packages, path, depth)
        }
        (
            Value::Variant {
                owner: actual,
                case,
                payload,
            },
            ResolvedType::Nominal(expected),
        ) if actual == expected => {
            let NominalShape::Variant(cases) = nominal_shape(packages, expected)? else {
                return Err(typed_error(path, "nominal record represented as variant"));
            };
            let expected_payload = cases
                .get(case)
                .ok_or_else(|| typed_error(path, "variant case is absent from its type"))?;
            let mut object = Map::new();
            object.insert("case".to_owned(), serde_json::Value::String(case.clone()));
            match (payload, expected_payload) {
                (None, None) => {}
                (Some(value), Some(ty)) => {
                    object.insert(
                        "value".to_owned(),
                        to_json(value, ty, packages, &format!("{path}.value"), depth + 1)?,
                    );
                }
                _ => return Err(typed_error(path, "variant payload does not match its type")),
            }
            Ok(serde_json::Value::Object(object))
        }
        (Value::List(values), ResolvedType::List(item)) => Ok(serde_json::Value::Array(
            values
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    to_json(
                        value,
                        item,
                        packages,
                        &format!("{path}[{index}]"),
                        depth + 1,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
        (Value::Map(values), ResolvedType::Map(key, item)) => Ok(serde_json::Value::Array(
            values
                .iter()
                .enumerate()
                .map(|(index, (map_key, value))| {
                    Ok(serde_json::Value::Array(vec![
                        to_json(
                            &map_key.to_value(),
                            key,
                            packages,
                            &format!("{path}[{index}][0]"),
                            depth + 1,
                        )?,
                        to_json(
                            value,
                            item,
                            packages,
                            &format!("{path}[{index}][1]"),
                            depth + 1,
                        )?,
                    ]))
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
        )),
        _ => Err(typed_error(
            path,
            format!("runtime value {value:?} does not match {ty:?}"),
        )),
    }
}

fn decode_record(
    value: &serde_json::Value,
    owner: Option<OwnerId>,
    fields: &[super::semantic::ResolvedField],
    packages: &BTreeMap<PackageId, ValidatedPackage>,
    path: &str,
    depth: usize,
) -> Result<Value, Diagnostic> {
    let object = value
        .as_object()
        .ok_or_else(|| typed_error(path, "expected JSON object"))?;
    let expected: BTreeSet<_> = fields.iter().map(|field| field.name.as_str()).collect();
    let actual: BTreeSet<_> = object.keys().map(String::as_str).collect();
    if expected != actual {
        let unknown = actual.difference(&expected).copied().collect::<Vec<_>>();
        let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
        return Err(typed_error(
            path,
            format!("record fields differ; unknown={unknown:?}, missing={missing:?}"),
        ));
    }
    let mut output = BTreeMap::new();
    for field in fields {
        let field_value = object.get(&field.name).ok_or_else(|| {
            typed_error(path, format!("record field '{}' disappeared", field.name))
        })?;
        output.insert(
            field.name.clone(),
            from_json(
                field_value,
                &field.ty,
                packages,
                &format!("{path}.{}", field.name),
                depth + 1,
            )?,
        );
    }
    Ok(Value::Record {
        owner,
        fields: output,
    })
}

fn encode_record(
    values: &BTreeMap<String, Value>,
    fields: &[super::semantic::ResolvedField],
    packages: &BTreeMap<PackageId, ValidatedPackage>,
    path: &str,
    depth: usize,
) -> Result<serde_json::Value, Diagnostic> {
    if values.len() != fields.len() {
        return Err(typed_error(
            path,
            "record field count differs from its type",
        ));
    }
    let mut object = Map::new();
    for field in fields {
        let value = values
            .get(&field.name)
            .ok_or_else(|| typed_error(path, format!("record omits field '{}'", field.name)))?;
        object.insert(
            field.name.clone(),
            to_json(
                value,
                &field.ty,
                packages,
                &format!("{path}.{}", field.name),
                depth + 1,
            )?,
        );
    }
    Ok(serde_json::Value::Object(object))
}

fn decode_variant(
    value: &serde_json::Value,
    owner: &OwnerId,
    cases: &BTreeMap<String, Option<ResolvedType>>,
    packages: &BTreeMap<PackageId, ValidatedPackage>,
    path: &str,
    depth: usize,
) -> Result<Value, Diagnostic> {
    let object = value
        .as_object()
        .ok_or_else(|| typed_error(path, "expected variant object"))?;
    if !object.keys().all(|key| key == "case" || key == "value") {
        return Err(typed_error(path, "variant object has an unknown field"));
    }
    let case = object
        .get("case")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| typed_error(path, "variant requires text field 'case'"))?;
    let expected = cases
        .get(case)
        .ok_or_else(|| typed_error(path, format!("variant has no case '{case}'")))?;
    let payload = match (expected, object.get("value")) {
        (None, None) => None,
        (Some(ty), Some(value)) => Some(from_json(
            value,
            ty,
            packages,
            &format!("{path}.value"),
            depth + 1,
        )?),
        (None, Some(_)) => return Err(typed_error(path, "payload-free case has field 'value'")),
        (Some(_), None) => return Err(typed_error(path, "payload case omits field 'value'")),
    };
    Ok(Value::variant(owner.clone(), case, payload))
}

fn decode_bytes(value: &serde_json::Value, path: &str) -> Result<Value, Diagnostic> {
    let object = value
        .as_object()
        .filter(|object| object.len() == 1)
        .ok_or_else(|| typed_error(path, "bytes require exactly {'$bytes': string}"))?;
    let encoded = object
        .get("$bytes")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| typed_error(path, "bytes require text field '$bytes'"))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| typed_error(path, "'$bytes' is not canonical base64"))?;
    if base64::engine::general_purpose::STANDARD.encode(&bytes) != encoded {
        return Err(typed_error(path, "'$bytes' is not canonical padded base64"));
    }
    Ok(Value::Bytes(Arc::from(bytes)))
}

fn nominal_shape<'a>(
    packages: &'a BTreeMap<PackageId, ValidatedPackage>,
    owner: &OwnerId,
) -> Result<&'a NominalShape, Diagnostic> {
    packages
        .get(&owner.package)
        .and_then(|package| package.nominal_shapes.get(owner))
        .ok_or_else(|| {
            typed_error(
                "$",
                format!("nominal owner '{}' is unavailable", owner.diagnostic_name()),
            )
        })
}

fn typed_error(path: &str, message: impl Into<String>) -> Diagnostic {
    json_error("json_type", format!("{path}: {}", message.into()))
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
            "the existing signed application boundary must remain unchanged"
        );
    }

    #[test]
    fn primitive_and_map_round_trip_is_deterministic() {
        let packages = BTreeMap::new();
        let ty = ResolvedType::Map(Box::new(ResolvedType::Text), Box::new(ResolvedType::I64));
        let value = decode_typed(
            br#"[["z",2],["a",1]]"#,
            &ty,
            &packages,
            JsonLimits::default(),
        )
        .expect("decode map");
        let bytes =
            encode_typed(&value, &ty, &packages, JsonLimits::default()).expect("encode map");
        assert_eq!(bytes, br#"[["a",1],["z",2]]"#);
    }
}
