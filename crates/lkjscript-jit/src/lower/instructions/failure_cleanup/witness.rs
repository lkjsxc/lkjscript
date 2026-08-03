use super::*;

pub(in crate::lower::instructions) fn call_result_witness_slot(
    function: &Function,
    value: ValueId,
    layouts: &LayoutInterner,
) -> Result<u16, LoweringError> {
    let instruction = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find(|instruction| instruction.id == value)
        .ok_or_else(|| invalid_structural("dynamic cleanup source is absent"))?;
    let InstructionKind::Call {
        instantiation: Some(instantiation),
        ..
    } = &instruction.kind
    else {
        return Err(invalid_structural(
            "dynamic cleanup source is not an authenticated call",
        ));
    };
    let mut parameters = instantiation
        .substitutions
        .iter()
        .filter(|substitution| substitution.ty == instruction.ty)
        .map(|substitution| substitution.parameter.as_str());
    let parameter = parameters
        .next()
        .filter(|_| parameters.next().is_none())
        .ok_or_else(|| invalid_structural("dynamic cleanup result parameter is ambiguous"))?;
    let witness = instantiation
        .memory_witnesses
        .iter()
        .find(|binding| binding.parameter == parameter)
        .ok_or_else(|| invalid_structural("dynamic cleanup witness is absent"))?;
    let storage = layouts.structural().owner_storage(function, value)?;
    layouts
        .witness_slot(witness.witness, storage)
        .ok_or_else(|| invalid_structural("dynamic cleanup witness slot is absent"))
}
