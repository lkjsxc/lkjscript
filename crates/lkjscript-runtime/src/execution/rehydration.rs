use lkjscript_contracts::PreparedProgramIdentity;
use lkjscript_core::{
    ExecutionOutcome, OwnedValue, SealedSemanticDagRuntime, SemanticDagKind, SemanticDagNode,
    SemanticDagNodeId, SemanticDagPayload, SemanticDagSnapshot, SemanticPayload, SemanticValue,
    StructuralKind, ValidatedChunk,
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
    pub complete_release_work: bool,
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
    let mut runtime = SealedSemanticDagRuntime::new().map_err(|error| error.to_string())?;
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
        complete_release_work: metrics.release_backlog == 0,
    };
    if report.final_domains != 0
        || report.final_owners != 0
        || report.final_loans != 0
        || report.final_dependencies != 0
        || report.release_backlog != 0
        || !report.complete_release_work
    {
        return Err("fresh-runtime semantic DAG teardown is not zero".into());
    }
    Ok((
        ExecutionOutcome::Returned(OwnedValue::from_semantic_dag(exported)),
        Some(report),
    ))
}

#[derive(Clone, Copy)]
enum Aggregate {
    Product,
    Enum(u64),
}

enum Task<'a> {
    Visit(&'a SemanticValue),
    Finish {
        value_type: lkjscript_core::StructuralType,
        aggregate: Aggregate,
        fields: usize,
    },
}

fn semantic_tree_dag(value: &SemanticValue) -> Result<SemanticDagSnapshot, String> {
    let mut nodes = Vec::new();
    let mut roots = Vec::new();
    let mut tasks = Vec::new();
    tasks
        .try_reserve(1)
        .map_err(|_| "semantic DAG traversal allocation failed".to_owned())?;
    tasks.push(Task::Visit(value));
    while let Some(task) = tasks.pop() {
        let (value_type, payload) = match task {
            Task::Visit(value) => match &value.payload {
                SemanticPayload::Product(fields) => {
                    schedule_tree_fields(&mut tasks, value.value_type, Aggregate::Product, fields)?;
                    continue;
                }
                SemanticPayload::Enum {
                    tag,
                    active_payload,
                } => {
                    schedule_tree_fields(
                        &mut tasks,
                        value.value_type,
                        Aggregate::Enum(*tag),
                        active_payload,
                    )?;
                    continue;
                }
                SemanticPayload::Inline(inline) => {
                    (value.value_type, SemanticDagPayload::Inline(*inline))
                }
                SemanticPayload::Static(leaf) => {
                    (value.value_type, SemanticDagPayload::Static(*leaf))
                }
                SemanticPayload::String(bytes) => (
                    value.value_type,
                    SemanticDagPayload::String(copy_tree_bytes(bytes)?),
                ),
                SemanticPayload::Path(bytes) => (
                    value.value_type,
                    SemanticDagPayload::Path(copy_tree_bytes(bytes)?),
                ),
                SemanticPayload::Bytes(bytes) => (
                    value.value_type,
                    SemanticDagPayload::Bytes(copy_tree_bytes(bytes)?),
                ),
                SemanticPayload::ByteVector(_) => {
                    return Err("byte-vector structural return is not a semantic DAG".into())
                }
            },
            Task::Finish {
                value_type,
                aggregate,
                fields,
            } => {
                let start = roots
                    .len()
                    .checked_sub(fields)
                    .ok_or_else(|| "semantic DAG traversal state underflow".to_owned())?;
                let children = roots.split_off(start);
                let payload = match aggregate {
                    Aggregate::Product => SemanticDagPayload::Product(children),
                    Aggregate::Enum(tag) => SemanticDagPayload::Enum {
                        tag,
                        fields: children,
                    },
                };
                (value_type, payload)
            }
        };
        let kind = structural_dag_kind(value_type.kind)?;
        let id = u32::try_from(nodes.len())
            .map(SemanticDagNodeId::new)
            .map_err(|_| "semantic DAG node count exceeds u32")?;
        nodes
            .try_reserve(1)
            .map_err(|_| "semantic DAG node allocation failed".to_owned())?;
        roots
            .try_reserve(1)
            .map_err(|_| "semantic DAG traversal allocation failed".to_owned())?;
        nodes.push(SemanticDagNode::new(
            lkjscript_core::SemanticDagType::new(value_type.layout, value_type.semantic_type, kind),
            payload,
        ));
        roots.push(id);
    }
    let root = roots
        .pop()
        .filter(|_| roots.is_empty())
        .ok_or_else(|| "semantic DAG traversal did not produce one root".to_owned())?;
    SemanticDagSnapshot::new(nodes, root).map_err(|error| error.to_string())
}

fn schedule_tree_fields<'a>(
    tasks: &mut Vec<Task<'a>>,
    value_type: lkjscript_core::StructuralType,
    aggregate: Aggregate,
    fields: &'a [SemanticValue],
) -> Result<(), String> {
    let additional = fields
        .len()
        .checked_add(1)
        .ok_or_else(|| "semantic DAG traversal work count overflow".to_owned())?;
    tasks
        .try_reserve(additional)
        .map_err(|_| "semantic DAG traversal allocation failed".to_owned())?;
    tasks.push(Task::Finish {
        value_type,
        aggregate,
        fields: fields.len(),
    });
    tasks.extend(fields.iter().rev().map(Task::Visit));
    Ok(())
}

fn copy_tree_bytes(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| "semantic DAG byte allocation failed".to_owned())?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

fn structural_dag_kind(kind: StructuralKind) -> Result<SemanticDagKind, String> {
    Ok(match kind {
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
    })
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
