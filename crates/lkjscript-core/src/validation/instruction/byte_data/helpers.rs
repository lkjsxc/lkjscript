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

fn new_owner(instruction: DecodedInstruction) -> Result<u32> {
    u32::try_from(instruction.offset())
        .ok()
        .and_then(|offset| offset.checked_add(0x6800_0001))
        .ok_or_else(|| Error::msg("conversion bytes owner identity overflow"))
}
