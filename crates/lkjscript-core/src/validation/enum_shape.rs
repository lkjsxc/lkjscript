use std::collections::HashSet;

use crate::{Chunk, Error, Result};

pub(super) fn validate(chunk: &Chunk) -> Result<usize> {
    let mut bytes = 0_usize;
    let mut enum_ids = HashSet::new();
    let mut enum_names = HashSet::new();
    let mut layouts = HashSet::new();
    let mut variant_ids = HashSet::new();
    let mut field_ids = HashSet::new();
    for definition in &chunk.enums {
        if !definition.id.is_resolved()
            || !definition.layout.is_resolved()
            || definition.name.is_empty()
            || definition.variants.is_empty()
            || !enum_ids.insert(definition.id)
            || !enum_names.insert(definition.name.as_str())
            || !layouts.insert(definition.layout)
            || !super::prelude_shape::valid(definition)
        {
            return Err(Error::msg("bytecode enum has invalid identity/name/layout"));
        }
        bytes = add(bytes, definition.name.len())?;
        let mut tags = HashSet::new();
        let mut names = HashSet::new();
        for (source_order, variant) in definition.variants.iter().enumerate() {
            if u64::try_from(source_order).ok() != Some(variant.source_order)
                || !variant.id.is_resolved()
                || variant.name.is_empty()
                || !variant_ids.insert(variant.id)
                || !names.insert(variant.name.as_str())
                || !tags.insert(variant.physical_tag)
                || usize::try_from(variant.physical_tag)
                    .map_or(true, |tag| tag >= definition.variants.len())
            {
                return Err(Error::msg("bytecode enum variant metadata is malformed"));
            }
            bytes = add(bytes, variant.name.len())?;
            let mut field_names = HashSet::new();
            for field in &variant.fields {
                if !field.id.is_resolved()
                    || field.name.is_empty()
                    || !field_ids.insert(field.id)
                    || !field_names.insert(field.name.as_str())
                {
                    return Err(Error::msg("bytecode enum field metadata is malformed"));
                }
                bytes = add(bytes, field.name.len())?;
            }
        }
    }
    validate_constructions(chunk)?;
    validate_variants(chunk)?;
    validate_fields(chunk)?;
    Ok(bytes)
}

fn validate_constructions(chunk: &Chunk) -> Result<()> {
    let mut unique = HashSet::new();
    for descriptor in &chunk.enum_constructions {
        let definition = enum_by_id(chunk, descriptor.enum_id)?;
        if definition.layout != descriptor.layout
            || descriptor.substitution_arity != definition.type_parameter_count
            || !definition
                .variants
                .iter()
                .any(|variant| variant.id == descriptor.variant)
            || !unique.insert(*descriptor)
        {
            return Err(Error::msg(
                "bytecode enum construction descriptor is malformed",
            ));
        }
    }
    Ok(())
}

fn validate_variants(chunk: &Chunk) -> Result<()> {
    let mut unique = HashSet::new();
    for descriptor in &chunk.enum_variants {
        let definition = enum_by_id(chunk, descriptor.enum_id)?;
        if definition.layout != descriptor.layout
            || !definition
                .variants
                .iter()
                .any(|variant| variant.id == descriptor.variant)
            || !unique.insert(*descriptor)
        {
            return Err(Error::msg("bytecode enum variant descriptor is malformed"));
        }
    }
    Ok(())
}

fn validate_fields(chunk: &Chunk) -> Result<()> {
    let mut unique = HashSet::new();
    for descriptor in &chunk.enum_fields {
        let definition = enum_by_id(chunk, descriptor.enum_id)?;
        let valid = definition.layout == descriptor.layout
            && definition.variants.iter().any(|variant| {
                variant.id == descriptor.variant
                    && variant
                        .fields
                        .iter()
                        .any(|field| field.id == descriptor.field)
            });
        if !valid || !unique.insert(*descriptor) {
            return Err(Error::msg("bytecode enum field descriptor is malformed"));
        }
    }
    Ok(())
}

fn enum_by_id(chunk: &Chunk, id: crate::EnumId) -> Result<&crate::EnumMetadata> {
    chunk
        .enums
        .iter()
        .find(|definition| definition.id == id)
        .ok_or_else(|| Error::msg("bytecode enum descriptor references unknown EnumId"))
}

fn add(left: usize, right: usize) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| Error::host("bytecode enum metadata byte size overflow"))
}
