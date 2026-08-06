fn pop_bytes(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let kind = pop(state, proto, instruction)?;
    if matches!(
        kind,
        Kind::StaticBytes | Kind::Bytes(_) | Kind::BytesBorrow { used: true, .. }
    ) {
        Ok(())
    } else {
        Err(Error::msg("conversion expects exact immutable bytes"))
    }
}

fn direct_owner(
    chunk: &Chunk,
    kind: crate::StructuralKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    structural_leaf_owner(chunk, kind, proto, instruction)
}

const fn new_owner(instruction: DecodedInstruction) -> Result<OwnerIdentity> {
    Ok(OwnerIdentity::instruction(instruction.offset(), 1))
}
