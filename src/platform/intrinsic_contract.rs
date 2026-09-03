//! Closed semantic contracts for trusted, pure native intrinsics.
//!
//! This module owns registration and signature validation. Runtime implementations live in the
//! execution layer, but accepted source is rejected here before it can become authority.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::semantic::{FunctionSignature, ResolvedField, ResolvedType};

pub fn validate_intrinsic(
    implementation: &str,
    signature: &FunctionSignature,
) -> Result<(), Diagnostic> {
    let valid = match implementation {
        "core.i64.add" | "core.i64.subtract" | "core.i64.multiply" | "core.i64.divide" => exact(
            signature,
            &[ResolvedType::I64, ResolvedType::I64],
            &ResolvedType::I64,
        ),
        "core.i64.equal" | "core.i64.less" | "core.i64.less-equal" => exact(
            signature,
            &[ResolvedType::I64, ResolvedType::I64],
            &ResolvedType::Bool,
        ),
        "core.i64.to-text" => exact(signature, &[ResolvedType::I64], &ResolvedType::Text),
        "core.i64.parse" => exact(signature, &[ResolvedType::Text], &ResolvedType::I64),
        "core.i64.parse-result" => {
            exact(signature, &[ResolvedType::Text], &parse_i64_result_type())
        }
        "core.bool.not" => exact(signature, &[ResolvedType::Bool], &ResolvedType::Bool),
        "core.bool.and" | "core.bool.or" => exact(
            signature,
            &[ResolvedType::Bool, ResolvedType::Bool],
            &ResolvedType::Bool,
        ),
        "core.text.concat" => exact(
            signature,
            &[ResolvedType::Text, ResolvedType::Text],
            &ResolvedType::Text,
        ),
        "core.text.equal" | "core.text.contains" | "core.text.starts-with" => exact(
            signature,
            &[ResolvedType::Text, ResolvedType::Text],
            &ResolvedType::Bool,
        ),
        "core.text.length" => exact(signature, &[ResolvedType::Text], &ResolvedType::I64),
        "core.text.empty" => exact(signature, &[ResolvedType::Text], &ResolvedType::Bool),
        "core.text.from-static" => {
            exact(signature, &[ResolvedType::StaticText], &ResolvedType::Text)
        }
        "core.html.escape-text" | "core.json.string" => {
            exact(signature, &[ResolvedType::Text], &ResolvedType::Text)
        }
        "core.json.encode" => {
            matches!(signature.parameters.as_slice(), [value] if value.is_durable())
                && signature.result == ResolvedType::Bytes
        }
        "core.json.decode-or" => match signature.parameters.as_slice() {
            [ResolvedType::Bytes, fallback] => {
                json_decodable(fallback)
                    && signature.result
                        == ResolvedType::Record(vec![
                            ResolvedField {
                                name: "valid".to_owned(),
                                ty: ResolvedType::Bool,
                            },
                            ResolvedField {
                                name: "value".to_owned(),
                                ty: fallback.clone(),
                            },
                            ResolvedField {
                                name: "error".to_owned(),
                                ty: ResolvedType::Text,
                            },
                        ])
            }
            _ => false,
        },
        "core.data.encode" => {
            matches!(signature.parameters.as_slice(), [value] if value.is_durable())
                && signature.result == ResolvedType::Bytes
        }
        "core.data.decode-or" => match signature.parameters.as_slice() {
            [ResolvedType::Bytes, fallback] => {
                fallback.is_durable() && signature.result == *fallback
            }
            _ => false,
        },
        "core.http.bearer-token" => exact(signature, &[http_headers_type()], &ResolvedType::Text),
        "core.http.media-type-is" => exact(
            signature,
            &[ResolvedType::Bytes, ResolvedType::Text],
            &ResolvedType::Bool,
        ),
        "core.bytes.from-text" => exact(signature, &[ResolvedType::Text], &ResolvedType::Bytes),
        "core.bytes.to-text" => exact(signature, &[ResolvedType::Bytes], &ResolvedType::Text),
        "core.bytes.concat" => exact(
            signature,
            &[ResolvedType::Bytes, ResolvedType::Bytes],
            &ResolvedType::Bytes,
        ),
        "core.bytes.length" => exact(signature, &[ResolvedType::Bytes], &ResolvedType::I64),
        "core.bytes.to-hex" => exact(signature, &[ResolvedType::Bytes], &ResolvedType::Text),
        "core.bytes.equal" => exact(
            signature,
            &[ResolvedType::Bytes, ResolvedType::Bytes],
            &ResolvedType::Bool,
        ),
        "core.bytes.blake3" => exact(signature, &[ResolvedType::Bytes], &ResolvedType::Bytes),
        "core.value.equal" => {
            signature.parameters.len() == 2
                && signature.parameters[0] == signature.parameters[1]
                && signature.result == ResolvedType::Bool
                && signature.parameters[0].is_durable()
        }
        "core.list.length" => {
            matches!(signature.parameters.as_slice(), [ResolvedType::List(_)])
                && signature.result == ResolvedType::I64
        }
        "core.list.get" => match signature.parameters.as_slice() {
            [ResolvedType::List(item), ResolvedType::I64] => signature.result == **item,
            _ => false,
        },
        "core.list.append" => match signature.parameters.as_slice() {
            [ResolvedType::List(item), value] => {
                **item == *value && signature.result == ResolvedType::List(item.clone())
            }
            _ => false,
        },
        "core.option.some" => match signature.parameters.as_slice() {
            [value] => signature.result == ResolvedType::Option(Box::new(value.clone())),
            _ => false,
        },
        "core.option.none" => {
            signature.parameters.is_empty() && matches!(&signature.result, ResolvedType::Option(_))
        }
        "core.option.get-or" => match signature.parameters.as_slice() {
            [ResolvedType::Option(item), fallback] => {
                **item == *fallback && signature.result == **item
            }
            _ => false,
        },
        "core.option.present" => {
            matches!(signature.parameters.as_slice(), [ResolvedType::Option(_)])
                && signature.result == ResolvedType::Bool
        }
        "core.map.length" => {
            matches!(signature.parameters.as_slice(), [ResolvedType::Map(_, _)])
                && signature.result == ResolvedType::I64
        }
        "core.map.get" => match signature.parameters.as_slice() {
            [ResolvedType::Map(key, value), actual_key] => {
                **key == *actual_key && signature.result == **value
            }
            _ => false,
        },
        "core.map.contains" => match signature.parameters.as_slice() {
            [ResolvedType::Map(key, _), actual_key] => {
                **key == *actual_key && signature.result == ResolvedType::Bool
            }
            _ => false,
        },
        "core.map.get-or" => match signature.parameters.as_slice() {
            [ResolvedType::Map(key, value), actual_key, default] => {
                **key == *actual_key && **value == *default && signature.result == **value
            }
            _ => false,
        },
        "core.map.insert" => match signature.parameters.as_slice() {
            [ResolvedType::Map(key, value), actual_key, actual_value] => {
                **key == *actual_key
                    && **value == *actual_value
                    && signature.result == ResolvedType::Map(key.clone(), value.clone())
            }
            _ => false,
        },
        "core.map.remove" => match signature.parameters.as_slice() {
            [ResolvedType::Map(key, value), actual_key] => {
                **key == *actual_key
                    && signature.result == ResolvedType::Map(key.clone(), value.clone())
            }
            _ => false,
        },
        "core.map.entries" => match signature.parameters.as_slice() {
            [ResolvedType::Map(key, value)] => {
                signature.result
                    == ResolvedType::List(Box::new(ResolvedType::Record(vec![
                        ResolvedField {
                            name: "key".to_owned(),
                            ty: (**key).clone(),
                        },
                        ResolvedField {
                            name: "value".to_owned(),
                            ty: (**value).clone(),
                        },
                    ])))
            }
            _ => false,
        },
        _ => {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "intrinsic_unknown",
                format!("external implementation '{implementation}' is not registered"),
            ));
        }
    };
    if !signature.task_capabilities.is_empty() || !valid {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            "intrinsic_signature",
            format!("external implementation '{implementation}' has a foreign signature"),
        ));
    }
    Ok(())
}

