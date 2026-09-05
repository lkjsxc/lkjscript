//! Closed semantic contracts for trusted, pure native intrinsics.
//!
//! This module owns registration and signature validation. Runtime implementations live in the
//! execution layer, but accepted source is rejected here before it can become authority.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::*;
use super::semantic::{FunctionSignature, ResolvedType};
use super::semantic_id::{DeclarationId, TypeParameterId};

pub fn validate_intrinsic(
    implementation: &str,
    signature: &FunctionSignature,
) -> Result<(), Diagnostic> {
    validate_shape(
        implementation,
        &IntrinsicSignature {
            parameters: signature.parameters.iter().map(legacy_type).collect(),
            result: legacy_type(&signature.result),
            effectful: !signature.task_capabilities.is_empty(),
        },
    )
}

fn validate_shape(implementation: &str, signature: &IntrinsicSignature) -> Result<(), Diagnostic> {
    let valid = match implementation {
        "core.i64.add" | "core.i64.subtract" | "core.i64.multiply" | "core.i64.divide" => exact(
            signature,
            &[IntrinsicType::I64, IntrinsicType::I64],
            &IntrinsicType::I64,
        ),
        "core.i64.equal" | "core.i64.less" | "core.i64.less-equal" => exact(
            signature,
            &[IntrinsicType::I64, IntrinsicType::I64],
            &IntrinsicType::Bool,
        ),
        "core.i64.to-text" => exact(signature, &[IntrinsicType::I64], &IntrinsicType::Text),
        "core.i64.parse" => exact(signature, &[IntrinsicType::Text], &IntrinsicType::I64),
        "core.i64.parse-result" => {
            exact(signature, &[IntrinsicType::Text], &parse_i64_result_type())
        }
        "core.bool.not" => exact(signature, &[IntrinsicType::Bool], &IntrinsicType::Bool),
        "core.bool.and" | "core.bool.or" => exact(
            signature,
            &[IntrinsicType::Bool, IntrinsicType::Bool],
            &IntrinsicType::Bool,
        ),
        "core.text.concat" => exact(
            signature,
            &[IntrinsicType::Text, IntrinsicType::Text],
            &IntrinsicType::Text,
        ),
        "core.text.equal" | "core.text.contains" | "core.text.starts-with" => exact(
            signature,
            &[IntrinsicType::Text, IntrinsicType::Text],
            &IntrinsicType::Bool,
        ),
        "core.text.length" => exact(signature, &[IntrinsicType::Text], &IntrinsicType::I64),
        "core.text.empty" => exact(signature, &[IntrinsicType::Text], &IntrinsicType::Bool),
        "core.text.from-static" => exact(
            signature,
            &[IntrinsicType::StaticText],
            &IntrinsicType::Text,
        ),
        "core.html.escape-text" | "core.json.string" => {
            exact(signature, &[IntrinsicType::Text], &IntrinsicType::Text)
        }
        "core.json.encode" => {
            matches!(signature.parameters.as_slice(), [value] if value.is_durable())
                && signature.result == IntrinsicType::Bytes
        }
        "core.json.decode-or" => match signature.parameters.as_slice() {
            [IntrinsicType::Bytes, fallback] => {
                json_decodable(fallback)
                    && signature.result
                        == record(vec![
                            IntrinsicField {
                                name: "valid".to_owned(),
                                ty: IntrinsicType::Bool,
                            },
                            IntrinsicField {
                                name: "value".to_owned(),
                                ty: fallback.clone(),
                            },
                            IntrinsicField {
                                name: "error".to_owned(),
                                ty: IntrinsicType::Text,
                            },
                        ])
            }
            _ => false,
        },
        "core.data.encode" => {
            matches!(signature.parameters.as_slice(), [value] if value.is_durable())
                && signature.result == IntrinsicType::Bytes
        }
        "core.data.decode-or" => match signature.parameters.as_slice() {
            [IntrinsicType::Bytes, fallback] => {
                fallback.is_durable() && signature.result == *fallback
            }
            _ => false,
        },
        "core.http.bearer-token" => exact(signature, &[http_headers_type()], &IntrinsicType::Text),
        "core.http.media-type-is" => exact(
            signature,
            &[IntrinsicType::Bytes, IntrinsicType::Text],
            &IntrinsicType::Bool,
        ),
        "core.bytes.from-text" => exact(signature, &[IntrinsicType::Text], &IntrinsicType::Bytes),
        "core.bytes.to-text" => exact(signature, &[IntrinsicType::Bytes], &IntrinsicType::Text),
        "core.bytes.concat" => exact(
            signature,
            &[IntrinsicType::Bytes, IntrinsicType::Bytes],
            &IntrinsicType::Bytes,
        ),
        "core.bytes.length" => exact(signature, &[IntrinsicType::Bytes], &IntrinsicType::I64),
        "core.bytes.to-hex" => exact(signature, &[IntrinsicType::Bytes], &IntrinsicType::Text),
        "core.bytes.equal" => exact(
            signature,
            &[IntrinsicType::Bytes, IntrinsicType::Bytes],
            &IntrinsicType::Bool,
        ),
        "core.bytes.blake3" => exact(signature, &[IntrinsicType::Bytes], &IntrinsicType::Bytes),
        "core.value.equal" => {
            signature.parameters.len() == 2
                && signature.parameters[0] == signature.parameters[1]
                && signature.result == IntrinsicType::Bool
                && signature.parameters[0].is_durable()
        }
        "core.list.length" => {
            matches!(signature.parameters.as_slice(), [IntrinsicType::List(_)])
                && signature.result == IntrinsicType::I64
        }
        "core.list.get" => match signature.parameters.as_slice() {
            [IntrinsicType::List(item), IntrinsicType::I64] => signature.result == **item,
            _ => false,
        },
        "core.list.append" => match signature.parameters.as_slice() {
            [IntrinsicType::List(item), value] => {
                **item == *value && signature.result == IntrinsicType::List(item.clone())
            }
            _ => false,
        },
        "core.option.some" => match signature.parameters.as_slice() {
            [value] => signature.result == IntrinsicType::Option(Box::new(value.clone())),
            _ => false,
        },
        "core.option.none" => {
            signature.parameters.is_empty() && matches!(&signature.result, IntrinsicType::Option(_))
        }
        "core.option.get-or" => match signature.parameters.as_slice() {
            [IntrinsicType::Option(item), fallback] => {
                **item == *fallback && signature.result == **item
            }
            _ => false,
        },
        "core.option.present" => {
            matches!(signature.parameters.as_slice(), [IntrinsicType::Option(_)])
                && signature.result == IntrinsicType::Bool
        }
        "core.map.length" => {
            matches!(signature.parameters.as_slice(), [IntrinsicType::Map(_, _)])
                && signature.result == IntrinsicType::I64
        }
        "core.map.get" => match signature.parameters.as_slice() {
            [IntrinsicType::Map(key, value), actual_key] => {
                **key == *actual_key && signature.result == **value
            }
            _ => false,
        },
        "core.map.contains" => match signature.parameters.as_slice() {
            [IntrinsicType::Map(key, _), actual_key] => {
                **key == *actual_key && signature.result == IntrinsicType::Bool
            }
            _ => false,
        },
        "core.map.get-or" => match signature.parameters.as_slice() {
            [IntrinsicType::Map(key, value), actual_key, default] => {
                **key == *actual_key && **value == *default && signature.result == **value
            }
            _ => false,
        },
        "core.map.insert" => match signature.parameters.as_slice() {
            [IntrinsicType::Map(key, value), actual_key, actual_value] => {
                **key == *actual_key
                    && **value == *actual_value
                    && signature.result == IntrinsicType::Map(key.clone(), value.clone())
            }
            _ => false,
        },
        "core.map.remove" => match signature.parameters.as_slice() {
            [IntrinsicType::Map(key, value), actual_key] => {
                **key == *actual_key
                    && signature.result == IntrinsicType::Map(key.clone(), value.clone())
            }
            _ => false,
        },
        "core.map.entries" => match signature.parameters.as_slice() {
            [IntrinsicType::Map(key, value)] => {
                signature.result
                    == IntrinsicType::List(Box::new(IntrinsicType::Record(vec![
                        IntrinsicField {
                            name: "key".to_owned(),
                            ty: (**key).clone(),
                        },
                        IntrinsicField {
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
    if signature.effectful || !valid {
        return Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            "intrinsic_signature",
            format!("external implementation '{implementation}' has a foreign signature"),
        ));
    }
    Ok(())
}

fn exact(
    signature: &IntrinsicSignature,
    parameters: &[IntrinsicType],
    result: &IntrinsicType,
) -> bool {
    signature.parameters == parameters && signature.result == *result
}

fn http_headers_type() -> IntrinsicType {
    IntrinsicType::List(Box::new(IntrinsicType::Record(vec![
        IntrinsicField {
            name: "name".to_owned(),
            ty: IntrinsicType::Text,
        },
        IntrinsicField {
            name: "value".to_owned(),
            ty: IntrinsicType::Bytes,
        },
    ])))
}

fn parse_i64_result_type() -> IntrinsicType {
    IntrinsicType::Record(vec![
        IntrinsicField {
            name: "valid".to_owned(),
            ty: IntrinsicType::Bool,
        },
        IntrinsicField {
            name: "value".to_owned(),
            ty: IntrinsicType::I64,
        },
    ])
}

fn json_decodable(ty: &IntrinsicType) -> bool {
    match ty {
        IntrinsicType::Unit
        | IntrinsicType::Bool
        | IntrinsicType::I64
        | IntrinsicType::Bytes
        | IntrinsicType::Text
        | IntrinsicType::Nominal(_, _) => true,
        IntrinsicType::Parameter(_, canonical_template) => *canonical_template,
        IntrinsicType::Record(fields) => fields.iter().all(|field| json_decodable(&field.ty)),
        IntrinsicType::List(item) => json_decodable(item),
        IntrinsicType::Map(key, value) => json_decodable(key) && json_decodable(value),
        IntrinsicType::StaticText
        | IntrinsicType::Secret
        | IntrinsicType::Option(_)
        | IntrinsicType::Result(_, _)
        | IntrinsicType::Stream(_)
        | IntrinsicType::Function(_, _) => false,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum IntrinsicType {
    Unit,
    Bool,
    I64,
    Bytes,
    Text,
    StaticText,
    Secret,
    Parameter(TypeParameterId, bool),
    Nominal(String, DeclarationId),
    Record(Vec<IntrinsicField>),
    List(Box<Self>),
    Map(Box<Self>, Box<Self>),
    Option(Box<Self>),
    Result(Box<Self>, Box<Self>),
    Stream(Box<Self>),
    Function(Vec<Self>, Box<Self>),
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct IntrinsicField {
    name: String,
    ty: IntrinsicType,
}
struct IntrinsicSignature {
    parameters: Vec<IntrinsicType>,
    result: IntrinsicType,
    effectful: bool,
}
fn record(mut fields: Vec<IntrinsicField>) -> IntrinsicType {
    fields.sort_by(|left, right| left.name.cmp(&right.name));
    IntrinsicType::Record(fields)
}
impl IntrinsicType {
    fn is_durable(&self) -> bool {
        match self {
            Self::Secret | Self::Stream(_) | Self::Function(_, _) => false,
            Self::Parameter(_, canonical_template) => *canonical_template,
            Self::Record(fields) => fields.iter().all(|field| field.ty.is_durable()),
            Self::List(item) | Self::Option(item) => item.is_durable(),
            Self::Map(key, value) | Self::Result(key, value) => {
                key.is_durable() && value.is_durable()
            }
            // Canonical external templates are rank-one. Concrete calls retain the owning
            // language's type/effect and runtime value admission; this grants no host capability.
            _ => true,
        }
    }
}
fn legacy_type(ty: &ResolvedType) -> IntrinsicType {
    match ty {
        ResolvedType::Unit => IntrinsicType::Unit,
        ResolvedType::Bool => IntrinsicType::Bool,
        ResolvedType::I64 => IntrinsicType::I64,
        ResolvedType::Bytes => IntrinsicType::Bytes,
        ResolvedType::Text => IntrinsicType::Text,
        ResolvedType::StaticText => IntrinsicType::StaticText,
        ResolvedType::Secret => IntrinsicType::Secret,
        ResolvedType::Parameter(id) => IntrinsicType::Parameter(*id, false),
        ResolvedType::Nominal(owner) => {
            IntrinsicType::Nominal(owner.package.as_str().to_owned(), owner.declaration_id)
        }
        ResolvedType::Record(fields) => record(
            fields
                .iter()
                .map(|field| IntrinsicField {
                    name: field.name.clone(),
                    ty: legacy_type(&field.ty),
                })
                .collect(),
        ),
        ResolvedType::List(item) => IntrinsicType::List(Box::new(legacy_type(item))),
        ResolvedType::Map(key, value) => {
            IntrinsicType::Map(Box::new(legacy_type(key)), Box::new(legacy_type(value)))
        }
        ResolvedType::Option(item) => IntrinsicType::Option(Box::new(legacy_type(item))),
        ResolvedType::Result(ok, error) => {
            IntrinsicType::Result(Box::new(legacy_type(ok)), Box::new(legacy_type(error)))
        }
        ResolvedType::Stream(item) => IntrinsicType::Stream(Box::new(legacy_type(item))),
        ResolvedType::Function(parameters, result) => IntrinsicType::Function(
            parameters.iter().map(legacy_type).collect(),
            Box::new(legacy_type(result)),
        ),
    }
}

/// The same closed host signature registry admits canonical external declarations. No package
/// identity is privileged. Charge expanded type visits to the caller's aggregate validation work.
pub(crate) fn validate_kernel_intrinsic(
    snapshot: &KernelSnapshot,
    external: &ExternalDeclaration,
    work: &mut usize,
    maximum: usize,
) -> Result<(), Diagnostic> {
    let mut parameters = Vec::new();
    for id in &external.parameters {
        let Some(OwnerRecord::Parameter(parameter)) =
            snapshot.owners.get(&OwnerKey::Parameter(*id))
        else {
            return Err(signature_error("external parameter is missing"));
        };
        if parameter.use_mode != ParameterUse::Unrestricted
            || parameter.resource_requirement.is_some()
        {
            return Err(signature_error("external cannot use or bind resources"));
        }
        parameters.push(kernel_type(snapshot, parameter.ty, 0, work, maximum)?);
    }
    let result = kernel_type(snapshot, external.result, 0, work, maximum)?;
    validate_shape(
        external.implementation.as_str(),
        &IntrinsicSignature {
            parameters,
            result,
            effectful: false,
        },
    )
}
fn signature_error(message: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, "intrinsic_signature", message)
}
fn kernel_type(
    snapshot: &KernelSnapshot,
    digest: TypeObjectDigest,
    depth: usize,
    work: &mut usize,
    maximum: usize,
) -> Result<IntrinsicType, Diagnostic> {
    *work = work.saturating_add(1);
    if *work > maximum || depth > super::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "kernel_full_work",
            "intrinsic signature exhausted canonical validation visits or type depth",
        ));
    }
    let object = snapshot
        .types
        .get(&digest)
        .or_else(|| snapshot.dependency_types.get(&digest))
        .ok_or_else(|| signature_error("external type is missing"))?;
    let mut child = |digest| kernel_type(snapshot, digest, depth + 1, work, maximum);
    Ok(match &object.form {
        TypeForm::Unit => IntrinsicType::Unit,
        TypeForm::Bool => IntrinsicType::Bool,
        TypeForm::I64 => IntrinsicType::I64,
        TypeForm::Bytes => IntrinsicType::Bytes,
        TypeForm::Text => IntrinsicType::Text,
        TypeForm::StaticText => IntrinsicType::StaticText,
        TypeForm::Secret => IntrinsicType::Secret,
        TypeForm::TypeParameter { parameter } => IntrinsicType::Parameter(*parameter, true),
        TypeForm::Named { declaration } => IntrinsicType::Nominal(
            super::semantic_id::encode_hex(&declaration.package.bytes()),
            declaration.declaration,
        ),
        TypeForm::CapabilityResource { .. } => {
            return Err(signature_error("external cannot accept resource types"));
        }
        TypeForm::StructuralRecord { fields } => record(
            fields
                .iter()
                .map(|field| {
                    Ok(IntrinsicField {
                        name: field.name.as_str().to_owned(),
                        ty: child(field.ty)?,
                    })
                })
                .collect::<Result<_, Diagnostic>>()?,
        ),
        TypeForm::List { item } => IntrinsicType::List(Box::new(child(*item)?)),
        TypeForm::Map { key, value } => {
            IntrinsicType::Map(Box::new(child(*key)?), Box::new(child(*value)?))
        }
        TypeForm::Option { item } => IntrinsicType::Option(Box::new(child(*item)?)),
        TypeForm::Result { ok, error } => {
            IntrinsicType::Result(Box::new(child(*ok)?), Box::new(child(*error)?))
        }
        TypeForm::Stream { item } => IntrinsicType::Stream(Box::new(child(*item)?)),
        TypeForm::Function { parameters, result } => IntrinsicType::Function(
            parameters
                .iter()
                .map(|digest| child(*digest))
                .collect::<Result<_, _>>()?,
            Box::new(child(*result)?),
        ),
    })
}
