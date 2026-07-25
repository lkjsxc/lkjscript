use super::*;

pub fn result_ok(arena: &mut Arena, value: Value) -> Result<Value> {
    arena.alloc(HeapObj::ResultOk(value))
}

pub fn result_err(arena: &mut Arena, value: Value) -> Result<Value> {
    arena.alloc(HeapObj::ResultErr(value))
}

pub fn language_result(arena: &mut Arena, result: Result<Value>) -> Result<Value> {
    match result {
        Ok(value) => result_ok(arena, value),
        Err(error) => {
            let message = arena.alloc(HeapObj::Str(error.to_string()))?;
            result_err(arena, message)
        }
    }
}

pub fn is_ok(arena: &Arena, value: Value) -> Result<Value> {
    match arena.get(value)? {
        HeapObj::ResultOk(_) => Ok(Value::TRUE),
        HeapObj::ResultErr(_) => Ok(Value::FALSE),
        _ => Err(Error::msg("is-ok: expected Result")),
    }
}

pub fn unwrap_ok(arena: &Arena, value: Value) -> Result<Value> {
    match arena.get(value)? {
        HeapObj::ResultOk(inner) => Ok(*inner),
        HeapObj::ResultErr(error) => {
            let message = match arena.get(*error) {
                Ok(HeapObj::Str(message)) => format!("unwrap-ok: {message}"),
                _ => "unwrap-ok on Err".to_string(),
            };
            Err(Error::msg(message))
        }
        _ => Err(Error::msg("unwrap-ok: expected Result")),
    }
}

pub fn unwrap_err(arena: &Arena, value: Value) -> Result<Value> {
    match arena.get(value)? {
        HeapObj::ResultErr(inner) => Ok(*inner),
        HeapObj::ResultOk(_) => Err(Error::msg("unwrap-err on Ok")),
        _ => Err(Error::msg("unwrap-err: expected Result")),
    }
}

pub fn option_some(arena: &mut Arena, value: Value) -> Result<Value> {
    arena.alloc(HeapObj::OptionSome(value))
}

pub fn is_some(arena: &Arena, value: Value) -> Result<Value> {
    if value.is_none() {
        return Ok(Value::FALSE);
    }
    match arena.get(value)? {
        HeapObj::OptionSome(_) => Ok(Value::TRUE),
        _ => Err(Error::msg("is-some: expected Option")),
    }
}

pub fn unwrap_some(arena: &Arena, value: Value) -> Result<Value> {
    if value.is_none() {
        return Err(Error::msg("unwrap-some on none"));
    }
    match arena.get(value)? {
        HeapObj::OptionSome(inner) => Ok(*inner),
        _ => Err(Error::msg("unwrap-some: expected Option")),
    }
}