fn exact(
    signature: &FunctionSignature,
    parameters: &[ResolvedType],
    result: &ResolvedType,
) -> bool {
    signature.parameters == parameters && signature.result == *result
}

fn http_headers_type() -> ResolvedType {
    ResolvedType::List(Box::new(ResolvedType::Record(vec![
        ResolvedField {
            name: "name".to_owned(),
            ty: ResolvedType::Text,
        },
        ResolvedField {
            name: "value".to_owned(),
            ty: ResolvedType::Bytes,
        },
    ])))
}

fn parse_i64_result_type() -> ResolvedType {
    ResolvedType::Record(vec![
        ResolvedField {
            name: "valid".to_owned(),
            ty: ResolvedType::Bool,
        },
        ResolvedField {
            name: "value".to_owned(),
            ty: ResolvedType::I64,
        },
    ])
}

fn json_decodable(ty: &ResolvedType) -> bool {
    match ty {
        ResolvedType::Unit
        | ResolvedType::Bool
        | ResolvedType::I64
        | ResolvedType::Bytes
        | ResolvedType::Text
        | ResolvedType::Nominal(_) => true,
        ResolvedType::Record(fields) => fields.iter().all(|field| json_decodable(&field.ty)),
        ResolvedType::List(item) => json_decodable(item),
        ResolvedType::Map(key, value) => json_decodable(key) && json_decodable(value),
        ResolvedType::StaticText
        | ResolvedType::Secret
        | ResolvedType::Parameter(_)
        | ResolvedType::Option(_)
        | ResolvedType::Result(_, _)
        | ResolvedType::Stream(_)
        | ResolvedType::Function(_, _) => false,
    }
}
