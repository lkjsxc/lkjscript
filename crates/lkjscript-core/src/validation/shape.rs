use std::collections::HashSet;

use crate::{Chunk, Constant, Error, FunctionProto, Result, ValidationLimits};

pub(super) fn validate_tables(chunk: &Chunk, limits: &ValidationLimits) -> Result<()> {
    let tables = [
        ("constants", chunk.constants.len()),
        ("prototypes", chunk.protos.len()),
        ("globals", chunk.global_names.len()),
        ("required capabilities", chunk.required_capabilities.len()),
        ("products", chunk.products.len()),
        ("product field descriptors", chunk.product_fields.len()),
        ("enums", chunk.enums.len()),
        ("enum constructors", chunk.enum_constructions.len()),
        ("enum variants", chunk.enum_variants.len()),
        ("enum fields", chunk.enum_fields.len()),
    ];
    for (name, length) in tables {
        if length > limits.max_table_entries {
            return Err(Error::msg(format!(
                "bytecode {name} table has {length} entries, limit {}",
                limits.max_table_entries
            )));
        }
    }
    super::entry_capabilities::validate(chunk)?;
    if !chunk.global_prototypes.is_empty()
        && chunk.global_prototypes.len() != chunk.global_names.len()
    {
        return Err(Error::msg(
            "bytecode global prototype metadata must cover every global",
        ));
    }
    for prototype in chunk.global_prototypes.iter().flatten() {
        if usize::try_from(*prototype)
            .ok()
            .is_none_or(|index| index >= chunk.protos.len())
        {
            return Err(Error::msg(
                "bytecode global prototype metadata index is out of range",
            ));
        }
    }
    validate_proto_shape(&chunk.main, "main", limits)?;
    if chunk.main.return_resource.is_some() {
        return Err(Error::msg(
            "bytecode main cannot declare a typed resource return",
        ));
    }
    if chunk.main.parameter_resources.iter().any(Option::is_some) {
        return Err(Error::msg(
            "bytecode main cannot declare typed resource parameters",
        ));
    }
    if chunk.main.parameter_uniques.iter().any(Option::is_some)
        || chunk
            .main
            .parameter_unique_places
            .iter()
            .any(Option::is_some)
    {
        return Err(Error::msg(
            "bytecode main cannot declare unique owner/view parameters",
        ));
    }
    if matches!(
        chunk.main.return_unique,
        Some(crate::UniqueValueKind::ByteSlice | crate::UniqueValueKind::ByteSliceMut)
    ) {
        return Err(Error::msg(
            "bytecode main cannot return a borrowed byte view",
        ));
    }

    let mut function_names = HashSet::with_capacity(chunk.protos.len());
    for proto in &chunk.protos {
        validate_proto_shape(proto, "prototype", limits)?;
        if proto.name.is_empty() {
            return Err(Error::msg("bytecode prototype has an empty name"));
        }
        if !function_names.insert(proto.name.as_str()) {
            return Err(Error::msg(format!(
                "duplicate bytecode prototype name {}",
                proto.name
            )));
        }
    }

    let mut metadata_bytes = super::entry_capabilities::metadata_bytes(chunk)?;
    let global_prototype_bytes = chunk
        .global_prototypes
        .len()
        .checked_mul(5)
        .ok_or_else(|| Error::msg("bytecode metadata byte size overflow"))?;
    metadata_bytes = checked_add(metadata_bytes, global_prototype_bytes, "metadata byte size")?;
    metadata_bytes = checked_add(
        metadata_bytes,
        chunk.main.parameter_resources.len(),
        "metadata byte size",
    )?;
    metadata_bytes = checked_add(
        metadata_bytes,
        chunk
            .main
            .parameter_uniques
            .len()
            .saturating_add(chunk.main.parameter_unique_places.len()),
        "metadata byte size",
    )?;
    metadata_bytes = checked_add(metadata_bytes, 3, "metadata byte size")?;
    metadata_bytes = checked_add(
        metadata_bytes,
        failure_metadata_bytes(&chunk.main)?,
        "metadata byte size",
    )?;
    let mut encoded_bytes = chunk.main.code.len();
    for proto in &chunk.protos {
        metadata_bytes = checked_add(metadata_bytes, proto.name.len(), "metadata byte size")?;
        metadata_bytes = checked_add(
            metadata_bytes,
            proto.parameter_resources.len(),
            "metadata byte size",
        )?;
        metadata_bytes = checked_add(
            metadata_bytes,
            proto
                .parameter_uniques
                .len()
                .saturating_add(proto.parameter_unique_places.len()),
            "metadata byte size",
        )?;
        metadata_bytes = checked_add(metadata_bytes, 3, "metadata byte size")?;
        metadata_bytes = checked_add(
            metadata_bytes,
            failure_metadata_bytes(proto)?,
            "metadata byte size",
        )?;
        encoded_bytes = checked_add(encoded_bytes, proto.code.len(), "encoded byte size")?;
    }

    let mut global_names = HashSet::with_capacity(chunk.global_names.len());
    for name in &chunk.global_names {
        if name.is_empty() {
            return Err(Error::msg("bytecode global has an empty name"));
        }
        if !global_names.insert(name.as_str()) {
            return Err(Error::msg(format!("duplicate bytecode global name {name}")));
        }
        metadata_bytes = checked_add(metadata_bytes, name.len(), "metadata byte size")?;
    }

    metadata_bytes = super::shape_products::validate(chunk, metadata_bytes)?;

    metadata_bytes = checked_add(
        metadata_bytes,
        super::enum_shape::validate(chunk)?,
        "metadata byte size",
    )?;

    (metadata_bytes, encoded_bytes) =
        measure_constants(chunk, limits, metadata_bytes, encoded_bytes)?;

    if metadata_bytes > limits.max_metadata_bytes {
        return Err(Error::msg(format!(
            "bytecode metadata has {metadata_bytes} bytes, limit {}",
            limits.max_metadata_bytes
        )));
    }
    if encoded_bytes > limits.max_encoded_bytes {
        return Err(Error::msg(format!(
            "encoded bytecode has {encoded_bytes} bytes, limit {}",
            limits.max_encoded_bytes
        )));
    }
    Ok(())
}

include!("shape/constants.rs");
include!("shape/prototypes.rs");

fn checked_add(left: usize, right: usize, category: &str) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| Error::msg(format!("bytecode {category} overflow")))
}
