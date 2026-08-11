#![allow(clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_vm::{run_chunk, ExecutionInputs};
use serde_json::{json, Value};

use super::*;

const MARKER: &str = "LKJSCRIPT_WORKSPACE_RECOMPUTE ";
const PAGE_SIZE: usize = 8;
const CONTROL_QUERY_ITERATIONS: usize = 1_000;

fn nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).expect("measurement duration exceeds u64 nanoseconds")
}

fn count(value: usize) -> u64 {
    u64::try_from(value).expect("measurement count exceeds u64")
}

fn line_count(value: &str) -> u64 {
    count(value.lines().count())
}

fn push_draft_node(nodes: &mut Vec<DraftNode>, node: DraftNode) -> DraftNodeId {
    let id = DraftNodeId::new(count(nodes.len()));
    nodes.push(node);
    id
}

fn counted_loop_draft(limit: i64) -> ExpressionDraft {
    let counter = DraftBindingId::new(0);
    let mut nodes = Vec::new();
    let initial = push_draft_node(&mut nodes, DraftNode::I64(0));
    let condition_load =
        push_draft_node(&mut nodes, DraftNode::Load(DraftBindingRef::Local(counter)));
    let limit_value = push_draft_node(&mut nodes, DraftNode::I64(limit));
    let condition = push_draft_node(
        &mut nodes,
        DraftNode::Operation {
            operation: crate::Operation::Less,
            arguments: vec![condition_load, limit_value],
        },
    );
    let increment_load =
        push_draft_node(&mut nodes, DraftNode::Load(DraftBindingRef::Local(counter)));
    let one = push_draft_node(&mut nodes, DraftNode::I64(1));
    let increment = push_draft_node(
        &mut nodes,
        DraftNode::Operation {
            operation: crate::Operation::Add,
            arguments: vec![increment_load, one],
        },
    );
    let set = push_draft_node(
        &mut nodes,
        DraftNode::SetLocal {
            target: DraftBindingRef::Local(counter),
            value: increment,
        },
    );
    let while_loop = push_draft_node(
        &mut nodes,
        DraftNode::While {
            condition,
            body: vec![set],
        },
    );
    let result = push_draft_node(&mut nodes, DraftNode::Load(DraftBindingRef::Local(counter)));
    let sequence = push_draft_node(&mut nodes, DraftNode::Sequence(vec![while_loop, result]));
    let root = push_draft_node(
        &mut nodes,
        DraftNode::MutableLocal {
            binding: counter,
            name: "counter".to_owned(),
            ty: SemanticType::I64,
            initial,
            body: sequence,
        },
    );
    ExpressionDraft::new(nodes, root)
}

fn create_width_fixture(seed: u64, helper_functions: usize) -> (Workspace, EntityId, HoleId) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("empty measured workspace");
    let mut edits = Vec::new();
    edits
        .try_reserve(helper_functions.checked_add(1).expect("fixture edit count"))
        .expect("fixture edit allocation");
    for index in 0..helper_functions {
        edits.push(Edit::CreateFunction {
            name: format!("helper{index:06}"),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::I64,
        });
    }
    edits.push(Edit::CreateMain {
        return_type: SemanticType::I64,
    });
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits,
        })
        .expect("create measured declarations");
    let main = created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Main)
        .expect("measured main entity")
        .id;
    let main_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("measured main hole")
        .id;
    let helper_holes: Vec<_> = created
        .snapshot
        .holes()
        .filter(|hole| hole.owner != main)
        .map(|hole| hole.id)
        .collect();
    if !helper_holes.is_empty() {
        let edits = helper_holes
            .into_iter()
            .enumerate()
            .map(|(index, hole)| Edit::FillHole {
                hole,
                draft: ExpressionDraft::scalar_i64(
                    i64::try_from(index % 17).expect("helper scalar value"),
                ),
            })
            .collect();
        workspace
            .apply(Transaction {
                base_revision: created.snapshot.revision(),
                edits,
            })
            .expect("fill measured helper bodies");
    }
    let current_hole = workspace
        .current()
        .holes()
        .find(|hole| hole.owner == main)
        .expect("retained measured main hole")
        .id;
    assert_eq!(main_hole, current_hole);
    (workspace, main, main_hole)
}

