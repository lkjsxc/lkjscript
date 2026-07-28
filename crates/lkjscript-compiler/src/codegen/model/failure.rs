use super::*;

pub(super) fn compile_failure_cleanups(
    function: &Function,
    slots: &HashMap<ValueId, u8>,
) -> Result<(Vec<BytecodeFailureCleanupPlan>, Vec<u16>)> {
    let mut plans = Vec::new();
    let mut mapping = Vec::with_capacity(function.failure_cleanups.len());
    for plan in &function.failure_cleanups {
        let actions = plan
            .actions
            .iter()
            .map(|action| compile_failure_action(function, slots, action))
            .collect::<Result<Vec<_>>>()?;
        let compiled = BytecodeFailureCleanupPlan { actions };
        let index = if let Some(index) = plans.iter().position(|plan| plan == &compiled) {
            index
        } else {
            plans.push(compiled);
            plans.len().saturating_sub(1)
        };
        mapping.push(
            u16::try_from(index)
                .map_err(|_| Error::msg("bytecode failure-cleanup plan count exceeds u16"))?,
        );
    }
    Ok((plans, mapping))
}

pub(super) fn compile_unentered_cleanup(
    function: &Function,
    instruction: &Instruction,
    slots: &HashMap<ValueId, u8>,
) -> Result<Vec<BytecodeFailureCleanupAction>> {
    let InstructionKind::Call { arguments, .. } = &instruction.kind else {
        return Ok(Vec::new());
    };
    arguments
        .iter()
        .rev()
        .filter(|value| {
            function.blocks.iter().any(|block| {
                block.instructions.iter().any(|candidate| {
                    candidate.id == **value
                        && matches!(candidate.kind, InstructionKind::Move { .. })
                })
            })
        })
        .map(|value| {
            let local = slots
                .get(value)
                .copied()
                .ok_or_else(|| Error::msg("unentered cleanup lost SSA local slot"))?;
            match ssa_value_type(function, *value)? {
                SsaType::ByteVector | SsaType::Bytes => {
                    Ok(BytecodeFailureCleanupAction::DropUnique {
                        local,
                        place: None,
                        kind: unique_value_kind(ssa_value_type(function, *value)?).ok_or_else(
                            || Error::msg("unentered cleanup owner has non-unique type"),
                        )?,
                    })
                }
                SsaType::Resource(kind) => Ok(BytecodeFailureCleanupAction::DropResource {
                    local,
                    place: None,
                    kind: *kind,
                }),
                _ => Err(Error::msg(
                    "unentered cleanup argument is not an owned value",
                )),
            }
        })
        .collect()
}

fn compile_failure_action(
    function: &Function,
    slots: &HashMap<ValueId, u8>,
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
        } => Ok(BytecodeFailureCleanupAction::EndBorrow {
            local: local(*value)?,
            place: place(*owner)?,
            kind: unique_value_kind(ssa_value_type(function, *value)?)
                .ok_or_else(|| Error::msg("failure cleanup loan has non-unique type"))?,
        }),
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
