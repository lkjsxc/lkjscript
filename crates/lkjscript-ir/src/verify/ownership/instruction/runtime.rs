fn process_runtime_instruction(
    program: &Program,
    operation: &crate::RuntimeOp,
    arguments: &[crate::ValueId],
    state: &mut OwnershipState,
    types: &[SsaType],
    nonowned_affine: &std::collections::HashSet<crate::ValueId>,
) -> crate::Result<()> {
    let closes = matches!(
        operation,
        crate::RuntimeOp::SysClose
            | crate::RuntimeOp::SysSqliteClose
            | crate::RuntimeOp::SysSqliteFinalize
    );
    let pending = if closes {
        let [value] = arguments else {
            return fail("SSA resource close must consume one exact owner");
        };
        current_owner_place(state, *value).map(|place| (place, *value))
    } else {
        None
    };
    consume_affine_arguments(
        program,
        arguments,
        state,
        types,
        nonowned_affine,
        false,
        closes,
    )?;
    if let Some((place, value)) = pending {
        if state.pending_drops_mut().insert(place, value).is_some() {
            return fail("SSA resource close duplicated a pending Drop event");
        }
    }
    Ok(())
}
