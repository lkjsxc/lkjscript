fn operand_index(operand: Option<u16>, proto: &FunctionProto, op: Op, at: usize) -> Result<usize> {
    operand
        .map(usize::from)
        .ok_or_else(|| instruction_error(proto, op, at, "missing decoded operand"))
}

fn operand_error<T>(proto: &FunctionProto, op: Op, at: usize, message: &str) -> Result<T> {
    Err(instruction_error(proto, op, at, message))
}
