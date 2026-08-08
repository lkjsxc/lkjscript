// `SemanticValue` is an owned tree: aggregate edges are values inside private `Vec` storage.
// Safe Rust cannot point an edge at an ancestor, and no unsafe constructor or aliasing owner exists
// in this module. Graph cycle checks belong to `SemanticDagSnapshot`, not this tree boundary.
pub(super) fn validate_structural_snapshot(
    value: &SemanticValue,
) -> Result<StructuralSnapshotMetrics> {
    let mut tasks = Vec::new();
    tasks
        .try_reserve(1)
        .map_err(|_| Error::msg("structural snapshot validation allocation failed"))?;
    tasks.push(value);
    let mut metrics = StructuralSnapshotMetrics::default();
    while let Some(value) = tasks.pop() {
        metrics.nodes = metrics
            .nodes
            .checked_add(1)
            .ok_or_else(|| Error::msg("structural snapshot node count exceeds u64"))?;
        charge_work(&mut metrics, 1)?;
        let expected = match &value.payload {
            SemanticPayload::Inline(inline) => inline_kind(*inline),
            SemanticPayload::Static(_) => StructuralKind::Static,
            SemanticPayload::String(bytes) => {
                std::str::from_utf8(bytes)
                    .map_err(|_| Error::msg("structural snapshot string is not UTF-8"))?;
                charge_bytes(&mut metrics, bytes.len(), ByteClass::String)?;
                StructuralKind::String
            }
            SemanticPayload::Path(bytes) => {
                validate_snapshot_path(bytes)?;
                charge_bytes(&mut metrics, bytes.len(), ByteClass::Path)?;
                StructuralKind::Path
            }
            SemanticPayload::Bytes(bytes) => {
                charge_bytes(&mut metrics, bytes.len(), ByteClass::Other)?;
                StructuralKind::Bytes
            }
            SemanticPayload::ByteVector(bytes) => {
                charge_bytes(&mut metrics, bytes.len(), ByteClass::Other)?;
                StructuralKind::ByteVector
            }
            SemanticPayload::Product(fields) => {
                schedule_fields(&mut tasks, fields, &mut metrics)?;
                StructuralKind::Product
            }
            SemanticPayload::Enum { active_payload, .. } => {
                schedule_fields(&mut tasks, active_payload, &mut metrics)?;
                StructuralKind::Enum
            }
        };
        require_snapshot_kind(value.value_type, expected)?;
    }
    metrics.decode_work = metrics.encode_work;
    Ok(metrics)
}

fn schedule_fields<'a>(
    tasks: &mut Vec<&'a SemanticValue>,
    fields: &'a [SemanticValue],
    metrics: &mut StructuralSnapshotMetrics,
) -> Result<()> {
    let count = u64::try_from(fields.len())
        .map_err(|_| Error::msg("structural snapshot field count exceeds u64"))?;
    metrics.fields = metrics
        .fields
        .checked_add(count)
        .ok_or_else(|| Error::msg("structural snapshot field count overflow"))?;
    charge_work(metrics, count)?;
    tasks
        .try_reserve(fields.len())
        .map_err(|_| Error::msg("structural snapshot validation allocation failed"))?;
    tasks.extend(fields.iter().rev());
    Ok(())
}

#[derive(Clone, Copy)]
enum ByteClass {
    String,
    Path,
    Other,
}

fn charge_bytes(
    metrics: &mut StructuralSnapshotMetrics,
    length: usize,
    class: ByteClass,
) -> Result<()> {
    let length =
        u64::try_from(length).map_err(|_| Error::msg("structural snapshot byte count overflow"))?;
    metrics.aggregate_bytes = checked_add(metrics.aggregate_bytes, length)?;
    if matches!(class, ByteClass::String) {
        metrics.string_bytes = checked_add(metrics.string_bytes, length)?;
    }
    if matches!(class, ByteClass::Path) {
        metrics.path_bytes = checked_add(metrics.path_bytes, length)?;
    }
    charge_work(metrics, length)
}

fn charge_work(metrics: &mut StructuralSnapshotMetrics, amount: u64) -> Result<()> {
    metrics.encode_work = checked_add(metrics.encode_work, amount)?;
    Ok(())
}

fn checked_add(left: u64, right: u64) -> Result<u64> {
    left.checked_add(right)
        .ok_or_else(|| Error::msg("structural snapshot accounting overflow"))
}

pub(super) fn require_snapshot_kind(actual: StructuralType, expected: StructuralKind) -> Result<()> {
    if actual.kind == expected {
        Ok(())
    } else {
        Err(Error::msg("structural snapshot type and payload disagree"))
    }
}

fn inline_kind(value: InlineStructuralValue) -> StructuralKind {
    match value {
        InlineStructuralValue::Unit => StructuralKind::Unit,
        InlineStructuralValue::Bool(_) => StructuralKind::Bool,
        InlineStructuralValue::I64(_) => StructuralKind::I64,
        InlineStructuralValue::F64Bits(_) => StructuralKind::F64,
    }
}

pub(super) fn validate_snapshot_path(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty() || bytes.first() != Some(&b'/') || bytes.contains(&0) {
        Err(Error::msg("structural snapshot path is invalid"))
    } else {
        Ok(())
    }
}
