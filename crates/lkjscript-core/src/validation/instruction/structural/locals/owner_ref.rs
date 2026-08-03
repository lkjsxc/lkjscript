fn load_owner_ref(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let slot = instruction_operand(proto, instruction)?;
    let value = state.locals.get(slot).copied().flatten().ok_or_else(|| {
        instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural owner local is empty",
        )
    })?;
    if value == Kind::Any {
        state.stack.push(Kind::Any);
        return Ok(());
    }
    let (representation, owner, active_variant) = match value {
        Kind::StructuralOwner {
            representation,
            owner,
            active_variant,
        }
        | Kind::StructuralOwnerRef {
            representation,
            owner,
            active_variant,
        } => (representation, owner, active_variant),
        _ => {
            return fail(
                proto,
                instruction,
                "structural owner reference expects an owner",
            )
        }
    };
    state.stack.push(Kind::StructuralOwnerRef {
        representation,
        owner,
        active_variant,
    });
    Ok(())
}
