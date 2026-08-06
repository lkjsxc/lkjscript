use crate::{
    Chunk, Error, Result, StructuralDestinationMetadata, StructuralFieldMetadata,
    StructuralFieldRoute, StructuralLayoutKind, StructuralRepresentationMetadata,
    StructuralValueCategory,
};

pub(super) fn validate(chunk: &Chunk) -> Result<usize> {
    validate_table_shape(chunk)?;
    let mut bytes = validate_witness_groups(chunk)?;
    bytes = add(
        bytes,
        validate_witnesses(chunk)?,
        "structural metadata bytes",
    )?;
    bytes = add(
        bytes,
        validate_layouts_and_types(chunk)?,
        "structural metadata bytes",
    )?;
    for (index, representation) in chunk.structural_representations.iter().enumerate() {
        if representation.id.index() != index {
            return Err(Error::msg(
                "bytecode structural RepresentationIds are not dense",
            ));
        }
        validate_representation(chunk, representation)?;
        bytes = add(bytes, 8, "structural metadata byte size")?;
    }
    for (index, destination) in chunk.structural_destinations.iter().enumerate() {
        if destination.id.index() != index {
            return Err(Error::msg(
                "bytecode structural DestinationIds are not dense",
            ));
        }
        validate_destination(chunk, destination)?;
        let field_bytes = destination
            .fields
            .len()
            .checked_mul(59)
            .ok_or_else(|| Error::host("bytecode structural metadata byte size overflow"))?;
        bytes = add(bytes, 9, "structural metadata byte size")?;
        bytes = add(bytes, field_bytes, "structural metadata byte size")?;
    }
    validate_operation_references(chunk, bytes)
}

include!("tables.rs");
include!("witness_groups.rs");
include!("witnesses.rs");
include!("layouts.rs");
include!("references.rs");
include!("metadata.rs");
