#![allow(clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_vm::{run_chunk, ExecutionInputs};
use serde_json::{json, Value};

use super::*;

const MARKER: &str = "LKJSCRIPT_WORKSPACE_RECOMPUTE ";
const PAGE_SIZE: usize = 8;
const REFINEMENT_MODE_ENV: &str = "LKJSCRIPT_WORKSPACE_REFINEMENT_MODE";

fn refinement_mode() -> &'static str {
    match std::env::var(REFINEMENT_MODE_ENV).as_deref() {
        Ok("full") => {
            super::transaction::set_force_full_recomputation(true);
            "full"
        }
        Ok("narrow") | Err(std::env::VarError::NotPresent) => {
            super::transaction::set_force_full_recomputation(false);
            "narrow"
        }
        Ok(other) => panic!("{REFINEMENT_MODE_ENV} must be narrow or full, not {other}"),
        Err(error) => panic!("invalid {REFINEMENT_MODE_ENV}: {error}"),
    }
}

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
        parameters: Vec::new(),
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
    assert!(outcome.cleanup_failures().is_none());
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

fn authoring_loop_wall(durations: &[Duration]) -> u64 {
    nanoseconds(
        durations
            .iter()
            .copied()
            .try_fold(Duration::ZERO, Duration::checked_add)
            .expect("authoring-loop duration overflow"),
    )
}

fn control_sample() -> Value {
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
        .expect("complete W0 fixture");
    let fixture_wall = fixture_started.elapsed();
    let old_snapshot = completed.snapshot;
    let root = old_snapshot.nodes()[0].id;

    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let transaction_started = Instant::now();
    let edited = workspace
        .apply(Transaction {
            base_revision: old_snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: root,
                draft: ExpressionDraft::scalar_i64(8),
            }],
        })
        .expect("replace W0 scalar");
    let transaction_wall = transaction_started.elapsed();
    let transaction = super::transaction::take_transaction_measurement();
    assert_eq!(edited.snapshot.node(root).expect("W0 root").id, root);

    super::query::reset_query_measurement();
    let query_started = Instant::now();
    let definition = edited
        .snapshot
        .definition(edited.snapshot.revision(), main)
        .expect("W0 definition");
    let semantics = edited
        .snapshot
        .node_semantics(edited.snapshot.revision(), root)
        .expect("W0 semantics");
    let query_wall = query_started.elapsed();
    let query = super::query::take_query_measurement();
    assert_eq!(definition.id, main);
    assert_eq!(semantics.actual, SemanticType::I64);

    super::projection::reset_projection_measurement();
    let projection_started = Instant::now();
    let projection = edited
        .snapshot
        .project(&[ProjectionSlice::Body(main), ProjectionSlice::Type(root)])
        .expect("W0 projection");
    let projection_wall = projection_started.elapsed();
    let projection_work = super::projection::take_projection_measurement();

    crate::pipeline::reset_lowering_invocations();
    let compile_started = Instant::now();
    let (executable, compile) = crate::pipeline::compile_snapshot_with_metrics(&edited.snapshot)
        .expect("compile W0 snapshot");
    let compile_wall = compile_started.elapsed();
    let (result, vm_wall) = returned_i64(&executable);
    assert_eq!(crate::pipeline::lowering_invocations(), 1);
    let old_result = compile_and_run_i64(&old_snapshot);
    assert_eq!((old_result, result), (7, 8));
    assert_eq!(crate::pipeline::lowering_invocations(), 2);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);

    json!({
        "schema": "lkjscript.workspace-recompute-sample.v2",
        "workload": "W0",
        "geometry": {
            "helper_functions": 0,
            "total_callables": 1,
            "total_entities": count(edited.snapshot.entities().len()),
            "total_semantic_nodes": count(edited.snapshot.nodes().len()),
            "affected_root_nodes": 1,
            "changed_semantic_nodes": 1,
            "draft_nodes": 1,
            "page_size": PAGE_SIZE,
            "retained_old_revisions": 1,
        },
        "fixture": { "wall_ns": nanoseconds(fixture_wall) },
        "transaction": transaction_value(transaction_wall, transaction, &edited),
        "queries": query_value(query_wall, query, 2),
        "projection": projection_value(projection_wall, projection_work, &projection),
        "compile": compile_value(compile_wall, compile, &executable),
        "vm": { "wall_ns": nanoseconds(vm_wall), "result_i64": result },
        "correctness": {
            "source_load_invocations": crate::source::source_load_invocation_count(),
            "parser_invocations": crate::source::parser_invocation_count(),
            "root_identity_preserved": true,
            "old_snapshot_result_i64": old_result,
            "new_snapshot_result_i64": result,
        },
        "agent_loop": {
            "commands": 1,
            "process_round_trips": 1,
            "selected_api_operations": 6,
            "edit_inspect_check_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, compile_wall]),
            "authoring_loop_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, compile_wall, vm_wall]),
            "output_bytes": count(projection.len()),
            "output_lines": line_count(&projection),
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
    let mode = refinement_mode();
    let transaction_started = Instant::now();
    let refined = workspace.apply(Transaction {
        base_revision: old_snapshot.revision(),
        edits: vec![Edit::RefineHole {
            hole,
            expected_type: Some(SemanticType::I64),
            goal: "return a representative scalar".to_owned(),
        }],
    });
    let transaction_wall = transaction_started.elapsed();
    super::transaction::set_force_full_recomputation(false);
    let refined = refined.expect("refine measured hole");
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
        "schema": "lkjscript.workspace-recompute-sample.v2",
        "workload": "W1",
        "refinement_mode": mode,
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
            "edit_inspect_check_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, incomplete_wall]),
            "authoring_loop_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, incomplete_wall]),
            "output_bytes": count(projection.len()),
            "output_lines": line_count(&projection),
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
    let query_operations = 4_usize + usize::from(second.is_some());
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
        "schema": "lkjscript.workspace-recompute-sample.v2",
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
            "selected_api_operations": count(query_operations + 4),
            "edit_inspect_check_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, compile_wall]),
            "authoring_loop_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, compile_wall, vm_wall]),
            "output_bytes": count(projection.len()),
            "output_lines": line_count(&projection),
        },
        "allocation_counts": Value::Null,
        "allocation_bytes": Value::Null,
        "retained_snapshot_bytes": Value::Null,
    })
}

fn return_i64_draft(result: i64) -> ExpressionDraft {
    ExpressionDraft::new(
        vec![
            DraftNode::I64(result),
            DraftNode::Return {
                value: DraftNodeId::new(0),
            },
        ],
        DraftNodeId::new(1),
    )
}

fn ownership_early_return_draft(result: i64) -> ExpressionDraft {
    let owner = DraftBindingId::new(0);
    ExpressionDraft::new(
        vec![
            DraftNode::I64(4),
            DraftNode::Operation {
                operation: crate::Operation::ByteVectorNew,
                arguments: vec![DraftNodeId::new(0)],
            },
            DraftNode::BorrowShared(DraftBindingRef::Local(owner)),
            DraftNode::Operation {
                operation: crate::Operation::ByteSliceLength,
                arguments: vec![DraftNodeId::new(2)],
            },
            DraftNode::I64(result),
            DraftNode::Return {
                value: DraftNodeId::new(4),
            },
            DraftNode::Sequence(vec![DraftNodeId::new(3), DraftNodeId::new(5)]),
            DraftNode::Let {
                bindings: vec![LocalDraft {
                    binding: owner,
                    name: "owned-bytes".to_owned(),
                    value: DraftNodeId::new(1),
                }],
                body: DraftNodeId::new(6),
            },
        ],
        DraftNodeId::new(7),
    )
}

