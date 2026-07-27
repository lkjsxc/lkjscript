use super::*;

pub fn as_buf(arena: &Arena, value: Value) -> Result<&[u8]> {
    match arena.get(value)? {
        HeapObj::Buf(buffer) => Ok(buffer.as_slice()),
        _ => Err(Error::msg("expected buf")),
    }
}

pub fn buf_new(arena: &mut Arena, size: i64) -> Result<Value> {
    if !(0..=MAX_BUFFER_BYTES as i64).contains(&size) {
        return Err(Error::msg("buf-new size out of range"));
    }
    let size = usize::try_from(size).map_err(|_| Error::msg("buf-new size out of range"))?;
    arena.alloc(HeapObj::Buf(vec![0_u8; size]))
}

pub fn buf_len(arena: &Arena, value: Value) -> Result<i64> {
    i64::try_from(as_buf(arena, value)?.len()).map_err(|_| Error::msg("buf-len out of range"))
}

pub fn buf_clone(arena: &mut Arena, value: Value) -> Result<Value> {
    let bytes = as_buf(arena, value)?.to_vec();
    arena.alloc(HeapObj::Buf(bytes))
}

pub fn buf_from_str(arena: &mut Arena, value: Value) -> Result<Value> {
    let bytes = crate::host_ext::as_str(arena, value)?.as_bytes();
    if bytes.len() > MAX_BUFFER_BYTES {
        return Err(Error::msg("buf-from-str string exceeds buffer limit"));
    }
    arena.alloc(HeapObj::Buf(bytes.to_vec()))
}

pub fn buf_to_str(
    arena: &mut Arena,
    value: Value,
) -> Result<std::result::Result<Value, lkjscript_core::Utf8Failure>> {
    let text = match lkjscript_core::validate_utf8(as_buf(arena, value)?) {
        Ok(text) => text.to_owned(),
        Err(error) => return Ok(Err(error)),
    };
    Ok(Ok(arena.alloc(HeapObj::Str(text))?))
}

pub fn buf_slice(arena: &mut Arena, value: Value, offset: i64, length: i64) -> Result<Value> {
    let range = buffer_range(arena, value, offset, length, "copy-buf-slice")?;
    let bytes = as_buf(arena, value)?
        .get(range)
        .ok_or_else(|| Error::msg("buf-slice range is invalid"))?
        .to_vec();
    arena.alloc(HeapObj::Buf(bytes))
}
