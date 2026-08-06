use super::*;

pub(super) fn build_frame_homes(
    function: &FunctionPlan,
) -> Result<Vec<crate::FrameHome>, NativeError> {
    let mut homes = Vec::with_capacity(function.locals.len() + function.values.len());
    for (index, local) in function.locals.iter().enumerate() {
        homes.push(frame_home(
            FrameHomeKind::Local(u64::try_from(index).map_err(|_| {
                NativeError::Encode(EncodeError::LimitExceeded("local frame identity"))
            })?),
            local.value_type,
            local_home_offset(index)?,
        ));
    }
    for (index, value) in function.values.iter().enumerate() {
        homes.push(frame_home(
            FrameHomeKind::Value(u64::try_from(index).map_err(|_| {
                NativeError::Encode(EncodeError::LimitExceeded("value frame identity"))
            })?),
            value.value_type,
            value_home_offset(function, index)?,
        ));
    }
    Ok(homes)
}

pub(super) fn returned_structural_owner_homes(function: &FunctionPlan) -> Vec<FrameHomeKind> {
    let mut returned = function
        .blocks
        .iter()
        .filter_map(|block| match block.terminator {
            Some(Terminator::Return(value))
                if function.values[value.host_index().unwrap_or(usize::MAX)]
                    .value_type
                    .is_structural_owner() =>
            {
                Some(FrameHomeKind::Value(value.index))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    returned.sort_unstable();
    returned.dedup();
    returned
}

pub(super) fn value_frame_home(
    function: &FunctionPlan,
    value: ValueId,
) -> Result<crate::FrameHome, NativeError> {
    let fact = function
        .values
        .get(value.host_index().unwrap_or(usize::MAX))
        .filter(|fact| fact.id == value)
        .ok_or(NativeError::Encode(EncodeError::InvalidValue))?;
    Ok(frame_home(
        FrameHomeKind::Value(value.index),
        fact.value_type,
        value_home_offset(function, value.host_index().unwrap_or(usize::MAX))?,
    ))
}

pub(super) fn local_home_offset(index: usize) -> Result<i32, NativeError> {
    let slot = 1_usize
        .checked_add(index)
        .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
    slot_offset(slot)
}

pub(super) fn value_home_offset(function: &FunctionPlan, index: usize) -> Result<i32, NativeError> {
    let slot = 1_usize
        .checked_add(function.locals.len())
        .and_then(|base| base.checked_add(index))
        .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
    slot_offset(slot)
}

pub(super) fn calculate_frame_bytes(function: &FunctionPlan) -> Result<u32, NativeError> {
    let slots = 1_usize
        .checked_add(function.locals.len())
        .and_then(|value| value.checked_add(function.values.len()))
        .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
    let bytes = slots
        .checked_mul(8)
        .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
    let aligned = bytes
        .checked_add(15)
        .map(|value| value & !15)
        .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
    if aligned > i32::MAX as usize {
        return Err(NativeError::Encode(EncodeError::FrameTooLarge));
    }
    u32::try_from(aligned).map_err(|_| NativeError::Encode(EncodeError::FrameTooLarge))
}

pub(super) fn maximum_outgoing_arguments(function: &FunctionPlan) -> Result<u8, NativeError> {
    let maximum = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.operation {
            Operation::Call(_, arguments)
            | Operation::RuntimeCall(_, arguments)
            | Operation::StructuralCall(_, arguments) => Some(
                arguments
                    .iter()
                    .filter(|argument| {
                        function
                            .values
                            .get(argument.host_index().unwrap_or(usize::MAX))
                            .map(|fact| fact.value_type != ValueType::Unit)
                            .unwrap_or(false)
                    })
                    .count(),
            ),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    u8::try_from(maximum).map_err(|_| NativeError::Encode(EncodeError::UnsupportedSignature))
}

pub(super) fn slot_offset(slot: usize) -> Result<i32, NativeError> {
    let bytes = slot
        .checked_add(1)
        .and_then(|value| value.checked_mul(8))
        .ok_or(NativeError::Encode(EncodeError::FrameTooLarge))?;
    let bytes =
        i32::try_from(bytes).map_err(|_| NativeError::Encode(EncodeError::FrameTooLarge))?;
    Ok(-bytes)
}
