use std::collections::HashSet;

use crate::verify::*;
#[path = "function/witness_groups.rs"]
mod witness_groups;
#[path = "function/witnesses.rs"]
mod witnesses;

use crate::{
    Program, SsaType, StructuralLayoutKind, StructuralStorage, StructuralValueCategory,
    MAX_STRUCTURAL_LAYOUTS, MAX_STRUCTURAL_REPRESENTATIONS, MAX_STRUCTURAL_TYPES,
};

pub(super) fn verify(program: &Program) -> crate::Result<()> {
    let memory = &program.memory;
    if memory.witness_groups.is_empty()
        && memory.witnesses.is_empty()
        && memory.types.is_empty()
        && memory.layouts.is_empty()
        && memory.representations.is_empty()
    {
        if program.region_products.is_empty() {
            if memory.plan.is_resolved() {
                return fail("SSA empty structural tables cannot carry a resolved MemoryPlanId");
            }
        } else if !memory.plan.is_resolved() {
            return fail("SSA region-product metadata requires a resolved MemoryPlanId");
        }
        return Ok(());
    }
    if !memory.plan.is_resolved() {
        return fail("SSA structural tables require a resolved MemoryPlanId");
    }
    witness_groups::verify(program)?;
    witnesses::verify(program)?;
    if memory.types.len() > MAX_STRUCTURAL_TYPES
        || memory.layouts.len() > MAX_STRUCTURAL_LAYOUTS
        || memory.representations.len() > MAX_STRUCTURAL_REPRESENTATIONS
    {
        return fail("SSA structural metadata exceeds bounded table limits");
    }
    for (index, layout) in memory.layouts.iter().enumerate() {
        if layout.id.index() != Some(index) || !layout.identity.is_resolved() {
            return fail("SSA structural layouts require dense IDs and stable identities");
        }
        match &layout.kind {
            StructuralLayoutKind::String | StructuralLayoutKind::Path => {}
            StructuralLayoutKind::Product { product, fields } => {
                let declared = product_by_id(program, *product)?;
                if declared
                    .fields
                    .iter()
                    .map(|item| &item.ty)
                    .ne(fields.iter())
                {
                    return fail("SSA structural product layout does not match exact field types");
                }
            }
            StructuralLayoutKind::Enum {
                enum_id,
                runtime_layout,
                variants,
            } => {
                let declared = enum_by_id(program, *enum_id)?;
                if declared.layout.identity != *runtime_layout
                    || variants.len() != declared.variants.len()
                {
                    return fail("SSA structural enum layout identity or variant count is stale");
                }
                for variant in variants {
                    let exact = declared
                        .variants
                        .iter()
                        .find(|item| item.id == variant.variant)
                        .ok_or_else(|| {
                            crate::IrError::new("SSA structural enum variant is stale")
                        })?;
                    if exact.source_order != variant.source_order
                        || exact.physical_tag != variant.physical_tag
                        || exact.fields.len() != variant.fields.len()
                    {
                        return fail("SSA structural active-payload layout is stale");
                    }
                }
            }
        }
    }
    let mut exact_types = HashSet::new();
    let mut exact_witnesses = HashSet::new();
    for (index, item) in memory.types.iter().enumerate() {
        if item.id.index() != Some(index)
            || !exact_types.insert(item.ty.clone())
            || memory
                .layouts
                .get(item.layout.index().unwrap_or(usize::MAX))
                .is_none_or(|layout| layout.id != item.layout)
        {
            return fail("SSA structural types must be dense and unique by semantic type");
        }
        if !item.witness.is_resolved() {
            return fail("SSA structural types require resolved memory witness identities");
        }
        if !exact_witnesses.insert(item.witness) {
            return fail("SSA structural memory witness identities must be unique");
        }
        verify_type(program, &item.ty, &[])?;
        verify_type_layout(program, &item.ty, item.layout)?;
    }
    let mut exact = HashSet::new();
    let mut categories = HashSet::new();
    for (index, representation) in memory.representations.iter().enumerate() {
        categories.insert((representation.type_id, representation.category));
        if representation.id.index() != Some(index)
            || !exact.insert((
                representation.type_id,
                representation.witness,
                representation.witness_group,
                representation.witness_member,
                representation.layout,
                representation.category,
                representation.storage,
                representation.route,
            ))
        {
            return fail("SSA structural representations must be dense and unique by full tuple");
        }
        let ty = memory
            .types
            .get(representation.type_id.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == representation.type_id)
            .ok_or_else(|| crate::IrError::new("SSA representation has stale type metadata"))?;
        let witness_matches = match memory.witness(representation.witness) {
            Some(witness) => {
                witness.group == representation.witness_group
                    && witness.ordinal == representation.witness_member
            }
            None => memory.witnesses.is_empty(),
        };
        if ty.layout != representation.layout
            || ty.witness != representation.witness
            || !witness_matches
            || representation.route == [0; 32]
            || !storage_matches(representation.category, representation.storage)
        {
            return fail("SSA structural representation has stale exact tuple metadata");
        }
    }
    for item in &memory.types {
        for category in [
            StructuralValueCategory::Owner,
            StructuralValueCategory::View,
            StructuralValueCategory::Destination,
        ] {
            if !categories.contains(&(item.id, category)) {
                return fail("SSA structural type lacks a closed representation category");
            }
        }
    }
    Ok(())
}

include!("structural_metadata/types.rs");