fn invalidated_names(values: &[InvalidatedDomain]) -> Vec<&'static str> {
    values
        .iter()
        .map(|value| match value {
            InvalidatedDomain::SemanticIndexes => "semantic-indexes",
            InvalidatedDomain::Types => "types",
            InvalidatedDomain::Effects => "effects",
            InvalidatedDomain::Ownership => "ownership",
            InvalidatedDomain::Diagnostics => "diagnostics",
            InvalidatedDomain::Executable => "executable",
            InvalidatedDomain::Provenance => "provenance",
        })
        .collect()
}

fn transaction_value(
    wall: Duration,
    measured: super::transaction::TransactionMeasurement,
    outcome: &TransactionOutcome,
) -> Value {
    json!({
        "wall_ns": nanoseconds(wall),
        "stage_wall_ns": nanoseconds(measured.stage_wall),
        "phases": {
            "program_clone_ns": nanoseconds(measured.program_clone),
            "edit_staging_ns": nanoseconds(measured.edit_staging),
            "compaction_ns": nanoseconds(measured.compaction),
            "effect_inference_ns": nanoseconds(measured.effect_inference),
            "complete_validation_ns": nanoseconds(measured.complete_validation),
            "index_build_ns": nanoseconds(measured.index_build),
            "identity_reconciliation_ns": nanoseconds(measured.identity_reconciliation),
            "finalization_ns": nanoseconds(measured.finalization),
        },
        "work": {
            "program_clones": count(measured.program_clones),
            "functions_cloned": count(measured.functions_cloned),
            "semantic_nodes_cloned": count(measured.semantic_nodes_cloned),
            "bindings_cloned": count(measured.bindings_cloned),
            "products_cloned": count(measured.products_cloned),
            "enums_cloned": count(measured.enums_cloned),
            "implementations_cloned": count(measured.implementations_cloned),
            "match_plans_cloned": count(measured.match_plans_cloned),
            "compaction_invocations": count(measured.compaction_invocations),
            "compaction_roots": count(measured.compaction_roots),
            "effect_inference_invocations": count(measured.effect_inference_invocations),
            "effect_roots": count(measured.effect_roots),
            "complete_hir_derivations": count(measured.complete_hir_derivations),
            "complete_hir_nodes": count(measured.complete_hir_nodes),
            "index_build_invocations": count(measured.index_build_invocations),
            "index_entities_built": count(measured.index_entities_built),
            "index_nodes_built": count(measured.index_nodes_built),
            "identity_reconciliation_invocations": count(measured.identity_reconciliation_invocations),
            "identity_entity_records_examined": count(measured.identity_entity_records_examined),
            "identity_node_records_examined": count(measured.identity_node_records_examined),
            "metadata_only_path_used": measured.metadata_only_path_used,
        },
        "diff_entries": count(outcome.diff.entries.len()),
        "diagnostics_returned": count(outcome.diagnostics.len()),
        "invalidated": invalidated_names(&outcome.invalidated),
    })
}

fn query_value(
    wall: Duration,
    measured: super::query::QueryMeasurement,
    semantic_items_observed: usize,
) -> Value {
    json!({
        "wall_ns": nanoseconds(wall),
        "candidates_scanned": count(measured.candidates_scanned),
        "results_materialized": count(measured.results_materialized),
        "sorted_items": count(measured.sorted_items),
        "items_returned": count(measured.items_returned),
        "pages_built": count(measured.pages_built),
        "semantic_items_observed": count(semantic_items_observed),
    })
}

fn projection_value(
    wall: Duration,
    measured: super::projection::ProjectionMeasurement,
    output: &str,
) -> Value {
    json!({
        "wall_ns": nanoseconds(wall),
        "snapshot_nodes_inspected": count(measured.snapshot_nodes_inspected),
        "nodes_emitted": count(measured.nodes_emitted),
        "reference_edges_inspected": count(measured.reference_edges_inspected),
        "references_emitted": count(measured.references_emitted),
        "visible_entities_inspected": count(measured.visible_entities_inspected),
        "visible_entities_emitted": count(measured.visible_entities_emitted),
        "bytes": count(output.len()),
        "lines": line_count(output),
        "sha256": lkjscript_core::sha256(output.as_bytes()),
    })
}

