use super::{instruction_error, types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, ResourceKind, Result, StructuralSliceExt};

pub(super) fn file_open(
    chunk: &Chunk,
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    kind: ResourceKind,
) -> Result<()> {
    pop_structural_leaf(
        chunk,
        state,
        crate::StructuralKind::Path,
        Kind::Path,
        proto,
        instruction,
    )?;
    expect_capability(state, crate::CapabilityKind::FileSystem, proto, instruction)?;
    state
        .stack
        .push(structural_resource_result(chunk, kind, proto, instruction)?);
    Ok(())
}

pub(super) fn structural_value_result(
    chunk: &Chunk,
    success: crate::StructuralKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    if chunk.memory_plan.is_none() {
        return Ok(result_kind());
    }
    let preferred = preferred_result_type(chunk, proto);
    let type_id = find_result_type(
        chunk,
        preferred,
        true,
        success == crate::StructuralKind::Unit,
        |field| field.runtime_type.is_some_and(|ty| ty.kind == success),
    );
    let Some(representation) = structural_result_representation(chunk, type_id) else {
        return Ok(result_kind());
    };
    Ok(Kind::StructuralOwner {
        representation,
        owner: super::bytes::new_owner(instruction)?,
        active_variant: None,
    })
}

pub(super) fn structural_option_result(
    chunk: &Chunk,
    item: crate::StructuralKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    if chunk.memory_plan.is_none() {
        return Ok(result_kind());
    }
    let preferred = preferred_result_type(chunk, proto);
    let type_id = find_result_type(chunk, preferred, true, false, |field| {
        let crate::StructuralFieldRoute::Structural(type_id) = field.route else {
            return false;
        };
        let Some(ty) = chunk.structural_types.get_structural(type_id) else {
            return false;
        };
        if !matches!(
            ty.kind,
            crate::StructuralTypeKind::Enum(enum_id) if enum_id.bytes() == crate::OPTION_ID
        ) {
            return false;
        }
        let Some(layout) = chunk.structural_layouts.get_structural(ty.layout) else {
            return false;
        };
        let crate::StructuralLayoutKind::Enum { variants, .. } = &layout.kind else {
            return false;
        };
        variants
            .iter()
            .flat_map(|variant| variant.fields.first())
            .any(|field| field.runtime_type.is_some_and(|ty| ty.kind == item))
    });
    let Some(representation) = structural_result_representation(chunk, type_id) else {
        return Ok(result_kind());
    };
    Ok(Kind::StructuralOwner {
        representation,
        owner: super::bytes::new_owner(instruction)?,
        active_variant: None,
    })
}

pub(super) fn structural_resource_result(
    chunk: &Chunk,
    resource: ResourceKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    if chunk.memory_plan.is_none() {
        return resource_result_kind(resource, proto, instruction);
    }
    let preferred = preferred_result_type(chunk, proto);
    let type_id = find_result_type(chunk, preferred, true, false, |field| {
        field.resource == Some(resource)
    });
    let Some(representation) = structural_result_representation(chunk, type_id) else {
        return resource_result_kind(resource, proto, instruction);
    };
    Ok(Kind::StructuralOwner {
        representation,
        owner: super::bytes::new_owner(instruction)?,
        active_variant: None,
    })
}

include!("types/results.rs");

pub(super) fn expect_resource(
    state: &mut State,
    allowed: &[ResourceKind],
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    let actual = pop(state, proto, instruction)?;
    if matches!(actual, Kind::Resource { kind, .. } if allowed.contains(&kind)) {
        Ok(actual)
    } else {
        Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            &format!("typed resource kind mismatch: got {actual}"),
        ))
    }
}

pub(super) fn consume_resource_owner(
    state: &mut State,
    resource: Kind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let Kind::Resource { owner, .. } = resource else {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "resource consumption requires an owned resource",
        ));
    };
    if owner.is_none() {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "borrowed resource cannot be consumed",
        ));
    }
    for local in &mut state.locals {
        if matches!(local, Some(Kind::Resource { owner: actual, .. }) if *actual == owner) {
            *local = None;
        }
    }
    Ok(())
}

pub(super) fn expect_capability(
    state: &mut State,
    kind: crate::CapabilityKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    expect_pop(state, Kind::Capability(kind), proto, instruction)
}
