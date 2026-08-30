//! Canonical exact typed application-value encoding.

use super::prepare::{NormalizedProgram, NormalizedRecordLayout, NormalizedVariantLayout};
use super::value::{NormalizedMapKey, NormalizedRecord, NormalizedValue};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{DeclarationReference, TypeForm, TypeObjectDigest};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const VALUE_MAGIC: &[u8; 8] = b"LKJDVAL1";
const VALUE_CONTRACT_VERSION: u16 = 1;
const VALUE_CHECKSUM_DOMAIN: &str = "lkjscript.data.typed-value-envelope.v1";
const LAYOUT_IDENTITY_DOMAIN: &str = "lkjscript.data.typed-layout.v1";
const MAXIMUM_VALUE_BYTES: usize = 4 * 1_048_576;
const MAXIMUM_VALUE_ITEMS: usize = 1_000_000;
const MAXIMUM_VALUE_DEPTH: usize = 128;

pub(crate) fn encode_typed(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
) -> Result<Vec<u8>, Diagnostic> {
    let layout = layout_identity(program, ty)?;
    let mut output = Vec::new();
    output.extend_from_slice(VALUE_MAGIC);
    output.extend_from_slice(&VALUE_CONTRACT_VERSION.to_be_bytes());
    output.extend_from_slice(&layout);
    let mut state = CodecState::default();
    encode_value(program, value, ty, &mut output, &mut state, 0)?;
    if output.len() > MAXIMUM_VALUE_BYTES.saturating_sub(32) {
        return Err(codec_error(
            DiagnosticClass::Resource,
            "normalized_data_value_bytes",
            "typed data value exceeds the canonical byte limit",
        ));
    }
    let checksum = digest(VALUE_CHECKSUM_DOMAIN, &output);
    output.extend_from_slice(&checksum);
    Ok(output)
}

pub(crate) fn decode_typed(
    program: &NormalizedProgram,
    bytes: &[u8],
    ty: TypeObjectDigest,
) -> Result<NormalizedValue, Diagnostic> {
    if bytes.len() > MAXIMUM_VALUE_BYTES {
        return Err(codec_error(
            DiagnosticClass::Resource,
            "normalized_data_value_bytes",
            "typed data value exceeds the canonical byte limit",
        ));
    }
    let payload_length = bytes.len().checked_sub(32).ok_or_else(|| {
        codec_error(
            DiagnosticClass::Corrupt,
            "normalized_data_value_truncated",
            "typed data value is truncated",
        )
    })?;
    let (payload, checksum) = bytes.split_at(payload_length);
    if digest(VALUE_CHECKSUM_DOMAIN, payload).as_slice() != checksum {
        return Err(codec_error(
            DiagnosticClass::Corrupt,
            "normalized_data_value_checksum",
            "typed data value checksum does not match its payload",
        ));
    }
    let mut cursor = Cursor::new(payload);
    if cursor.take(8, "normalized_data_value_magic")? != VALUE_MAGIC {
        return Err(codec_error(
            DiagnosticClass::Corrupt,
            "normalized_data_value_magic",
            "typed data value has a foreign magic value",
        ));
    }
    if cursor.u16("normalized_data_value_version")? != VALUE_CONTRACT_VERSION {
        return Err(codec_error(
            DiagnosticClass::Corrupt,
            "normalized_data_value_version",
            "typed data value belongs to a foreign format version",
        ));
    }
    let expected_layout = layout_identity(program, ty)?;
    if cursor.array_32("normalized_data_value_layout")? != expected_layout {
        return Err(codec_error(
            DiagnosticClass::Corrupt,
            "normalized_data_value_layout",
            "typed data value belongs to a foreign nominal or runtime layout",
        ));
    }
    let mut state = CodecState::default();
    let value = decode_value(program, ty, &mut cursor, &mut state, 0)?;
    cursor.finish("normalized_data_value_trailing")?;
    Ok(value)
}

