use super::*;

pub(super) fn compile_failure_cleanups(
    function: &Function,
    slots: &HashMap<ValueId, u8>,
    chunk: &Chunk,
) -> Result<(Vec<BytecodeFailureCleanupPlan>, Vec<u16>)> {
    let mut plans = Vec::new();
    let mut mapping = Vec::with_capacity(function.failure_cleanups.len());
    for plan in &function.failure_cleanups {
        let actions = plan
            .actions
            .iter()
            .map(|action| compile_failure_action(function, slots, chunk, action))
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
    chunk: &Chunk,
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
                ty
                @ (SsaType::Str | SsaType::Path | SsaType::Product(_) | SsaType::Enum { .. }) => {
                    Ok(BytecodeFailureCleanupAction::DropStructural {
                        local,
                        place: None,
                        representation: structural_owner_representation(chunk, ty).ok_or_else(
                            || Error::msg("unentered structural owner has no representation"),
                        )?,
                    })
                }
                SsaType::StructuralDestination(_) => {
                    Ok(BytecodeFailureCleanupAction::AbortStructuralDestination {
                        local,
                        destination: structural_destination_for_value(function, chunk, *value)?,
                    })
                }
                SsaType::Unit
                | SsaType::Bool
                | SsaType::I64
                | SsaType::F64
                | SsaType::Symbol
                | SsaType::ByteSlice
                | SsaType::ByteSliceMut
                | SsaType::Capability(_)
                | SsaType::List(_)
                | SsaType::Function(_)
                | SsaType::TypeParameter(_) => Err(Error::msg(
                    "unentered cleanup argument is not an owned value",
                )),
            }
        })
        .collect()
}

include!("failure/action.rs");
