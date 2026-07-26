use super::*;

pub const MAX_PATH_BYTES: usize = 4095;

pub fn as_path(arena: &Arena, value: Value) -> Result<&[u8]> {
    match arena.get(value)? {
        HeapObj::Path(path) => Ok(path),
        _ => Err(Error::msg("expected Path")),
    }
}

pub fn path_object(bytes: &[u8]) -> Result<HeapObj> {
    validate_path(bytes)?;
    Ok(HeapObj::Path(copy_bytes(bytes)?))
}

pub fn path_buffer_object(bytes: &[u8]) -> Result<HeapObj> {
    Ok(HeapObj::Buf(copy_bytes(bytes)?))
}

fn copy_bytes(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| Error::msg("Path allocation failed"))?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

#[cfg(test)]
pub fn allocate_path(arena: &mut Arena, bytes: &[u8]) -> Result<Value> {
    arena.alloc(path_object(bytes)?)
}

pub fn validate_path(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.len() > MAX_PATH_BYTES {
        return Err(Error::msg("Path must contain 1 through 4095 bytes"));
    }
    if bytes.first() != Some(&b'/') {
        return Err(Error::msg("Path must be absolute"));
    }
    if bytes.contains(&0) {
        return Err(Error::msg("Path contains an interior NUL"));
    }
    Ok(())
}
