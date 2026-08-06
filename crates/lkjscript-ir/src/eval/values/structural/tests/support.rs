use super::*;

pub(super) fn install_structural_type(
    program: &mut Program,
    ty: SsaType,
    kind: StructuralLayoutKind,
) {
    if !program.memory.plan.is_resolved() {
        program.memory.plan = MemoryPlanId::new([1; 32]);
    }
    let index = u16::try_from(program.memory.types.len()).unwrap_or(u16::MAX);
    let type_id = StructuralTypeId::new(index);
    let layout = StructuralLayoutId::new(index);
    program.memory.layouts.push(StructuralLayoutMetadata {
        id: layout,
        identity: RuntimeLayoutId::new([index.saturating_add(1) as u8; 32]),
        kind,
    });
    let witness = MemoryWitnessId::new([index.saturating_add(1) as u8; 32]);
    program.memory.types.push(StructuralTypeMetadata {
        id: type_id,
        witness,
        ty,
        layout,
        mode: StructuralTypeMode::Immutable,
    });
    for (category, storage) in [
        (
            StructuralValueCategory::Owner,
            StructuralStorage::UniqueStructural,
        ),
        (
            StructuralValueCategory::View,
            StructuralStorage::BorrowedView,
        ),
        (
            StructuralValueCategory::Destination,
            StructuralStorage::UniqueStructural,
        ),
    ] {
        let id = u16::try_from(program.memory.representations.len()).unwrap_or(u16::MAX);
        program
            .memory
            .representations
            .push(StructuralRepresentationMetadata {
                id: StructuralRepresentationId::new(id),
                type_id,
                witness,
                witness_group: MemoryWitnessGroupId::new([0; 32]),
                witness_member: 0,
                layout,
                category,
                storage,
                route: if category == StructuralValueCategory::View {
                    [2; 32]
                } else {
                    [1; 32]
                },
            });
    }
}

pub(super) fn allocating_metadata(position: u32, effects: EffectSet) -> InstructionMetadata {
    InstructionMetadata {
        origin: Origin::SYNTHETIC,
        effects,
        failure: match (
            effects.contains(EffectSet::MAY_TRAP),
            effects.contains(EffectSet::MAY_EXIT) || effects.contains(EffectSet::ALLOCATES),
        ) {
            (false, false) => FailureBehavior::None,
            (true, false) => FailureBehavior::Trap,
            (false, true) => FailureBehavior::StructuredOutcome,
            (true, true) => FailureBehavior::TrapOrOutcome,
        },
        failure_cleanup: None,
        frame_state: Some(FrameState {
            bytecode_position: position,
            locals: Vec::new(),
            operand_stack: Vec::new(),
        }),
    }
}

pub(super) fn allocating_cleanup_metadata(
    position: u32,
    effects: EffectSet,
    cleanup: u32,
) -> InstructionMetadata {
    let mut metadata = allocating_metadata(position, effects);
    metadata.failure_cleanup = Some(FailureCleanupRoots::single(FailureCleanupId::new(
        u64::from(cleanup),
    )));
    metadata
}
