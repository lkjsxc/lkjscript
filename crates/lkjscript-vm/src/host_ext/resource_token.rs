use super::*;

pub(super) fn reservation_exhausted(error: &ResourceTableError) -> bool {
    matches!(
        error,
        ResourceTableError::LimitExceeded { .. }
            | ResourceTableError::GenerationExhausted { .. }
            | ResourceTableError::AcquisitionSequenceExhausted
    )
}

pub(super) fn encode_key(key: &ResourceKey) -> Result<Value> {
    let parts = key.token_parts();
    let slot = u32::try_from(parts.slot())
        .ok()
        .filter(|slot| *slot <= TOKEN_SLOT_MASK)
        .ok_or_else(|| Error::msg("resource handle slot exhausted"))?;
    let generation = u32::try_from(parts.generation().get())
        .ok()
        .filter(|generation| (1..=TOKEN_GENERATION_MAX).contains(generation))
        .ok_or_else(|| Error::msg("resource handle generation exhausted"))?;
    Ok(Value::from_resource((generation << TOKEN_SLOT_BITS) | slot))
}

pub(super) fn stdin_value() -> Value {
    Value::from_resource(1 << TOKEN_SLOT_BITS)
}

pub(super) fn decode_parts(handle: Value, operation: &str) -> Result<ResourceTokenParts> {
    let token = handle
        .as_resource()
        .ok_or_else(|| Error::msg(format!("{operation}: expected typed resource")))?;
    let slot = usize::try_from(token & TOKEN_SLOT_MASK)
        .map_err(|_| Error::msg(format!("{operation}: invalid resource slot")))?;
    let generation = NonZeroU64::new(u64::from(token >> TOKEN_SLOT_BITS))
        .ok_or_else(|| Error::msg(format!("{operation}: invalid resource generation")))?;
    Ok(ResourceTokenParts::new(slot, generation))
}

pub(super) fn provider_for_kind(kind: ResourceKind) -> ProviderId {
    match kind {
        ResourceKind::InputStream | ResourceKind::OutputStream => STDIO_PROVIDER,
        ResourceKind::FileReader
        | ResourceKind::FileWriter
        | ResourceKind::FileAppender
        | ResourceKind::Directory => FILESYSTEM_PROVIDER,
        ResourceKind::TcpListener | ResourceKind::TcpStream => NETWORK_PROVIDER,
        ResourceKind::SqliteConnection | ResourceKind::SqliteStatement => SQLITE_PROVIDER,
        ResourceKind::TerminalSession => TERMINAL_PROVIDER,
    }
}