fn ownership_edit_sample() -> Value {
    let fixture_started = Instant::now();
    let (mut workspace, main, hole) = create_width_fixture(7_300, 0);
    let completed = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ownership_early_return_draft(7),
            }],
        })
        .expect("complete W3 ownership fixture");
    let fixture_wall = fixture_started.elapsed();
    let old_snapshot = completed.snapshot;
    let return_node = old_snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Return)
        .expect("W3 return node")
        .id;
    let target = return_node;
    let owner = old_snapshot
        .entities()
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::ImmutableLocal && entity.name.as_ref() == "owned-bytes"
        })
        .expect("W3 owned local")
        .id;

    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let transaction_started = Instant::now();
    let edited = workspace
        .apply(Transaction {
            base_revision: old_snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target,
                draft: return_i64_draft(9),
            }],
        })
        .expect("edit W3 return subtree");
    let transaction_wall = transaction_started.elapsed();
    let transaction = super::transaction::take_transaction_measurement();
    assert_eq!(edited.snapshot.node(target).expect("W3 target").id, target);
    assert_eq!(
        edited
            .snapshot
            .node_semantics(edited.snapshot.revision(), target)
            .expect("W3 type")
            .actual,
        SemanticType::Never
    );

    super::query::reset_query_measurement();
    let query_started = Instant::now();
    let semantics = edited
        .snapshot
        .node_semantics(edited.snapshot.revision(), target)
        .expect("W3 semantics");
    let references = edited
        .snapshot
        .references_to(
            edited.snapshot.revision(),
            owner,
            PageRequest::new(PAGE_SIZE).expect("W3 reference page"),
            None,
        )
        .expect("W3 references");
    let query_wall = query_started.elapsed();
    let query = super::query::take_query_measurement();
    assert_eq!(semantics.actual, SemanticType::Never);
    assert!(!references.items.is_empty());

    super::projection::reset_projection_measurement();
    let projection_started = Instant::now();
    let projection = edited
        .snapshot
        .project(&[ProjectionSlice::Body(main), ProjectionSlice::Type(target)])
        .expect("W3 projection");
    let projection_wall = projection_started.elapsed();
    let projection_work = super::projection::take_projection_measurement();

    crate::pipeline::reset_lowering_invocations();
    let compile_started = Instant::now();
    let (executable, compile) =
        crate::pipeline::compile_snapshot_with_metrics(&edited.snapshot).expect("compile W3");
    let compile_wall = compile_started.elapsed();
    let drop_obligations = executable
        .memory_plan()
        .obligations
        .iter()
        .filter(|obligation| {
            obligation.kind == crate::memory_plan::MemoryObligationKind::DropWholeValue
        })
        .count();
    let end_borrows = executable
        .memory_plan()
        .obligations
        .iter()
        .filter(|obligation| obligation.kind == crate::memory_plan::MemoryObligationKind::EndBorrow)
        .count();
    assert!(drop_obligations >= 1);
    assert!(end_borrows >= 1);
    let (new_result, vm_wall) = returned_i64(&executable);
    let old_result = compile_and_run_i64(&old_snapshot);
    assert_eq!((old_result, new_result), (7, 9));
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);

    json!({
        "schema": "lkjscript.workspace-recompute-sample.v2",
        "workload": "W3",
        "geometry": {
            "helper_functions": 0,
            "total_callables": 1,
            "total_entities": count(edited.snapshot.entities().len()),
            "total_semantic_nodes": count(edited.snapshot.nodes().len()),
            "affected_root_nodes": count(old_snapshot.nodes().len()),
            "changed_semantic_nodes": 2,
            "draft_nodes": 2,
            "page_size": PAGE_SIZE,
            "retained_old_revisions": 1,
            "owned_byte_vector_length": 4,
        },
        "fixture": { "wall_ns": nanoseconds(fixture_wall) },
        "transaction": transaction_value(transaction_wall, transaction, &edited),
        "queries": query_value(query_wall, query, references.items.len() + 1),
        "projection": projection_value(projection_wall, projection_work, &projection),
        "compile": compile_value(compile_wall, compile, &executable),
        "vm": { "wall_ns": nanoseconds(vm_wall), "result_i64": new_result },
        "memory": {
            "drop_whole_value_obligations": count(drop_obligations),
            "end_borrow_obligations": count(end_borrows),
            "cleanup_failures": 0,
        },
        "correctness": {
            "source_load_invocations": crate::source::source_load_invocation_count(),
            "parser_invocations": crate::source::parser_invocation_count(),
            "return_identity_preserved": true,
            "return_type_preserved": true,
            "old_snapshot_result_i64": old_result,
            "new_snapshot_result_i64": new_result,
        },
        "agent_loop": {
            "commands": 1,
            "process_round_trips": 1,
            "selected_api_operations": 6,
            "edit_inspect_check_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, compile_wall]),
            "authoring_loop_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, compile_wall, vm_wall]),
            "output_bytes": count(projection.len()),
            "output_lines": line_count(&projection),
        },
        "allocation_counts": Value::Null,
        "allocation_bytes": Value::Null,
        "retained_snapshot_bytes": Value::Null,
    })
}

fn product_enum_match_draft(
    pair: EntityId,
    left: EntityId,
    right: EntityId,
    some: EntityId,
    none: EntityId,
    payload_field: EntityId,
) -> ExpressionDraft {
    let pair_local = DraftBindingId::new(0);
    let payload = DraftBindingId::new(1);
    ExpressionDraft::new(
        vec![
            DraftNode::I64(42),
            DraftNode::I64(1),
            DraftNode::ProductValue {
                product: pair,
                fields: vec![
                    DraftFieldValue {
                        field: left,
                        value: DraftNodeId::new(0),
                    },
                    DraftFieldValue {
                        field: right,
                        value: DraftNodeId::new(1),
                    },
                ],
            },
            DraftNode::Load(DraftBindingRef::Local(pair_local)),
            DraftNode::ProductField {
                field: left,
                value: DraftNodeId::new(3),
            },
            DraftNode::EnumValue {
                variant: some,
                type_arguments: Vec::new(),
                fields: vec![DraftFieldValue {
                    field: payload_field,
                    value: DraftNodeId::new(4),
                }],
            },
            DraftNode::Load(DraftBindingRef::Local(payload)),
            DraftNode::I64(0),
            DraftNode::Match {
                scrutinee: DraftNodeId::new(5),
                arms: vec![
                    MatchArmDraft {
                        pattern: PatternDraft::new(
                            vec![
                                DraftPatternNode::Binding {
                                    binding: payload,
                                    name: "selected-payload".to_owned(),
                                },
                                DraftPatternNode::EnumVariant {
                                    variant: some,
                                    fields: vec![DraftPatternField {
                                        field: payload_field,
                                        pattern: DraftPatternNodeId::new(0),
                                    }],
                                },
                            ],
                            DraftPatternNodeId::new(1),
                        ),
                        body: DraftNodeId::new(6),
                    },
                    MatchArmDraft {
                        pattern: PatternDraft::new(
                            vec![DraftPatternNode::EnumVariant {
                                variant: none,
                                fields: Vec::new(),
                            }],
                            DraftPatternNodeId::new(0),
                        ),
                        body: DraftNodeId::new(7),
                    },
                ],
            },
            DraftNode::Let {
                bindings: vec![LocalDraft {
                    binding: pair_local,
                    name: "pair-value".to_owned(),
                    value: DraftNodeId::new(2),
                }],
                body: DraftNodeId::new(8),
            },
        ],
        DraftNodeId::new(9),
    )
}

