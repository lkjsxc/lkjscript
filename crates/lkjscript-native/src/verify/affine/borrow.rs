use super::*;

pub(super) fn check_borrow_conflicts(
    function: &FunctionPlan,
    instruction: &crate::plan::Instruction,
    values: &[bool],
) -> Result<(), VerificationError> {
    let (owner, exclusive) = match &instruction.operation {
        Operation::StructuralCall(descriptor, arguments) => match descriptor.operation() {
            StructuralOperation::Borrow { projection } => (
                arguments.first().copied(),
                projection.view_type().exclusive(),
            ),
            _ => return Ok(()),
        },
        Operation::RuntimeCall(slot, arguments) => match slot {
            RuntimeCallSlot::ByteVectorBorrowShared | RuntimeCallSlot::BytesBorrowShared => {
                (arguments.first().copied(), false)
            }
            RuntimeCallSlot::ByteVectorBorrowExclusive => (arguments.first().copied(), true),
            _ => return Ok(()),
        },
        _ => return Ok(()),
    };
    let Some(owner) = owner else {
        return Ok(());
    };
    for fact in &function.values {
        if values.get(fact.id.index as usize).copied().unwrap_or(false)
            && borrowed_from(function, fact.id) == Some(owner)
            && (exclusive || borrowed_exclusively(function, fact.id))
        {
            return Err(VerificationError::BorrowConflict(owner));
        }
    }
    Ok(())
}

pub(super) fn reject_live_borrow(
    function: &FunctionPlan,
    owner: ValueId,
    values: &[bool],
) -> Result<(), VerificationError> {
    if function.values.iter().any(|fact| {
        values.get(fact.id.index as usize).copied().unwrap_or(false)
            && borrowed_from(function, fact.id) == Some(owner)
    }) {
        return Err(VerificationError::LiveLoan(owner));
    }
    Ok(())
}

fn borrowed_from(function: &FunctionPlan, value: ValueId) -> Option<ValueId> {
    let instruction = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find(|instruction| instruction.output == value)?;
    match &instruction.operation {
        Operation::StructuralCall(descriptor, arguments) => match descriptor.operation() {
            StructuralOperation::Borrow { .. } => arguments.first().copied(),
            _ => None,
        },
        Operation::RuntimeCall(
            RuntimeCallSlot::ByteVectorBorrowShared
            | RuntimeCallSlot::ByteVectorBorrowExclusive
            | RuntimeCallSlot::BytesBorrowShared,
            arguments,
        ) => arguments.first().copied(),
        _ => None,
    }
}

fn borrowed_exclusively(function: &FunctionPlan, value: ValueId) -> bool {
    matches!(
        value_type(function, value),
        Ok(ValueType::StructuralView(view)) if view.exclusive()
    ) || matches!(
        value_type(function, value),
        Ok(ValueType::Loan(LoanType::ByteSliceMut))
    )
}