fn compile_value(
    wall: Duration,
    measured: crate::pipeline::SnapshotCompileMetrics,
    executable: &crate::ExecutableProgram,
) -> Value {
    json!({
        "status": "complete",
        "wall_ns": nanoseconds(wall),
        "complete_hir_derivation_ns": nanoseconds(measured.complete_hir_derivation),
        "memory_planning_ns": nanoseconds(measured.memory_planning),
        "ssa_construction_ns": nanoseconds(measured.ssa_construction),
        "ssa_verification_ns": nanoseconds(measured.ssa_verification),
        "normalization_ns": nanoseconds(measured.normalization),
        "bytecode_lowering_ns": nanoseconds(measured.bytecode_lowering),
        "bytecode_validation_ns": nanoseconds(measured.bytecode_validation),
        "package_validation_ns": nanoseconds(measured.package_validation),
        "main_instructions": count(executable.bytecode().main_instructions().len()),
        "main_physical_locals": count(executable.bytecode().main().locals),
    })
}

fn returned_i64(executable: &crate::ExecutableProgram) -> (i64, Duration) {
    let started = Instant::now();
    let outcome = run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    let wall = started.elapsed();
    let value = match outcome {
        ExecutionOutcome::Returned(value) => value.as_i64().expect("returned i64"),
        other => panic!("unexpected measured VM outcome: {other:?}"),
    };
    (value, wall)
}

fn compile_and_run_i64(snapshot: &WorkspaceSnapshot) -> i64 {
    let executable = crate::compile_snapshot(snapshot).expect("compile measured snapshot");
    returned_i64(&executable).0
}

fn control_sample() -> Value {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let fixture_started = Instant::now();
    let (mut workspace, main, hole) = create_width_fixture(7_000, 0);
    let completed = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::scalar_i64(7),
            }],
        })
        .expect("complete control fixture");
    let fixture_wall = fixture_started.elapsed();

    let query_started = Instant::now();
    for _ in 0..CONTROL_QUERY_ITERATIONS {
        let definition = completed
            .snapshot
            .definition(completed.snapshot.revision(), main)
            .expect("control definition query");
        assert_eq!(definition.id, main);
    }
    let query_wall = query_started.elapsed();

    crate::pipeline::reset_lowering_invocations();
    let compile_started = Instant::now();
    let (executable, compile) = crate::pipeline::compile_snapshot_with_metrics(&completed.snapshot)
        .expect("compile control snapshot");
    let compile_wall = compile_started.elapsed();
    assert_eq!(crate::pipeline::lowering_invocations(), 1);
    let (result, vm_wall) = returned_i64(&executable);
    assert_eq!(result, 7);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);

    json!({
        "schema": "lkjscript.workspace-recompute-sample.v1",
        "workload": "W0",
        "geometry": {
            "helper_functions": 0,
            "total_callables": 1,
            "total_entities": count(completed.snapshot.entities().len()),
            "total_semantic_nodes": count(completed.snapshot.nodes().len()),
            "affected_root_nodes": 0,
            "draft_nodes": 0,
            "page_size": PAGE_SIZE,
            "retained_old_revisions": 0,
        },
        "fixture": { "wall_ns": nanoseconds(fixture_wall) },
        "transaction": Value::Null,
        "queries": {
            "wall_ns": nanoseconds(query_wall),
            "direct_identity_queries": count(CONTROL_QUERY_ITERATIONS),
        },
        "projection": Value::Null,
        "compile": compile_value(compile_wall, compile, &executable),
        "vm": { "wall_ns": nanoseconds(vm_wall), "result_i64": result },
        "correctness": {
            "source_load_invocations": crate::source::source_load_invocation_count(),
            "parser_invocations": crate::source::parser_invocation_count(),
            "old_snapshot_result_i64": Value::Null,
            "new_snapshot_result_i64": result,
        },
        "agent_loop": {
            "commands": 1,
            "process_round_trips": 1,
            "selected_api_operations": CONTROL_QUERY_ITERATIONS + 2,
        },
        "allocation_counts": Value::Null,
        "allocation_bytes": Value::Null,
        "retained_snapshot_bytes": Value::Null,
    })
}

