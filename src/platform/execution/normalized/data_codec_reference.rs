//! Implementation-disjoint oracle for canonical typed application values.
//!
//! This module intentionally shares no encoding or decoding helpers with the production codec.
//! The reference evaluator uses it so differential execution can detect changes in envelope,
//! layout, ordering, and value behavior.

use super::prepare::{NormalizedProgram, NormalizedRecordLayout, NormalizedVariantLayout};
use super::value::{
    NormalizedMapKey, NormalizedRecord, NormalizedValue, RecordLayoutIndex, VariantLayoutIndex,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{DeclarationReference, TypeForm, TypeObjectDigest};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const MAGIC: &[u8; 8] = b"LKJDVAL1";
const VERSION: u16 = 1;
const CHECKSUM_DOMAIN: &str = "lkjscript.data.typed-value-envelope.v1";
const LAYOUT_DOMAIN: &str = "lkjscript.data.typed-layout.v1";
const BYTE_LIMIT: usize = 4 * 1_048_576;
const ITEM_LIMIT: usize = 1_000_000;
const DEPTH_LIMIT: usize = 128;

pub(super) fn encode_typed(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
) -> Result<Vec<u8>, Diagnostic> {
    let mut encoded = Vec::new();
    encoded.extend_from_slice(MAGIC);
    encoded.extend_from_slice(&VERSION.to_be_bytes());
    encoded.extend_from_slice(&layout_digest(program, ty)?);
    let mut budget = ReferenceBudget::default();
    write_value(program, ty, value, &mut encoded, &mut budget, 0)?;
    ensure_payload_size(encoded.len())?;
    let checksum = digest(CHECKSUM_DOMAIN, &encoded);
    encoded.extend_from_slice(&checksum);
    Ok(encoded)
}

pub(super) fn decode_typed(
    program: &NormalizedProgram,
    encoded: &[u8],
    ty: TypeObjectDigest,
) -> Result<NormalizedValue, Diagnostic> {
    if encoded.len() > BYTE_LIMIT {
        return Err(resource_error(
            "typed data value exceeds the canonical byte limit",
        ));
    }
    let body_size = encoded.len().checked_sub(32).ok_or_else(|| {
        corrupt_error(
            "normalized_data_value_truncated",
            "typed data value is truncated",
        )
    })?;
    let (body, supplied_checksum) = encoded.split_at(body_size);
    if digest(CHECKSUM_DOMAIN, body).as_slice() != supplied_checksum {
        return Err(corrupt_error(
            "normalized_data_value_checksum",
            "typed data value checksum does not match its payload",
        ));
    }

    let mut input = ReferenceInput::new(body);
    if input.read_exact(8, "normalized_data_value_magic")? != MAGIC {
        return Err(corrupt_error(
            "normalized_data_value_magic",
            "typed data value has a foreign magic value",
        ));
    }
    if input.read_u16("normalized_data_value_version")? != VERSION {
        return Err(corrupt_error(
            "normalized_data_value_version",
            "typed data value belongs to a foreign format version",
        ));
    }
    let observed_layout = input.read_array("normalized_data_value_layout")?;
    if observed_layout != layout_digest(program, ty)? {
        return Err(corrupt_error(
            "normalized_data_value_layout",
            "typed data value belongs to a foreign nominal or runtime layout",
        ));
    }
    let mut budget = ReferenceBudget::default();
    let value = read_value(program, ty, &mut input, &mut budget, 0)?;
    if !input.is_finished() {
        return Err(corrupt_error(
            "normalized_data_value_trailing",
            "typed data value contains trailing input",
        ));
    }
    Ok(value)
}

#[derive(Default)]
struct ReferenceBudget {
    items: usize,
}

impl ReferenceBudget {
    fn visit(&mut self, depth: usize) -> Result<(), Diagnostic> {
        if depth > DEPTH_LIMIT {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "normalized_data_value_depth",
                "typed data value exceeds the nesting-depth limit",
            ));
        }
        self.charge(1)
    }

    fn charge(&mut self, count: usize) -> Result<(), Diagnostic> {
        self.items = self.items.checked_add(count).ok_or_else(item_error)?;
        if self.items > ITEM_LIMIT {
            return Err(item_error());
        }
        Ok(())
    }
}

