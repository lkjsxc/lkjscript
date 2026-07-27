use crate::operation::*;

impl Operation {
    pub fn effects(self) -> crate::hir::EffectSet {
        lkjscript_contracts::operation_semantics_by_id(self.identity())
            .map_or(crate::hir::EffectSet::PURE, |record| {
                crate::hir::EffectSet::from_bits(record.effects.0)
            })
    }
}
