use super::*;

pub fn as_str(arena: &Arena, value: Value) -> Result<&str> {
    match arena.get(value)? {
        HeapObj::Str(text) | HeapObj::Symbol(text) => Ok(text.as_str()),
        _ => Err(Error::msg("expected string")),
    }
}

pub fn str_len(arena: &Arena, value: Value) -> Result<i64> {
    i64::try_from(as_str(arena, value)?.len()).map_err(|_| Error::msg("str-len out of range"))
}

pub fn str_ref(arena: &Arena, string: Value, index: i64) -> Result<i64> {
    let index = usize::try_from(index).map_err(|_| Error::msg("str-ref index out of range"))?;
    let byte = *as_str(arena, string)?
        .as_bytes()
        .get(index)
        .ok_or_else(|| Error::msg("str-ref out of bounds"))?;
    Ok(i64::from(byte))
}

pub fn str_append(arena: &mut Arena, left: Value, right: Value) -> Result<Value> {
    let mut output = as_str(arena, left)?.to_string();
    output.push_str(as_str(arena, right)?);
    arena.alloc(HeapObj::Str(output))
}

pub fn str_slice(arena: &mut Arena, string: Value, start: i64, end: i64) -> Result<Value> {
    let start = usize::try_from(start).map_err(|_| Error::msg("str-slice start out of range"))?;
    let end = usize::try_from(end).map_err(|_| Error::msg("str-slice end out of range"))?;
    let bytes = as_str(arena, string)?.as_bytes();
    if start > end || end > bytes.len() {
        return Err(Error::msg("str-slice out of bounds"));
    }
    let text = std::str::from_utf8(&bytes[start..end])
        .map_err(|_| Error::msg("str-slice splits UTF-8"))?;
    arena.alloc(HeapObj::Str(text.to_string()))
}

pub fn str_from_byte(arena: &mut Arena, number: i64) -> Result<Value> {
    let byte = u8::try_from(number).map_err(|_| Error::msg("str-from-byte out of range"))?;
    arena.alloc(HeapObj::Str(String::from(char::from(byte))))
}

pub fn str_from_i64(arena: &mut Arena, number: i64) -> Result<Value> {
    arena.alloc(HeapObj::Str(number.to_string()))
}

pub fn str_from_f64(arena: &mut Arena, number: Value) -> Result<Value> {
    let number = number
        .as_f64()
        .ok_or_else(|| Error::msg("str-from-f64 expects F64"))?;
    arena.alloc(HeapObj::Str(number.to_string()))
}