fn hole_refinement_sample(helper_functions: usize) -> Value {
    assert!(helper_functions > 0, "W1 requires positive helper geometry");
    let seed = 7_100_u64
        .checked_add(u64::try_from(helper_functions).expect("W1 helper geometry"))
        .expect("W1 seed overflow");
    let fixture_started = Instant::now();
    let (mut workspace, _main, hole) = create_width_fixture(seed, helper_functions);
    let fixture_wall = fixture_started.elapsed();
    let old_snapshot = workspace.current();
    let old_hole = old_snapshot
        .hole_context(old_snapshot.revision(), hole)
        .expect("old W1 hole");
    let old_projection = old_snapshot
        .project(&[ProjectionSlice::Hole(hole)])
        .expect("old W1 projection");

    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let transaction_started = Instant::now();
    let refined = workspace
        .apply(Transaction {
            base_revision: old_snapshot.revision(),
            edits: vec![Edit::RefineHole {
                hole,
                expected_type: Some(SemanticType::I64),
                goal: "return a representative scalar".to_owned(),
            }],
        })
        .expect("refine measured hole");
    let transaction_wall = transaction_started.elapsed();
    let transaction = super::transaction::take_transaction_measurement();
    assert_eq!(
        refined.snapshot.revision().sequence(),
        old_snapshot.revision().sequence() + 1
    );
    assert_eq!(refined.snapshot.entities(), old_snapshot.entities());
    assert_eq!(refined.snapshot.nodes(), old_snapshot.nodes());
    assert_eq!(old_hole.id, hole);
    assert_eq!(old_hole.goal.as_ref(), "provide the entry-point body");
    assert_eq!(
        old_snapshot
            .hole_context(old_snapshot.revision(), hole)
            .expect("retained old W1 hole")
            .goal
            .as_ref(),
        "provide the entry-point body"
    );

    super::query::reset_query_measurement();
    let query_started = Instant::now();
    let context = refined
        .snapshot
        .hole_context(refined.snapshot.revision(), hole)
        .expect("refined W1 context");
    let diagnostics = refined
        .snapshot
        .diagnostic_page(
            refined.snapshot.revision(),
            PageRequest::new(PAGE_SIZE).expect("W1 page request"),
            None,
        )
        .expect("W1 diagnostics");
    let constructors = refined
        .snapshot
        .legal_constructors(
            refined.snapshot.revision(),
            hole,
            PageRequest::new(PAGE_SIZE).expect("W1 constructor page"),
            None,
        )
        .expect("W1 constructors");
    let semantics = refined
        .snapshot
        .node_semantics(refined.snapshot.revision(), hole.node())
        .expect("W1 hole semantics");
    let query_wall = query_started.elapsed();
    let query = super::query::take_query_measurement();
    assert_eq!(context.goal.as_ref(), "return a representative scalar");
    assert_eq!(semantics.kind, NodeKind::Hole);
    assert_eq!(diagnostics.items.len(), 1);
    assert!(diagnostics.items[0]
        .message
        .contains("return a representative scalar"));
    assert!(constructors.items.contains(&LegalConstructor::I64Literal));
    let semantic_items_observed = diagnostics
        .items
        .len()
        .checked_add(constructors.items.len())
        .and_then(|value| value.checked_add(2))
        .expect("W1 observed item count");

    super::projection::reset_projection_measurement();
    let projection_started = Instant::now();
    let projection = refined
        .snapshot
        .project(&[ProjectionSlice::Hole(hole)])
        .expect("W1 hole projection");
    let projection_wall = projection_started.elapsed();
    let projection_work = super::projection::take_projection_measurement();
    assert_ne!(projection, old_projection);
    assert_eq!(
        projection,
        refined
            .snapshot
            .project(&[ProjectionSlice::Hole(hole)])
            .expect("repeat W1 projection")
    );

    crate::pipeline::reset_lowering_invocations();
    let incomplete_started = Instant::now();
    let incomplete = crate::compile_snapshot(&refined.snapshot)
        .expect_err("W1 incomplete snapshot must not compile");
    let incomplete_wall = incomplete_started.elapsed();
    assert!(matches!(
        incomplete,
        crate::CompileSnapshotError::Incomplete(_)
    ));
    assert_eq!(crate::pipeline::lowering_invocations(), 0);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);

    json!({
        "schema": "lkjscript.workspace-recompute-sample.v1",
        "workload": "W1",
        "geometry": {
            "helper_functions": count(helper_functions),
            "total_callables": count(helper_functions + 1),
            "total_entities": count(refined.snapshot.entities().len()),
            "total_semantic_nodes": count(refined.snapshot.nodes().len()),
            "affected_root_nodes": 0,
            "changed_semantic_nodes": 0,
            "draft_nodes": 0,
            "page_size": PAGE_SIZE,
            "retained_old_revisions": 1,
        },
        "fixture": { "wall_ns": nanoseconds(fixture_wall) },
        "transaction": transaction_value(transaction_wall, transaction, &refined),
        "queries": query_value(query_wall, query, semantic_items_observed),
        "projection": projection_value(projection_wall, projection_work, &projection),
        "compile": {
            "status": "incomplete",
            "wall_ns": nanoseconds(incomplete_wall),
            "lowering_invocations": crate::pipeline::lowering_invocations(),
        },
        "vm": Value::Null,
        "correctness": {
            "source_load_invocations": crate::source::source_load_invocation_count(),
            "parser_invocations": crate::source::parser_invocation_count(),
            "hole_identity_preserved": context.id == old_hole.id,
            "hole_owner_preserved": context.owner == old_hole.owner,
            "old_snapshot_goal_preserved": old_hole.goal.as_ref() == "provide the entry-point body",
            "program_arc_shared": Arc::ptr_eq(&old_snapshot.program, &refined.snapshot.program),
            "index_arc_shared": Arc::ptr_eq(&old_snapshot.indexes, &refined.snapshot.indexes),
            "projection_deterministic": true,
        },
        "agent_loop": {
            "commands": 1,
            "process_round_trips": 1,
            "selected_api_operations": 7,
        },
        "allocation_counts": Value::Null,
        "allocation_bytes": Value::Null,
        "retained_snapshot_bytes": Value::Null,
    })
}