#[derive(Default)]
struct CodecState {
    items: usize,
}

impl CodecState {
    fn enter(&mut self, depth: usize) -> Result<(), Diagnostic> {
        if depth > MAXIMUM_VALUE_DEPTH {
            return Err(codec_error(
                DiagnosticClass::Resource,
                "normalized_data_value_depth",
                "typed data value exceeds the nesting-depth limit",
            ));
        }
        self.items = self.items.checked_add(1).ok_or_else(item_limit)?;
        if self.items > MAXIMUM_VALUE_ITEMS {
            return Err(item_limit());
        }
        Ok(())
    }
}

fn encode_value(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    output: &mut Vec<u8>,
    state: &mut CodecState,
    depth: usize,
) -> Result<(), Diagnostic> {
    state.enter(depth)?;
    let form = type_form(program, ty)?;
    match (value, form) {
        (NormalizedValue::Unit, TypeForm::Unit) => {}
        (NormalizedValue::Bool(value), TypeForm::Bool) => output.push(u8::from(*value)),
        (NormalizedValue::I64(value), TypeForm::I64) => {
            output.extend_from_slice(&value.to_be_bytes());
        }
        (NormalizedValue::Bytes(value), TypeForm::Bytes) => push_blob(output, value)?,
        (NormalizedValue::Text(value), TypeForm::Text) => push_blob(output, value.as_bytes())?,
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
                return Err(runtime_layout_error("structural record"));
            }
            for ((_, value), field) in values.iter().zip(fields) {
                encode_value(program, value, field.ty, output, state, depth + 1)?;
            }
        }
        (value, TypeForm::Named { declaration }) => {
            encode_named(program, value, *declaration, output, state, depth)?;
        }
        (NormalizedValue::List(values), TypeForm::List { item }) => {
            push_count(output, values.len())?;
            for value in values.iter() {
                encode_value(program, value, *item, output, state, depth + 1)?;
            }
        }
        (NormalizedValue::Map(values), TypeForm::Map { key, value: item }) => {
            push_count(output, values.len())?;
            for (map_key, value) in values.iter() {
                let key_value = map_key_value(map_key);
                encode_value(program, &key_value, *key, output, state, depth + 1)?;
                encode_value(program, value, *item, output, state, depth + 1)?;
            }
        }
        (_, TypeForm::StaticText) => return Err(unsupported("StaticText")),
        (_, TypeForm::Secret) => return Err(unsupported("Secret")),
        (_, TypeForm::Stream { .. }) => return Err(unsupported("Stream")),
        (_, TypeForm::Function { .. }) => return Err(unsupported("Function")),
        (_, TypeForm::TypeParameter { .. }) => {
            return Err(unsupported("unresolved type parameter"));
        }
        (_, TypeForm::Option { .. } | TypeForm::Result { .. }) => {
            return Err(unsupported("unrepresented Option or Result"));
        }
        _ => return Err(runtime_layout_error("value")),
    }
    if output.len() > MAXIMUM_VALUE_BYTES.saturating_sub(32) {
        return Err(codec_error(
            DiagnosticClass::Resource,
            "normalized_data_value_bytes",
            "typed data value exceeds the canonical byte limit",
        ));
    }
    Ok(())
}

