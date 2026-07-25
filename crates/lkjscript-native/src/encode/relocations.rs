use super::*;

pub(super) fn patch_relative(
    bytes: &mut [u8],
    displacement_offset: usize,
    target: usize,
) -> Result<(), NativeError> {
    let after = displacement_offset
        .checked_add(4)
        .ok_or(NativeError::Encode(EncodeError::InvalidRelocation))?;
    if after > bytes.len() {
        return Err(NativeError::Encode(EncodeError::InvalidRelocation));
    }
    let target =
        i64::try_from(target).map_err(|_| NativeError::Encode(EncodeError::InvalidRelocation))?;
    let after =
        i64::try_from(after).map_err(|_| NativeError::Encode(EncodeError::InvalidRelocation))?;
    let displacement = i32::try_from(target - after)
        .map_err(|_| NativeError::Encode(EncodeError::InvalidRelocation))?;
    let end = displacement_offset + 4;
    let destination = bytes
        .get_mut(displacement_offset..end)
        .ok_or(NativeError::Encode(EncodeError::InvalidRelocation))?;
    destination.copy_from_slice(&displacement.to_le_bytes());
    Ok(())
}

pub(super) fn integer_condition(comparison: I64Comparison) -> u8 {
    match comparison {
        I64Comparison::Equal => 0x94,
        I64Comparison::NotEqual => 0x95,
        I64Comparison::LessThan => 0x9c,
        I64Comparison::LessThanOrEqual => 0x9e,
        I64Comparison::GreaterThan => 0x9f,
        I64Comparison::GreaterThanOrEqual => 0x9d,
    }
}

pub(super) fn trap_index(trap: TrapCode) -> usize {
    match trap {
        TrapCode::I64Overflow => 0,
        TrapCode::DivisionByZero => 1,
        TrapCode::Explicit => 2,
    }
}

pub(super) fn to_u32(value: usize) -> Result<u32, NativeError> {
    u32::try_from(value).map_err(|_| NativeError::Encode(EncodeError::LimitExceeded("u32 offset")))
}