fn imperative_edit_sample(helper_functions: usize) -> Value {
    assert!(helper_functions > 0, "W2 requires positive helper geometry");
    let seed = 8_100_u64
        .checked_add(u64::try_from(helper_functions).expect("W2 helper geometry"))
        .expect("W2 seed overflow");
    let fixture_started = Instant::now();
    let (mut workspace, main, hole) = create_width_fixture(seed, helper_functions);
    let completed = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: counted_loop_draft(100),
            }],
        })
        .expect("complete W2 counted loop");
    let fixture_wall = fixture_started.elapsed();
    let old_snapshot = completed.snapshot;
    assert_eq!(old_snapshot.state(), ProgramState::Complete);
    let affected_root_nodes = old_snapshot
        .indexes
        .node_enclosing_entities
        .iter()
        .filter(|owner| **owner == main)
        .count();
    let main_i64_nodes: Vec<_> = old_snapshot
        .nodes()
        .iter()
        .zip(&old_snapshot.indexes.node_enclosing_entities)
        .filter(|(node, owner)| node.kind == NodeKind::Literal && **owner == main)
        .map(|(node, _)| node.id)
        .collect();
    let target = *main_i64_nodes.get(1).expect("W2 loop-limit literal");
    let helper = old_snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("W2 unaffected helper")
        .id;
    let helper_node = old_snapshot
        .nodes()
        .iter()
        .zip(&old_snapshot.indexes.node_enclosing_entities)
        .find(|(_, owner)| **owner == helper)
        .expect("W2 unaffected helper node")
        .0
        .id;
    let local = old_snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::MutableLocal && entity.owner == Some(main))
        .expect("W2 mutable local")
        .id;

    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let transaction_started = Instant::now();
    let edited = workspace
        .apply(Transaction {
            base_revision: old_snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target,
                draft: ExpressionDraft::scalar_i64(101),
            }],
        })
        .expect("replace W2 loop limit");
    let transaction_wall = transaction_started.elapsed();
    let transaction = super::transaction::take_transaction_measurement();
    assert_eq!(
        edited
            .snapshot
            .node(target)
            .expect("preserved W2 target")
            .id,
        target
    );
    assert_eq!(
        edited
            .snapshot
            .node(helper_node)
            .expect("preserved W2 helper node")
            .id,
        helper_node
    );
    assert_eq!(
        edited
            .snapshot
            .entity(helper)
            .expect("preserved W2 helper")
            .id,
        helper
    );
    assert_eq!(
        old_snapshot
            .node(helper_node)
            .expect("retained old W2 helper node")
            .id,
        helper_node
    );

    super::query::reset_query_measurement();
    let query_started = Instant::now();
    let semantics = edited
        .snapshot
        .node_semantics(edited.snapshot.revision(), target)
        .expect("W2 changed-node semantics");
    let first = edited
        .snapshot
        .entity_page(
            edited.snapshot.revision(),
            PageRequest::new(PAGE_SIZE).expect("W2 entity page"),
            None,
        )
        .expect("W2 first entity page");
    let second = first
        .continuation
        .as_ref()
        .map(|continuation| {
            edited.snapshot.entity_page(
                edited.snapshot.revision(),
                PageRequest::new(PAGE_SIZE).expect("W2 continuation page"),
                Some(continuation),
            )
        })
        .transpose()
        .expect("W2 second entity page");
    let references = edited
        .snapshot
        .references_to(
            edited.snapshot.revision(),
            local,
            PageRequest::new(PAGE_SIZE).expect("W2 reference page"),
            None,
        )
        .expect("W2 local references");
    let definition = edited
        .snapshot
        .definition(edited.snapshot.revision(), helper)
        .expect("W2 helper definition");
    let query_wall = query_started.elapsed();
    let query = super::query::take_query_measurement();
    assert_eq!(semantics.node, target);
    assert_eq!(definition.id, helper);
    assert!(!references.items.is_empty());
    let mut observed_entities = first.items.len();
    if let Some(second) = &second {
        observed_entities = observed_entities
            .checked_add(second.items.len())
            .expect("W2 observed entities");
        assert!(first.items.last().map(|item| item.id) < second.items.first().map(|item| item.id));
    }
    let semantic_items_observed = observed_entities
        .checked_add(references.items.len())
        .and_then(|value| value.checked_add(2))
        .expect("W2 observed item count");

    super::projection::reset_projection_measurement();
    let projection_started = Instant::now();
    let projection = edited
        .snapshot
        .project(&[ProjectionSlice::Body(main)])
        .expect("W2 body projection");
    let projection_wall = projection_started.elapsed();
    let projection_work = super::projection::take_projection_measurement();
    assert_eq!(
        projection,
        edited
            .snapshot
            .project(&[ProjectionSlice::Body(main)])
            .expect("repeat W2 body projection")
    );

    crate::pipeline::reset_lowering_invocations();
    let compile_started = Instant::now();
    let (executable, compile) = crate::pipeline::compile_snapshot_with_metrics(&edited.snapshot)
        .expect("compile edited W2 snapshot");
    let compile_wall = compile_started.elapsed();
    assert_eq!(crate::pipeline::lowering_invocations(), 1);
    let (new_result, vm_wall) = returned_i64(&executable);
    assert_eq!(new_result, 101);
    let old_result = compile_and_run_i64(&old_snapshot);
    assert_eq!(old_result, 100);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);

    json!({
        "schema": "lkjscript.workspace-recompute-sample.v1",
        "workload": "W2",
        "geometry": {
            "helper_functions": count(helper_functions),
            "total_callables": count(helper_functions + 1),
            "total_entities": count(edited.snapshot.entities().len()),
            "total_semantic_nodes": count(edited.snapshot.nodes().len()),
            "affected_root_nodes": count(affected_root_nodes),
            "changed_semantic_nodes": 1,
            "draft_nodes": 1,
            "page_size": PAGE_SIZE,
            "retained_old_revisions": 1,
        },
        "fixture": { "wall_ns": nanoseconds(fixture_wall) },
        "transaction": transaction_value(transaction_wall, transaction, &edited),
        "queries": query_value(query_wall, query, semantic_items_observed),
        "projection": projection_value(projection_wall, projection_work, &projection),
        "compile": compile_value(compile_wall, compile, &executable),
        "vm": { "wall_ns": nanoseconds(vm_wall), "result_i64": new_result },
        "correctness": {
            "source_load_invocations": crate::source::source_load_invocation_count(),
            "parser_invocations": crate::source::parser_invocation_count(),
            "target_identity_preserved": true,
            "unaffected_entity_identity_preserved": true,
            "unaffected_node_identity_preserved": true,
            "old_snapshot_result_i64": old_result,
            "new_snapshot_result_i64": new_result,
            "projection_deterministic": true,
        },
        "agent_loop": {
            "commands": 1,
            "process_round_trips": 1,
            "selected_api_operations": 9,
        },
        "allocation_counts": Value::Null,
        "allocation_bytes": Value::Null,
        "retained_snapshot_bytes": Value::Null,
    })
}

