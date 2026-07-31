pub(super) fn register_instruction_result(
    program: &Program,
    instruction: &Instruction,
    state: &mut OwnershipState,
) {
    let static_bytes = matches!(
        instruction.kind,
        InstructionKind::Constant(crate::Constant::StaticBytes(_))
    );
    let borrowed_bytes = matches!(instruction.kind, InstructionKind::Borrow { .. })
        && instruction.ty == SsaType::Bytes;
    let structural_view = matches!(
        instruction.kind,
        InstructionKind::Borrow { .. } | InstructionKind::AggregateFieldBorrow { .. }
    ) && program.memory.type_for(&instruction.ty).is_some();
    let borrowed_resource = matches!(
        instruction.kind,
        InstructionKind::Runtime {
            operation: crate::RuntimeOp::StdinHandle,
            ..
        }
    );
    if is_owned_value(program, &instruction.ty)
        && !matches!(instruction.kind, InstructionKind::Move { .. })
        && !static_bytes
        && !borrowed_bytes
        && !structural_view
        && !borrowed_resource
    {
        state.affine.insert(
            instruction.id,
            AffineFact {
                provenance: AffineProvenance::Fresh(instruction.id),
                transferred: false,
            },
        );
    }
}
