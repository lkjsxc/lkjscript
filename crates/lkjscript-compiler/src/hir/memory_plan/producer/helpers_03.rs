fn allocation_failure(effects: u16) -> MemoryAllocationFailure {
    let allocates = effects & crate::hir::EffectSet::ALLOCATES.bits() != 0;
    let trap = effects & crate::hir::EffectSet::MAY_TRAP.bits() != 0;
    let outcome = effects & crate::hir::EffectSet::MAY_EXIT.bits() != 0 || allocates;
    match (trap, outcome) {
        (false, false) => MemoryAllocationFailure::Impossible,
        (true, false) => MemoryAllocationFailure::Trap,
        (false, true) => MemoryAllocationFailure::StructuredOutcome,
        (true, true) => MemoryAllocationFailure::TrapOrOutcome,
    }
}
fn obligation_for_type(ty: &Type) -> Option<(MemoryObligationKind, MemoryDropGlueId)> {
    match ty {
        Type::ByteVector => {
            Some((MemoryObligationKind::DropValue, MemoryDropGlueId::new(0)))
        }
        Type::Resource(kind) => Some((
            MemoryObligationKind::DropResource(*kind),
            resource_glue(*kind),
        )),
        _ => None,
    }
}
pub(crate) const fn resource_glue(kind: ResourceKind) -> MemoryDropGlueId {
    MemoryDropGlueId::new(1 + kind as u32)
}
fn drop_glues() -> Vec<MemoryDropGluePlan> {
    let mut glues = Vec::with_capacity(ResourceKind::ALL.len().saturating_add(1));
    glues.push(MemoryDropGluePlan {
        id: MemoryDropGlueId::new(0),
        kind: MemoryDropGlueKind::ByteVector,
    });
    glues.extend(
        ResourceKind::ALL
            .into_iter()
            .map(|kind| MemoryDropGluePlan {
                id: resource_glue(kind),
                kind: MemoryDropGlueKind::Resource(kind),
            }),
    );
    glues
}