fn encode_named(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    declaration: DeclarationReference,
    output: &mut Vec<u8>,
    state: &mut CodecState,
    depth: usize,
) -> Result<(), Diagnostic> {
    if let Some((layout_index, layout)) = record_layout(program, declaration) {
        let NormalizedValue::Record(NormalizedRecord::Nominal {
            layout: actual,
            fields,
        }) = value
        else {
            return Err(runtime_layout_error("nominal record"));
        };
        if *actual != layout_index || fields.len() != layout.fields.len() {
            return Err(runtime_layout_error("nominal record"));
        }
        for (value, field) in fields.iter().zip(layout.fields.iter()) {
            encode_value(program, value, field.ty, output, state, depth + 1)?;
        }
        return Ok(());
    }
    if let Some((layout_index, layout)) = variant_layout(program, declaration) {
        let NormalizedValue::Variant {
            layout: actual,
            case,
            payload,
        } = value
        else {
            return Err(runtime_layout_error("nominal variant"));
        };
        if *actual != layout_index {
            return Err(runtime_layout_error("nominal variant"));
        }
        let case_index =
            usize::try_from(*case).map_err(|_| runtime_layout_error("variant case"))?;
        let layout_case = layout
            .cases
            .get(case_index)
            .ok_or_else(|| runtime_layout_error("variant case"))?;
        output.extend_from_slice(&case.to_be_bytes());
        match (payload, layout_case.payload) {
            (None, None) => Ok(()),
            (Some(value), Some(ty)) => encode_value(program, value, ty, output, state, depth + 1),
            _ => Err(runtime_layout_error("variant payload")),
        }?;
        return Ok(());
    }
    Err(codec_error(
        DiagnosticClass::Corrupt,
        "normalized_data_named_layout",
        "named typed data value has no prepared record or variant layout",
    ))
}

fn decode_value(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    cursor: &mut Cursor<'_>,
    state: &mut CodecState,
    depth: usize,
) -> Result<NormalizedValue, Diagnostic> {
    state.enter(depth)?;
    match type_form(program, ty)? {
        TypeForm::Unit => Ok(NormalizedValue::Unit),
        TypeForm::Bool => match cursor.u8("normalized_data_bool")? {
            0 => Ok(NormalizedValue::Bool(false)),
            1 => Ok(NormalizedValue::Bool(true)),
            _ => Err(codec_error(
                DiagnosticClass::Corrupt,
                "normalized_data_bool",
                "typed data value contains a noncanonical boolean",
            )),
        },
        TypeForm::I64 => Ok(NormalizedValue::I64(cursor.i64("normalized_data_i64")?)),
        TypeForm::Bytes => Ok(NormalizedValue::bytes(
            cursor.blob("normalized_data_bytes")?,
        )),
        TypeForm::Text => {
            let bytes = cursor.blob("normalized_data_text")?;
            let value = String::from_utf8(bytes).map_err(|_| {
                codec_error(
                    DiagnosticClass::Corrupt,
                    "normalized_data_text",
                    "typed data text is not valid UTF-8",
                )
            })?;
            Ok(NormalizedValue::text(value))
        }
        TypeForm::StructuralRecord { fields } => {
            let mut output = Vec::with_capacity(fields.len());
            for field in fields {
                output.push((
                    field.name.clone(),
                    decode_value(program, field.ty, cursor, state, depth + 1)?,
                ));
            }
            Ok(NormalizedValue::Record(NormalizedRecord::Structural {
                fields: Arc::new(output),
            }))
        }
        TypeForm::Named { declaration } => {
            decode_named(program, *declaration, cursor, state, depth)
        }
        TypeForm::List { item } => {
            let count = cursor.count("normalized_data_list_count")?;
            charge_count(state, count)?;
            let mut output = Vec::with_capacity(count);
            for _ in 0..count {
                output.push(decode_value(program, *item, cursor, state, depth + 1)?);
            }
            Ok(NormalizedValue::List(Arc::new(output)))
        }
        TypeForm::Map { key, value } => {
            let count = cursor.count("normalized_data_map_count")?;
            charge_count(state, count.saturating_mul(2))?;
            let mut output = BTreeMap::new();
            let mut previous = None;
            for _ in 0..count {
                let decoded_key = decode_value(program, *key, cursor, state, depth + 1)?;
                let map_key = NormalizedMapKey::from_value(decoded_key).ok_or_else(|| {
                    codec_error(
                        DiagnosticClass::Corrupt,
                        "normalized_data_map_key",
                        "typed data map key is not a supported ordered primitive",
                    )
                })?;
                if previous.as_ref().is_some_and(|prior| prior >= &map_key) {
                    return Err(codec_error(
                        DiagnosticClass::Corrupt,
                        "normalized_data_map_order",
                        "typed data map keys are duplicate or not in canonical order",
                    ));
                }
                previous = Some(map_key.clone());
                let decoded_value = decode_value(program, *value, cursor, state, depth + 1)?;
                output.insert(map_key, decoded_value);
            }
            Ok(NormalizedValue::Map(Arc::new(output)))
        }
        TypeForm::StaticText => Err(unsupported("StaticText")),
        TypeForm::Secret => Err(unsupported("Secret")),
        TypeForm::Stream { .. } => Err(unsupported("Stream")),
        TypeForm::Function { .. } => Err(unsupported("Function")),
        TypeForm::TypeParameter { .. } => Err(unsupported("unresolved type parameter")),
        TypeForm::Option { .. } | TypeForm::Result { .. } => {
            Err(unsupported("unrepresented Option or Result"))
        }
    }
}

