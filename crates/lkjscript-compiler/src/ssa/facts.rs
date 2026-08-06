use crate::ssa::*;

pub(in crate::ssa) fn effects(effects: hir::EffectSet) -> EffectSet {
    EffectSet::from_bits(effects.bits())
}

pub(in crate::ssa) fn failure_behavior(effects: EffectSet) -> FailureBehavior {
    match (
        effects.contains(EffectSet::MAY_TRAP),
        effects.contains(EffectSet::MAY_EXIT) || effects.contains(EffectSet::ALLOCATES),
    ) {
        (false, false) => FailureBehavior::None,
        (true, false) => FailureBehavior::Trap,
        (false, true) => FailureBehavior::StructuredOutcome,
        (true, true) => FailureBehavior::TrapOrOutcome,
    }
}

pub(in crate::ssa) const fn origin(source: u64, node: u64) -> Origin {
    Origin::source(source, node)
}

pub(in crate::ssa) fn ir_error(error: lkjscript_ir::IrError) -> Error {
    Error::msg(format!("typed SSA verification failed: {error}"))
}