fn product_enum_match_sample() -> Value {
    let fixture_started = Instant::now();
    let mut workspace = Workspace::empty_deterministic(7_400).expect("W4 workspace");
    let declared = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateProduct {
                    name: "pair".to_owned(),
                    fields: vec![
                        ProductFieldDraft {
                            name: "left".to_owned(),
                            ty: SemanticType::I64,
                        },
                        ProductFieldDraft {
                            name: "right".to_owned(),
                            ty: SemanticType::I64,
                        },
                    ],
                },
                Edit::CreateEnum {
                    name: "choice".to_owned(),
                    variants: vec![
                        EnumVariantDraft {
                            name: "some".to_owned(),
                            fields: vec![EnumFieldDraft {
                                name: "payload".to_owned(),
                                ty: SemanticType::I64,
                            }],
                        },
                        EnumVariantDraft {
                            name: "none".to_owned(),
                            fields: Vec::new(),
                        },
                    ],
                },
                Edit::CreateMain {
                    parameters: Vec::new(),
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("declare W4 forms");
    let named = |kind, name: &str| {
        declared
            .snapshot
            .entities()
            .iter()
            .find(|entity| entity.kind == kind && entity.name.as_ref() == name)
            .expect("W4 named entity")
            .id
    };
    let pair = named(EntityKind::Product, "pair");
    let left = named(EntityKind::ProductField, "left");
    let right = named(EntityKind::ProductField, "right");
    let some = named(EntityKind::EnumVariant, "some");
    let none = named(EntityKind::EnumVariant, "none");
    let payload_field = named(EntityKind::EnumField, "payload");
    let main = named(EntityKind::Main, "main");
    let hole = declared.snapshot.holes().next().expect("W4 main hole").id;
    let completed = workspace
        .apply(Transaction {
            base_revision: declared.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: product_enum_match_draft(pair, left, right, some, none, payload_field),
            }],
        })
        .expect("complete W4");
    let fixture_wall = fixture_started.elapsed();
    let old_snapshot = completed.snapshot;
    let match_site = old_snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Match)
        .expect("W4 match")
        .id;
    let old_view = old_snapshot
        .match_view(old_snapshot.revision(), match_site)
        .expect("W4 old match view");
    assert!(old_view.exhaustive);
    let selected_arm = old_view.arms[0].body;
    let payload_binding = old_view.arms[0]
        .patterns
        .iter()
        .find_map(|pattern| match pattern.kind {
            MatchPatternKindView::Binding { binding } => Some(binding),
            _ => None,
        })
        .expect("W4 payload binding");

    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let transaction_started = Instant::now();
    let edited = workspace
        .apply(Transaction {
            base_revision: old_snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: selected_arm,
                draft: ExpressionDraft::scalar_i64(43),
            }],
        })
        .expect("edit W4 selected arm");
    let transaction_wall = transaction_started.elapsed();
    let transaction = super::transaction::take_transaction_measurement();
    let new_view = edited
        .snapshot
        .match_view(edited.snapshot.revision(), match_site)
        .expect("W4 new match view");
    assert!(new_view.exhaustive);
    assert_eq!(new_view.arms[0].body, selected_arm);
    assert!(edited.snapshot.entity(payload_binding).is_ok());
    assert!(edited.snapshot.entity(payload_field).is_ok());

    super::query::reset_query_measurement();
    let query_started = Instant::now();
    let match_view = edited
        .snapshot
        .match_view(edited.snapshot.revision(), match_site)
        .expect("W4 match query");
    let match_type = edited
        .snapshot
        .node_type(edited.snapshot.revision(), match_site)
        .expect("W4 match type");
    let payload_type = edited
        .snapshot
        .entity_type(edited.snapshot.revision(), payload_binding)
        .expect("W4 payload type");
    let semantics = edited
        .snapshot
        .node_semantics(edited.snapshot.revision(), selected_arm)
        .expect("W4 arm semantics");
    let references = edited
        .snapshot
        .references_to(
            edited.snapshot.revision(),
            payload_field,
            PageRequest::new(PAGE_SIZE).expect("W4 references page"),
            None,
        )
        .expect("W4 references");
    let query_wall = query_started.elapsed();
    let query = super::query::take_query_measurement();
    assert!(match_view.exhaustive);
    assert_eq!(match_type.actual, SemanticType::I64);
    assert_eq!(payload_type.declared, Some(SemanticType::I64));
    assert_eq!(semantics.actual, SemanticType::I64);
    assert!(!references.items.is_empty());
    assert!(references
        .items
        .iter()
        .all(|reference| reference.target == payload_field));

    super::projection::reset_projection_measurement();
    let projection_started = Instant::now();
    let projection = edited
        .snapshot
        .project(&[
            ProjectionSlice::Entity(pair),
            ProjectionSlice::Entity(payload_field),
            ProjectionSlice::Body(main),
            ProjectionSlice::Match(match_site),
            ProjectionSlice::Type(selected_arm),
        ])
        .expect("W4 projection");
    let projection_wall = projection_started.elapsed();
    let projection_work = super::projection::take_projection_measurement();

    crate::pipeline::reset_lowering_invocations();
    let compile_started = Instant::now();
    let (executable, compile) =
        crate::pipeline::compile_snapshot_with_metrics(&edited.snapshot).expect("compile W4");
    let compile_wall = compile_started.elapsed();
    let (new_result, vm_wall) = returned_i64(&executable);
    let old_result = compile_and_run_i64(&old_snapshot);
    assert_eq!((old_result, new_result), (42, 43));
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);

    json!({
        "schema": "lkjscript.workspace-recompute-sample.v2", "workload": "W4",
        "geometry": {
            "helper_functions": 0, "total_callables": 1,
            "total_entities": count(edited.snapshot.entities().len()),
            "total_semantic_nodes": count(edited.snapshot.nodes().len()),
            "affected_root_nodes": count(old_snapshot.nodes().len()), "changed_semantic_nodes": 1,
            "draft_nodes": 1, "page_size": PAGE_SIZE, "retained_old_revisions": 1,
            "product_fields": 2, "enum_variants": 2, "match_arms": 2,
        },
        "fixture": { "wall_ns": nanoseconds(fixture_wall) },
        "transaction": transaction_value(transaction_wall, transaction, &edited),
        "queries": query_value(query_wall, query, references.items.len() + 4),
        "projection": projection_value(projection_wall, projection_work, &projection),
        "compile": compile_value(compile_wall, compile, &executable),
        "vm": { "wall_ns": nanoseconds(vm_wall), "result_i64": new_result },
        "correctness": {
            "source_load_invocations": crate::source::source_load_invocation_count(),
            "parser_invocations": crate::source::parser_invocation_count(),
            "match_exhaustive": true, "selected_arm_identity_preserved": true,
            "payload_binding_identity_preserved": true, "payload_member_identity_preserved": true,
            "old_snapshot_result_i64": old_result, "new_snapshot_result_i64": new_result,
        },
        "agent_loop": {
            "commands": 1, "process_round_trips": 1, "selected_api_operations": 9,
            "edit_inspect_check_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, compile_wall]),
            "authoring_loop_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, compile_wall, vm_wall]),
            "output_bytes": count(projection.len()), "output_lines": line_count(&projection),
        },
        "allocation_counts": Value::Null, "allocation_bytes": Value::Null, "retained_snapshot_bytes": Value::Null,
    })
}

