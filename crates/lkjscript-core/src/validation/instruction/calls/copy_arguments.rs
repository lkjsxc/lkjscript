fn validate_copy_arguments(
    callee: &FunctionProto,
    arguments: &[Kind],
    caller: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    for (index, expected) in callee.parameter_copy_kinds.iter().copied().enumerate() {
        let Some(expected) = expected else {
            continue;
        };
        if !arguments
            .get(index)
            .copied()
            .is_some_and(|actual| copy_argument_matches(expected, actual))
        {
            return Err(instruction_error(
                caller,
                instruction.op(),
                instruction.offset(),
                "copy call argument does not match exact scalar metadata",
            ));
        }
    }
    Ok(())
}

fn validate_copy_return(
    proto: &FunctionProto,
    actual: Kind,
    instruction: DecodedInstruction,
) -> Result<()> {
    if actual == Kind::Any
        || proto
            .return_copy_kind
            .is_none_or(|expected| copy_argument_matches(expected, actual))
    {
        Ok(())
    } else {
        Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "copy return does not match exact scalar metadata",
        ))
    }
}

fn copy_result_kind(kind: crate::StructuralKind) -> Option<Kind> {
    match kind {
        crate::StructuralKind::Unit => Some(Kind::Unit),
        crate::StructuralKind::Bool => Some(Kind::Bool),
        crate::StructuralKind::I64 => Some(Kind::I64),
        crate::StructuralKind::F64 => Some(Kind::F64),
        crate::StructuralKind::Static => Some(Kind::Symbol),
        crate::StructuralKind::String
        | crate::StructuralKind::Path
        | crate::StructuralKind::Bytes
        | crate::StructuralKind::ByteVector
        | crate::StructuralKind::Product
        | crate::StructuralKind::Enum => None,
    }
}

fn copy_argument_matches(expected: crate::StructuralKind, actual: Kind) -> bool {
    matches!(
        (expected, actual),
        (crate::StructuralKind::Unit, Kind::Unit)
            | (crate::StructuralKind::Bool, Kind::Bool)
            | (crate::StructuralKind::I64, Kind::I64)
            | (crate::StructuralKind::F64, Kind::F64)
            | (crate::StructuralKind::Static, Kind::Symbol)
    )
}
