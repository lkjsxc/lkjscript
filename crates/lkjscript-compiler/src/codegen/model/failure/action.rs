fn compile_failure_action(
    function: &Function,
    slots: &HashMap<ValueId, u8>,
    chunk: &Chunk,
    action: &SsaFailureCleanupAction,
) -> Result<BytecodeFailureCleanupAction> {
    let local = |value: ValueId| {
        slots
            .get(&value)
            .copied()
            .ok_or_else(|| Error::msg("failure cleanup lost SSA local slot"))
    };
    let place = |place: lkjscript_ir::PlaceId| {
        u8::try_from(place.raw()).map_err(|_| Error::msg("failure cleanup PlaceId exceeds u8"))
    };
    match action {
        SsaFailureCleanupAction::EndBorrow {
            place: owner,
            value,
            ..
        } => {
            let ty = ssa_value_type(function, *value)?;
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
            kind: unique_value_kind(ssa_value_type(function, *value)?)
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
            destination: structural_destination_for_value(function, chunk, *value)?,
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
                    ssa_value_type(function, *value).ok()?,
                ))
                .ok_or_else(|| Error::msg(
                    "failure cleanup structural owner has no exact representation",
                ))?,
        }),
    }
}

fn structural_destination_for_value(
    function: &Function,
    chunk: &Chunk,
    value: ValueId,
) -> Result<StructuralDestinationId> {
    let instruction = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find(|instruction| instruction.id == value)
        .ok_or_else(|| Error::msg("failure cleanup destination has no defining instruction"))?;
    match &instruction.kind {
        InstructionKind::DestinationCreate {
            representation,
            active_variant,
        } => structural_destination(chunk, *representation, *active_variant),
        InstructionKind::DestinationFieldInit { destination, .. } => {
            structural_destination_for_value(function, chunk, *destination)
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

fn ssa_value_type(function: &Function, value: ValueId) -> Result<&SsaType> {
    function
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
        .ok_or_else(|| Error::msg("failure cleanup references missing SSA value type"))
}
