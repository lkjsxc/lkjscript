use super::*;

pub fn sys_read_into(
    arena: &mut Arena,
    handles: &ResourceTable,
    handle: Value,
    buffer: Value,
    offset: i64,
    requested: i64,
) -> Result<i64> {
    let range = bulk_range(arena, buffer, offset, requested, "read-into")?;
    let count = arena.mutate(buffer, |object| {
        let HeapObj::Buf(bytes) = object else {
            return Err(Error::msg("expected buf"));
        };
        let destination = bytes
            .get_mut(range)
            .ok_or_else(|| Error::msg("sys-read-into range is invalid"))?;
        handles.read_into(handle, destination)
    })?;
    i64::try_from(count).map_err(|_| Error::msg("sys-read-into count out of range"))
}

pub fn sys_write_from(
    arena: &Arena,
    handles: &ResourceTable,
    handle: Value,
    buffer: Value,
    offset: i64,
    requested: i64,
) -> Result<i64> {
    let range = bulk_range(arena, buffer, offset, requested, "write-from")?;
    let source = as_buf(arena, buffer)?
        .get(range)
        .ok_or_else(|| Error::msg("sys-write-from range is invalid"))?;
    let count = handles.write_from(handle, source)?;
    i64::try_from(count).map_err(|_| Error::msg("sys-write-from count out of range"))
}

pub fn sys_random_fill(
    arena: &mut Arena,
    buffer: Value,
    offset: i64,
    requested: i64,
) -> Result<Value> {
    let range = bulk_range(arena, buffer, offset, requested, "fill-random")?;
    arena.mutate(buffer, |object| {
        let HeapObj::Buf(bytes) = object else {
            return Err(Error::msg("expected buf"));
        };
        let destination = bytes
            .get_mut(range)
            .ok_or_else(|| Error::msg("sys-random-fill range is invalid"))?;
        lkjscript_sys::random_fill(destination)
            .map_err(|error| Error::msg(format!("sys-random-fill: {error}")))?;
        Ok(Value::UNIT)
    })
}

pub fn sys_sha256(arena: &mut Arena, buffer: Value, offset: i64, requested: i64) -> Result<Value> {
    let range = bulk_range(arena, buffer, offset, requested, "sha256")?;
    let source = as_buf(arena, buffer)?
        .get(range)
        .ok_or_else(|| Error::msg("sys-sha256 range is invalid"))?;
    let digest = lkjscript_core::sha256(source);
    arena.alloc(HeapObj::Buf(digest.to_vec()))
}

pub fn sys_tty_get(
    arena: &mut Arena,
    handles: &ResourceTable,
    handle: Value,
    buffer: Value,
) -> Result<Value> {
    let raw = handles.raw_fd(handle, "get-terminal-state")?;
    arena.mutate(buffer, |object| {
        let HeapObj::Buf(state) = object else {
            return Err(Error::msg("expected buf"));
        };
        lkjscript_sys::tty_get(raw, state)
            .map_err(|error| Error::msg(format!("sys-tty-get: {error}")))?;
        Ok(Value::UNIT)
    })
}

pub fn sys_tty_set(
    arena: &Arena,
    handles: &ResourceTable,
    handle: Value,
    buffer: Value,
) -> Result<Value> {
    let raw = handles.raw_fd(handle, "set-terminal-state")?;
    let state = as_buf(arena, buffer)?;
    lkjscript_sys::tty_set(raw, state)
        .map_err(|error| Error::msg(format!("sys-tty-set: {error}")))?;
    Ok(Value::UNIT)
}

pub fn sys_poll(handles: &ResourceTable, handle: Value, timeout: i64) -> Result<i64> {
    let raw = handles.raw_fd(handle, "poll-streams")?;
    let timeout =
        i32::try_from(timeout).map_err(|_| Error::msg("sys-poll timeout out of range"))?;
    if timeout < 0 {
        return Err(Error::msg("sys-poll timeout out of range"));
    }
    let ready = lkjscript_sys::poll_fd(raw, timeout)
        .map_err(|error| Error::msg(format!("sys-poll: {error}")))?;
    Ok(i64::from(ready))
}

pub fn stdin_handle() -> Value {
    ResourceTable::stdin_handle()
}

pub fn sys_isatty(handles: &ResourceTable, handle: Value) -> Result<Value> {
    let raw = handles.raw_fd(handle, "is-terminal")?;
    Ok(Value::from_bool(lkjscript_sys::is_tty(raw)))
}

pub fn sys_tty_guard_save(arena: &Arena, buffer: Value) -> Result<Value> {
    let state = as_buf(arena, buffer)?;
    lkjscript_sys::tty_guard_save(state)
        .map_err(|error| Error::msg(format!("sys-tty-guard-save: {error}")))?;
    Ok(Value::UNIT)
}

pub fn sys_tty_guard_clear() -> Result<Value> {
    lkjscript_sys::tty_guard_clear()
        .map_err(|error| Error::msg(format!("sys-tty-guard-clear: {error}")))?;
    Ok(Value::UNIT)
}