fn write_value(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    value: &NormalizedValue,
    output: &mut Vec<u8>,
    budget: &mut ReferenceBudget,
    depth: usize,
) -> Result<(), Diagnostic> {
    budget.visit(depth)?;
    match (form(program, ty)?, value) {
        (TypeForm::Unit, NormalizedValue::Unit) => {}
        (TypeForm::Bool, NormalizedValue::Bool(value)) => output.push(u8::from(*value)),
        (TypeForm::I64, NormalizedValue::I64(value)) => {
            output.extend_from_slice(&value.to_be_bytes());
        }
        (TypeForm::Bytes, NormalizedValue::Bytes(value)) => write_blob(output, value)?,
        (TypeForm::Text, NormalizedValue::Text(value)) => write_blob(output, value.as_bytes())?,
        (
            TypeForm::StructuralRecord { fields: layout },
            NormalizedValue::Record(NormalizedRecord::Structural { fields: values }),
        ) => {
            if layout.len() != values.len()
                || layout
                    .iter()
                    .zip(values.iter())
                    .any(|(field, (name, _))| &field.name != name)
            {
                return Err(layout_error("structural record"));
            }
            for (field, (_, field_value)) in layout.iter().zip(values.iter()) {
                write_value(program, field.ty, field_value, output, budget, depth + 1)?;
            }
        }
        (TypeForm::Named { declaration }, _) => {
            write_nominal(program, *declaration, value, output, budget, depth)?;
        }
        (TypeForm::List { item }, NormalizedValue::List(values)) => {
            write_length(output, values.len())?;
            for item_value in values.iter() {
                write_value(program, *item, item_value, output, budget, depth + 1)?;
            }
        }
        (TypeForm::Map { key, value: item }, NormalizedValue::Map(entries)) => {
            write_length(output, entries.len())?;
            for (map_key, item_value) in entries.iter() {
                let key_value = key_as_value(map_key);
                write_value(program, *key, &key_value, output, budget, depth + 1)?;
                write_value(program, *item, item_value, output, budget, depth + 1)?;
            }
        }
        (TypeForm::StaticText, _) => return Err(unsupported("StaticText")),
        (TypeForm::Secret, _) => return Err(unsupported("Secret")),
        (TypeForm::Stream { .. }, _) => return Err(unsupported("Stream")),
        (TypeForm::Function { .. }, _) => return Err(unsupported("Function")),
        (TypeForm::TypeParameter { .. }, _) => {
            return Err(unsupported("unresolved type parameter"));
        }
        (TypeForm::Option { .. } | TypeForm::Result { .. }, _) => {
            return Err(unsupported("unrepresented Option or Result"));
        }
        _ => return Err(layout_error("value")),
    }
    ensure_payload_size(output.len())
}

fn write_nominal(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
    value: &NormalizedValue,
    output: &mut Vec<u8>,
    budget: &mut ReferenceBudget,
    depth: usize,
) -> Result<(), Diagnostic> {
    if let Some((expected_index, record)) = find_record(program, declaration) {
        let NormalizedValue::Record(NormalizedRecord::Nominal { layout, fields }) = value else {
            return Err(layout_error("nominal record"));
        };
        if *layout != expected_index || fields.len() != record.fields.len() {
            return Err(layout_error("nominal record"));
        }
        for (field, field_value) in record.fields.iter().zip(fields.iter()) {
            write_value(program, field.ty, field_value, output, budget, depth + 1)?;
        }
        return Ok(());
    }
    if let Some((expected_index, variant)) = find_variant(program, declaration) {
        let NormalizedValue::Variant {
            layout,
            case,
            payload,
        } = value
        else {
            return Err(layout_error("nominal variant"));
        };
        if *layout != expected_index {
            return Err(layout_error("nominal variant"));
        }
        let selected = usize::try_from(*case)
            .ok()
            .and_then(|index| variant.cases.get(index))
            .ok_or_else(|| layout_error("variant case"))?;
        output.extend_from_slice(&case.to_be_bytes());
        match (&selected.payload, payload) {
            (None, None) => Ok(()),
            (Some(payload_type), Some(payload_value)) => write_value(
                program,
                *payload_type,
                payload_value,
                output,
                budget,
                depth + 1,
            ),
            _ => Err(layout_error("variant payload")),
        }?;
        return Ok(());
    }
    Err(missing_named_layout())
}

