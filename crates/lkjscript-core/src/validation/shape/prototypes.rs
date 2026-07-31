#[path = "prototypes/failure.rs"]
mod failure;
use failure::*;

fn validate_proto_shape(
    proto: &FunctionProto,
    category: &str,
    limits: &ValidationLimits,
) -> Result<()> {
    if proto.arity > proto.locals {
        return Err(Error::msg(format!(
            "bytecode {category} {} has arity {} greater than local count {}",
            proto.name, proto.arity, proto.locals
        )));
    }
    if !proto.parameter_structurals.is_empty()
        && proto.parameter_structurals.len() != usize::from(proto.arity)
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} structural parameter metadata does not match arity",
            proto.name
        )));
    }
    if !proto.parameter_structural_places.is_empty()
        && proto.parameter_structural_places.len() != usize::from(proto.arity)
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} structural owner-place metadata does not match arity",
            proto.name
        )));
    }
    if proto.memory_plan.is_none()
        && (proto.parameter_structurals.iter().any(Option::is_some)
            || proto.return_structural.is_some())
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} structural signature lacks a MemoryPlanId",
            proto.name
        )));
    }
    if !proto.parameter_resources.is_empty()
        && proto.parameter_resources.len() != usize::from(proto.arity)
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} resource parameter metadata does not match arity",
            proto.name
        )));
    }
    if !proto.parameter_resource_places.is_empty()
        && proto.parameter_resource_places.len() != usize::from(proto.arity)
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} resource owner-place metadata does not match arity",
            proto.name
        )));
    }
    if !proto.parameter_uniques.is_empty()
        && proto.parameter_uniques.len() != usize::from(proto.arity)
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} unique parameter metadata does not match arity",
            proto.name
        )));
    }
    if !proto.parameter_unique_places.is_empty()
        && proto.parameter_unique_places.len() != usize::from(proto.arity)
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} owner-place parameter metadata does not match arity",
            proto.name
        )));
    }
    for index in 0..usize::from(proto.arity) {
        let structural = proto.parameter_structurals.get(index).copied().flatten();
        let structural_place = proto
            .parameter_structural_places
            .get(index)
            .copied()
            .flatten();
        let resource = proto.parameter_resources.get(index).copied().flatten();
        let resource_place = proto
            .parameter_resource_places
            .get(index)
            .copied()
            .flatten();
        let unique = proto.parameter_uniques.get(index).copied().flatten();
        let place = proto.parameter_unique_places.get(index).copied().flatten();
        if resource.is_some() && (unique.is_some() || structural.is_some())
        {
            return Err(Error::msg(format!(
                "bytecode {category} {} parameter has overlapping resource and unique metadata",
                proto.name
            )));
        }
        if resource_place.is_some() && resource.is_none() {
            return Err(Error::msg(format!(
                "bytecode {category} {} resource owner-place metadata lacks its parameter",
                proto.name
            )));
        }
        if resource_place.is_some_and(|place| place >= proto.unique_places) {
            return Err(Error::msg(format!(
                "bytecode {category} {} resource owner-place metadata is out of range",
                proto.name
            )));
        }
        if unique.is_some() && structural.is_some() {
            return Err(Error::msg(format!(
                "bytecode {category} {} parameter overlaps unique and structural metadata",
                proto.name
            )));
        }
        if structural_place.is_some() && structural.is_none() {
            return Err(Error::msg(format!(
                "bytecode {category} {} structural owner-place metadata does not match its parameter",
                proto.name
            )));
        }
        if structural_place.is_some_and(|place| place >= proto.unique_places) {
            return Err(Error::msg(format!(
                "bytecode {category} {} structural owner-place metadata is out of range",
                proto.name
            )));
        }
        if place.is_some()
            != matches!(
                unique,
                Some(crate::UniqueValueKind::Bytes | crate::UniqueValueKind::ByteVector)
            )
        {
            return Err(Error::msg(format!(
                "bytecode {category} {} owner-place metadata does not match owned unique parameter",
                proto.name
            )));
        }
        if place.is_some_and(|place| place >= proto.unique_places) {
            return Err(Error::msg(format!(
                "bytecode {category} {} owner-place metadata is out of range",
                proto.name
            )));
        }
    }
    if usize::from(proto.arity) > proto.parameter_structural_places.len()
        && !proto.parameter_structural_places.is_empty()
    {
        return Err(Error::msg("bytecode structural parameter place metadata is truncated"));
    }
    if usize::from(proto.arity) > proto.parameter_structurals.len()
        && !proto.parameter_structurals.is_empty()
    {
        return Err(Error::msg("bytecode structural parameter metadata is truncated"));
    }
    if [proto.return_resource.is_some(), proto.return_unique.is_some(), proto.return_structural.is_some()]
        .into_iter()
        .filter(|present| *present)
        .count()
        > 1
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} has overlapping resource, unique, or structural return metadata",
            proto.name
        )));
    }
    validate_failure_cleanup_shape(proto, category, limits)?;
    if matches!(
        proto.return_unique,
        Some(crate::UniqueValueKind::ByteSlice | crate::UniqueValueKind::ByteSliceMut)
    ) {
        return Err(Error::msg(format!(
            "bytecode {category} {} cannot return a borrowed byte view",
            proto.name
        )));
    }
    Ok(())
}
