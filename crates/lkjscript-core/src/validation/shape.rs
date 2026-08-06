use std::collections::HashSet;

use crate::{Chunk, Constant, Error, FunctionProto, Result};

pub(super) fn validate_tables(chunk: &Chunk) -> Result<usize> {
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
    validate_proto_shape(&chunk.main, "main")?;
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
        || chunk.main.parameter_structurals.iter().any(Option::is_some)
        || chunk
            .main
            .parameter_structural_places
            .iter()
            .any(Option::is_some)
    {
        return Err(Error::msg(
            "bytecode main cannot declare unique or structural owner/view parameters",
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
        validate_proto_shape(proto, "prototype")?;
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

    let (mut metadata_bytes, mut encoded_bytes) = function_metadata_bytes(chunk)?;

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
        super::shape_structural::validate(chunk)?,
        "metadata byte size",
    )?;

    metadata_bytes = checked_add(
        metadata_bytes,
        super::enum_shape::validate(chunk)?,
        "metadata byte size",
    )?;

    (metadata_bytes, encoded_bytes) = measure_constants(chunk, metadata_bytes, encoded_bytes)?;

    checked_add(metadata_bytes, encoded_bytes, "total encoded byte size")
}

include!("shape/constants.rs");
include!("shape/metadata.rs");
include!("shape/prototypes.rs");

fn checked_add(left: usize, right: usize, category: &str) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| Error::host(format!("bytecode {category} overflow")))
}
