fn decode_semantic_dag(
    input: &mut Decoder<'_>,
    limits: StructuralSnapshotLimits,
) -> Result<SemanticDagSnapshot> {
    let limits = limits.validate()?;
    let count = input.u32()?;
    if count == 0 || count > limits.max_nodes {
        return Err(Error::msg("semantic DAG node count exceeds bound"));
    }
    let root = SemanticDagNodeId::new(input.u32()?);
    let count = usize::try_from(count)
        .map_err(|_| Error::msg("semantic DAG node count exceeds platform"))?;
    let mut nodes = Vec::new();
    nodes
        .try_reserve_exact(count)
        .map_err(|_| Error::msg("semantic DAG node allocation failed"))?;
    let mut budget = SemanticDagDecodeBudget::new(limits);
    for index in 0..count {
        budget.node()?;
        let value_type = decode_semantic_dag_type(input)?;
        let payload = decode_semantic_dag_payload(input, value_type.kind, &mut budget)?;
        let node = SemanticDagNode::new(value_type, payload);
        validate_decoded_semantic_dag_node(&node, &nodes, index)?;
        nodes.push(node);
    }
    SemanticDagSnapshot::from_decoded(nodes, root, limits, budget.finish())
}

fn decode_semantic_dag_type(input: &mut Decoder<'_>) -> Result<SemanticDagType> {
    let layout = NonZeroU64::new(input.u64()?)
        .ok_or_else(|| Error::msg("zero semantic DAG layout identity"))?;
    let semantic_type = NonZeroU64::new(input.u64()?)
        .ok_or_else(|| Error::msg("zero semantic DAG type identity"))?;
    Ok(SemanticDagType::new(
        LayoutIdentity::new(layout),
        SemanticTypeIdentity::new(semantic_type),
        decode_semantic_dag_kind(input.u8()?)?,
    ))
}

fn decode_semantic_dag_payload(
    input: &mut Decoder<'_>,
    kind: SemanticDagKind,
    budget: &mut SemanticDagDecodeBudget,
) -> Result<SemanticDagPayload> {
    Ok(match kind {
        SemanticDagKind::Unit => SemanticDagPayload::Inline(InlineStructuralValue::Unit),
        SemanticDagKind::Bool => match input.u8()? {
            0 => SemanticDagPayload::Inline(InlineStructuralValue::Bool(false)),
            1 => SemanticDagPayload::Inline(InlineStructuralValue::Bool(true)),
            _ => return Err(Error::msg("semantic DAG bool payload is not canonical")),
        },
        SemanticDagKind::I64 => {
            SemanticDagPayload::Inline(InlineStructuralValue::I64(input.u64()? as i64))
        }
        SemanticDagKind::F64 => {
            SemanticDagPayload::Inline(InlineStructuralValue::F64Bits(input.u64()?))
        }
        SemanticDagKind::Static => {
            SemanticDagPayload::Static(decode_structural_static(input)?)
        }
        SemanticDagKind::String => SemanticDagPayload::String(decode_semantic_dag_bytes(
            input,
            budget,
            DagWireByteClass::String,
        )?),
        SemanticDagKind::Path => SemanticDagPayload::Path(decode_semantic_dag_bytes(
            input,
            budget,
            DagWireByteClass::Path,
        )?),
        SemanticDagKind::Bytes => SemanticDagPayload::Bytes(decode_semantic_dag_bytes(
            input,
            budget,
            DagWireByteClass::Other,
        )?),
        SemanticDagKind::Product => {
            SemanticDagPayload::Product(decode_semantic_dag_fields(input, budget)?)
        }
        SemanticDagKind::Enum => SemanticDagPayload::Enum {
            tag: input.u16()?,
            fields: decode_semantic_dag_fields(input, budget)?,
        },
        SemanticDagKind::EmptyList => SemanticDagPayload::EmptyList,
        SemanticDagKind::List => {
            budget.fields(2)?;
            SemanticDagPayload::List {
                head: SemanticDagNodeId::new(input.u32()?),
                tail: SemanticDagNodeId::new(input.u32()?),
            }
        }
    })
}

fn decode_semantic_dag_fields(
    input: &mut Decoder<'_>,
    budget: &mut SemanticDagDecodeBudget,
) -> Result<Vec<SemanticDagNodeId>> {
    let count = input.u32()?;
    budget.fields(count)?;
    let count = usize::try_from(count)
        .map_err(|_| Error::msg("semantic DAG edge count exceeds platform"))?;
    let mut fields = Vec::new();
    fields
        .try_reserve_exact(count)
        .map_err(|_| Error::msg("semantic DAG edge allocation failed"))?;
    for _ in 0..count {
        fields.push(SemanticDagNodeId::new(input.u32()?));
    }
    Ok(fields)
}

fn decode_semantic_dag_bytes(
    input: &mut Decoder<'_>,
    budget: &mut SemanticDagDecodeBudget,
    class: DagWireByteClass,
) -> Result<Vec<u8>> {
    let bytes = input.bytes()?;
    budget.bytes(bytes.len(), class)?;
    if matches!(class, DagWireByteClass::String) {
        std::str::from_utf8(bytes).map_err(|_| Error::msg("semantic DAG string is not UTF-8"))?;
    }
    if matches!(class, DagWireByteClass::Path) {
        validate_dag_path(bytes)?;
    }
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| Error::msg("semantic DAG byte allocation failed"))?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

fn validate_decoded_semantic_dag_node(
    node: &SemanticDagNode,
    previous: &[SemanticDagNode],
    index: usize,
) -> Result<()> {
    validate_dag_kind(node)?;
    for child in dag_children(&node.payload) {
        dag_child_index(child, index)?;
    }
    if let SemanticDagPayload::List { tail, .. } = node.payload {
        let tail = &previous[tail.get() as usize];
        if !matches!(tail.value_type.kind, SemanticDagKind::EmptyList | SemanticDagKind::List)
            || tail.value_type.layout != node.value_type.layout
            || tail.value_type.semantic_type != node.value_type.semantic_type
        {
            return Err(Error::msg("semantic DAG list tail type/layout identity mismatch"));
        }
    }
    Ok(())
}