#[test]
fn metadata_only_hole_refinement_is_shared_atomic_and_revision_safe() {
    let (mut workspace, _main, hole) = create_width_fixture(9_001, 8);
    let before = workspace.current();
    let before_projection = before
        .project(&[ProjectionSlice::Hole(hole)])
        .expect("before refinement projection");
    let before_diagnostics = before.diagnostics().to_vec();
    let first_page = before
        .entity_page(
            before.revision(),
            PageRequest::new(1).expect("continuation page size"),
            None,
        )
        .expect("before refinement page");
    let continuation = first_page
        .continuation
        .expect("before refinement continuation");

    let mismatch = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::RefineHole {
            hole,
            expected_type: Some(SemanticType::Bool),
            goal: "invalid type".to_owned(),
        }],
    });
    assert!(matches!(mismatch, Err(WorkspaceError::TypeMismatch { .. })));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    assert_eq!(workspace.current().diagnostics(), before_diagnostics);
    assert_eq!(
        workspace
            .current()
            .project(&[ProjectionSlice::Hole(hole)])
            .expect("projection after failed type refinement"),
        before_projection
    );

    let empty_goal = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::RefineHole {
            hole,
            expected_type: None,
            goal: String::new(),
        }],
    });
    assert!(matches!(
        empty_goal,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let mut foreign = Workspace::empty_deterministic(9_002).expect("foreign workspace");
    let foreign_created = foreign
        .apply(Transaction {
            base_revision: foreign.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("foreign main");
    let foreign_hole = foreign_created
        .snapshot
        .holes()
        .next()
        .expect("foreign hole")
        .id;
    let foreign_failure = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::RefineHole {
            hole: foreign_hole,
            expected_type: None,
            goal: "foreign".to_owned(),
        }],
    });
    assert!(matches!(
        foreign_failure,
        Err(WorkspaceError::ForeignNamespace(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let refined = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![
                Edit::RefineHole {
                    hole,
                    expected_type: Some(SemanticType::I64),
                    goal: "intermediate goal".to_owned(),
                },
                Edit::RefineHole {
                    hole,
                    expected_type: None,
                    goal: "final goal".to_owned(),
                },
            ],
        })
        .expect("metadata-only refinements");
    let measured = super::transaction::take_transaction_measurement();
    assert!(measured.metadata_only_path_used);
    assert_eq!(measured.program_clones, 0);
    assert_eq!(measured.compaction_invocations, 0);
    assert_eq!(measured.effect_inference_invocations, 0);
    assert_eq!(measured.complete_hir_derivations, 0);
    assert_eq!(measured.index_build_invocations, 0);
    assert_eq!(measured.identity_reconciliation_invocations, 0);
    assert!(Arc::ptr_eq(&before.program, &refined.snapshot.program));
    assert!(Arc::ptr_eq(&before.indexes, &refined.snapshot.indexes));
    assert!(Arc::ptr_eq(&before.blockers, &refined.snapshot.blockers));
    assert!(!Arc::ptr_eq(&before.holes, &refined.snapshot.holes));
    assert!(!Arc::ptr_eq(
        &before.diagnostics,
        &refined.snapshot.diagnostics
    ));
    assert_eq!(
        refined.snapshot.revision().sequence(),
        before.revision().sequence() + 1
    );
    assert_eq!(
        refined.snapshot.completeness_blockers(),
        before.completeness_blockers()
    );
    assert_eq!(
        invalidated_names(&refined.invalidated),
        vec![
            "semantic-indexes",
            "types",
            "effects",
            "ownership",
            "diagnostics",
            "executable",
            "provenance",
        ]
    );
    assert_eq!(refined.diff.entries.len(), 2);
    let goal_changes: Vec<_> = refined
        .diff
        .entries
        .iter()
        .filter_map(|entry| match entry {
            SemanticDiffEntry::HoleRefined {
                hole: changed,
                old_goal,
                new_goal,
            } if *changed == hole => Some((old_goal.as_ref(), new_goal.as_ref())),
            _ => None,
        })
        .collect();
    assert_eq!(
        goal_changes,
        vec![
            ("intermediate goal", "final goal"),
            ("provide the entry-point body", "intermediate goal"),
        ]
    );
    assert_eq!(
        refined
            .snapshot
            .hole_context(refined.snapshot.revision(), hole)
            .expect("refined hole")
            .goal
            .as_ref(),
        "final goal"
    );
    assert!(refined.snapshot.diagnostics()[0]
        .message
        .contains("final goal"));
    assert_eq!(before.diagnostics(), before_diagnostics);
    assert_eq!(
        before
            .project(&[ProjectionSlice::Hole(hole)])
            .expect("retained before projection"),
        before_projection
    );
    assert!(refined
        .snapshot
        .project(&[ProjectionSlice::Hole(hole)])
        .expect("refined projection")
        .contains("goal=\"final goal\""));
    assert!(matches!(
        refined.snapshot.entity_page(
            refined.snapshot.revision(),
            PageRequest::new(1).expect("stale continuation page size"),
            Some(&continuation),
        ),
        Err(WorkspaceError::InvalidContinuation(_))
    ));
    crate::pipeline::reset_lowering_invocations();
    assert!(matches!(
        crate::compile_snapshot(&refined.snapshot),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));
    assert_eq!(crate::pipeline::lowering_invocations(), 0);

    let helper = before
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("mixed refinement helper")
        .id;
    let mut mixed_workspace = Workspace::new((*before).clone()).expect("mixed workspace");
    let mixed = mixed_workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![
                Edit::RefineHole {
                    hole,
                    expected_type: None,
                    goal: "mixed goal".to_owned(),
                },
                Edit::RenameEntity {
                    entity: helper,
                    new_name: "renamed-helper".to_owned(),
                },
            ],
        })
        .expect("mixed transaction");
    let mixed_measurement = super::transaction::take_transaction_measurement();
    assert!(!mixed_measurement.metadata_only_path_used);
    assert_eq!(mixed_measurement.program_clones, 1);
    assert_eq!(
        mixed
            .snapshot
            .hole_context(mixed.snapshot.revision(), hole)
            .expect("mixed hole")
            .goal
            .as_ref(),
        "mixed goal"
    );

    let mut control_workspace = Workspace::new((*before).clone()).expect("allocator control");
    let create = |revision| Transaction {
        base_revision: revision,
        edits: vec![Edit::CreateFunction {
            name: "allocator-probe".to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::I64,
        }],
    };
    let control_revision = control_workspace.current().revision();
    let control_created = control_workspace
        .apply(create(control_revision))
        .expect("control allocation");
    let refined_revision = workspace.current().revision();
    let refined_created = workspace
        .apply(create(refined_revision))
        .expect("post-refinement allocation");
    let control_entity = control_created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.name.as_ref() == "allocator-probe")
        .expect("control allocated entity")
        .id;
    let refined_entity = refined_created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.name.as_ref() == "allocator-probe")
        .expect("post-refinement allocated entity")
        .id;
    assert_eq!(control_entity, refined_entity);
    let control_hole = control_created
        .snapshot
        .holes()
        .find(|state| state.owner == control_entity)
        .expect("control allocated hole")
        .id;
    let refined_hole = refined_created
        .snapshot
        .holes()
        .find(|state| state.owner == refined_entity)
        .expect("post-refinement allocated hole")
        .id;
    assert_eq!(control_hole, refined_hole);
}