fn read_value(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    input: &mut ReferenceInput<'_>,
    budget: &mut ReferenceBudget,
    depth: usize,
) -> Result<NormalizedValue, Diagnostic> {
    budget.visit(depth)?;
    match form(program, ty)? {
        TypeForm::Unit => Ok(NormalizedValue::Unit),
        TypeForm::Bool => match input.read_u8("normalized_data_bool")? {
            0 => Ok(NormalizedValue::Bool(false)),
            1 => Ok(NormalizedValue::Bool(true)),
            _ => Err(corrupt_error(
                "normalized_data_bool",
                "typed data value contains a noncanonical boolean",
            )),
        },
        TypeForm::I64 => Ok(NormalizedValue::I64(input.read_i64("normalized_data_i64")?)),
        TypeForm::Bytes => Ok(NormalizedValue::bytes(
            input.read_blob("normalized_data_bytes")?,
        )),
        TypeForm::Text => {
            let bytes = input.read_blob("normalized_data_text")?;
            String::from_utf8(bytes)
                .map(NormalizedValue::text)
                .map_err(|_| {
                    corrupt_error("normalized_data_text", "typed data text is not valid UTF-8")
                })
        }
        TypeForm::StructuralRecord { fields } => {
            let mut values = Vec::with_capacity(fields.len());
            for field in fields {
                values.push((
                    field.name.clone(),
                    read_value(program, field.ty, input, budget, depth + 1)?,
                ));
            }
            Ok(NormalizedValue::Record(NormalizedRecord::Structural {
                fields: Arc::new(values),
            }))
        }
        TypeForm::Named { declaration } => {
            read_nominal(program, *declaration, input, budget, depth)
        }
        TypeForm::List { item } => {
            let count = input.read_count("normalized_data_list_count")?;
            budget.charge(count)?;
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                values.push(read_value(program, *item, input, budget, depth + 1)?);
            }
            Ok(NormalizedValue::List(Arc::new(values)))
        }
        TypeForm::Map { key, value } => {
            let count = input.read_count("normalized_data_map_count")?;
            budget.charge(count.saturating_mul(2))?;
            let mut entries = BTreeMap::new();
            let mut previous: Option<NormalizedMapKey> = None;
            for _ in 0..count {
                let key_value = read_value(program, *key, input, budget, depth + 1)?;
                let map_key = NormalizedMapKey::from_value(key_value).ok_or_else(|| {
                    corrupt_error(
                        "normalized_data_map_key",
                        "typed data map key is not a supported ordered primitive",
                    )
                })?;
                if previous.as_ref().is_some_and(|prior| prior >= &map_key) {
                    return Err(corrupt_error(
                        "normalized_data_map_order",
                        "typed data map keys are duplicate or not in canonical order",
                    ));
                }
                previous = Some(map_key.clone());
                let item = read_value(program, *value, input, budget, depth + 1)?;
                entries.insert(map_key, item);
            }
            Ok(NormalizedValue::Map(Arc::new(entries)))
        }
        TypeForm::StaticText => Err(unsupported("StaticText")),
        TypeForm::Secret => Err(unsupported("Secret")),
        TypeForm::CapabilityResource { .. } => Err(unsupported("CapabilityResource")),
        TypeForm::Stream { .. } => Err(unsupported("Stream")),
        TypeForm::Function { .. } => Err(unsupported("Function")),
        TypeForm::TypeParameter { .. } => Err(unsupported("unresolved type parameter")),
        TypeForm::Option { .. } | TypeForm::Result { .. } => {
            Err(unsupported("unrepresented Option or Result"))
        }
    }
}

fn read_nominal(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
    input: &mut ReferenceInput<'_>,
    budget: &mut ReferenceBudget,
    depth: usize,
) -> Result<NormalizedValue, Diagnostic> {
    if let Some((layout, record)) = find_record(program, declaration) {
        let mut fields = Vec::with_capacity(record.fields.len());
        for field in record.fields.iter() {
            fields.push(read_value(program, field.ty, input, budget, depth + 1)?);
        }
        return Ok(NormalizedValue::Record(NormalizedRecord::Nominal {
            layout,
            fields: Arc::new(fields),
        }));
    }
    if let Some((layout, variant)) = find_variant(program, declaration) {
        let case = input.read_u32("normalized_data_variant_case")?;
        let selected = usize::try_from(case)
            .ok()
            .and_then(|index| variant.cases.get(index))
            .ok_or_else(|| {
                corrupt_error(
                    "normalized_data_variant_case",
                    "typed data value selects a foreign variant case",
                )
            })?;
        let payload = selected
            .payload
            .map(|payload_type| {
                read_value(program, payload_type, input, budget, depth + 1).map(Box::new)
            })
            .transpose()?;
        return Ok(NormalizedValue::Variant {
            layout,
            case,
            payload,
        });
    }
    Err(missing_named_layout())
}

