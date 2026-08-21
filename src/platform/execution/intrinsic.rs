use super::{ExecutionError, ExecutionFailureClass};
use crate::platform::diagnostic::Diagnostic;
use crate::platform::json::{JsonLimits, decode_typed, encode_typed};
use crate::platform::package::PackageId;
use crate::platform::semantic::{FunctionSignature, ValidatedPackage};
use crate::platform::value::Value;
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn call_intrinsic(
    implementation: &str,
    signature: &FunctionSignature,
    arguments: Vec<Value>,
    packages: &BTreeMap<PackageId, ValidatedPackage>,
) -> Result<Value, ExecutionError> {
    match implementation {
        "core.i64.add" => binary_i64(arguments, i64::checked_add, "integer addition overflow"),
        "core.i64.subtract" => {
            binary_i64(arguments, i64::checked_sub, "integer subtraction overflow")
        }
        "core.i64.multiply" => binary_i64(
            arguments,
            i64::checked_mul,
            "integer multiplication overflow",
        ),
        "core.i64.divide" => {
            let (left, right) = i64_pair(arguments)?;
            left.checked_div(right).map(Value::I64).ok_or_else(|| {
                trap(
                    "integer_division",
                    "integer division by zero or signed overflow",
                )
            })
        }
        "core.i64.equal" => {
            let (left, right) = i64_pair(arguments)?;
            Ok(Value::Bool(left == right))
        }
        "core.i64.less" => {
            let (left, right) = i64_pair(arguments)?;
            Ok(Value::Bool(left < right))
        }
        "core.i64.less-equal" => {
            let (left, right) = i64_pair(arguments)?;
            Ok(Value::Bool(left <= right))
        }
        "core.i64.to-text" => {
            let [Value::I64(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::text(value.to_string()))
        }
        "core.i64.parse" => {
            let [Value::Text(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let spelling = value.as_ref();
            let digits = spelling.strip_prefix('-').unwrap_or(spelling);
            if spelling.is_empty()
                || digits.is_empty()
                || !digits.bytes().all(|byte| byte.is_ascii_digit())
                || (digits.len() > 1 && digits.starts_with('0'))
                || spelling == "-0"
            {
                return Err(trap(
                    "integer_parse",
                    "text is not a canonical signed 64-bit integer",
                ));
            }
            spelling.parse::<i64>().map(Value::I64).map_err(|_| {
                trap(
                    "integer_parse",
                    "text is outside the signed 64-bit integer range",
                )
            })
        }
        "core.i64.parse-result" => {
            let [Value::Text(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let parsed = parse_canonical_i64(value);
            Ok(Value::record(
                None,
                [
                    ("valid".to_owned(), Value::Bool(parsed.is_some())),
                    ("value".to_owned(), Value::I64(parsed.unwrap_or_default())),
                ],
            ))
        }
        "core.bool.not" => {
            let [Value::Bool(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::Bool(!value))
        }
        "core.bool.and" | "core.bool.or" => {
            let [Value::Bool(left), Value::Bool(right)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::Bool(if implementation == "core.bool.and" {
                *left && *right
            } else {
                *left || *right
            }))
        }
        "core.text.concat" => {
            let [Value::Text(left), Value::Text(right)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let length = left.len().checked_add(right.len()).ok_or_else(|| {
                ExecutionError::resource("text_length", "text concatenation length overflow")
            })?;
            let mut result = String::with_capacity(length);
            result.push_str(left);
            result.push_str(right);
            Ok(Value::Text(Arc::from(result)))
        }
        "core.text.equal" => {
            let [Value::Text(left), Value::Text(right)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::Bool(left == right))
        }
        "core.text.contains" => {
            let [Value::Text(value), Value::Text(needle)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::Bool(value.contains(needle.as_ref())))
        }
        "core.text.starts-with" => {
            let [Value::Text(value), Value::Text(prefix)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::Bool(value.starts_with(prefix.as_ref())))
        }
        "core.text.length" => {
            let [Value::Text(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::I64(length_i64(value.len())?))
        }
        "core.text.empty" => {
            let [Value::Text(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::Bool(value.is_empty()))
        }
        "core.text.from-static" => {
            let [Value::StaticText(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::text(value.clone()))
        }
        "core.html.escape-text" => {
            let [Value::Text(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let mut escaped = String::with_capacity(value.len());
            for character in value.chars() {
                match character {
                    '&' => escaped.push_str("&amp;"),
                    '<' => escaped.push_str("&lt;"),
                    '>' => escaped.push_str("&gt;"),
                    '"' => escaped.push_str("&quot;"),
                    '\'' => escaped.push_str("&#39;"),
                    _ => escaped.push(character),
                }
            }
            Ok(Value::text(escaped))
        }
        "core.json.string" => {
            let [Value::Text(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            serde_json::to_string(value.as_ref())
                .map(Value::text)
                .map_err(|_| {
                    ExecutionError::new(
                        ExecutionFailureClass::Infrastructure,
                        "json_string_encode",
                        "JSON string encoding failed",
                    )
                })
        }
        "core.json.encode" => {
            let [value] = arguments.as_slice() else {
                return Err(internal_type());
            };
            encode_typed(
                value,
                &signature.parameters[0],
                packages,
                JsonLimits::default(),
            )
            .map(Value::bytes)
            .map_err(json_encode_error)
        }
        "core.json.decode-or" => {
            let [Value::Bytes(bytes), fallback] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let decoded = decode_typed(
                bytes,
                &signature.parameters[1],
                packages,
                JsonLimits::default(),
            );
            let (valid, value, error) = match decoded {
                Ok(value) => (true, value, String::new()),
                Err(error) => (false, fallback.clone(), error.code),
            };
            Ok(Value::record(
                None,
                [
                    ("valid".to_owned(), Value::Bool(valid)),
                    ("value".to_owned(), value),
                    ("error".to_owned(), Value::text(error)),
                ],
            ))
        }
        "core.http.bearer-token" => {
            let [Value::List(headers)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let mut token = None;
            for header in headers.iter() {
                let Value::Record { fields, .. } = header else {
                    return Err(internal_type());
                };
                let (Some(Value::Text(name)), Some(Value::Bytes(value))) =
                    (fields.get("name"), fields.get("value"))
                else {
                    return Err(internal_type());
                };
                if !name.eq_ignore_ascii_case("authorization") {
                    continue;
                }
                if token.is_some() {
                    return Ok(Value::text(""));
                }
                let Ok(value) = std::str::from_utf8(value) else {
                    return Ok(Value::text(""));
                };
                let Some(value) = value.strip_prefix("Bearer ") else {
                    return Ok(Value::text(""));
                };
                if value.is_empty()
                    || value.len() > 512
                    || !value.bytes().all(|byte| byte.is_ascii_graphic())
                {
                    return Ok(Value::text(""));
                }
                token = Some(value.to_owned());
            }
            Ok(Value::text(token.unwrap_or_default()))
        }
        "core.bytes.from-text" => {
            let [Value::Text(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::Bytes(Arc::from(value.as_bytes())))
        }
        "core.bytes.to-text" => {
            let [Value::Bytes(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let text = std::str::from_utf8(value)
                .map_err(|_| trap("bytes_utf8", "bytes are not a valid UTF-8 text encoding"))?;
            Ok(Value::Text(Arc::from(text)))
        }
        "core.bytes.concat" => {
            let [Value::Bytes(left), Value::Bytes(right)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let length = left.len().checked_add(right.len()).ok_or_else(|| {
                ExecutionError::resource("bytes_length", "byte concatenation length overflow")
            })?;
            let mut result = Vec::with_capacity(length);
            result.extend_from_slice(left);
            result.extend_from_slice(right);
            Ok(Value::Bytes(Arc::from(result)))
        }
        "core.bytes.length" => {
            let [Value::Bytes(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::I64(length_i64(value.len())?))
        }
        "core.bytes.to-hex" => {
            let [Value::Bytes(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let mut output =
                String::with_capacity(value.len().checked_mul(2).ok_or_else(|| {
                    ExecutionError::resource("text_length", "hex output length overflowed")
                })?);
            const HEX: &[u8; 16] = b"0123456789abcdef";
            for byte in value.iter().copied() {
                output.push(char::from(HEX[usize::from(byte >> 4)]));
                output.push(char::from(HEX[usize::from(byte & 0x0f)]));
            }
            Ok(Value::text(output))
        }
        "core.bytes.equal" => {
            let [Value::Bytes(left), Value::Bytes(right)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::Bool(left == right))
        }
        "core.bytes.blake3" => {
            let [Value::Bytes(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::bytes(blake3::hash(value).as_bytes().to_vec()))
        }
        "core.value.equal" => {
            let [left, right] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::Bool(equal(left, right)?))
        }
        "core.list.length" => {
            let [Value::List(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::I64(length_i64(value.len())?))
        }
        "core.list.get" => {
            let [Value::List(values), Value::I64(index)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let index = usize::try_from(*index)
                .map_err(|_| trap("list_index", "list index is negative or excessive"))?;
            values
                .get(index)
                .cloned()
                .ok_or_else(|| trap("list_index", "list index is out of bounds"))
        }
        "core.list.append" => {
            let [Value::List(values), value] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let mut output = Vec::with_capacity(values.len().checked_add(1).ok_or_else(|| {
                ExecutionError::resource("list_length", "list length overflowed")
            })?);
            output.extend(values.iter().cloned());
            output.push(value.clone());
            Ok(Value::List(Arc::new(output)))
        }
        "core.map.length" => {
            let [Value::Map(value)] = arguments.as_slice() else {
                return Err(internal_type());
            };
            Ok(Value::I64(length_i64(value.len())?))
        }
        "core.map.get" => {
            let [Value::Map(values), key] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let key = crate::platform::value::MapKey::from_value(key.clone()).map_err(|_| {
                trap(
                    "map_key",
                    "map lookup key is not a deterministically ordered primitive",
                )
            })?;
            values
                .get(&key)
                .cloned()
                .ok_or_else(|| trap("map_key_absent", "map lookup key is absent"))
        }
        "core.map.contains" => {
            let [Value::Map(values), key] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let key = crate::platform::value::MapKey::from_value(key.clone()).map_err(|_| {
                trap(
                    "map_key",
                    "map membership key is not a deterministically ordered primitive",
                )
            })?;
            Ok(Value::Bool(values.contains_key(&key)))
        }
        "core.map.get-or" => {
            let [Value::Map(values), key, default] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let key = crate::platform::value::MapKey::from_value(key.clone()).map_err(|_| {
                trap(
                    "map_key",
                    "map lookup key is not a deterministically ordered primitive",
                )
            })?;
            Ok(values.get(&key).cloned().unwrap_or_else(|| default.clone()))
        }
        "core.map.insert" => {
            let [Value::Map(values), key, value] = arguments.as_slice() else {
                return Err(internal_type());
            };
            let key = crate::platform::value::MapKey::from_value(key.clone()).map_err(|_| {
                trap(
                    "map_key",
                    "map insertion key is not a deterministically ordered primitive",
                )
            })?;
            let mut output = values.as_ref().clone();
            output.insert(key, value.clone());
            Ok(Value::Map(Arc::new(output)))
        }
        _ => Err(ExecutionError::new(
            ExecutionFailureClass::Infrastructure,
            "intrinsic_missing",
            "prepared intrinsic implementation disappeared",
        )),
    }
}

fn parse_canonical_i64(value: &str) -> Option<i64> {
    let digits = value.strip_prefix('-').unwrap_or(value);
    if value.is_empty()
        || digits.is_empty()
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
        || value == "-0"
    {
        return None;
    }
    value.parse().ok()
}

fn json_encode_error(error: Diagnostic) -> ExecutionError {
    ExecutionError::new(
        if error.code == "json_output_too_large" {
            ExecutionFailureClass::Resource
        } else {
            ExecutionFailureClass::Infrastructure
        },
        error.code,
        "typed JSON encoding failed",
    )
}

fn binary_i64(
    arguments: Vec<Value>,
    operation: fn(i64, i64) -> Option<i64>,
    message: &'static str,
) -> Result<Value, ExecutionError> {
    let (left, right) = i64_pair(arguments)?;
    operation(left, right)
        .map(Value::I64)
        .ok_or_else(|| trap("integer_overflow", message))
}

fn i64_pair(arguments: Vec<Value>) -> Result<(i64, i64), ExecutionError> {
    let [Value::I64(left), Value::I64(right)] = arguments.as_slice() else {
        return Err(internal_type());
    };
    Ok((*left, *right))
}

fn length_i64(length: usize) -> Result<i64, ExecutionError> {
    i64::try_from(length).map_err(|_| {
        ExecutionError::resource("value_length", "value length exceeds signed 64-bit range")
    })
}

fn equal(left: &Value, right: &Value) -> Result<bool, ExecutionError> {
    match (left, right) {
        (Value::Unit, Value::Unit) => Ok(true),
        (Value::Bool(left), Value::Bool(right)) => Ok(left == right),
        (Value::I64(left), Value::I64(right)) => Ok(left == right),
        (Value::Bytes(left), Value::Bytes(right)) => Ok(left == right),
        (Value::Text(left), Value::Text(right)) => Ok(left == right),
        (Value::StaticText(left), Value::StaticText(right)) => Ok(left == right),
        (
            Value::Record {
                owner: left_owner,
                fields: left,
            },
            Value::Record {
                owner: right_owner,
                fields: right,
            },
        ) => {
            if left_owner != right_owner || left.len() != right.len() {
                return Ok(false);
            }
            for (name, left) in left {
                let Some(right) = right.get(name) else {
                    return Ok(false);
                };
                if !equal(left, right)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (
            Value::Variant {
                owner: left_owner,
                case: left_case,
                payload: left,
            },
            Value::Variant {
                owner: right_owner,
                case: right_case,
                payload: right,
            },
        ) => {
            if left_owner != right_owner || left_case != right_case {
                return Ok(false);
            }
            match (left, right) {
                (None, None) => Ok(true),
                (Some(left), Some(right)) => equal(left, right),
                _ => Ok(false),
            }
        }
        (Value::List(left), Value::List(right)) => {
            if left.len() != right.len() {
                return Ok(false);
            }
            for (left, right) in left.iter().zip(right.iter()) {
                if !equal(left, right)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (Value::Map(left), Value::Map(right)) => {
            if left.len() != right.len() {
                return Ok(false);
            }
            for (key, left) in left.iter() {
                let Some(right) = right.get(key) else {
                    return Ok(false);
                };
                if !equal(left, right)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (Value::Function(_), _) | (Value::Resource { .. }, _) => Err(trap(
            "value_not_comparable",
            "functions and live resources do not support semantic equality",
        )),
        _ => Ok(false),
    }
}

fn internal_type() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "intrinsic_argument_type",
        "validated intrinsic received a foreign runtime value",
    )
}

fn trap(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Trap, code, message)
}
