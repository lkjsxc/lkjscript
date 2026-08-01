use std::collections::HashSet;

use crate::verify::*;
#[path = "function/witnesses.rs"]
mod witnesses;

use crate::{
    Program, SsaType, StructuralLayoutKind, StructuralStorage, StructuralValueCategory,
    MAX_STRUCTURAL_LAYOUTS, MAX_STRUCTURAL_LAYOUT_FIELDS, MAX_STRUCTURAL_REPRESENTATIONS,
    MAX_STRUCTURAL_TYPES,
};

pub(super) fn verify(program: &Program) -> crate::Result<()> {
    let memory = &program.memory;
    if memory.witnesses.is_empty()
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
    witnesses::verify(program)?;
    if memory.types.len() > MAX_STRUCTURAL_TYPES
        || memory.layouts.len() > MAX_STRUCTURAL_LAYOUTS
        || memory.representations.len() > MAX_STRUCTURAL_REPRESENTATIONS
    {
        return fail("SSA structural metadata exceeds bounded table limits");
    }
    let mut field_work = 0usize;
    for (index, layout) in memory.layouts.iter().enumerate() {
        if layout.id.index() != Some(index) || !layout.identity.is_resolved() {
            return fail("SSA structural layouts require dense IDs and stable identities");
        }
        let fields = match &layout.kind {
            StructuralLayoutKind::String | StructuralLayoutKind::Path => 0,
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
                fields.len()
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
                let mut count = 0usize;
                for variant in variants {
                    let exact = declared
                        .variants
                        .iter()
                        .find(|item| item.id == variant.variant)
                        .ok_or_else(|| {
                            crate::IrError::new("SSA structural enum variant is stale")
                        })?;
                    if exact.physical_tag != variant.physical_tag
                        || exact.fields.len() != variant.fields.len()
                    {
                        return fail("SSA structural active-payload layout is stale");
                    }
                    count = count
                        .checked_add(variant.fields.len())
                        .ok_or_else(|| crate::IrError::new("SSA structural field work overflow"))?;
                }
                count
            }
        };
        field_work = field_work
            .checked_add(fields)
            .ok_or_else(|| crate::IrError::new("SSA structural field work overflow"))?;
        if field_work > MAX_STRUCTURAL_LAYOUT_FIELDS {
            return fail("SSA structural layout fields exceed bounded maximum");
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
    for (index, representation) in memory.representations.iter().enumerate() {
        if representation.id.index() != Some(index)
            || !exact.insert((representation.type_id, representation.category))
        {
            return fail("SSA structural representations must be dense and unique by category");
        }
        let ty = memory
            .types
            .get(representation.type_id.index().unwrap_or(usize::MAX))
            .filter(|item| item.id == representation.type_id)
            .ok_or_else(|| crate::IrError::new("SSA representation has stale type metadata"))?;
        if ty.layout != representation.layout
            || !storage_matches(representation.category, representation.storage)
        {
            return fail("SSA structural representation has stale layout or storage category");
        }
    }
    for item in &memory.types {
        for category in [
            StructuralValueCategory::Owner,
            StructuralValueCategory::View,
            StructuralValueCategory::Destination,
        ] {
            if !exact.contains(&(item.id, category)) {
                return fail("SSA structural type lacks a closed representation category");
            }
        }
    }
    Ok(())
}

fn verify_type_layout(
    program: &Program,
    ty: &SsaType,
    layout: crate::StructuralLayoutId,
) -> crate::Result<()> {
    let kind = &program.memory.layouts[layout.index().unwrap_or(usize::MAX)].kind;
    let matches = match (ty, kind) {
        (SsaType::Str, StructuralLayoutKind::String)
        | (SsaType::Path, StructuralLayoutKind::Path) => true,
        (SsaType::Product(left), StructuralLayoutKind::Product { product: right, .. }) => {
            left == right
        }
        (SsaType::Enum { id: left, .. }, StructuralLayoutKind::Enum { enum_id: right, .. }) => {
            left == right
        }
        _ => false,
    };
    if matches {
        Ok(())
    } else {
        fail("SSA structural type and layout kind do not match")
    }
}

fn storage_matches(category: StructuralValueCategory, storage: StructuralStorage) -> bool {
    matches!(
        (category, storage),
        (StructuralValueCategory::Owner, StructuralStorage::Unique)
            | (StructuralValueCategory::View, StructuralStorage::Stack)
            | (
                StructuralValueCategory::Destination,
                StructuralStorage::CallerDestination
            )
    )
}