fn layout_digest(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<[u8; 32], Diagnostic> {
    let mut description = Vec::new();
    let mut ancestors = BTreeSet::new();
    describe_type(program, ty, &mut ancestors, &mut description, 0)?;
    Ok(digest(LAYOUT_DOMAIN, &description))
}

fn describe_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    ancestors: &mut BTreeSet<DeclarationReference>,
    description: &mut Vec<u8>,
    depth: usize,
) -> Result<(), Diagnostic> {
    if depth > DEPTH_LIMIT {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "normalized_data_layout_depth",
            "typed data layout exceeds the nesting-depth limit",
        ));
    }
    description.extend_from_slice(&ty.bytes());
    match form(program, ty)? {
        TypeForm::Unit => description.push(0),
        TypeForm::Bool => description.push(1),
        TypeForm::I64 => description.push(2),
        TypeForm::Bytes => description.push(3),
        TypeForm::Text => description.push(4),
        TypeForm::Named { declaration } => {
            description.push(5);
            write_blob(description, declaration.package.to_string().as_bytes())?;
            write_blob(description, declaration.declaration.to_string().as_bytes())?;
            if !ancestors.insert(*declaration) {
                description.push(0);
                return Ok(());
            }
            description.push(1);
            if let Some((_, record)) = find_record(program, *declaration) {
                description.push(0);
                write_length(description, record.fields.len())?;
                for field in record.fields.iter() {
                    write_blob(description, field.reference.package.to_string().as_bytes())?;
                    write_blob(description, field.reference.field.to_string().as_bytes())?;
                    write_blob(description, field.name.as_str().as_bytes())?;
                    describe_type(program, field.ty, ancestors, description, depth + 1)?;
                }
            } else if let Some((_, variant)) = find_variant(program, *declaration) {
                description.push(1);
                write_length(description, variant.cases.len())?;
                for case in variant.cases.iter() {
                    write_blob(description, case.reference.package.to_string().as_bytes())?;
                    write_blob(description, case.reference.case.to_string().as_bytes())?;
                    write_blob(description, case.name.as_str().as_bytes())?;
                    if let Some(payload) = case.payload {
                        description.push(1);
                        describe_type(program, payload, ancestors, description, depth + 1)?;
                    } else {
                        description.push(0);
                    }
                }
            } else {
                return Err(missing_named_layout());
            }
            ancestors.remove(declaration);
        }
        TypeForm::StructuralRecord { fields } => {
            description.push(6);
            write_length(description, fields.len())?;
            for field in fields {
                write_blob(description, field.name.as_str().as_bytes())?;
                describe_type(program, field.ty, ancestors, description, depth + 1)?;
            }
        }
        TypeForm::List { item } => {
            description.push(7);
            describe_type(program, *item, ancestors, description, depth + 1)?;
        }
        TypeForm::Map { key, value } => {
            description.push(8);
            describe_type(program, *key, ancestors, description, depth + 1)?;
            describe_type(program, *value, ancestors, description, depth + 1)?;
        }
        TypeForm::StaticText => return Err(unsupported("StaticText")),
        TypeForm::Secret => return Err(unsupported("Secret")),
        TypeForm::CapabilityResource { .. } => {
            return Err(unsupported("CapabilityResource"));
        }
        TypeForm::Stream { .. } => return Err(unsupported("Stream")),
        TypeForm::Function { .. } => return Err(unsupported("Function")),
        TypeForm::TypeParameter { .. } => {
            return Err(unsupported("unresolved type parameter"));
        }
        TypeForm::Option { .. } | TypeForm::Result { .. } => {
            return Err(unsupported("unrepresented Option or Result"));
        }
    }
    Ok(())
}

fn form(program: &NormalizedProgram, ty: TypeObjectDigest) -> Result<&TypeForm, Diagnostic> {
    program
        .types
        .get(&ty)
        .map(|object| &object.form)
        .ok_or_else(|| {
            corrupt_error(
                "normalized_data_type_missing",
                "typed data value references a missing exact type object",
            )
        })
}

fn find_record(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
) -> Option<(RecordLayoutIndex, &NormalizedRecordLayout)> {
    program
        .records
        .iter()
        .enumerate()
        .find(|(_, layout)| layout.declaration == declaration)
        .and_then(|(position, layout)| {
            u32::try_from(position)
                .ok()
                .map(RecordLayoutIndex)
                .map(|index| (index, layout))
        })
}

