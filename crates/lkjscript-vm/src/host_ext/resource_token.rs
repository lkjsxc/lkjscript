use super::*;

pub(super) fn reservation_exhausted(error: &ResourceTableError) -> bool {
    matches!(
        error,
        ResourceTableError::LimitExceeded { .. }
            | ResourceTableError::GenerationExhausted { .. }
            | ResourceTableError::AcquisitionSequenceExhausted
    )
}

pub(super) fn encode_key(table: &mut ResourceTable, key: &ResourceKey) -> Result<Value> {
    table
        .tokens
        .try_reserve(1)
        .map_err(|_| Error::msg("resource handle token allocation failed"))?;
    table
        .token_by_identity
        .try_reserve(1)
        .map_err(|_| Error::msg("resource handle identity allocation failed"))?;
    let token = table
        .next_token
        .ok_or_else(|| Error::msg("resource handle token identity exhausted"))?;
    table.next_token = token.get().checked_add(1).and_then(NonZeroU64::new);
    let identity = key.token_parts();
    if table.token_by_identity.contains_key(&identity) {
        return Err(Error::msg("resource handle identity collision"));
    }
    if table.tokens.contains_key(&token.get()) {
        return Err(Error::msg("resource handle token collision"));
    }
    table.tokens.insert(token.get(), identity);
    table.token_by_identity.insert(identity, token.get());
    Ok(Value::from_resource(token.get()))
}

pub(super) fn decode_parts(
    table: &ResourceTable,
    handle: Value,
    operation: &str,
) -> Result<ResourceTokenParts> {
    let token = handle
        .as_resource()
        .ok_or_else(|| Error::msg(format!("{operation}: expected typed resource")))?;
    match table.tokens.get(&token).copied() {
        Some(identity) => Ok(identity),
        None => {
            table.update_metrics(|metrics| {
                metrics.stale_key_failures = metrics.stale_key_failures.saturating_add(1);
            });
            Err(Error::msg(format!(
                "{operation}: stale or forged resource handle"
            )))
        }
    }
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
