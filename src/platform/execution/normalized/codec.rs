//! Strict typed JSON conversion at normalized Graph 7 runner boundaries.

use super::prepare::{NormalizedProgram, NormalizedRecordLayout, NormalizedVariantLayout};
use super::value::{
    NormalizedMapKey, NormalizedRecord, NormalizedValue, RecordLayoutIndex, VariantLayoutIndex,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::json::{JsonLimits, decode_strict};
use crate::platform::kernel::{DeclarationReference, TypeForm, TypeObjectDigest};
use base64::Engine;
use serde_json::{Map, Value as JsonValue};
use std::collections::BTreeMap;
use std::sync::Arc;

pub fn decode_typed(
    program: &NormalizedProgram,
    bytes: &[u8],
    ty: TypeObjectDigest,
    limits: JsonLimits,
) -> Result<NormalizedValue, Diagnostic> {
    let value = decode_strict(bytes, limits)?;
    decode_value(program, &value, ty, limits)
}

pub fn decode_value(
    program: &NormalizedProgram,
    value: &JsonValue,
    ty: TypeObjectDigest,
    limits: JsonLimits,
) -> Result<NormalizedValue, Diagnostic> {
    from_json(program, value, ty, limits, "$", 0)
}

pub fn encode_typed(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    limits: JsonLimits,
) -> Result<Vec<u8>, Diagnostic> {
    let value = encode_value(program, value, ty, limits)?;
    let bytes = serde_json::to_vec(&value).map_err(|error| {
        json_error(
            DiagnosticClass::Infrastructure,
            "normalized_json_encode",
            format!("typed JSON encoding failed: {error}"),
        )
    })?;
    if bytes.len() > limits.maximum_bytes {
        return Err(json_error(
            DiagnosticClass::Resource,
            "normalized_json_output_bytes",
            format!(
                "typed JSON output has {} bytes; the limit is {}",
                bytes.len(),
                limits.maximum_bytes
            ),
        ));
    }
    Ok(bytes)
}

pub fn encode_value(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    limits: JsonLimits,
) -> Result<JsonValue, Diagnostic> {
    let mut state = EncodeState { limits, items: 0 };
    to_json(program, value, ty, &mut state, "$", 0)
}

fn from_json(
    program: &NormalizedProgram,
    value: &JsonValue,
    ty: TypeObjectDigest,
    limits: JsonLimits,
    path: &str,
    depth: usize,
) -> Result<NormalizedValue, Diagnostic> {
    require_depth(limits, path, depth)?;
    let form = type_form(program, ty)?;
    match form {
        TypeForm::Unit if value.is_null() => Ok(NormalizedValue::Unit),
        TypeForm::Unit => Err(type_error(path, "expected null for Unit")),
        TypeForm::Bool => value
            .as_bool()
            .map(NormalizedValue::Bool)
            .ok_or_else(|| type_error(path, "expected boolean")),
        TypeForm::I64 => value
            .as_i64()
            .map(NormalizedValue::I64)
            .ok_or_else(|| type_error(path, "expected signed 64-bit integer")),
        TypeForm::Bytes => decode_bytes(value, path),
        TypeForm::Text => value
            .as_str()
            .map(|value| NormalizedValue::Text(Arc::from(value)))
            .ok_or_else(|| type_error(path, "expected text string")),
        TypeForm::StaticText => Err(type_error(
            path,
            "static text must originate in accepted meaning and cannot be decoded from JSON",
        )),
        TypeForm::Named { declaration } => {
            decode_named(program, value, *declaration, limits, path, depth)
        }
        TypeForm::StructuralRecord { fields } => {
            let object = value
                .as_object()
                .ok_or_else(|| type_error(path, "expected structural-record object"))?;
            if object.len() != fields.len() {
                return Err(type_error(
                    path,
                    "structural-record fields do not equal the exact type",
                ));
            }
            let mut values = Vec::with_capacity(fields.len());
            for field in fields {
                let field_path = format!("{path}.{}", field.name.as_str());
                let value = object.get(field.name.as_str()).ok_or_else(|| {
                    type_error(&field_path, "required structural field is missing")
                })?;
                values.push((
                    field.name.clone(),
                    from_json(program, value, field.ty, limits, &field_path, depth + 1)?,
                ));
            }
            Ok(NormalizedValue::Record(NormalizedRecord::Structural {
                fields: Arc::new(values),
            }))
        }
        TypeForm::List { item } => {
            let items = value
                .as_array()
                .ok_or_else(|| type_error(path, "expected JSON array"))?;
            let values = items
                .iter()
                .enumerate()
                .map(|(index, value)| {
                    from_json(
                        program,
                        value,
                        *item,
                        limits,
                        &format!("{path}[{index}]"),
                        depth + 1,
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(NormalizedValue::List(Arc::new(values)))
        }
        TypeForm::Map { key, value: item } => {
            let entries = value
                .as_array()
                .ok_or_else(|| type_error(path, "expected map entry array"))?;
            let mut output = BTreeMap::new();
            for (index, entry) in entries.iter().enumerate() {
                let pair = entry
                    .as_array()
                    .filter(|pair| pair.len() == 2)
                    .ok_or_else(|| type_error(path, "map entry must be [key, value]"))?;
                let key_value = from_json(
                    program,
                    &pair[0],
                    *key,
                    limits,
                    &format!("{path}[{index}][0]"),
                    depth + 1,
                )?;
                let key = NormalizedMapKey::from_value(key_value)
                    .ok_or_else(|| type_error(path, "map key type is not orderable"))?;
                let item = from_json(
                    program,
                    &pair[1],
                    *item,
                    limits,
                    &format!("{path}[{index}][1]"),
                    depth + 1,
                )?;
                if output.insert(key, item).is_some() {
                    return Err(type_error(path, "map contains a duplicate key"));
                }
            }
            Ok(NormalizedValue::Map(Arc::new(output)))
        }
        TypeForm::Secret
        | TypeForm::CapabilityResource { .. }
        | TypeForm::Stream { .. }
        | TypeForm::Function { .. }
        | TypeForm::TypeParameter { .. } => Err(type_error(
            path,
            "live, callable, or unresolved generic values cannot be decoded from JSON",
        )),
        TypeForm::Option { .. } | TypeForm::Result { .. } => Err(type_error(
            path,
            "Option and Result boundary values are not represented by normalized runtime contract 1",
        )),
    }
}

fn decode_named(
    program: &NormalizedProgram,
    value: &JsonValue,
    declaration: DeclarationReference,
    limits: JsonLimits,
    path: &str,
    depth: usize,
) -> Result<NormalizedValue, Diagnostic> {
    if let Some((index, layout)) = record_layout(program, declaration) {
        let object = value
            .as_object()
            .ok_or_else(|| type_error(path, "expected nominal-record object"))?;
        if object.len() != layout.fields.len() {
            return Err(type_error(
                path,
                "nominal-record fields do not equal the exact layout",
            ));
        }
        let mut fields = Vec::with_capacity(layout.fields.len());
        for field in layout.fields.iter() {
            let field_path = format!("{path}.{}", field.name.as_str());
            let value = object
                .get(field.name.as_str())
                .ok_or_else(|| type_error(&field_path, "required nominal field is missing"))?;
            fields.push(from_json(
                program,
                value,
                field.ty,
                limits,
                &field_path,
                depth + 1,
            )?);
        }
        return Ok(NormalizedValue::Record(NormalizedRecord::Nominal {
            layout: index,
            fields: Arc::new(fields),
        }));
    }
    if let Some((index, layout)) = variant_layout(program, declaration) {
        let object = value
            .as_object()
            .ok_or_else(|| type_error(path, "expected nominal-variant object"))?;
        let case_name = object
            .get("case")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| type_error(path, "variant requires one string 'case' field"))?;
        let (tag, case) = layout
            .cases
            .iter()
            .enumerate()
            .find(|(_, case)| case.name.as_str() == case_name)
            .ok_or_else(|| type_error(path, "variant case is absent from the exact layout"))?;
        let payload = match case.payload {
            Some(ty) => {
                if object.len() != 2 {
                    return Err(type_error(
                        path,
                        "payload variant requires exactly 'case' and 'value' fields",
                    ));
                }
                let value = object
                    .get("value")
                    .ok_or_else(|| type_error(path, "variant payload is missing"))?;
                Some(Box::new(from_json(
                    program,
                    value,
                    ty,
                    limits,
                    &format!("{path}.value"),
                    depth + 1,
                )?))
            }
            None => {
                if object.len() != 1 {
                    return Err(type_error(path, "payload-free variant accepts only 'case'"));
                }
                None
            }
        };
        return Ok(NormalizedValue::Variant {
            layout: index,
            case: u32::try_from(tag).map_err(|_| {
                json_error(
                    DiagnosticClass::Resource,
                    "normalized_json_variant_tag",
                    "variant case count exceeds the dense runtime index domain",
                )
            })?,
            payload,
        });
    }
    Err(json_error(
        DiagnosticClass::Corrupt,
        "normalized_json_named_layout",
        "named boundary type has no exact prepared record or variant layout",
    ))
}

fn to_json(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    state: &mut EncodeState,
    path: &str,
    depth: usize,
) -> Result<JsonValue, Diagnostic> {
    state.require_depth(path, depth)?;
    let form = type_form(program, ty)?;
    match (value, form) {
        (NormalizedValue::Unit, TypeForm::Unit) => Ok(JsonValue::Null),
        (NormalizedValue::Bool(value), TypeForm::Bool) => Ok(JsonValue::Bool(*value)),
        (NormalizedValue::I64(value), TypeForm::I64) => Ok(JsonValue::from(*value)),
        (NormalizedValue::Bytes(value), TypeForm::Bytes) => {
            state.charge(1, path)?;
            let encoded_length = value
                .len()
                .checked_add(2)
                .and_then(|length| length.checked_div(3))
                .and_then(|groups| groups.checked_mul(4))
                .ok_or_else(|| {
                    json_error(
                        DiagnosticClass::Resource,
                        "normalized_json_base64_length",
                        "base64 output length overflowed its platform domain",
                    )
                })?;
            if encoded_length > state.limits.maximum_string_bytes
                || encoded_length > state.limits.maximum_bytes
            {
                return Err(type_error(
                    path,
                    "base64 bytes exceed the JSON string or output-byte limit",
                ));
            }
            Ok(serde_json::json!({
                "$bytes": base64::engine::general_purpose::STANDARD.encode(value),
            }))
        }
        (NormalizedValue::Text(value), TypeForm::Text)
        | (NormalizedValue::StaticText(value), TypeForm::StaticText) => {
            if value.len() > state.limits.maximum_string_bytes {
                return Err(type_error(path, "text exceeds the JSON string-byte limit"));
            }
            Ok(JsonValue::String(value.to_string()))
        }
        (value, TypeForm::Named { declaration }) => {
            encode_named(program, value, *declaration, state, path, depth)
        }
        (
            NormalizedValue::Record(NormalizedRecord::Structural { fields: values }),
            TypeForm::StructuralRecord { fields },
        ) => {
            if values.len() != fields.len()
                || values
                    .iter()
                    .zip(fields)
                    .any(|((actual, _), expected)| actual != &expected.name)
            {
                return Err(type_error(
                    path,
                    "structural runtime record disagrees with its exact type",
                ));
            }
            state.charge(values.len(), path)?;
            let mut object = Map::new();
            for ((name, value), field) in values.iter().zip(fields) {
                let field_path = format!("{path}.{}", name.as_str());
                object.insert(
                    name.as_str().to_owned(),
                    to_json(program, value, field.ty, state, &field_path, depth + 1)?,
                );
            }
            Ok(JsonValue::Object(object))
        }
        (NormalizedValue::List(values), TypeForm::List { item }) => {
            state.charge(values.len(), path)?;
            Ok(JsonValue::Array(
                values
                    .iter()
                    .enumerate()
                    .map(|(index, value)| {
                        to_json(
                            program,
                            value,
                            *item,
                            state,
                            &format!("{path}[{index}]"),
                            depth + 1,
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        (NormalizedValue::Map(values), TypeForm::Map { key, value: item }) => {
            state.charge(values.len().saturating_mul(2), path)?;
            let entries = values
                .iter()
                .enumerate()
                .map(|(index, (map_key, value))| {
                    let key_value = map_key_value(map_key);
                    Ok(JsonValue::Array(vec![
                        to_json(
                            program,
                            &key_value,
                            *key,
                            state,
                            &format!("{path}[{index}][0]"),
                            depth + 1,
                        )?,
                        to_json(
                            program,
                            value,
                            *item,
                            state,
                            &format!("{path}[{index}][1]"),
                            depth + 1,
                        )?,
                    ]))
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?;
            Ok(JsonValue::Array(entries))
        }
        (
            _,
            TypeForm::Secret
            | TypeForm::CapabilityResource { .. }
            | TypeForm::Stream { .. }
            | TypeForm::Function { .. },
        ) => Err(type_error(
            path,
            "live or callable values cannot be encoded as JSON",
        )),
        (_, TypeForm::TypeParameter { .. }) => Err(type_error(
            path,
            "unresolved generic values cannot be encoded as JSON",
        )),
        (_, TypeForm::Option { .. } | TypeForm::Result { .. }) => Err(type_error(
            path,
            "Option and Result boundary values are not represented by normalized runtime contract 1",
        )),
        _ => Err(type_error(
            path,
            "runtime value does not match the exact boundary type",
        )),
    }
}

fn encode_named(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    declaration: DeclarationReference,
    state: &mut EncodeState,
    path: &str,
    depth: usize,
) -> Result<JsonValue, Diagnostic> {
    if let Some((index, layout)) = record_layout(program, declaration) {
        let NormalizedValue::Record(NormalizedRecord::Nominal {
            layout: actual,
            fields,
        }) = value
        else {
            return Err(type_error(path, "expected nominal runtime record"));
        };
        if *actual != index || fields.len() != layout.fields.len() {
            return Err(type_error(
                path,
                "nominal runtime record disagrees with its exact layout",
            ));
        }
        state.charge(fields.len(), path)?;
        let mut object = Map::new();
        for (value, field) in fields.iter().zip(layout.fields.iter()) {
            let field_path = format!("{path}.{}", field.name.as_str());
            object.insert(
                field.name.as_str().to_owned(),
                to_json(program, value, field.ty, state, &field_path, depth + 1)?,
            );
        }
        return Ok(JsonValue::Object(object));
    }
    if let Some((index, layout)) = variant_layout(program, declaration) {
        let NormalizedValue::Variant {
            layout: actual,
            case,
            payload,
        } = value
        else {
            return Err(type_error(path, "expected nominal runtime variant"));
        };
        if *actual != index {
            return Err(type_error(
                path,
                "nominal runtime variant disagrees with its exact layout",
            ));
        }
        let case = layout
            .cases
            .get(*case as usize)
            .ok_or_else(|| type_error(path, "runtime variant case escaped its exact layout"))?;
        state.charge(if case.payload.is_some() { 2 } else { 1 }, path)?;
        let mut object = Map::new();
        object.insert(
            "case".to_owned(),
            JsonValue::String(case.name.as_str().to_owned()),
        );
        match (&case.payload, payload) {
            (None, None) => {}
            (Some(ty), Some(value)) => {
                object.insert(
                    "value".to_owned(),
                    to_json(
                        program,
                        value,
                        *ty,
                        state,
                        &format!("{path}.value"),
                        depth + 1,
                    )?,
                );
            }
            _ => {
                return Err(type_error(
                    path,
                    "runtime variant payload disagrees with its case",
                ));
            }
        }
        return Ok(JsonValue::Object(object));
    }
    Err(json_error(
        DiagnosticClass::Corrupt,
        "normalized_json_named_layout",
        "named boundary type has no exact prepared record or variant layout",
    ))
}

fn record_layout(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
) -> Option<(RecordLayoutIndex, &NormalizedRecordLayout)> {
    program
        .records
        .iter()
        .enumerate()
        .find(|(_, layout)| layout.declaration == declaration)
        .and_then(|(index, layout)| {
            u32::try_from(index)
                .ok()
                .map(|index| (RecordLayoutIndex(index), layout))
        })
}

fn variant_layout(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
) -> Option<(VariantLayoutIndex, &NormalizedVariantLayout)> {
    program
        .variants
        .iter()
        .enumerate()
        .find(|(_, layout)| layout.declaration == declaration)
        .and_then(|(index, layout)| {
            u32::try_from(index)
                .ok()
                .map(|index| (VariantLayoutIndex(index), layout))
        })
}

fn type_form(program: &NormalizedProgram, ty: TypeObjectDigest) -> Result<&TypeForm, Diagnostic> {
    program
        .types
        .get(&ty)
        .map(|object| &object.form)
        .ok_or_else(|| {
            json_error(
                DiagnosticClass::Corrupt,
                "normalized_json_type_missing",
                "boundary type object is absent from the exact artifact closure",
            )
        })
}

fn decode_bytes(value: &JsonValue, path: &str) -> Result<NormalizedValue, Diagnostic> {
    let object = value
        .as_object()
        .filter(|object| object.len() == 1)
        .ok_or_else(|| type_error(path, "bytes require exactly one '$bytes' field"))?;
    let encoded = object
        .get("$bytes")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| type_error(path, "'$bytes' must be a base64 string"))?;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| type_error(path, "'$bytes' is not canonical base64 data"))?;
    if base64::engine::general_purpose::STANDARD.encode(&bytes) != encoded {
        return Err(type_error(path, "'$bytes' is not canonical base64 data"));
    }
    Ok(NormalizedValue::Bytes(Arc::from(bytes)))
}

fn map_key_value(key: &NormalizedMapKey) -> NormalizedValue {
    match key {
        NormalizedMapKey::Bool(value) => NormalizedValue::Bool(*value),
        NormalizedMapKey::I64(value) => NormalizedValue::I64(*value),
        NormalizedMapKey::Bytes(value) => NormalizedValue::Bytes(Arc::from(value.clone())),
        NormalizedMapKey::Text(value) => NormalizedValue::Text(Arc::from(value.as_str())),
    }
}

#[derive(Clone, Copy)]
struct EncodeState {
    limits: JsonLimits,
    items: usize,
}

impl EncodeState {
    fn require_depth(&self, path: &str, depth: usize) -> Result<(), Diagnostic> {
        require_depth(self.limits, path, depth)
    }

    fn charge(&mut self, items: usize, path: &str) -> Result<(), Diagnostic> {
        self.items = self.items.checked_add(items).ok_or_else(|| {
            json_error(
                DiagnosticClass::Resource,
                "normalized_json_item_overflow",
                "typed JSON item accounting overflowed",
            )
        })?;
        if self.items > self.limits.maximum_items {
            return Err(type_error(path, "typed JSON exceeds the item-count limit"));
        }
        Ok(())
    }
}

fn require_depth(limits: JsonLimits, path: &str, depth: usize) -> Result<(), Diagnostic> {
    if depth > limits.maximum_depth {
        return Err(type_error(
            path,
            "typed JSON exceeds the nesting-depth limit",
        ));
    }
    Ok(())
}

fn type_error(path: &str, message: impl Into<String>) -> Diagnostic {
    json_error(
        DiagnosticClass::Source,
        "normalized_json_type",
        format!("{path}: {}", message.into()),
    )
}

fn json_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
