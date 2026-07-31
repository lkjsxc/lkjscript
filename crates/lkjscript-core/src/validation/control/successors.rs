fn successors(
    proto: &FunctionProto,
    instructions: &[DecodedInstruction],
    by_offset: &HashMap<usize, usize>,
    index: usize,
    instruction: DecodedInstruction,
) -> Result<Vec<usize>> {
    let target = || -> Result<usize> {
        let offset = instruction.operand().map(usize::from).ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "missing jump target",
            )
        })?;
        by_offset.get(&offset).copied().ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "jump target is not an instruction boundary",
            )
        })
    };
    match instruction.op().info().control {
        crate::ControlFlow::Return | crate::ControlFlow::Exit | crate::ControlFlow::Trap => {
            Ok(Vec::new())
        }
        crate::ControlFlow::Jump => Ok(vec![target()?]),
        crate::ControlFlow::Branch => {
            let next = index
                .checked_add(1)
                .filter(|next| *next < instructions.len())
                .ok_or_else(|| {
                    instruction_error(
                        proto,
                        instruction.op(),
                        instruction.offset(),
                        "reachable branch falls through the end of the function",
                    )
                })?;
            Ok(vec![target()?, next])
        }
        crate::ControlFlow::Next => {
            let next = index
                .checked_add(1)
                .filter(|next| *next < instructions.len())
                .ok_or_else(|| {
                    instruction_error(
                        proto,
                        instruction.op(),
                        instruction.offset(),
                        "reachable execution falls through the end of the function",
                    )
                })?;
            Ok(vec![next])
        }
    }
}