fn generic_mixed_sample(helper_functions: usize) -> Value {
    assert!(helper_functions > 0, "W5 requires positive helper geometry");
    let fixture_started = Instant::now();
    let seed = 7_500_u64
        .checked_add(u64::try_from(helper_functions).expect("W5 geometry"))
        .expect("W5 seed");
    let mut workspace = Workspace::empty_deterministic(seed).expect("W5 workspace");
    let binder = DraftTypeParameterId::new(0);
    let mut edits = Vec::new();
    edits
        .try_reserve(
            helper_functions
                .checked_add(6)
                .expect("W5 declaration count"),
        )
        .expect("W5 declaration allocation");
    let helper_split = helper_functions / 2;
    for index in 0..helper_split {
        edits.push(Edit::CreateFunction {
            name: format!("scalar-helper-{index:06}"),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::I64,
        });
    }
    edits.extend([
        Edit::CreateProduct {
            name: "mixed-pair".to_owned(),
            fields: vec![
                ProductFieldDraft {
                    name: "mixed-left".to_owned(),
                    ty: SemanticType::I64,
                },
                ProductFieldDraft {
                    name: "mixed-right".to_owned(),
                    ty: SemanticType::I64,
                },
            ],
        },
        Edit::CreateEnum {
            name: "mixed-choice".to_owned(),
            variants: vec![
                EnumVariantDraft {
                    name: "mixed-some".to_owned(),
                    fields: vec![EnumFieldDraft {
                        name: "mixed-payload".to_owned(),
                        ty: SemanticType::I64,
                    }],
                },
                EnumVariantDraft {
                    name: "mixed-none".to_owned(),
                    fields: Vec::new(),
                },
            ],
        },
        Edit::CreateFunction {
            name: "control-form".to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::I64,
        },
        Edit::CreateFunction {
            name: "nominal-form".to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::I64,
        },
        Edit::CreateFunction {
            name: "copy-identity".to_owned(),
            type_parameters: vec![TypeParameterDraft {
                id: binder,
                name: "t".to_owned(),
                bounds: vec![SemanticTrait::Builtin(BuiltinTrait::Copy)],
            }],
            parameters: vec![ParameterDraft {
                name: "value".to_owned(),
                ty: DeclarationType::DraftTypeParameter(binder),
            }],
            return_type: DeclarationType::DraftTypeParameter(binder),
        },
        Edit::CreateFunction {
            name: "generic-call-target".to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::I64,
        },
    ]);
    for index in helper_split..helper_functions {
        edits.push(Edit::CreateFunction {
            name: format!("scalar-helper-{index:06}"),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::I64,
        });
    }
    edits.push(Edit::CreateMain {
        parameters: Vec::new(),
        return_type: SemanticType::I64,
    });
    let declared = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits,
        })
        .expect("declare W5 workspace");
    let named = |kind, name: &str| {
        declared
            .snapshot
            .entities()
            .iter()
            .find(|entity| entity.kind == kind && entity.name.as_ref() == name)
            .expect("W5 named entity")
            .id
    };
    let pair = named(EntityKind::Product, "mixed-pair");
    let left = named(EntityKind::ProductField, "mixed-left");
    let right = named(EntityKind::ProductField, "mixed-right");
    let some = named(EntityKind::EnumVariant, "mixed-some");
    let none = named(EntityKind::EnumVariant, "mixed-none");
    let payload_field = named(EntityKind::EnumField, "mixed-payload");
    let control = named(EntityKind::Function, "control-form");
    let nominal = named(EntityKind::Function, "nominal-form");
    let identity = named(EntityKind::Function, "copy-identity");
    let target_function = named(EntityKind::Function, "generic-call-target");
    let main = named(EntityKind::Main, "main");
    let signature = declared
        .snapshot
        .function_signature(declared.snapshot.revision(), identity)
        .expect("W5 signature");
    let type_parameter = signature.type_parameters[0].id;
    let parameter = signature.parameters[0].entity;
    let mut fills = Vec::new();
    fills
        .try_reserve(helper_functions.checked_add(5).expect("W5 fill count"))
        .expect("W5 fill allocation");
    for hole in declared.snapshot.holes() {
        let owner = hole.owner;
        let name = declared
            .snapshot
            .entity(owner)
            .expect("W5 hole owner")
            .name
            .as_ref();
        let draft = if owner == identity {
            ExpressionDraft::new(
                vec![DraftNode::Load(DraftBindingRef::Entity(parameter))],
                DraftNodeId::new(0),
            )
        } else if owner == target_function {
            ExpressionDraft::new(
                vec![
                    DraftNode::I64(10),
                    DraftNode::Call {
                        callee: identity,
                        type_arguments: vec![TypeArgumentDraft {
                            parameter: type_parameter,
                            argument: SemanticType::I64,
                        }],
                        arguments: vec![DraftNodeId::new(0)],
                    },
                ],
                DraftNodeId::new(1),
            )
        } else if owner == main {
            ExpressionDraft::new(
                vec![DraftNode::Call {
                    callee: target_function,
                    type_arguments: Vec::new(),
                    arguments: Vec::new(),
                }],
                DraftNodeId::new(0),
            )
        } else if owner == control {
            counted_loop_draft(3)
        } else if owner == nominal {
            product_enum_match_draft(pair, left, right, some, none, payload_field)
        } else {
            assert!(name.starts_with("scalar-helper-"));
            ExpressionDraft::scalar_i64(1)
        };
        fills.push(Edit::FillHole {
            hole: hole.id,
            draft,
        });
    }
    let completed = workspace
        .apply(Transaction {
            base_revision: declared.snapshot.revision(),
            edits: fills,
        })
        .expect("complete W5 workspace");
    let fixture_wall = fixture_started.elapsed();
    let old_snapshot = completed.snapshot;
    let call = old_snapshot
        .nodes()
        .iter()
        .find(|node| {
            node.kind == NodeKind::Call
                && old_snapshot
                    .indexes
                    .node_enclosing_entities
                    .get(old_snapshot.indexes.node_lookup[&node.id])
                    .is_some_and(|owner| *owner == target_function)
        })
        .expect("W5 exact generic call")
        .id;
    let argument = old_snapshot
        .containment()
        .iter()
        .find_map(|edge| match (edge.owner, edge.child) {
            (SemanticOwner::Node(owner), SemanticChild::Node(child)) if owner == call => {
                Some(child)
            }
            _ => None,
        })
        .expect("W5 call argument");
    let old_instantiation = old_snapshot
        .call_instantiation(old_snapshot.revision(), call)
        .expect("W5 old instantiation");

    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let transaction_started = Instant::now();
    let edited = workspace
        .apply(Transaction {
            base_revision: old_snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: argument,
                draft: ExpressionDraft::scalar_i64(11),
            }],
        })
        .expect("edit W5 value argument");
    let transaction_wall = transaction_started.elapsed();
    let transaction = super::transaction::take_transaction_measurement();
    let new_instantiation = edited
        .snapshot
        .call_instantiation(edited.snapshot.revision(), call)
        .expect("W5 new instantiation");
    assert_eq!(
        old_instantiation.type_arguments,
        new_instantiation.type_arguments
    );
    assert_eq!(old_instantiation.witnesses, new_instantiation.witnesses);
    assert_eq!(old_instantiation.parameters, new_instantiation.parameters);
    assert_eq!(old_instantiation.result, new_instantiation.result);
    assert_eq!(new_instantiation.witnesses.len(), 1);
    let witness = &new_instantiation.witnesses[0];
    assert_eq!(witness.parameter, type_parameter);
    assert_eq!(
        witness.trait_identity,
        SemanticTrait::Builtin(BuiltinTrait::Copy)
    );
    assert_eq!(witness.ty, SemanticType::I64);
    assert_eq!(witness.kind, TraitWitnessKindView::AutoTrait);

    super::query::reset_query_measurement();
    let query_started = Instant::now();
    let signature_query = edited
        .snapshot
        .function_signature(edited.snapshot.revision(), identity)
        .expect("W5 signature query");
    let call_query = edited
        .snapshot
        .call_instantiation(edited.snapshot.revision(), call)
        .expect("W5 call query");
    let callers = edited
        .snapshot
        .callers_of(
            edited.snapshot.revision(),
            identity,
            PageRequest::new(PAGE_SIZE).expect("W5 callers page"),
            None,
        )
        .expect("W5 callers");
    let callees = edited
        .snapshot
        .callees_of(
            edited.snapshot.revision(),
            target_function,
            PageRequest::new(PAGE_SIZE).expect("W5 callees page"),
            None,
        )
        .expect("W5 callees");
    let argument_type = edited
        .snapshot
        .node_type(edited.snapshot.revision(), argument)
        .expect("W5 argument type");
    let function_type = edited
        .snapshot
        .entity_type(edited.snapshot.revision(), identity)
        .expect("W5 function type");
    let argument_semantics = edited
        .snapshot
        .node_semantics(edited.snapshot.revision(), argument)
        .expect("W5 argument semantics");
    let references = edited
        .snapshot
        .references_to(
            edited.snapshot.revision(),
            identity,
            PageRequest::new(PAGE_SIZE).expect("W5 references page"),
            None,
        )
        .expect("W5 references");
    let search = edited
        .snapshot
        .search_entities(
            edited.snapshot.revision(),
            "scalar-helper",
            PageRequest::new(PAGE_SIZE).expect("W5 search page"),
            None,
        )
        .expect("W5 search");
    let second_page = search
        .continuation
        .as_ref()
        .map(|continuation| {
            edited.snapshot.search_entities(
                edited.snapshot.revision(),
                "scalar-helper",
                PageRequest::new(PAGE_SIZE).expect("W5 second search page"),
                Some(continuation),
            )
        })
        .transpose()
        .expect("W5 second search");
    let query_wall = query_started.elapsed();
    let query = super::query::take_query_measurement();
    assert_eq!(signature_query.type_parameters.len(), 1);
    assert_eq!(call_query.type_arguments[0].argument, SemanticType::I64);
    assert_eq!(argument_type.actual, SemanticType::I64);
    assert!(function_type.declared.is_some());
    assert_eq!(argument_semantics.actual, SemanticType::I64);
    assert_eq!(callers.items.len(), 1);
    assert_eq!(callees.items.len(), 1);
    assert_eq!(references.items.len(), 1);
    let query_operations = 9_usize + usize::from(second_page.is_some());
    let observed = signature_query.type_parameters.len()
        + signature_query.parameters.len()
        + call_query.type_arguments.len()
        + call_query.witnesses.len()
        + callers.items.len()
        + callees.items.len()
        + references.items.len()
        + search.items.len()
        + second_page.as_ref().map_or(0, |page| page.items.len())
        + 1;

    super::projection::reset_projection_measurement();
    let projection_started = Instant::now();
    let projection = edited
        .snapshot
        .project(&[
            ProjectionSlice::Entity(identity),
            ProjectionSlice::Call(call),
            ProjectionSlice::References(identity),
            ProjectionSlice::Body(target_function),
            ProjectionSlice::Body(main),
            ProjectionSlice::Type(argument),
        ])
        .expect("W5 projection");
    let projection_wall = projection_started.elapsed();
    let projection_work = super::projection::take_projection_measurement();

    crate::pipeline::reset_lowering_invocations();
    let compile_started = Instant::now();
    let (executable, compile) =
        crate::pipeline::compile_snapshot_with_metrics(&edited.snapshot).expect("compile W5");
    let compile_wall = compile_started.elapsed();
    let (new_result, vm_wall) = returned_i64(&executable);
    let old_result = compile_and_run_i64(&old_snapshot);
    assert_eq!((old_result, new_result), (10, 11));
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);

    json!({
        "schema": "lkjscript.workspace-recompute-sample.v2", "workload": "W5",
        "geometry": {
            "helper_functions": count(helper_functions), "total_callables": count(helper_functions.checked_add(5).expect("W5 callable geometry")),
            "total_entities": count(edited.snapshot.entities().len()), "total_semantic_nodes": count(edited.snapshot.nodes().len()),
            "affected_root_nodes": count(old_snapshot.indexes.node_enclosing_entities.iter().filter(|owner| **owner == target_function).count()),
            "changed_semantic_nodes": 1, "draft_nodes": 1, "page_size": PAGE_SIZE, "retained_old_revisions": 1,
            "product_fields": 2, "enum_variants": 2, "match_arms": 2, "generic_type_parameters": 1,
            "helpers_before_target": count(helper_split), "helpers_after_target": count(helper_functions - helper_split),
        },
        "fixture": { "wall_ns": nanoseconds(fixture_wall) },
        "transaction": transaction_value(transaction_wall, transaction, &edited),
        "queries": query_value(query_wall, query, observed),
        "projection": projection_value(projection_wall, projection_work, &projection),
        "compile": compile_value(compile_wall, compile, &executable),
        "vm": { "wall_ns": nanoseconds(vm_wall), "result_i64": new_result },
        "correctness": {
            "source_load_invocations": crate::source::source_load_invocation_count(), "parser_invocations": crate::source::parser_invocation_count(),
            "call_identity_preserved": true, "argument_identity_preserved": true,
            "substitutions_unchanged": true, "witnesses_unchanged": true,
            "old_snapshot_result_i64": old_result, "new_snapshot_result_i64": new_result,
        },
        "agent_loop": {
            "commands": 1, "process_round_trips": 1, "selected_api_operations": count(query_operations + 4),
            "edit_inspect_check_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, compile_wall]),
            "authoring_loop_wall_ns": authoring_loop_wall(&[transaction_wall, query_wall, projection_wall, compile_wall, vm_wall]),
            "output_bytes": count(projection.len()), "output_lines": line_count(&projection),
        },
        "allocation_counts": Value::Null, "allocation_bytes": Value::Null, "retained_snapshot_bytes": Value::Null,
    })
}