fn decode_named(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
    cursor: &mut Cursor<'_>,
    state: &mut CodecState,
    depth: usize,
) -> Result<NormalizedValue, Diagnostic> {
    if let Some((layout, record)) = record_layout(program, declaration) {
        let mut fields = Vec::with_capacity(record.fields.len());
        for field in record.fields.iter() {
            fields.push(decode_value(program, field.ty, cursor, state, depth + 1)?);
        }
        return Ok(NormalizedValue::Record(NormalizedRecord::Nominal {
            layout,
            fields: Arc::new(fields),
        }));
    }
    if let Some((layout, variant)) = variant_layout(program, declaration) {
        let case = cursor.u32("normalized_data_variant_case")?;
        let case_layout = variant
            .cases
            .get(usize::try_from(case).map_err(|_| runtime_layout_error("variant case"))?)
            .ok_or_else(|| {
                codec_error(
                    DiagnosticClass::Corrupt,
                    "normalized_data_variant_case",
                    "typed data value selects a foreign variant case",
                )
            })?;
        let payload = case_layout
            .payload
            .map(|ty| decode_value(program, ty, cursor, state, depth + 1).map(Box::new))
            .transpose()?;
        return Ok(NormalizedValue::Variant {
            layout,
            case,
            payload,
        });
    }
    Err(codec_error(
        DiagnosticClass::Corrupt,
        "normalized_data_named_layout",
        "named typed data value has no prepared record or variant layout",
    ))
}

