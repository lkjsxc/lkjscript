use super::*;

pub fn buf_ref(arena: &Arena, value: Value, index: i64) -> Result<i64> {
    let index = buffer_index(index, "buf-byte-at")?;
    let byte = *as_buf(arena, value)?
        .get(index)
        .ok_or_else(|| Error::msg("buf-ref out of bounds"))?;
    Ok(i64::from(byte))
}

pub fn buf_set(arena: &mut Arena, value: Value, index: i64, byte: i64) -> Result<Value> {
    let index = buffer_index(index, "buf-set-byte")?;
    let byte = u8::try_from(byte).map_err(|_| Error::msg("buf-set byte out of range"))?;
    arena.mutate(value, |object| {
        let HeapObj::Buf(buffer) = object else {
            return Err(Error::msg("expected buf"));
        };
        let slot = buffer
            .get_mut(index)
            .ok_or_else(|| Error::msg("buf-set out of bounds"))?;
        *slot = byte;
        Ok(Value::UNIT)
    })
}

pub fn buf_get_u32(arena: &Arena, value: Value, index: i64) -> Result<i64> {
    let index = buffer_index(index, "buf-read-u32")?;
    let end = index
        .checked_add(4)
        .ok_or_else(|| Error::msg("buf-get-u32 index overflow"))?;
    let bytes = as_buf(arena, value)?
        .get(index..end)
        .ok_or_else(|| Error::msg("buf-get-u32 out of bounds"))?;
    let mut word = [0_u8; 4];
    word.copy_from_slice(bytes);
    Ok(i64::from(u32::from_le_bytes(word)))
}

pub fn buf_set_u32(arena: &mut Arena, value: Value, index: i64, number: i64) -> Result<Value> {
    let index = buffer_index(index, "buf-write-u32")?;
    let end = index
        .checked_add(4)
        .ok_or_else(|| Error::msg("buf-set-u32 index overflow"))?;
    let number = u32::try_from(number).map_err(|_| Error::msg("buf-set-u32 value out of range"))?;
    arena.mutate(value, |object| {
        let HeapObj::Buf(buffer) = object else {
            return Err(Error::msg("expected buf"));
        };
        let destination = buffer
            .get_mut(index..end)
            .ok_or_else(|| Error::msg("buf-set-u32 out of bounds"))?;
        destination.copy_from_slice(&number.to_le_bytes());
        Ok(Value::UNIT)
    })
}

pub(crate) fn buffer_range(
    arena: &Arena,
    buffer: Value,
    offset: i64,
    requested: i64,
    operation: &str,
) -> Result<std::ops::Range<usize>> {
    let offset = usize::try_from(offset)
        .map_err(|_| Error::msg(format!("{operation} offset out of range")))?;
    let requested = usize::try_from(requested)
        .map_err(|_| Error::msg(format!("{operation} length out of range")))?;
    let end = offset
        .checked_add(requested)
        .ok_or_else(|| Error::msg(format!("{operation} range overflow")))?;
    if end > as_buf(arena, buffer)?.len() {
        return Err(Error::msg(format!("{operation} range out of bounds")));
    }
    Ok(offset..end)
}

pub(crate) fn bulk_range(
    arena: &Arena,
    buffer: Value,
    offset: i64,
    requested: i64,
    operation: &str,
) -> Result<std::ops::Range<usize>> {
    let range = buffer_range(arena, buffer, offset, requested, operation)?;
    if range.len() > MAX_BULK_IO_BYTES {
        return Err(Error::msg(format!(
            "{operation} length exceeds bulk I/O limit"
        )));
    }
    Ok(range)
}

pub(crate) fn buffer_index(index: i64, operation: &str) -> Result<usize> {
    usize::try_from(index).map_err(|_| Error::msg(format!("{operation} index out of range")))
}
