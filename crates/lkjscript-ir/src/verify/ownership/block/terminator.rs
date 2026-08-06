fn process_ownership_terminator(
    program: &Program,
    function: &Function,
    block: &Block,
    state: &OwnershipState,
    types: &[SsaType],
    nonowned_affine: &HashSet<ValueId>,
) -> crate::Result<Vec<(BlockId, OwnershipState)>> {
    let terminal = !matches!(
        block.terminator,
        Terminator::Branch { .. } | Terminator::ConditionalBranch { .. }
    );
    let incomplete = !state.pending_drops.is_empty()
        || state
            .owners
            .values()
            .any(|value| value_type(types, *value).is_ok_and(|ty| is_owned_value(program, ty)));
    if terminal && !matches!(block.terminator, Terminator::Return(_)) && incomplete {
        return fail(format!(
            "SSA structured terminator {:?} in block {} has incomplete cleanup owners {:?} pending {:?}",
            block.terminator, block.id.raw(), state.owners, state.pending_drops
        ));
    }

    let edge_context = EdgeTransferContext {
        program,
        function,
        types,
        nonowned_affine,
    };
    match &block.terminator {
        Terminator::Branch { target, arguments } => Ok(vec![(
            *target,
            transfer_edge(&edge_context, state, *target, arguments)?,
        )]),
        Terminator::ConditionalBranch {
            true_target,
            true_arguments,
            false_target,
            false_arguments,
            ..
        } => Ok(vec![
            (
                *true_target,
                transfer_edge(&edge_context, state, *true_target, true_arguments)?,
            ),
            (
                *false_target,
                transfer_edge(&edge_context, state, *false_target, false_arguments)?,
            ),
        ]),
        Terminator::Return(value) => {
            if is_affine(program, value_type(types, *value)?) && !nonowned_affine.contains(value) {
                let fact = state
                    .affine
                    .get(value)
                    .ok_or_else(|| IrError::new("SSA Return reuses an unavailable affine value"))?;
                if is_owned_value(program, value_type(types, *value)?) {
                    if state.owners.values().any(|owner| owner == value) {
                        return fail(
                            "SSA cannot return an Owned place value without explicit Move",
                        );
                    }
                    if !fact.transferred && !matches!(fact.provenance, AffineProvenance::Fresh(_)) {
                        return fail("SSA affine return lacks explicit Move transfer provenance");
                    }
                }
            }
            if !state.pending_drops.is_empty()
                || state.owners.values().any(|owner| {
                    value_type(types, *owner).is_ok_and(|ty| is_owned_value(program, ty))
                })
            {
                return fail("SSA Return has incomplete whole-place cleanup");
            }
            Ok(Vec::new())
        }
        Terminator::Exit { code } => {
            verify_terminator_affine_available(program, state, [*code], types, nonowned_affine)?;
            Ok(Vec::new())
        }
        Terminator::Outcome { detail, .. } => {
            verify_terminator_affine_available(
                program,
                state,
                detail.iter().copied(),
                types,
                nonowned_affine,
            )?;
            Ok(Vec::new())
        }
        Terminator::Trap { .. } => Ok(Vec::new()),
    }
}
