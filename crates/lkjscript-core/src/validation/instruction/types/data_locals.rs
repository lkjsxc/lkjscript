fn load_local(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let kind = state.locals.get(slot).copied().flatten().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "local is not definitely initialized",
        )
    })?;
    if !is_affine_resource(kind)
        && is_unique(kind)
        && !matches!(kind, Kind::StructuralOwnerRef { .. })
    {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "unique owners/views require typed local opcodes",
        ));
    }
    state.stack.push(kind);
    Ok(())
}

fn store_local(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = top(state, proto, instruction)?;
    let resource = is_affine_resource(value);
    if !resource && is_unique(value) && !matches!(value, Kind::StructuralOwnerRef { .. }) {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            &format!("unique owners/views require typed local opcodes: {value:?}"),
        ));
    }
    let target = state.locals.get(slot).copied().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "local index is out of range",
        )
    })?;
    if resource && target.is_some_and(|target| target != value) {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "resource store would overwrite a distinct live affine handle",
        ));
    }
    state.set_local(proto, slot, Some(value));
    if resource {
        let top = state.stack.last_mut().ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "resource store lost its operand",
            )
        })?;
        *top = Kind::Unit;
    }
    Ok(())
}