fn lifecycle_sample() -> Value {
    let fixture_started = Instant::now();
    let mut workspace = Workspace::empty_deterministic(7_600).expect("W6 workspace");
    let initial_workspace_wall = fixture_started.elapsed();

    let create_started = Instant::now();
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateFunction {
                name: "survivor".to_owned(),
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                return_type: DeclarationType::I64,
            }],
        })
        .expect("W6 independent function creation");
    let create_wall = create_started.elapsed();
    let create_measurement = super::transaction::take_transaction_measurement();
    let survivor = created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("W6 survivor")
        .id;
    let survivor_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == survivor)
        .expect("W6 survivor hole")
        .id;
    let create_value = transaction_value(create_wall, create_measurement, &created);

    let complete_started = Instant::now();
    let body_completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: survivor_hole,
                draft: ExpressionDraft::scalar_i64(5),
            }],
        })
        .expect("W6 independent body completion");
    let complete_wall = complete_started.elapsed();
    let complete_measurement = super::transaction::take_transaction_measurement();
    let complete_value = transaction_value(complete_wall, complete_measurement, &body_completed);

    let setup_started = Instant::now();
    let declarations = workspace
        .apply(Transaction {
            base_revision: body_completed.snapshot.revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "disposable".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::I64,
                },
                Edit::CreateFunction {
                    name: "tail-survivor".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    parameters: Vec::new(),
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("W6 create main and disposable");
    let disposable = declarations
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function && entity.name.as_ref() == "disposable")
        .expect("W6 disposable")
        .id;
    let tail_survivor = declarations
        .snapshot
        .entities()
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::Function && entity.name.as_ref() == "tail-survivor"
        })
        .expect("W6 tail survivor")
        .id;
    let main = declarations
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Main)
        .expect("W6 main")
        .id;
    let disposable_hole = declarations
        .snapshot
        .holes()
        .find(|hole| hole.owner == disposable)
        .expect("W6 disposable hole")
        .id;
    let tail_hole = declarations
        .snapshot
        .holes()
        .find(|hole| hole.owner == tail_survivor)
        .expect("W6 tail-survivor hole")
        .id;
    let main_hole = declarations
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("W6 main hole")
        .id;
    workspace
        .apply(Transaction {
            base_revision: declarations.snapshot.revision(),
            edits: vec![
                Edit::FillHole {
                    hole: disposable_hole,
                    draft: ExpressionDraft::scalar_i64(99),
                },
                Edit::FillHole {
                    hole: tail_hole,
                    draft: ExpressionDraft::scalar_i64(6),
                },
                Edit::FillHole {
                    hole: main_hole,
                    draft: ExpressionDraft::new(
                        vec![DraftNode::Call {
                            callee: survivor,
                            type_arguments: Vec::new(),
                            arguments: Vec::new(),
                        }],
                        DraftNodeId::new(0),
                    ),
                },
            ],
        })
        .expect("W6 complete executable workspace");
    super::transaction::take_transaction_measurement();
    let setup_wall = setup_started.elapsed();

    let rename_started = Instant::now();
    let renamed = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::RenameEntity {
                entity: survivor,
                new_name: "renamed-survivor".to_owned(),
            }],
        })
        .expect("W6 supported rename");
    let rename_wall = rename_started.elapsed();
    let rename_measurement = super::transaction::take_transaction_measurement();
    let rename_value = transaction_value(rename_wall, rename_measurement, &renamed);
    assert_eq!(
        renamed
            .snapshot
            .entity(survivor)
            .expect("W6 renamed survivor")
            .id,
        survivor
    );

    let published = workspace.current();
    let published_revision = published.revision();
    let invalid_started = Instant::now();
    let invalid = workspace.apply(Transaction {
        base_revision: body_completed.snapshot.revision(),
        edits: vec![Edit::RenameEntity {
            entity: survivor,
            new_name: "must-not-publish".to_owned(),
        }],
    });
    let invalid_wall = invalid_started.elapsed();
    assert!(matches!(invalid, Err(WorkspaceError::StaleRevision)));
    assert!(Arc::ptr_eq(&published, &workspace.current()));
    assert_eq!(workspace.current().revision(), published_revision);

    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let old_snapshot = workspace.current();
    let disposable_nodes: Vec<_> = old_snapshot
        .nodes()
        .iter()
        .filter(|node| node.owner == SemanticOwner::Entity(disposable))
        .map(|node| node.id)
        .collect();
    let tail_root = old_snapshot
        .nodes()
        .iter()
        .find(|node| node.owner == SemanticOwner::Entity(tail_survivor))
        .expect("W6 tail-survivor root")
        .id;
    let old_tail_address =
        old_snapshot.indexes.entity_addresses[old_snapshot.indexes.entity_lookup[&tail_survivor]];
    let transaction_started = Instant::now();
    let deleted = workspace
        .apply(Transaction {
            base_revision: old_snapshot.revision(),
            edits: vec![Edit::DeleteEntity { entity: disposable }],
        })
        .expect("W6 dependency-closed deletion");
    let transaction_wall = transaction_started.elapsed();
    let transaction = super::transaction::take_transaction_measurement();
    assert_eq!(transaction.compaction_invocations, 1);
    assert!(deleted.snapshot.entity(disposable).is_err());
    assert!(old_snapshot.entity(disposable).is_ok());
    assert!(deleted.snapshot.entity(survivor).is_ok());
    assert!(deleted.snapshot.entity(tail_survivor).is_ok());
    assert!(deleted.snapshot.entity(main).is_ok());
    assert_eq!(
        deleted
            .snapshot
            .node(tail_root)
            .expect("W6 relocated tail node")
            .id,
        tail_root
    );
    let new_tail_address = deleted.snapshot.indexes.entity_addresses
        [deleted.snapshot.indexes.entity_lookup[&tail_survivor]];
    assert_ne!(old_tail_address, new_tail_address);
    for node in &disposable_nodes {
        assert!(deleted.snapshot.node(*node).is_err());
    }

    super::query::reset_query_measurement();
    let query_started = Instant::now();
    let survivor_definition = deleted
        .snapshot
        .definition(deleted.snapshot.revision(), survivor)
        .expect("W6 survivor definition");
    let tail_definition = deleted
        .snapshot
        .definition(deleted.snapshot.revision(), tail_survivor)
        .expect("W6 tail-survivor definition");
    let callers = deleted
        .snapshot
        .callers_of(
            deleted.snapshot.revision(),
            survivor,
            PageRequest::new(PAGE_SIZE).expect("W6 callers page"),
            None,
        )
        .expect("W6 callers");
    let search = deleted
        .snapshot
        .search_entities(
            deleted.snapshot.revision(),
            "survivor",
            PageRequest::new(PAGE_SIZE).expect("W6 search page"),
            None,
        )
        .expect("W6 search");
    let tombstone = deleted
        .snapshot
        .definition(deleted.snapshot.revision(), disposable);
    let query_wall = query_started.elapsed();
    let query = super::query::take_query_measurement();
    assert_eq!(survivor_definition.name.as_ref(), "renamed-survivor");
    assert_eq!(tail_definition.id, tail_survivor);
    assert_eq!(callers.items.len(), 1);
    assert_eq!(search.items.len(), 2);
    assert!(matches!(tombstone, Err(WorkspaceError::StaleIdentity(_))));

    super::projection::reset_projection_measurement();
    let projection_started = Instant::now();
    let projection = deleted
        .snapshot
        .project(&[
            ProjectionSlice::Entity(survivor),
            ProjectionSlice::Entity(tail_survivor),
            ProjectionSlice::Body(main),
            ProjectionSlice::References(survivor),
        ])
        .expect("W6 projection");
    let projection_wall = projection_started.elapsed();
    let projection_work = super::projection::take_projection_measurement();

    crate::pipeline::reset_lowering_invocations();
    let compile_started = Instant::now();
    let (executable, compile) =
        crate::pipeline::compile_snapshot_with_metrics(&deleted.snapshot).expect("compile W6");
    let compile_wall = compile_started.elapsed();
    let (new_result, vm_wall) = returned_i64(&executable);
    let old_result = compile_and_run_i64(&old_snapshot);
    assert_eq!((old_result, new_result), (5, 5));
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);

    json!({
        "schema": "lkjscript.workspace-recompute-sample.v2", "workload": "W6",
        "geometry": {
            "helper_functions": 0, "total_callables": 3, "surviving_callables": 2,
            "total_entities": count(deleted.snapshot.entities().len()), "total_semantic_nodes": count(deleted.snapshot.nodes().len()),
            "affected_root_nodes": count(disposable_nodes.len()), "changed_semantic_nodes": count(disposable_nodes.len()), "draft_nodes": 0,
            "page_size": PAGE_SIZE, "retained_old_revisions": 4, "deleted_entities": 1,
            "deleted_semantic_nodes": count(disposable_nodes.len()),
        },
        "fixture": { "initial_workspace_wall_ns": nanoseconds(initial_workspace_wall), "setup_wall_ns": nanoseconds(setup_wall) },
        "transaction": transaction_value(transaction_wall, transaction, &deleted),
        "sequence": {
            "create_function": create_value, "complete_function_body": complete_value, "rename": rename_value,
            "invalid_stale_edit": { "wall_ns": nanoseconds(invalid_wall), "error": "stale-revision", "revision_unchanged": true, "snapshot_arc_unchanged": true },
            "delete": { "revision": deleted.snapshot.revision().sequence() },
        },
        "queries": query_value(query_wall, query, callers.items.len() + search.items.len() + 3),
        "projection": projection_value(projection_wall, projection_work, &projection),
        "compile": compile_value(compile_wall, compile, &executable),
        "vm": { "wall_ns": nanoseconds(vm_wall), "result_i64": new_result },
        "correctness": {
            "source_load_invocations": crate::source::source_load_invocation_count(), "parser_invocations": crate::source::parser_invocation_count(),
            "survivor_identity_preserved": true, "relocated_survivor_identity_preserved": true,
            "relocated_survivor_node_identity_preserved": true, "deleted_identity_tombstoned": true, "old_snapshot_preserved": true,
            "failed_edit_atomic": true, "private_binding_relocated": true, "private_compaction_observed": true,
            "old_snapshot_result_i64": old_result, "new_snapshot_result_i64": new_result,
        },
        "agent_loop": {
            "commands": 1, "process_round_trips": 1, "selected_api_operations": 13,
            "edit_inspect_check_wall_ns": authoring_loop_wall(&[create_wall, complete_wall, rename_wall, invalid_wall, transaction_wall, query_wall, projection_wall, compile_wall]),
            "authoring_loop_wall_ns": authoring_loop_wall(&[create_wall, complete_wall, rename_wall, invalid_wall, transaction_wall, query_wall, projection_wall, compile_wall, vm_wall]),
            "output_bytes": count(projection.len()), "output_lines": line_count(&projection),
        },
        "allocation_counts": Value::Null, "allocation_bytes": Value::Null, "retained_snapshot_bytes": Value::Null,
    })
}

