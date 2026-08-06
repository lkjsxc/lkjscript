fn string_utf8_view(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let expected = representation_operand(chunk, proto, instruction)?;
    let input = pop(state, proto, instruction)?;
    let Kind::StructuralOwnerRef {
        representation,
        owner,
        ..
    } = input
    else {
        return fail(
            proto,
            instruction,
            "UTF-8 view expects a structural owner reference",
        );
    };
    require_same_type(chunk, representation, expected.id, proto, instruction)?;
    let ty = chunk
        .structural_types
        .get_structural(expected.type_id)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "structural string type is missing",
            )
        })?;
    if ty.kind != crate::StructuralTypeKind::String {
        return fail(proto, instruction, "UTF-8 view metadata is not a string");
    }
    state.stack.push(Kind::ByteSlice {
        owner,
        mutable: false,
        used: false,
    });
    Ok(())
}
