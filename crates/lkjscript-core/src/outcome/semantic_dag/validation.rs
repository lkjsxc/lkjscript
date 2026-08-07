fn validate_semantic_dag(
    nodes: &[SemanticDagNode],
    root: SemanticDagNodeId,
) -> Result<StructuralSnapshotMetrics> {
    if nodes.is_empty() {
        return Err(Error::msg("semantic DAG must contain a root node"));
    }
    let node_count = u64::try_from(nodes.len())
        .map_err(|_| Error::msg("semantic DAG node count exceeds u64"))?;
    let final_id = node_count - 1;
    if root.get() != final_id {
        return Err(Error::msg("semantic DAG root must be the final node"));
    }

    let mut metrics = StructuralSnapshotMetrics::default();
    for (index, node) in nodes.iter().enumerate() {
        metrics.nodes = metrics
            .nodes
            .checked_add(1)
            .ok_or_else(|| Error::msg("semantic DAG node count overflow"))?;
        dag_charge_work(&mut metrics, 1)?;
        validate_dag_kind(node)?;
        for child in dag_children(&node.payload) {
            dag_child_index(child, index)?;
            metrics.fields = metrics
                .fields
                .checked_add(1)
                .ok_or_else(|| Error::msg("semantic DAG edge count overflow"))?;
            dag_charge_work(&mut metrics, 1)?;
        }
        validate_dag_payload(node, nodes, index, &mut metrics)?;
    }

    validate_dag_reachability(nodes, root)?;
    metrics.decode_work = metrics.encode_work;
    Ok(metrics)
}

pub(super) fn validate_dag_kind(node: &SemanticDagNode) -> Result<()> {
    let expected = match &node.payload {
        SemanticDagPayload::Inline(InlineStructuralValue::Unit) => SemanticDagKind::Unit,
        SemanticDagPayload::Inline(InlineStructuralValue::Bool(_)) => SemanticDagKind::Bool,
        SemanticDagPayload::Inline(InlineStructuralValue::I64(_)) => SemanticDagKind::I64,
        SemanticDagPayload::Inline(InlineStructuralValue::F64Bits(_)) => SemanticDagKind::F64,
        SemanticDagPayload::Static(_) => SemanticDagKind::Static,
        SemanticDagPayload::String(_) => SemanticDagKind::String,
        SemanticDagPayload::Path(_) => SemanticDagKind::Path,
        SemanticDagPayload::Bytes(_) => SemanticDagKind::Bytes,
        SemanticDagPayload::Product(_) => SemanticDagKind::Product,
        SemanticDagPayload::Enum { .. } => SemanticDagKind::Enum,
        SemanticDagPayload::EmptyList => SemanticDagKind::EmptyList,
        SemanticDagPayload::List { .. } => SemanticDagKind::List,
    };
    if node.value_type.kind == expected {
        Ok(())
    } else {
        Err(Error::msg("semantic DAG type and payload kind disagree"))
    }
}

fn validate_dag_payload(
    node: &SemanticDagNode,
    nodes: &[SemanticDagNode],
    index: usize,
    metrics: &mut StructuralSnapshotMetrics,
) -> Result<()> {
    match &node.payload {
        SemanticDagPayload::String(bytes) => {
            std::str::from_utf8(bytes).map_err(|_| Error::msg("semantic DAG string is not UTF-8"))?;
            dag_charge_bytes(metrics, bytes.len(), DagByteClass::String)
        }
        SemanticDagPayload::Path(bytes) => {
            validate_dag_path(bytes)?;
            dag_charge_bytes(metrics, bytes.len(), DagByteClass::Path)
        }
        SemanticDagPayload::Bytes(bytes) => {
            dag_charge_bytes(metrics, bytes.len(), DagByteClass::Other)
        }
        SemanticDagPayload::List { tail, .. } => {
            let tail_index = dag_child_index(*tail, index)?;
            let tail = &nodes[tail_index];
            if !matches!(tail.value_type.kind, SemanticDagKind::EmptyList | SemanticDagKind::List)
                || tail.value_type.layout != node.value_type.layout
                || tail.value_type.semantic_type != node.value_type.semantic_type
            {
                return Err(Error::msg("semantic DAG list tail type/layout identity mismatch"));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_dag_reachability(
    nodes: &[SemanticDagNode],
    root: SemanticDagNodeId,
) -> Result<()> {
    let mut visited = dag_vec(nodes.len(), false, "reachability")?;
    let mut pending = Vec::new();
    pending
        .try_reserve_exact(nodes.len())
        .map_err(|_| Error::msg("semantic DAG reachability allocation failed"))?;
    pending.push(root);
    while let Some(id) = pending.pop() {
        let index = usize::try_from(id.get())
            .map_err(|_| Error::msg("semantic DAG node ID exceeds platform"))?;
        let node = nodes
            .get(index)
            .ok_or_else(|| Error::msg("semantic DAG node ID out of range"))?;
        if std::mem::replace(&mut visited[index], true) {
            continue;
        }
        pending.extend(dag_children(&node.payload));
    }
    if visited.into_iter().all(|visited| visited) {
        Ok(())
    } else {
        Err(Error::msg("semantic DAG contains unreachable nodes"))
    }
}
