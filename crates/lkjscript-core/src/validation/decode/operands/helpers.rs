fn operand_index(
    operand: crate::DecodedOperand,
    proto: &FunctionProto,
    op: Op,
    at: usize,
) -> Result<usize> {
    operand
        .index()
        .ok_or_else(|| instruction_error(proto, op, at, "missing or malformed decoded index operand"))
}

fn operand_place_local(
    operand: crate::DecodedOperand,
    proto: &FunctionProto,
    op: Op,
    at: usize,
) -> Result<(usize, usize)> {
    operand.place_local().ok_or_else(|| {
        instruction_error(
            proto,
            op,
            at,
            "missing or malformed decoded place/local operand",
        )
    })
}

fn operand_error<T>(proto: &FunctionProto, op: Op, at: usize, message: &str) -> Result<T> {
    Err(instruction_error(proto, op, at, message))
}
