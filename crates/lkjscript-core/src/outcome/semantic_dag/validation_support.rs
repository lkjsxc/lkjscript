pub(super) fn dag_children(
    payload: &SemanticDagPayload,
) -> impl Iterator<Item = SemanticDagNodeId> + '_ {
    let fields: &[SemanticDagNodeId] = match payload {
        SemanticDagPayload::Product(fields) | SemanticDagPayload::Enum { fields, .. } => fields,
        _ => &[],
    };
    let pair = match payload {
        SemanticDagPayload::List { head, tail } => Some([*head, *tail]),
        _ => None,
    };
    fields.iter().copied().chain(pair.into_iter().flatten())
}

pub(super) fn dag_child_index(child: SemanticDagNodeId, parent: usize) -> Result<usize> {
    let child = usize::try_from(child.get())
        .map_err(|_| Error::msg("semantic DAG node ID exceeds platform"))?;
    if child >= parent {
        Err(Error::msg(
            "semantic DAG child is a forward reference or cycle",
        ))
    } else {
        Ok(child)
    }
}

fn dag_charge_bytes(
    metrics: &mut StructuralSnapshotMetrics,
    length: usize,
    class: DagByteClass,
    limits: StructuralSnapshotLimits,
    work: DagWork,
) -> Result<()> {
    let length = u64::try_from(length).map_err(|_| Error::msg("semantic DAG byte count overflow"))?;
    metrics.aggregate_bytes = metrics
        .aggregate_bytes
        .checked_add(length)
        .ok_or_else(|| Error::msg("semantic DAG byte count overflow"))?;
    if matches!(class, DagByteClass::String) {
        metrics.string_bytes = metrics
            .string_bytes
            .checked_add(length)
            .ok_or_else(|| Error::msg("semantic DAG string byte count overflow"))?;
    }
    if matches!(class, DagByteClass::Path) {
        metrics.path_bytes = metrics
            .path_bytes
            .checked_add(length)
            .ok_or_else(|| Error::msg("semantic DAG path byte count overflow"))?;
    }
    if metrics.aggregate_bytes > limits.max_aggregate_bytes
        || metrics.string_bytes > limits.max_string_bytes
        || metrics.path_bytes > limits.max_path_bytes
    {
        return Err(Error::msg("semantic DAG bytes exceed bound"));
    }
    dag_charge_work(metrics, length, limits, work)
}

fn dag_charge_work(
    metrics: &mut StructuralSnapshotMetrics,
    amount: u64,
    limits: StructuralSnapshotLimits,
    work: DagWork,
) -> Result<()> {
    metrics.encode_work = metrics
        .encode_work
        .checked_add(amount)
        .ok_or_else(|| Error::msg("semantic DAG work overflow"))?;
    let bound = match work {
        DagWork::Encode => limits.max_encode_work,
        DagWork::Decode => limits.max_decode_work,
    };
    if metrics.encode_work > bound {
        Err(Error::msg("semantic DAG work exceeds bound"))
    } else {
        Ok(())
    }
}

pub(super) fn validate_dag_path(bytes: &[u8]) -> Result<()> {
    if bytes.is_empty()
        || bytes.len() > super::MAX_STRUCTURAL_SNAPSHOT_PATH_BYTES
        || bytes.first() != Some(&b'/')
        || bytes.contains(&0)
    {
        Err(Error::msg("semantic DAG path is invalid"))
    } else {
        Ok(())
    }
}

fn dag_vec<T: Clone>(length: usize, value: T, name: &str) -> Result<Vec<T>> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| Error::msg(format!("semantic DAG {name} allocation failed")))?;
    values.resize(length, value);
    Ok(values)
}

#[derive(Clone, Copy)]
enum DagByteClass {
    String,
    Path,
    Other,
}
