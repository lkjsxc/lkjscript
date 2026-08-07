use super::*;

pub(crate) fn reachable_group(
    program: &lkjscript_ir::Program,
    root: FunctionId,
) -> Result<Vec<FunctionId>, LoweringError> {
    let mut marks = vec![0_u8; program.functions.len()];
    let mut reached = Vec::new();
    visit(program, root, &mut marks, &mut reached)?;
    reached.sort_by_key(|function| function.raw());
    Ok(reached)
}

pub(super) fn visit(
    program: &lkjscript_ir::Program,
    function: FunctionId,
    marks: &mut [u8],
    reached: &mut Vec<FunctionId>,
) -> Result<(), LoweringError> {
    let index = function.index().ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function),
            "function ID is outside the verified program",
        )
    })?;
    let mark = marks.get(index).copied().ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::InvalidFunction,
            Some(function),
            "function ID is outside the verified program",
        )
    })?;
    if mark == 2 {
        return Ok(());
    }
    if mark == 1 {
        // The declaration pass precedes every definition, so direct and mutual
        // recursion are ordinary bounded native calls within one installed SCC.
        return Ok(());
    }
    marks[index] = 1;
    let item = program
        .functions
        .get(index)
        .filter(|item| item.id == function)
        .ok_or_else(|| {
            LoweringError::new(
                LoweringFailureCode::InvalidFunction,
                Some(function),
                "verified function storage is inconsistent",
            )
        })?;
    for block in &item.blocks {
        for instruction in &block.instructions {
            if let InstructionKind::Call { target, .. } = &instruction.kind {
                match target {
                    CallTarget::Direct(callee) => visit(program, *callee, marks, reached)?,
                    CallTarget::Indirect(_) => {
                        return Err(LoweringError::new(
                            LoweringFailureCode::IndirectCall,
                            Some(function),
                            "indirect native calls are unsupported",
                        ));
                    }
                }
            }
        }
    }
    marks[index] = 2;
    reached.push(function);
    Ok(())
}
