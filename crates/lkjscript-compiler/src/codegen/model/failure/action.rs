fn compile_failure_action(
    function: &Function,
    slots: &HashMap<ValueId, usize>,
    chunk: &Chunk,
    index: &FailureCodegenIndex<'_>,
    action: &SsaFailureCleanupAction,
) -> Result<BytecodeFailureCleanupAction> {
    let local = |value: ValueId| {
        slots
            .get(&value)
            .copied()
            .ok_or_else(|| Error::msg("failure cleanup lost SSA local slot"))
    };
    let place = |place: lkjscript_ir::PlaceId| {
        usize::try_from(place.raw())
            .map_err(|_| Error::msg("failure cleanup PlaceId exceeds host usize"))
    };
    match action {
        SsaFailureCleanupAction::EndBorrow {
            place: owner,
            value,
            ..
        } => {
            let ty = index.value_type(*value)?;
            if let Some(representation) = structural_view_representation(chunk, ty) {
                Ok(BytecodeFailureCleanupAction::EndStructuralBorrow {
                    local: local(*value)?,
                    place: place(*owner)?,
                    representation,
                })
            } else {
                Ok(BytecodeFailureCleanupAction::EndBorrow {
                    local: local(*value)?,
                    place: place(*owner)?,
                    kind: unique_value_kind(ty)
                        .ok_or_else(|| Error::msg("failure cleanup loan has non-unique type"))?,
                })
            }
        }
        SsaFailureCleanupAction::DropOwner {
            place: owner,
            value,
            glue: DropGlueIdentity::ByteVector | DropGlueIdentity::Bytes,
        } => Ok(BytecodeFailureCleanupAction::DropUnique {
            local: local(*value)?,
            place: owner.map(place).transpose()?,
            kind: unique_value_kind(index.value_type(*value)?)
                .ok_or_else(|| Error::msg("failure cleanup owner has non-unique type"))?,
        }),
        SsaFailureCleanupAction::DropOwner {
            place: owner,
            value,
            glue: DropGlueIdentity::Resource(kind),
        } => Ok(BytecodeFailureCleanupAction::DropResource {
            local: local(*value)?,
            place: owner.map(place).transpose()?,
            kind: *kind,
        }),
        SsaFailureCleanupAction::DropOwner {
            value,
            glue: DropGlueIdentity::Structural(StructuralDropGlueIdentity::Destination { .. }),
            ..
        } => Ok(BytecodeFailureCleanupAction::AbortStructuralDestination {
            local: local(*value)?,
            destination: structural_destination_for_value(chunk, index, *value)?,
        }),
        SsaFailureCleanupAction::DropOwner {
            place: owner,
            value,
            glue: DropGlueIdentity::Structural(_),
        } => Ok(BytecodeFailureCleanupAction::DropStructural {
            local: local(*value)?,
            place: owner.map(place).transpose()?,
            representation: structural_owner_representation_for_value(function, chunk, *value)
                .or_else(|| structural_owner_representation(
                    chunk,
                    index.value_type(*value).ok()?,
                ))
                .ok_or_else(|| Error::msg(
                    "failure cleanup structural owner has no exact representation",
                ))?,
        }),
    }
}

fn structural_destination_for_value(
    chunk: &Chunk,
    index: &FailureCodegenIndex<'_>,
    value: ValueId,
) -> Result<StructuralDestinationId> {
    let instruction = index.definition(value)?;
    match &instruction.kind {
        InstructionKind::DestinationCreate {
            representation,
            active_variant,
        } => structural_destination(chunk, *representation, *active_variant),
        InstructionKind::DestinationFieldInit { destination, .. } => {
            structural_destination_for_value(chunk, index, *destination)
        }
        InstructionKind::Constant(_)
        | InstructionKind::Copy(_)
        | InstructionKind::PlaceInit { .. }
        | InstructionKind::PlaceEnd { .. }
        | InstructionKind::EndBorrow { .. }
        | InstructionKind::Drop { .. }
        | InstructionKind::Move { .. }
        | InstructionKind::Borrow { .. }
        | InstructionKind::StructuralPublish { .. }
        | InstructionKind::DestinationFinish { .. }
        | InstructionKind::DestinationAbort { .. }
        | InstructionKind::AggregateFieldBorrow { .. }
        | InstructionKind::AggregateTag { .. }
        | InstructionKind::AggregateConsumePayload { .. }
        | InstructionKind::StringUtf8View { .. }
        | InstructionKind::StructuralCopy { .. }
        | InstructionKind::MemoryWitnessIndependentOwner { .. }
        | InstructionKind::MemoryWitnessCompare { .. }
        | InstructionKind::MemoryWitnessDispose { .. }
        | InstructionKind::FunctionRef(_)
        | InstructionKind::Runtime { .. }
        | InstructionKind::F64FromI64Exact { .. }
        | InstructionKind::F64FromI64Rounded { .. }
        | InstructionKind::I64FromF64Exact { .. }
        | InstructionKind::I64FromF64Trunc { .. }
        | InstructionKind::Call { .. }
        | InstructionKind::ProductValue { .. }
        | InstructionKind::ProductField { .. }
        | InstructionKind::WithProductField { .. }
        | InstructionKind::EnumValue { .. }
        | InstructionKind::EnumIsVariant { .. }
        | InstructionKind::EnumField { .. } => Err(Error::msg(
            "failure cleanup destination metadata provenance is invalid",
        )),
    }
}