#[test]
fn workspace_recompute_measurement_is_semantically_exact() {
    let control = control_sample();
    assert_eq!(control["vm"]["result_i64"], 7);
    let incomplete = hole_refinement_sample(8);
    assert_eq!(incomplete["workload"], "W1");
    assert_eq!(incomplete["compile"]["lowering_invocations"], 0);
    let complete = imperative_edit_sample(8);
    assert_eq!(complete["correctness"]["old_snapshot_result_i64"], 100);
    assert_eq!(complete["correctness"]["new_snapshot_result_i64"], 101);
}

#[test]
#[ignore = "locked-release semantic-workspace recomputation measurement"]
fn workspace_recompute_scale_sample() {
    let workload = std::env::var("LKJSCRIPT_WORKSPACE_WORKLOAD")
        .expect("LKJSCRIPT_WORKSPACE_WORKLOAD must select W0, W1, or W2");
    let sample = match workload.as_str() {
        "W0" => control_sample(),
        "W1" | "W2" => {
            let helper_functions = std::env::var("LKJSCRIPT_WORKSPACE_FUNCTIONS")
                .expect("LKJSCRIPT_WORKSPACE_FUNCTIONS is required for W1 and W2")
                .parse::<usize>()
                .expect("LKJSCRIPT_WORKSPACE_FUNCTIONS must be a positive integer");
            assert!(
                helper_functions > 0,
                "LKJSCRIPT_WORKSPACE_FUNCTIONS must be positive"
            );
            if workload == "W1" {
                hole_refinement_sample(helper_functions)
            } else {
                imperative_edit_sample(helper_functions)
            }
        }
        other => panic!("unsupported workspace measurement workload {other}"),
    };
    eprintln!(
        "{MARKER}{}",
        serde_json::to_string(&sample).expect("serialize workspace recomputation sample")
    );
}
