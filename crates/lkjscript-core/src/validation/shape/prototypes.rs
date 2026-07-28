fn validate_proto_shape(proto: &FunctionProto, category: &str) -> Result<()> {
    if proto.arity > proto.locals {
        return Err(Error::msg(format!(
            "bytecode {category} {} has arity {} greater than local count {}",
            proto.name, proto.arity, proto.locals
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
        let unique = proto.parameter_uniques.get(index).copied().flatten();
        let place = proto.parameter_unique_places.get(index).copied().flatten();
        if proto
            .parameter_resources
            .get(index)
            .copied()
            .flatten()
            .is_some()
            && unique.is_some()
        {
            return Err(Error::msg(format!(
                "bytecode {category} {} parameter has overlapping resource and unique metadata",
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
    if proto.return_resource.is_some() && proto.return_unique.is_some() {
        return Err(Error::msg(format!(
            "bytecode {category} {} has overlapping resource and unique return metadata",
            proto.name
        )));
    }
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
