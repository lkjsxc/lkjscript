use super::*;

pub(in crate::lower) fn unsupported_operation<T>(
    function: FunctionId,
    operation: &str,
) -> Result<T, LoweringError> {
    Err(LoweringError::new(
        LoweringFailureCode::UnsupportedOperation,
        Some(function),
        format!("{operation} is unsupported by allocation-free scalar native code"),
    ))
}

pub(super) fn require_domain_type(
    function: FunctionId,
    ty: &SsaType,
    layouts: &LayoutInterner,
    domain: LoweringDomain,
) -> Result<(), LoweringError> {
    match domain {
        LoweringDomain::ResourceIsland => require_resource_island_type(function, ty),
        LoweringDomain::UniqueIsland => require_unique_island_type(function, ty),
        LoweringDomain::StructuralIsland => require_structural_island_type(function, ty, layouts),
        LoweringDomain::Legacy => Ok(()),
    }
}

pub(super) fn preflight_instruction_type(
    function: &Function,
    instruction: &Instruction,
    layouts: &LayoutInterner,
    modes: &BytesModes,
    domain: LoweringDomain,
) -> Result<(), LoweringError> {
    if static_trap_message(function, instruction.id).is_some() {
        return Ok(());
    }
    if matches!(
        instruction.kind,
        InstructionKind::DestinationCreate { .. } | InstructionKind::DestinationFieldInit { .. }
    ) {
        layouts.structural().destination(function, instruction.id)?;
    } else {
        lower_value_type(function.id, instruction.id, &instruction.ty, modes, layouts)?;
    }
    require_domain_type(function.id, &instruction.ty, layouts, domain)
}

pub(in crate::lower) fn selected_structural_source(
    function: &Function,
    value: ValueId,
    layouts: &LayoutInterner,
) -> Result<bool, LoweringError> {
    let ty = function
        .blocks
        .iter()
        .find_map(|block| {
            block
                .parameters
                .iter()
                .find(|parameter| parameter.id == value)
                .map(|parameter| &parameter.ty)
                .or_else(|| {
                    block
                        .instructions
                        .iter()
                        .find(|instruction| instruction.id == value)
                        .map(|instruction| &instruction.ty)
                })
        })
        .ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function.id),
                "structural product source type is missing",
            )
        })?;
    Ok(layouts.structural().selected(ty))
}

pub(in crate::lower) fn explicit_structural(kind: &InstructionKind) -> bool {
    matches!(
        kind,
        InstructionKind::StructuralPublish { .. }
            | InstructionKind::DestinationCreate { .. }
            | InstructionKind::DestinationFieldInit { .. }
            | InstructionKind::DestinationFinish { .. }
            | InstructionKind::DestinationAbort { .. }
            | InstructionKind::AggregateFieldBorrow { .. }
            | InstructionKind::AggregateTag { .. }
            | InstructionKind::AggregateConsumePayload { .. }
            | InstructionKind::StringUtf8View { .. }
            | InstructionKind::StructuralCopy { .. }
    )
}
