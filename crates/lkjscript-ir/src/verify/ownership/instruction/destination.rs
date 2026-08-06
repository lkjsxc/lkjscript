fn process_structural_publish(
    value: &crate::ValueId,
    state: &mut OwnershipState,
) -> crate::Result<()> {
    if current_owner_place(state, *value).is_some() {
        return fail("SSA structural publish cannot republish a placed owner");
    }
    state.affine_mut().remove(value);
    Ok(())
}

fn process_destination_field_init(
    program: &Program,
    destination: &crate::ValueId,
    value: &crate::ValueId,
    state: &mut OwnershipState,
    types: &[SsaType],
) -> crate::Result<()> {
    state.affine_mut().remove(destination).ok_or_else(|| {
        IrError::new("SSA destination field init consumes unavailable destination")
    })?;
    if is_affine(program, value_type(types, *value)?) {
        state.affine_mut().remove(value).ok_or_else(|| {
            IrError::new("SSA destination field init consumes unavailable field owner")
        })?;
    }
    Ok(())
}

fn process_destination_terminal(
    destination: &crate::ValueId,
    state: &mut OwnershipState,
) -> crate::Result<()> {
    state.affine_mut().remove(destination).ok_or_else(|| {
        IrError::new("SSA destination terminal operation consumes unavailable destination")
    })?;
    Ok(())
}