fn hole_lifecycle_sample() -> Value {
    let fixture_started = Instant::now();
    let (mut workspace, main, hole) = create_width_fixture(7_700, 0);
    let completed = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::scalar_i64(7),
            }],
        })
        .expect("complete W7 fixture");
    let fixture_wall = fixture_started.elapsed();
    let complete_snapshot = completed.snapshot;
    let root = complete_snapshot.nodes()[0].id;

    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let introduce_started = Instant::now();
    let introduced = workspace
        .apply(Transaction {
            base_revision: complete_snapshot.revision(),
            edits: vec![Edit::IntroduceHole {
                target: root,
                goal: "choose the final scalar".to_owned(),
            }],
        })
        .expect("introduce W7 typed hole");
    let introduce_wall = introduce_started.elapsed();
    let introduce_measurement = super::transaction::take_transaction_measurement();
    let introduce_value = transaction_value(introduce_wall, introduce_measurement, &introduced);
    let typed_hole = HoleId(root);
    assert_eq!(introduced.snapshot.state(), ProgramState::Incomplete);
    assert_eq!(
        introduced.snapshot.node(root).expect("W7 hole root").kind,
        NodeKind::Hole
    );

    super::query::reset_query_measurement();
    let query_started = Instant::now();
    let context = introduced
        .snapshot
        .hole_context(introduced.snapshot.revision(), typed_hole)
        .expect("W7 hole context");
    let blockers = introduced.snapshot.completeness_blockers();
    let diagnostics = introduced
        .snapshot
        .diagnostic_page(
            introduced.snapshot.revision(),
            PageRequest::new(PAGE_SIZE).expect("W7 diagnostics page"),
            None,
        )
        .expect("W7 diagnostics");
    let constructors = introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            typed_hole,
            PageRequest::new(PAGE_SIZE).expect("W7 constructors page"),
            None,
        )
        .expect("W7 constructors");
    let semantics = introduced
        .snapshot
        .node_semantics(introduced.snapshot.revision(), root)
        .expect("W7 hole semantics");
    let query_wall = query_started.elapsed();
    let query = super::query::take_query_measurement();
    assert_eq!(context.expected_type, SemanticType::I64);
    assert_eq!(blockers.len(), 1);
    assert_eq!(diagnostics.items.len(), 1);
    assert!(constructors.items.contains(&LegalConstructor::I64Literal));
    assert_eq!(semantics.kind, NodeKind::Hole);

    super::projection::reset_projection_measurement();
    let projection_started = Instant::now();
    let incomplete_projection = introduced
        .snapshot
        .project(&[
            ProjectionSlice::Hole(typed_hole),
            ProjectionSlice::Body(main),
            ProjectionSlice::Type(root),
        ])
        .expect("W7 incomplete projection");
    let projection_wall = projection_started.elapsed();
    let projection_work = super::projection::take_projection_measurement();

    crate::pipeline::reset_lowering_invocations();
    let incomplete_compile_started = Instant::now();
    let incomplete_error =
        crate::compile_snapshot(&introduced.snapshot).expect_err("W7 incomplete compile");
    let incomplete_compile_wall = incomplete_compile_started.elapsed();
    assert!(matches!(
        incomplete_error,
        crate::CompileSnapshotError::Incomplete(_)
    ));
    assert_eq!(crate::pipeline::lowering_invocations(), 0);

    let transaction_started = Instant::now();
    let filled = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: typed_hole,
                draft: ExpressionDraft::scalar_i64(9),
            }],
        })
        .expect("fill W7 typed hole");
    let transaction_wall = transaction_started.elapsed();
    let transaction = super::transaction::take_transaction_measurement();
    assert_eq!(
        filled.snapshot.node(root).expect("W7 refilled root").id,
        root
    );
    assert_eq!(filled.snapshot.state(), ProgramState::Complete);

    let compile_started = Instant::now();
    let (executable, compile) = crate::pipeline::compile_snapshot_with_metrics(&filled.snapshot)
        .expect("compile W7 complete");
    let compile_wall = compile_started.elapsed();
    let (new_result, vm_wall) = returned_i64(&executable);
    let old_result = compile_and_run_i64(&complete_snapshot);
    assert_eq!((old_result, new_result), (7, 9));
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);

    json!({
        "schema": "lkjscript.workspace-recompute-sample.v2", "workload": "W7",
        "geometry": {
            "helper_functions": 0, "total_callables": 1, "total_entities": count(filled.snapshot.entities().len()),
            "total_semantic_nodes": count(filled.snapshot.nodes().len()), "affected_root_nodes": 1,
            "changed_semantic_nodes": 1, "draft_nodes": 1, "page_size": PAGE_SIZE, "retained_old_revisions": 2,
        },
        "fixture": { "wall_ns": nanoseconds(fixture_wall) },
        "transaction": transaction_value(transaction_wall, transaction, &filled),
        "sequence": {
            "introduce_hole": introduce_value,
            "incomplete_compile": { "status": "incomplete", "wall_ns": nanoseconds(incomplete_compile_wall), "lowering_invocations": 0 },
            "fill_hole": { "revision": filled.snapshot.revision().sequence() },
        },
        "queries": query_value(query_wall, query, blockers.len() + diagnostics.items.len() + constructors.items.len() + 2),
        "projection": projection_value(projection_wall, projection_work, &incomplete_projection),
        "compile": compile_value(compile_wall, compile, &executable),
        "vm": { "wall_ns": nanoseconds(vm_wall), "result_i64": new_result },
        "correctness": {
            "source_load_invocations": crate::source::source_load_invocation_count(), "parser_invocations": crate::source::parser_invocation_count(),
            "root_identity_preserved": true, "typed_hole_expected_i64": true, "compile_stopped_before_lowering": true,
            "old_snapshot_result_i64": old_result, "new_snapshot_result_i64": new_result,
        },
        "agent_loop": {
            "commands": 1, "process_round_trips": 1, "selected_api_operations": 10,
            "edit_inspect_check_wall_ns": authoring_loop_wall(&[introduce_wall, query_wall, projection_wall, incomplete_compile_wall, transaction_wall, compile_wall]),
            "authoring_loop_wall_ns": authoring_loop_wall(&[introduce_wall, query_wall, projection_wall, incomplete_compile_wall, transaction_wall, compile_wall, vm_wall]),
            "output_bytes": count(incomplete_projection.len()), "output_lines": line_count(&incomplete_projection),
        },
        "allocation_counts": Value::Null, "allocation_bytes": Value::Null, "retained_snapshot_bytes": Value::Null,
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
                parameters: Vec::new(),
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

    let partially_staged_failure = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![
            Edit::RefineHole {
                hole,
                expected_type: Some(SemanticType::I64),
                goal: "staged but unpublished".to_owned(),
            },
            Edit::RefineHole {
                hole: foreign_hole,
                expected_type: None,
                goal: "foreign after valid refinement".to_owned(),
            },
        ],
    });
    assert!(matches!(
        partially_staged_failure,
        Err(WorkspaceError::ForeignNamespace(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    assert_eq!(workspace.current().diagnostics(), before_diagnostics);
    assert_eq!(
        workspace
            .current()
            .project(&[ProjectionSlice::Hole(hole)])
            .expect("projection after partially staged refinement failure"),
        before_projection
    );

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
    assert_eq!(refined.diff.entries.len(), 1);
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
        vec![("provide the entry-point body", "final goal")]
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
                    goal: "mixed intermediate goal".to_owned(),
                },
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
    let mixed_goal_changes: Vec<_> = mixed
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
        mixed_goal_changes,
        vec![("provide the entry-point body", "mixed goal")]
    );
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
fn hole_refinement_diff_is_net_deterministic_and_full_path_equivalent() {
    let (workspace, _main, hole) = create_width_fixture(9_003, 4);
    let base = workspace.current();
    let transaction = |goal: &str| Transaction {
        base_revision: base.revision(),
        edits: vec![Edit::RefineHole {
            hole,
            expected_type: Some(SemanticType::I64),
            goal: goal.to_owned(),
        }],
    };

    let mut no_op = Workspace::new((*base).clone()).expect("no-op refinement workspace");
    let unchanged = no_op
        .apply(transaction("provide the entry-point body"))
        .expect("publish no-op refinement revision");
    assert!(unchanged.diff.entries.is_empty());

    let mut round_trip = Workspace::new((*base).clone()).expect("round-trip refinement workspace");
    let returned = round_trip
        .apply(Transaction {
            base_revision: base.revision(),
            edits: vec![
                Edit::RefineHole {
                    hole,
                    expected_type: None,
                    goal: "temporary goal".to_owned(),
                },
                Edit::RefineHole {
                    hole,
                    expected_type: None,
                    goal: "provide the entry-point body".to_owned(),
                },
            ],
        })
        .expect("publish net-no-op refinement revision");
    assert!(returned.diff.entries.is_empty());

    let mut narrow = Workspace::new((*base).clone()).expect("narrow refinement workspace");
    let narrow_outcome = narrow
        .apply(transaction("equivalent goal"))
        .expect("narrow refinement");
    let narrow_measurement = super::transaction::take_transaction_measurement();
    assert!(narrow_measurement.metadata_only_path_used);

    let mut full = Workspace::new((*base).clone()).expect("full refinement workspace");
    super::transaction::set_force_full_recomputation(true);
    let full_result = full.apply(transaction("equivalent goal"));
    super::transaction::set_force_full_recomputation(false);
    let full_outcome = full_result.expect("full refinement");
    let full_measurement = super::transaction::take_transaction_measurement();
    assert!(!full_measurement.metadata_only_path_used);
    assert_eq!(narrow_outcome.diff, full_outcome.diff);
    assert_eq!(narrow_outcome.invalidated, full_outcome.invalidated);
    assert_eq!(
        narrow_outcome.snapshot.entities(),
        full_outcome.snapshot.entities()
    );
    assert_eq!(
        narrow_outcome.snapshot.nodes(),
        full_outcome.snapshot.nodes()
    );
    assert_eq!(
        narrow_outcome.snapshot.holes().collect::<Vec<_>>(),
        full_outcome.snapshot.holes().collect::<Vec<_>>()
    );
    assert_eq!(
        narrow_outcome.snapshot.diagnostics(),
        full_outcome.snapshot.diagnostics()
    );
    assert_eq!(
        narrow_outcome.snapshot.completeness_blockers(),
        full_outcome.snapshot.completeness_blockers()
    );
    assert_eq!(
        narrow_outcome
            .snapshot
            .project(&[])
            .expect("narrow projection"),
        full_outcome.snapshot.project(&[]).expect("full projection")
    );
    assert!(Arc::ptr_eq(&base.program, &narrow_outcome.snapshot.program));
    assert!(!Arc::ptr_eq(&base.program, &full_outcome.snapshot.program));

    let mut multi = Workspace::empty_deterministic(9_004).expect("multi-hole workspace");
    let created = multi
        .apply(Transaction {
            base_revision: multi.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "second-hole".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    parameters: Vec::new(),
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create two holes");
    let mut holes: Vec<_> = created.snapshot.holes().map(|state| state.id).collect();
    holes.sort_unstable();
    let refined = multi
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: holes
                .iter()
                .rev()
                .map(|hole| Edit::RefineHole {
                    hole: *hole,
                    expected_type: None,
                    goal: format!("goal for slot {}", hole.node().slot()),
                })
                .collect(),
        })
        .expect("refine two holes in reverse order");
    let diff_holes: Vec<_> = refined
        .diff
        .entries
        .iter()
        .filter_map(|entry| match entry {
            SemanticDiffEntry::HoleRefined { hole, .. } => Some(*hole),
            _ => None,
        })
        .collect();
    assert_eq!(diff_holes, holes);
}

#[test]
fn workspace_recompute_measurement_is_semantically_exact() {
    let samples = [
        control_sample(),
        hole_refinement_sample(8),
        imperative_edit_sample(8),
        ownership_edit_sample(),
        product_enum_match_sample(),
        generic_mixed_sample(8),
        lifecycle_sample(),
        hole_lifecycle_sample(),
    ];
    let expected_operations = [6_u64, 7, 9, 6, 9, 13, 13, 10];
    for (index, sample) in samples.iter().enumerate() {
        assert_eq!(sample["schema"], "lkjscript.workspace-recompute-sample.v2");
        assert_eq!(sample["workload"], format!("W{index}"));
        assert_eq!(sample["correctness"]["source_load_invocations"], 0);
        assert_eq!(sample["correctness"]["parser_invocations"], 0);
        assert!(sample["agent_loop"]["edit_inspect_check_wall_ns"].is_u64());
        assert!(sample["agent_loop"]["authoring_loop_wall_ns"].is_u64());
        assert_eq!(
            sample["agent_loop"]["selected_api_operations"],
            expected_operations[index]
        );
    }
    assert_eq!(samples[0]["correctness"]["new_snapshot_result_i64"], 8);
    assert_eq!(samples[1]["compile"]["lowering_invocations"], 0);
    assert_eq!(samples[2]["correctness"]["new_snapshot_result_i64"], 101);
    assert_eq!(samples[3]["memory"]["cleanup_failures"], 0);
    assert_eq!(samples[4]["correctness"]["new_snapshot_result_i64"], 43);
    assert_eq!(samples[5]["correctness"]["new_snapshot_result_i64"], 11);
    assert_eq!(samples[6]["correctness"]["failed_edit_atomic"], true);
    assert_eq!(
        samples[7]["sequence"]["incomplete_compile"]["lowering_invocations"],
        0
    );
}

#[test]
#[ignore = "locked-release semantic-workspace recomputation measurement"]
fn workspace_recompute_scale_sample() {
    let workload = std::env::var("LKJSCRIPT_WORKSPACE_WORKLOAD")
        .expect("LKJSCRIPT_WORKSPACE_WORKLOAD must select W0 through W7");
    let helper_functions = || {
        let value = std::env::var("LKJSCRIPT_WORKSPACE_FUNCTIONS")
            .expect("LKJSCRIPT_WORKSPACE_FUNCTIONS is required for W1, W2, and W5")
            .parse::<usize>()
            .expect("LKJSCRIPT_WORKSPACE_FUNCTIONS must be a positive integer");
        assert!(value > 0, "LKJSCRIPT_WORKSPACE_FUNCTIONS must be positive");
        value
    };
    let sample = match workload.as_str() {
        "W0" => control_sample(),
        "W1" => hole_refinement_sample(helper_functions()),
        "W2" => imperative_edit_sample(helper_functions()),
        "W3" => ownership_edit_sample(),
        "W4" => product_enum_match_sample(),
        "W5" => generic_mixed_sample(helper_functions()),
        "W6" => lifecycle_sample(),
        "W7" => hole_lifecycle_sample(),
        other => panic!("unsupported workspace measurement workload {other}"),
    };
    eprintln!(
        "{MARKER}{}",
        serde_json::to_string(&sample).expect("serialize workspace recomputation sample")
    );
}
