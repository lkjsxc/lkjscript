use super::*;

pub(in crate::run) fn is_byte_view<J: RuntimeTier>(vm: &Vm<'_, J>, value: Value) -> bool {
    value.as_structural_view().is_some_and(|word| {
        vm.structural
            .as_ref()
            .and_then(|structural| structural.views.get(&word))
            .is_some_and(|record| record.utf8)
    })
}

pub(in crate::run) fn len<J: RuntimeTier>(vm: &Vm<'_, J>, value: Value) -> Result<i64> {
    let (key, record) = invocation(vm)?.view(value)?;
    if !record.utf8 {
        return Err(Error::msg("structural view is not a UTF-8 byte view"));
    }
    let text = invocation(vm)?
        .runtime
        .utf8_view(key)
        .map_err(map_value_error)?;
    i64::try_from(text.len()).map_err(|_| Error::msg("structural UTF-8 view length exceeds i64"))
}

pub(in crate::run) fn byte_at<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    value: Value,
    index: i64,
) -> Result<i64> {
    let index = usize::try_from(index)
        .map_err(|_| Error::msg("structural UTF-8 byte index is out of range"))?;
    let (key, record) = invocation(vm)?.view(value)?;
    if !record.utf8 {
        return Err(Error::msg("structural view is not a UTF-8 byte view"));
    }
    invocation(vm)?
        .runtime
        .utf8_view(key)
        .map_err(map_value_error)?
        .as_bytes()
        .get(index)
        .copied()
        .map(i64::from)
        .ok_or_else(|| Error::msg("structural UTF-8 byte index is out of bounds"))
}

pub(in crate::run) fn read_u32_little_endian<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    value: Value,
    index: i64,
) -> Result<i64> {
    let index = usize::try_from(index)
        .map_err(|_| Error::msg("structural UTF-8 u32 index is out of range"))?;
    let end = index
        .checked_add(4)
        .ok_or_else(|| Error::msg("structural UTF-8 u32 range overflow"))?;
    let (key, record) = invocation(vm)?.view(value)?;
    if !record.utf8 {
        return Err(Error::msg("structural view is not a UTF-8 byte view"));
    }
    let bytes = invocation(vm)?
        .runtime
        .utf8_view(key)
        .map_err(map_value_error)?
        .as_bytes();
    let word: [u8; 4] = bytes
        .get(index..end)
        .ok_or_else(|| Error::msg("structural UTF-8 u32 read is out of bounds"))?
        .try_into()
        .map_err(|_| Error::msg("structural UTF-8 u32 read has invalid width"))?;
    Ok(i64::from(u32::from_le_bytes(word)))
}

pub(in crate::run) fn end_byte_view<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    value: Value,
) -> Result<()> {
    let (key, record) = invocation(vm)?.view(value)?;
    if !record.utf8 {
        return Err(Error::msg("structural view is not a UTF-8 byte view"));
    }
    invocation_mut(vm)?
        .runtime
        .end_view(key)
        .map_err(map_value_error)?;
    invocation_mut(vm)?.views.remove(&key.get());
    Ok(())
}