fn find_variant(
    program: &NormalizedProgram,
    declaration: DeclarationReference,
) -> Option<(VariantLayoutIndex, &NormalizedVariantLayout)> {
    program
        .variants
        .iter()
        .enumerate()
        .find(|(_, layout)| layout.declaration == declaration)
        .and_then(|(position, layout)| {
            u32::try_from(position)
                .ok()
                .map(VariantLayoutIndex)
                .map(|index| (index, layout))
        })
}

fn key_as_value(key: &NormalizedMapKey) -> NormalizedValue {
    match key {
        NormalizedMapKey::Bool(value) => NormalizedValue::Bool(*value),
        NormalizedMapKey::I64(value) => NormalizedValue::I64(*value),
        NormalizedMapKey::Bytes(value) => NormalizedValue::bytes(value.clone()),
        NormalizedMapKey::Text(value) => NormalizedValue::text(value.clone()),
    }
}

fn write_length(output: &mut Vec<u8>, length: usize) -> Result<(), Diagnostic> {
    let length = u32::try_from(length).map_err(|_| item_error())?;
    output.extend_from_slice(&length.to_be_bytes());
    Ok(())
}

fn write_blob(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), Diagnostic> {
    let length = u32::try_from(bytes.len())
        .map_err(|_| resource_error("typed data field exceeds the canonical byte domain"))?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(bytes);
    Ok(())
}

fn ensure_payload_size(length: usize) -> Result<(), Diagnostic> {
    if length > BYTE_LIMIT.saturating_sub(32) {
        Err(resource_error(
            "typed data value exceeds the canonical byte limit",
        ))
    } else {
        Ok(())
    }
}

struct ReferenceInput<'a> {
    remaining: &'a [u8],
}

impl<'a> ReferenceInput<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }

    fn read_exact(&mut self, length: usize, code: &'static str) -> Result<&'a [u8], Diagnostic> {
        let Some((value, rest)) = self.remaining.split_at_checked(length) else {
            return Err(corrupt_error(code, "typed data value is truncated"));
        };
        self.remaining = rest;
        Ok(value)
    }

    fn read_u8(&mut self, code: &'static str) -> Result<u8, Diagnostic> {
        self.read_exact(1, code)?
            .first()
            .copied()
            .ok_or_else(|| corrupt_error(code, "typed data value is truncated"))
    }

    fn read_u16(&mut self, code: &'static str) -> Result<u16, Diagnostic> {
        let bytes = self.read_exact(2, code)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn read_u32(&mut self, code: &'static str) -> Result<u32, Diagnostic> {
        let bytes = self.read_exact(4, code)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn read_i64(&mut self, code: &'static str) -> Result<i64, Diagnostic> {
        let bytes = self.read_exact(8, code)?;
        let bytes: [u8; 8] = bytes
            .try_into()
            .map_err(|_| corrupt_error(code, "typed data value is truncated"))?;
        Ok(i64::from_be_bytes(bytes))
    }

    fn read_array(&mut self, code: &'static str) -> Result<[u8; 32], Diagnostic> {
        self.read_exact(32, code)?
            .try_into()
            .map_err(|_| corrupt_error(code, "typed data value is truncated"))
    }

    fn read_count(&mut self, code: &'static str) -> Result<usize, Diagnostic> {
        let count = usize::try_from(self.read_u32(code)?).map_err(|_| item_error())?;
        if count > ITEM_LIMIT {
            return Err(item_error());
        }
        Ok(count)
    }

    fn read_blob(&mut self, code: &'static str) -> Result<Vec<u8>, Diagnostic> {
        let length = self.read_count(code)?;
        if length > BYTE_LIMIT {
            return Err(resource_error(
                "typed data field exceeds the canonical byte limit",
            ));
        }
        Ok(self.read_exact(length, code)?.to_vec())
    }

    const fn is_finished(&self) -> bool {
        self.remaining.is_empty()
    }
}

fn digest(domain: &'static str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn item_error() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "normalized_data_value_items",
        "typed data value exceeds the canonical item limit",
    )
}

fn resource_error(message: &'static str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Resource,
        "normalized_data_value_bytes",
        message,
    )
}

fn corrupt_error(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

fn unsupported(name: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Source,
        "normalized_data_value_type",
        format!("{name} cannot be encoded as durable typed application data"),
    )
}

fn layout_error(name: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "normalized_data_runtime_layout",
        format!("runtime {name} disagrees with its exact typed layout"),
    )
}

fn missing_named_layout() -> Diagnostic {
    corrupt_error(
        "normalized_data_named_layout",
        "named typed data value has no prepared record or variant layout",
    )
}
