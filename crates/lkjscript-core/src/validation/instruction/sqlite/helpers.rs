fn pop_leaf(
    chunk: &Chunk,
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    structural: StructuralKind,
    legacy: Kind,
) -> Result<()> {
    pop_structural_leaf(chunk, state, structural, legacy, proto, instruction)
}

fn connection_result(
    chunk: &Chunk,
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    success: StructuralKind,
) -> Result<()> {
    connection(state, proto, instruction)?;
    push_result(chunk, state, proto, instruction, success)
}

fn statement_result(
    chunk: &Chunk,
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    success: StructuralKind,
) -> Result<()> {
    statement(state, proto, instruction)?;
    push_result(chunk, state, proto, instruction, success)
}

fn bind_result(
    chunk: &Chunk,
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    expect_pop(state, Kind::I64, proto, instruction)?;
    statement_result(chunk, state, proto, instruction, StructuralKind::Unit)
}

fn column_result(
    chunk: &Chunk,
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    success: StructuralKind,
) -> Result<()> {
    expect_pop(state, Kind::I64, proto, instruction)?;
    statement_result(chunk, state, proto, instruction, success)
}

fn column_option_result(
    chunk: &Chunk,
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    item: StructuralKind,
) -> Result<()> {
    expect_pop(state, Kind::I64, proto, instruction)?;
    statement(state, proto, instruction)?;
    state
        .stack
        .push(structural_option_result(chunk, item, proto, instruction)?);
    Ok(())
}

fn push_result(
    chunk: &Chunk,
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    success: StructuralKind,
) -> Result<()> {
    state
        .stack
        .push(structural_value_result(chunk, success, proto, instruction)?);
    Ok(())
}

fn connection(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    expect_resource(state, &[ResourceKind::SqliteConnection], proto, instruction)
}

fn statement(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    expect_resource(state, &[ResourceKind::SqliteStatement], proto, instruction)
}
