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

pub(in crate::ssa) const fn origin(source: hir::Origin, node: u64) -> Origin {
    match source {
        hir::Origin::Source(source) => Origin::source(source.raw(), node),
        hir::Origin::Semantic | hir::Origin::Builtin => Origin::SYNTHETIC,
    }
}

pub(in crate::ssa) fn ir_error(error: lkjscript_ir::IrError) -> Error {
    Error::msg(format!("typed SSA verification failed: {error}"))
}
