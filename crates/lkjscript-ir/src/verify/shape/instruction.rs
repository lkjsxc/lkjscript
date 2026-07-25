use crate::verify::*;
use crate::{
    EffectSet, FailureBehavior, Function, Instruction, InstructionKind, Program, Safepoint, SsaType,
};

pub(crate) fn verify_instruction(
    program: &Program,
    function: &Function,
    instruction: &Instruction,
    types: &[SsaType],
    type_parameters: &[&str],
) -> crate::Result<()> {
    let expected_effects =
        expected_instruction_effects(program, function, instruction, types, type_parameters)?;
    if instruction.metadata.effects != expected_effects {
        return fail(format!(
            "SSA value {} has invalid effect metadata",
            instruction.id.raw()
        ));
    }
    let expected_safepoint = if matches!(instruction.kind, InstructionKind::Call { .. })
        || expected_effects.contains(EffectSet::ALLOCATES)
        || expected_effects.contains(EffectSet::HOST_IO)
    {
        Safepoint::Required
    } else {
        Safepoint::None
    };
    if instruction.metadata.safepoint != expected_safepoint
        || (expected_safepoint == Safepoint::Required && instruction.metadata.frame_state.is_none())
    {
        return fail(format!(
            "SSA value {} has invalid safepoint metadata",
            instruction.id.raw()
        ));
    }
    let expected_failure = failure_behavior(expected_effects);
    if instruction.metadata.failure != expected_failure {
        return fail(format!(
            "SSA value {} has invalid failure metadata",
            instruction.id.raw()
        ));
    }
    verify_type(program, &instruction.ty, type_parameters)
}

pub(crate) fn failure_behavior(effects: EffectSet) -> FailureBehavior {
    let trap = effects.contains(EffectSet::MAY_TRAP);
    let outcome = effects.contains(EffectSet::MAY_EXIT) || effects.contains(EffectSet::ALLOCATES);
    match (trap, outcome) {
        (false, false) => FailureBehavior::None,
        (true, false) => FailureBehavior::Trap,
        (false, true) => FailureBehavior::StructuredOutcome,
        (true, true) => FailureBehavior::TrapOrOutcome,
    }
}