fn layout_identity(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<[u8; 32], Diagnostic> {
    let mut bytes = Vec::new();
    let mut active = BTreeSet::new();
    describe_layout(program, ty, &mut active, &mut bytes, 0)?;
    Ok(digest(LAYOUT_IDENTITY_DOMAIN, &bytes))
}

fn describe_layout(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    active: &mut BTreeSet<DeclarationReference>,
    output: &mut Vec<u8>,
    depth: usize,
) -> Result<(), Diagnostic> {
    if depth > MAXIMUM_VALUE_DEPTH {
        return Err(codec_error(
            DiagnosticClass::Resource,
            "normalized_data_layout_depth",
            "typed data layout exceeds the nesting-depth limit",
        ));
    }
    output.extend_from_slice(&ty.bytes());
    match type_form(program, ty)? {
        TypeForm::Unit => output.push(0),
        TypeForm::Bool => output.push(1),
        TypeForm::I64 => output.push(2),
        TypeForm::Bytes => output.push(3),
        TypeForm::Text => output.push(4),
        TypeForm::Named { declaration } => {
            output.push(5);
            push_blob(output, declaration.package.to_string().as_bytes())?;
            push_blob(output, declaration.declaration.to_string().as_bytes())?;
            if !active.insert(*declaration) {
                output.push(0);
                return Ok(());
            }
            output.push(1);
            if let Some((_, layout)) = record_layout(program, *declaration) {
                output.push(0);
                push_count(output, layout.fields.len())?;
                for field in layout.fields.iter() {
                    push_blob(output, field.reference.package.to_string().as_bytes())?;
                    push_blob(output, field.reference.field.to_string().as_bytes())?;
                    push_blob(output, field.name.as_str().as_bytes())?;
                    describe_layout(program, field.ty, active, output, depth + 1)?;
                }
            } else if let Some((_, layout)) = variant_layout(program, *declaration) {
                output.push(1);
                push_count(output, layout.cases.len())?;
                for case in layout.cases.iter() {
                    push_blob(output, case.reference.package.to_string().as_bytes())?;
                    push_blob(output, case.reference.case.to_string().as_bytes())?;
                    push_blob(output, case.name.as_str().as_bytes())?;
                    match case.payload {
                        Some(payload) => {
                            output.push(1);
                            describe_layout(program, payload, active, output, depth + 1)?;
                        }
                        None => output.push(0),
                    }
                }
            } else {
                return Err(codec_error(
                    DiagnosticClass::Corrupt,
                    "normalized_data_named_layout",
                    "named typed data layout has no prepared declaration",
                ));
            }
            active.remove(declaration);
        }
        TypeForm::StructuralRecord { fields } => {
            output.push(6);
            push_count(output, fields.len())?;
            for field in fields {
                push_blob(output, field.name.as_str().as_bytes())?;
                describe_layout(program, field.ty, active, output, depth + 1)?;
            }
        }
        TypeForm::List { item } => {
            output.push(7);
            describe_layout(program, *item, active, output, depth + 1)?;
        }
        TypeForm::Map { key, value } => {
            output.push(8);
            describe_layout(program, *key, active, output, depth + 1)?;
            describe_layout(program, *value, active, output, depth + 1)?;
        }
        TypeForm::StaticText => return Err(unsupported("StaticText")),
        TypeForm::Secret => return Err(unsupported("Secret")),
        TypeForm::Stream { .. } => return Err(unsupported("Stream")),
        TypeForm::Function { .. } => return Err(unsupported("Function")),
        TypeForm::TypeParameter { .. } => return Err(unsupported("unresolved type parameter")),
        TypeForm::Option { .. } | TypeForm::Result { .. } => {
            return Err(unsupported("unrepresented Option or Result"));
        }
    }
    Ok(())
}

fn record_layout(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
) -> Option<(super::value::RecordLayoutIndex, &NormalizedRecordLayout)> {
    program
        .records
        .iter()
        .enumerate()
        .find(|(_, layout)| layout.declaration == declaration)
        .and_then(|(index, layout)| {
            u32::try_from(index)
                .ok()
                .map(super::value::RecordLayoutIndex)
                .map(|index| (index, layout))
        })
}

fn variant_layout(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
) -> Option<(super::value::VariantLayoutIndex, &NormalizedVariantLayout)> {
    program
        .variants
        .iter()
        .enumerate()
        .find(|(_, layout)| layout.declaration == declaration)
        .and_then(|(index, layout)| {
            u32::try_from(index)
                .ok()
                .map(super::value::VariantLayoutIndex)
                .map(|index| (index, layout))
        })
}

fn type_form(program: &NormalizedProgram, ty: TypeObjectDigest) -> Result<&TypeForm, Diagnostic> {
    program
        .types
        .get(&ty)
        .map(|object| &object.form)
        .ok_or_else(|| {
            codec_error(
                DiagnosticClass::Corrupt,
                "normalized_data_type_missing",
                "typed data value references a missing exact type object",
            )
        })
}

fn map_key_value(key: &NormalizedMapKey) -> NormalizedValue {
    match key {
        NormalizedMapKey::Bool(value) => NormalizedValue::Bool(*value),
        NormalizedMapKey::I64(value) => NormalizedValue::I64(*value),
        NormalizedMapKey::Bytes(value) => NormalizedValue::bytes(value.clone()),
        NormalizedMapKey::Text(value) => NormalizedValue::text(value.clone()),
    }
}

fn push_count(output: &mut Vec<u8>, value: usize) -> Result<(), Diagnostic> {
    let value = u32::try_from(value).map_err(|_| item_limit())?;
    output.extend_from_slice(&value.to_be_bytes());
    Ok(())
}

fn push_blob(output: &mut Vec<u8>, value: &[u8]) -> Result<(), Diagnostic> {
    let length = u32::try_from(value.len()).map_err(|_| {
        codec_error(
            DiagnosticClass::Resource,
            "normalized_data_value_bytes",
            "typed data field exceeds the canonical byte domain",
        )
    })?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize, code: &'static str) -> Result<&'a [u8], Diagnostic> {
        let end = self.offset.checked_add(length).ok_or_else(item_limit)?;
        let bytes = self.bytes.get(self.offset..end).ok_or_else(|| {
            codec_error(
                DiagnosticClass::Corrupt,
                code,
                "typed data value is truncated",
            )
        })?;
        self.offset = end;
        Ok(bytes)
    }

    fn u8(&mut self, code: &'static str) -> Result<u8, Diagnostic> {
        self.take(1, code)?.first().copied().ok_or_else(|| {
            codec_error(
                DiagnosticClass::Corrupt,
                code,
                "typed data value is truncated",
            )
        })
    }

    fn u16(&mut self, code: &'static str) -> Result<u16, Diagnostic> {
        let bytes = self.take(2, code)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self, code: &'static str) -> Result<u32, Diagnostic> {
        let bytes = self.take(4, code)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i64(&mut self, code: &'static str) -> Result<i64, Diagnostic> {
        let bytes = self.take(8, code)?;
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn array_32(&mut self, code: &'static str) -> Result<[u8; 32], Diagnostic> {
        self.take(32, code)?.try_into().map_err(|_| {
            codec_error(
                DiagnosticClass::Corrupt,
                code,
                "typed data value is truncated",
            )
        })
    }

    fn count(&mut self, code: &'static str) -> Result<usize, Diagnostic> {
        let count = usize::try_from(self.u32(code)?).map_err(|_| item_limit())?;
        if count > MAXIMUM_VALUE_ITEMS {
            return Err(item_limit());
        }
        Ok(count)
    }

    fn blob(&mut self, code: &'static str) -> Result<Vec<u8>, Diagnostic> {
        let length = self.count(code)?;
        if length > MAXIMUM_VALUE_BYTES {
            return Err(codec_error(
                DiagnosticClass::Resource,
                code,
                "typed data field exceeds the canonical byte limit",
            ));
        }
        Ok(self.take(length, code)?.to_vec())
    }

    fn finish(self, code: &'static str) -> Result<(), Diagnostic> {
        if self.offset != self.bytes.len() {
            return Err(codec_error(
                DiagnosticClass::Corrupt,
                code,
                "typed data value contains trailing input",
            ));
        }
        Ok(())
    }
}

fn charge_count(state: &mut CodecState, count: usize) -> Result<(), Diagnostic> {
    state.items = state.items.checked_add(count).ok_or_else(item_limit)?;
    if state.items > MAXIMUM_VALUE_ITEMS {
        return Err(item_limit());
    }
    Ok(())
}

fn digest(domain: &'static str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn item_limit() -> Diagnostic {
    codec_error(
        DiagnosticClass::Resource,
        "normalized_data_value_items",
        "typed data value exceeds the canonical item limit",
    )
}

fn unsupported(name: &str) -> Diagnostic {
    codec_error(
        DiagnosticClass::Source,
        "normalized_data_value_type",
        format!("{name} cannot be encoded as durable typed application data"),
    )
}

fn runtime_layout_error(name: &str) -> Diagnostic {
    codec_error(
        DiagnosticClass::Corrupt,
        "normalized_data_runtime_layout",
        format!("runtime {name} disagrees with its exact typed layout"),
    )
}

fn codec_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
