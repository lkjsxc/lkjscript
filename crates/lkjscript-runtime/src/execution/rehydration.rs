use lkjscript_contracts::PreparedProgramIdentity;
use lkjscript_core::{
    ExecutionOutcome, OwnedValue, SealedSemanticDagRuntime, SemanticDagKind, SemanticDagNode,
    SemanticDagNodeId, SemanticDagPayload, SemanticDagSnapshot, SemanticPayload, SemanticValue,
    StructuralKind, StructuralLimits, StructuralSnapshotLimits, ValidatedChunk,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RehydrationReport {
    pub input_canonical_dag_hash: [u8; 32],
    pub output_canonical_dag_hash: [u8; 32],
    pub nodes: u32,
    pub bytes: u64,
    pub allocations: u64,
    pub releases: u64,
    pub cells_reclaimed: u64,
    pub dependency_releases: u64,
    pub release_work: u64,
    pub final_domains: u64,
    pub final_owners: u64,
    pub final_loans: u64,
    pub final_dependencies: u64,
    pub release_backlog: u64,
    pub bounded_release_work: bool,
}

pub fn rehydrate_process_outcome(
    outcome: ExecutionOutcome,
    chunk: &ValidatedChunk,
    prepared: PreparedProgramIdentity,
) -> Result<(ExecutionOutcome, Option<RehydrationReport>), String> {
    chunk
        .require_prepared_identity(prepared)
        .map_err(|error| error.to_string())?;
    let ExecutionOutcome::Returned(value) = outcome else {
        return Ok((outcome, None));
    };
    let snapshot = match value.as_semantic_dag().cloned() {
        Some(snapshot) => snapshot,
        None => match value.as_structural() {
            Some(value) => semantic_tree_dag(value)?,
            None => return Ok((ExecutionOutcome::Returned(value), None)),
        },
    };
    let input_hash = canonical_hash(&snapshot)?;
    let snapshot_metrics = snapshot.metrics();
    let limits = StructuralLimits::default();
    let mut runtime = SealedSemanticDagRuntime::new(limits).map_err(|error| error.to_string())?;
    let owner = runtime
        .rehydrate_authenticated_return(chunk, snapshot)
        .map_err(|failure| failure.error.to_string())?;
    let borrow = runtime
        .begin_borrow(&owner)
        .map_err(|error| error.to_string())?;
    let exported = runtime
        .export_snapshot(&borrow)
        .map_err(|error| error.to_string())?;
    runtime
        .end_borrow(borrow)
        .map_err(|failure| failure.error.to_string())?;
    let released = runtime
        .release(owner)
        .map_err(|failure| failure.error.to_string())?;
    runtime.validate().map_err(|error| error.to_string())?;
    let metrics = runtime.metrics();
    let output_hash = canonical_hash(&exported)?;
    if input_hash != output_hash {
        return Err("fresh-runtime semantic DAG canonical identity mismatch".into());
    }
    let report = RehydrationReport {
        input_canonical_dag_hash: input_hash,
        output_canonical_dag_hash: output_hash,
        nodes: snapshot_metrics.nodes,
        bytes: snapshot_metrics.aggregate_bytes,
        allocations: metrics.sealed.regions_sealed,
        releases: metrics.sealed.releases,
        cells_reclaimed: released.cells_released,
        dependency_releases: released.dependency_releases,
        release_work: metrics.sealed.release_work,
        final_domains: metrics.runtime.live_domains,
        final_owners: metrics.live_owners,
        final_loans: metrics.live_loans,
        final_dependencies: metrics.live_dependencies,
        release_backlog: metrics.release_backlog,
        bounded_release_work: metrics.release_backlog == 0
            && metrics.sealed.release_work <= u64::from(limits.max_release_work),
    };
    if report.final_domains != 0
        || report.final_owners != 0
        || report.final_loans != 0
        || report.final_dependencies != 0
        || report.release_backlog != 0
        || !report.bounded_release_work
    {
        return Err("fresh-runtime semantic DAG teardown is not zero".into());
    }
    Ok((
        ExecutionOutcome::Returned(OwnedValue::from_semantic_dag(exported)),
        Some(report),
    ))
}

fn semantic_tree_dag(value: &SemanticValue) -> Result<SemanticDagSnapshot, String> {
    let mut nodes = Vec::new();
    let root = append_semantic_tree(value, &mut nodes)?;
    SemanticDagSnapshot::new(nodes, root, StructuralSnapshotLimits::DEFAULT)
        .map_err(|error| error.to_string())
}

fn append_semantic_tree(
    value: &SemanticValue,
    nodes: &mut Vec<SemanticDagNode>,
) -> Result<SemanticDagNodeId, String> {
    let payload = match &value.payload {
        SemanticPayload::Inline(value) => SemanticDagPayload::Inline(*value),
        SemanticPayload::Static(value) => SemanticDagPayload::Static(*value),
        SemanticPayload::String(value) => SemanticDagPayload::String(value.clone()),
        SemanticPayload::Path(value) => SemanticDagPayload::Path(value.clone()),
        SemanticPayload::Bytes(value) => SemanticDagPayload::Bytes(value.clone()),
        SemanticPayload::ByteVector(_) => {
            return Err("byte-vector structural return is not a semantic DAG".into())
        }
        SemanticPayload::Product(fields) => SemanticDagPayload::Product(
            fields
                .iter()
                .map(|field| append_semantic_tree(field, nodes))
                .collect::<Result<Vec<_>, _>>()?,
        ),
        SemanticPayload::Enum {
            tag,
            active_payload,
        } => SemanticDagPayload::Enum {
            tag: *tag,
            fields: active_payload
                .iter()
                .map(|field| append_semantic_tree(field, nodes))
                .collect::<Result<Vec<_>, _>>()?,
        },
    };
    let kind = match value.value_type.kind {
        StructuralKind::Unit => SemanticDagKind::Unit,
        StructuralKind::Bool => SemanticDagKind::Bool,
        StructuralKind::I64 => SemanticDagKind::I64,
        StructuralKind::F64 => SemanticDagKind::F64,
        StructuralKind::Static => SemanticDagKind::Static,
        StructuralKind::String => SemanticDagKind::String,
        StructuralKind::Path => SemanticDagKind::Path,
        StructuralKind::Bytes => SemanticDagKind::Bytes,
        StructuralKind::Product => SemanticDagKind::Product,
        StructuralKind::Enum => SemanticDagKind::Enum,
        StructuralKind::ByteVector => {
            return Err("byte-vector structural return is not a semantic DAG".into())
        }
    };
    let id = u32::try_from(nodes.len())
        .map(SemanticDagNodeId::new)
        .map_err(|_| "semantic DAG node count exceeds u32")?;
    nodes.push(SemanticDagNode::new(
        lkjscript_core::SemanticDagType::new(
            value.value_type.layout,
            value.value_type.semantic_type,
            kind,
        ),
        payload,
    ));
    Ok(id)
}

fn canonical_hash(snapshot: &SemanticDagSnapshot) -> Result<[u8; 32], String> {
    let outcome = ExecutionOutcome::Returned(OwnedValue::from_semantic_dag(snapshot.clone()));
    let bytes = lkjscript_core::encode_execution_outcome(
        &outcome,
        crate::process_cell_protocol::PROCESS_OUTCOME_CODEC_LIMITS,
    )
    .map_err(|error| error.to_string())?;
    Ok(lkjscript_contracts::sha256(&bytes))
}
