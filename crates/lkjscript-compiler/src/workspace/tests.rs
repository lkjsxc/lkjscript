#![allow(clippy::expect_used, clippy::panic)]

use std::collections::HashSet;

use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_vm::{run_chunk, ExecutionInputs};

use super::*;

const SCALAR: &str = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n42\n/main\n";
const FUNCTION_PROGRAM: &str = "def/\nname/\nidentity\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nvalue\ni64\n/params\nvalue\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nidentity/\n41\n/identity\n/main\n";
const FUNCTION_PROGRAM_42: &str = "def/\nname/\nidentity\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nvalue\ni64\n/params\nvalue\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nidentity/\n42\n/identity\n/main\n";
const CONDITIONAL: &str =
    "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nif/\ntrue\n1\n2\n/if\n/main\n";

fn run_i64(snapshot: &WorkspaceSnapshot) -> i64 {
    let executable = crate::compile_snapshot(snapshot).expect("compile complete snapshot");
    match run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ) {
        ExecutionOutcome::Returned(value) => value.as_i64().expect("returned i64"),
        outcome => panic!("unexpected execution outcome: {outcome:?}"),
    }
}

fn run_bool(snapshot: &WorkspaceSnapshot) -> bool {
    let executable = crate::compile_snapshot(snapshot).expect("compile complete snapshot");
    match run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ) {
        ExecutionOutcome::Returned(value) => value.as_bool().expect("returned Boolean"),
        outcome => panic!("unexpected execution outcome: {outcome:?}"),
    }
}

fn push_draft_node(nodes: &mut Vec<DraftNode>, node: DraftNode) -> DraftNodeId {
    let id = DraftNodeId::new(u64::try_from(nodes.len()).expect("draft node identity"));
    nodes.push(node);
    id
}

fn return_i64_draft(value: i64) -> ExpressionDraft {
    ExpressionDraft::new(
        vec![
            DraftNode::Return {
                value: DraftNodeId::new(1),
            },
            DraftNode::I64(value),
        ],
        DraftNodeId::new(0),
    )
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

fn create_source_free_declarations(
    seed: u64,
) -> (Workspace, EntityId, EntityId, EntityId, HoleId, HoleId) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("empty workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "identity".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "value".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create source-free declarations");
    let function = created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("function")
        .id;
    let parameter = created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Parameter)
        .expect("parameter")
        .id;
    let main = created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Main)
        .expect("main")
        .id;
    let function_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == function)
        .expect("function hole")
        .id;
    let main_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;
    (
        workspace,
        function,
        parameter,
        main,
        function_hole,
        main_hole,
    )
}

fn fill_source_free_identity(
    workspace: &mut Workspace,
    function: EntityId,
    parameter: EntityId,
    function_hole: HoleId,
    main_hole: HoleId,
) -> Arc<WorkspaceSnapshot> {
    let function_filled = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole: function_hole,
                draft: ExpressionDraft::new(
                    vec![DraftNode::Load(DraftBindingRef::Entity(parameter))],
                    DraftNodeId::new(0),
                ),
            }],
        })
        .expect("fill identity function");
    workspace
        .apply(Transaction {
            base_revision: function_filled.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::Call {
                            callee: function,
                            type_arguments: Vec::new(),
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("fill main")
        .snapshot
}

#[test]
fn deterministic_import_assigns_stable_opaque_ids_in_an_injected_namespace() {
    let namespace = WorkspaceNamespace::deterministic(7);
    let first =
        importer::import_source_with_namespace(SCALAR, "workspace-stable.lkjscript", namespace)
            .expect("import first deterministic snapshot");
    let second =
        importer::import_source_with_namespace(SCALAR, "workspace-stable.lkjscript", namespace)
            .expect("import second deterministic snapshot");

    assert_eq!(first.namespace(), namespace);
    assert_eq!(first.revision(), second.revision());
    assert_eq!(first.entities(), second.entities());
    assert_eq!(first.nodes(), second.nodes());
    assert_eq!(first.containment(), second.containment());
    assert_eq!(first.references(), second.references());
    assert_eq!(first.calls(), second.calls());
    assert_eq!(first.dependencies(), second.dependencies());
    assert_eq!(first.state(), ProgramState::Complete);
}

#[test]
fn identities_from_another_workspace_are_rejected_before_lookup() {
    let first = import_source(SCALAR, "workspace-first.lkjscript").expect("import first workspace");
    let second =
        import_source(SCALAR, "workspace-second.lkjscript").expect("import second workspace");
    let foreign_entity = first.entities()[0].id;
    let foreign_node = first.nodes()[0].id;

    assert!(second
        .entity(foreign_entity)
        .expect_err("reject foreign entity")
        .to_string()
        .contains("different workspace namespace"));
    assert!(second
        .node(foreign_node)
        .expect_err("reject foreign node")
        .to_string()
        .contains("different workspace namespace"));
    assert!(second
        .require_revision(first.revision())
        .expect_err("reject foreign revision")
        .to_string()
        .contains("different workspace namespace"));
}

#[test]
fn source_free_construction_never_invokes_parser_and_executes() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    crate::pipeline::reset_lowering_invocations();
    let mut workspace = Workspace::empty_deterministic(12).expect("empty workspace");
    let empty = workspace.current();
    assert_eq!(empty.state(), ProgramState::Incomplete);
    assert!(empty.entities().is_empty());
    assert!(empty.nodes().is_empty());
    assert!(empty.attachments().is_none());
    assert!(empty.source_origins.is_empty());
    empty
        .check_consistency()
        .expect("consistent empty snapshot");
    assert_eq!(empty.diagnostics().len(), 1);
    assert_eq!(
        empty.diagnostics()[0].code.as_ref(),
        "workspace.missing-entry-point"
    );
    assert_eq!(
        empty.project(&[]).expect("empty projection"),
        "workspace revision=1 state=incomplete\nblocker missing-entry-point\n"
    );
    assert_eq!(
        empty.completeness_blockers(),
        &[CompletenessBlocker::MissingEntryPoint]
    );
    assert!(matches!(
        crate::compile_snapshot(&empty),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));
    assert_eq!(crate::pipeline::lowering_invocations(), 0);

    let created = workspace
        .apply(Transaction {
            base_revision: empty.revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "identity".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "value".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create declarations");
    let function = created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("function")
        .id;
    let parameter = created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Parameter)
        .expect("parameter")
        .id;
    let main = created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Main)
        .expect("main")
        .id;
    assert_eq!(created.snapshot.holes().len(), 2);
    let created_ids = created
        .diff
        .entries
        .iter()
        .filter_map(|entry| match entry {
            SemanticDiffEntry::EntityCreated { entity, .. } => Some(*entity),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(created_ids.len(), 3);
    assert!(created_ids.contains(&function));
    assert!(created_ids.contains(&parameter));
    assert!(created_ids.contains(&main));
    assert!(created
        .snapshot
        .completeness_blockers()
        .iter()
        .all(|blocker| matches!(blocker, CompletenessBlocker::MissingBody { .. })));
    let identity_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == function)
        .expect("identity hole")
        .id;
    let main_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;
    let introduced_holes = created
        .diff
        .entries
        .iter()
        .filter_map(|entry| match entry {
            SemanticDiffEntry::HoleIntroduced { hole } => Some(*hole),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(introduced_holes.len(), 2);
    assert!(introduced_holes.contains(&identity_hole));
    assert!(introduced_holes.contains(&main_hole));
    let identity_context = created
        .snapshot
        .hole_context(created.snapshot.revision(), identity_hole)
        .expect("identity context");
    assert_eq!(identity_context.expected_type, SemanticType::I64);
    assert!(identity_context.visible_entities.contains(&parameter));
    assert!(identity_context.visible_entities.contains(&function));
    assert!(matches!(
        crate::compile_snapshot(&created.snapshot),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));
    assert_eq!(crate::pipeline::lowering_invocations(), 0);

    let identity_filled = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: identity_hole,
                draft: ExpressionDraft::new(
                    vec![DraftNode::Load(DraftBindingRef::Entity(parameter))],
                    DraftNodeId::new(0),
                ),
            }],
        })
        .expect("fill identity");
    assert_eq!(identity_filled.snapshot.state(), ProgramState::Incomplete);
    assert!(identity_filled
        .snapshot
        .program
        .functions
        .iter()
        .find(|candidate| candidate.binding.raw() == 0)
        .expect("identity semantic function")
        .summary
        .is_known());
    assert!(matches!(
        crate::compile_snapshot(&identity_filled.snapshot),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));
    assert_eq!(crate::pipeline::lowering_invocations(), 0);
    let completed = workspace
        .apply(Transaction {
            base_revision: identity_filled.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::Call {
                            callee: function,
                            type_arguments: Vec::new(),
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("fill main");
    assert_eq!(completed.snapshot.state(), ProgramState::Complete);
    assert!(completed.snapshot.completeness_blockers().is_empty());
    assert!(completed.snapshot.diagnostics().is_empty());
    completed
        .snapshot
        .check_consistency()
        .expect("consistent complete snapshot");
    assert!(completed.snapshot.source_origins.is_empty());
    assert_eq!(completed.snapshot.entities().len(), 3);
    assert_eq!(completed.snapshot.calls().len(), 1);
    assert_eq!(completed.snapshot.calls()[0].callee, function);
    assert!(completed
        .snapshot
        .references()
        .iter()
        .any(|edge| edge.target == parameter));
    let projection = completed
        .snapshot
        .project(&[
            ProjectionSlice::Entity(function),
            ProjectionSlice::Body(function),
            ProjectionSlice::Entity(main),
            ProjectionSlice::Body(main),
        ])
        .expect("source-free projection");
    assert!(projection.contains("name=\"identity\""));
    assert_eq!(
        projection,
        completed
            .snapshot
            .project(&[
                ProjectionSlice::Entity(function),
                ProjectionSlice::Body(function),
                ProjectionSlice::Entity(main),
                ProjectionSlice::Body(main),
            ])
            .expect("repeat source-free projection")
    );
    assert_eq!(run_i64(&completed.snapshot), 42);
    assert_eq!(crate::pipeline::lowering_invocations(), 1);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn unresolved_value_reference_lifecycle_is_source_free_and_executes() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    crate::pipeline::reset_lowering_invocations();
    let (mut workspace, function, parameter, _main, function_hole, main_hole) =
        create_source_free_declarations(201);
    let missing = workspace.current();
    let root = function_hole.node();

    let introduced = workspace
        .apply(Transaction {
            base_revision: missing.revision(),
            edits: vec![Edit::IntroduceUnresolvedValueReference {
                target: root,
                requested_name: "value".to_owned(),
            }],
        })
        .expect("introduce unresolved value reference");
    assert_eq!(introduced.snapshot.state(), ProgramState::Incomplete);
    introduced
        .snapshot
        .check_consistency()
        .expect("consistent unresolved snapshot");
    assert!(introduced
        .snapshot
        .holes()
        .all(|hole| hole.id != function_hole));
    assert_eq!(
        introduced
            .snapshot
            .node(root)
            .expect("unresolved node")
            .kind,
        NodeKind::UnresolvedValueReference
    );
    assert!(introduced.snapshot.references().is_empty());
    let unresolved_expression = &introduced
        .snapshot
        .program
        .functions
        .iter()
        .find(|candidate| candidate.binding.raw() == 0)
        .expect("unresolved semantic function")
        .body;
    assert_eq!(unresolved_expression.origin, crate::hir::Origin::Semantic);
    assert!(matches!(
        &unresolved_expression.kind,
        crate::hir::ExprKind::UnresolvedValueReference { requested_name }
            if requested_name.as_ref() == "value"
    ));
    let mut child_count = 0;
    crate::hir::for_each_expression_child(unresolved_expression, &mut |_| child_count += 1);
    assert_eq!(child_count, 0);
    let reference = introduced
        .snapshot
        .unresolved_value_references()
        .next()
        .expect("unresolved reference")
        .id;
    assert_eq!(reference.node(), root);
    let state = introduced
        .snapshot
        .unresolved_value_reference(introduced.snapshot.revision(), reference)
        .expect("unresolved state");
    assert_eq!(state.revision, introduced.snapshot.revision());
    assert_eq!(state.requested_name.as_ref(), "value");
    assert_eq!(state.expected_type, SemanticType::I64);
    assert_eq!(state.owner, function);
    assert_eq!(state.context, root);
    assert_eq!(state.intent, ValueReferenceIntent::CopyLoad);
    assert!(state.visible_entities.contains(&parameter));
    let unresolved_semantics = introduced
        .snapshot
        .node_semantics(introduced.snapshot.revision(), root)
        .expect("unresolved semantics");
    assert_eq!(
        unresolved_semantics.kind,
        NodeKind::UnresolvedValueReference
    );
    assert_eq!(unresolved_semantics.actual, SemanticType::I64);
    assert_eq!(unresolved_semantics.expected, Some(SemanticType::I64));
    assert!(!unresolved_semantics.effects.is_known());
    assert!(introduced.snapshot.diagnostics().iter().any(|diagnostic| {
        diagnostic.code.as_ref() == "workspace.unresolved-value-reference"
            && diagnostic.subject == Some(SemanticChild::Node(root))
            && diagnostic.message.contains("value")
    }));
    assert!(introduced
        .snapshot
        .completeness_blockers()
        .iter()
        .any(|blocker| matches!(
            blocker,
            CompletenessBlocker::UnresolvedValueReference {
                reference: blocked,
                requested_name,
                expected_type: SemanticType::I64,
                owner,
                context,
            } if *blocked == reference
                && requested_name.as_ref() == "value"
                && *owner == function
                && *context == root
        )));
    let unresolved_projection = introduced
        .snapshot
        .project(&[
            ProjectionSlice::Body(function),
            ProjectionSlice::Type(root),
            ProjectionSlice::UnresolvedValueReference(reference),
        ])
        .expect("project unresolved value reference");
    assert!(unresolved_projection.contains(
        "kind=unresolved-value-reference type=\"i64\" expected=\"i64\" operation=- effects=[unknown] [UNRESOLVED]"
    ));
    assert!(unresolved_projection
        .contains("[UNRESOLVED] intent=copy-load requested=\"value\" expected=\"i64\""));
    assert!(!unresolved_projection.contains("candidate"));
    assert_eq!(
        introduced
            .snapshot
            .without_attachments()
            .project(&[
                ProjectionSlice::Body(function),
                ProjectionSlice::Type(root),
                ProjectionSlice::UnresolvedValueReference(reference),
            ])
            .expect("attachment-free unresolved projection"),
        unresolved_projection
    );
    assert!(introduced.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::UnresolvedValueReferenceIntroduced { reference: item }
            if *item == reference
    )));

    let candidates = introduced
        .snapshot
        .unresolved_value_reference_candidates(
            introduced.snapshot.revision(),
            reference,
            PageRequest::new(1).expect("candidate page"),
            None,
        )
        .expect("value-reference candidates");
    assert_eq!(candidates.revision, introduced.snapshot.revision());
    assert_eq!(candidates.items.len(), 1);
    assert_eq!(candidates.items[0].entity, parameter);
    assert_eq!(candidates.items[0].name.as_ref(), "value");
    assert_eq!(candidates.items[0].kind, EntityKind::Parameter);
    assert_eq!(candidates.items[0].declared_type, SemanticType::I64);
    assert!(candidates.items[0].exact_name_match);
    assert_eq!(
        candidates.items[0].status,
        ValueReferenceCandidateStatus::RequiresCanonicalValidation
    );
    assert!(candidates.continuation.is_none());
    assert_eq!(crate::pipeline::lowering_invocations(), 0);
    assert!(introduced
        .snapshot
        .program
        .try_complete(&introduced.snapshot.source_origins)
        .is_err());
    let mut missing_record = (*introduced.snapshot).clone();
    missing_record.unresolved_value_references = Arc::from([]);
    assert!(missing_record.check_consistency().is_err());
    let mut duplicate_record = (*introduced.snapshot).clone();
    duplicate_record.unresolved_value_references = Arc::from([
        introduced.snapshot.unresolved_value_references[0].clone(),
        introduced.snapshot.unresolved_value_references[0].clone(),
    ]);
    assert!(duplicate_record.check_consistency().is_err());
    let failure =
        crate::compile_snapshot(&introduced.snapshot).expect_err("unresolved snapshot rejects");
    assert!(matches!(
        failure,
        crate::CompileSnapshotError::Incomplete(ref error)
            if error.blockers.iter().any(|blocker| matches!(
                blocker,
                CompletenessBlocker::UnresolvedValueReference { reference: blocked, .. }
                    if *blocked == reference
            ))
    ));
    assert_eq!(crate::pipeline::lowering_invocations(), 0);

    let old = Arc::clone(&introduced.snapshot);
    let resolved = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::ResolveUnresolvedValueReference {
                reference,
                target: parameter,
            }],
        })
        .expect("resolve value reference");
    assert_eq!(resolved.snapshot.state(), ProgramState::Incomplete);
    assert!(resolved
        .snapshot
        .completeness_blockers()
        .iter()
        .any(|blocker| matches!(blocker, CompletenessBlocker::MissingBody { hole, .. } if *hole == main_hole)));
    assert_eq!(
        resolved.snapshot.node(root).expect("resolved node").kind,
        NodeKind::Load
    );
    let resolved_semantics = resolved
        .snapshot
        .node_semantics(resolved.snapshot.revision(), root)
        .expect("resolved semantics");
    assert!(resolved_semantics.effects.is_known());
    assert!(resolved_semantics.effects.is_pure());
    assert!(resolved
        .snapshot
        .unresolved_value_references()
        .next()
        .is_none());
    assert!(resolved
        .snapshot
        .references()
        .iter()
        .any(|edge| edge.site == root && edge.target == parameter));
    assert!(!resolved
        .snapshot
        .diagnostics()
        .iter()
        .any(|diagnostic| { diagnostic.code.as_ref() == "workspace.unresolved-value-reference" }));
    assert!(resolved.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::UnresolvedValueReferenceResolved {
            reference: item,
            target,
        } if *item == reference && *target == parameter
    )));
    assert!(resolved.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::ReferenceRewired {
            site,
            old_target: None,
            new_target: Some(target),
        } if *site == root && *target == parameter
    )));
    assert_eq!(
        old.project(&[
            ProjectionSlice::Body(function),
            ProjectionSlice::Type(root),
            ProjectionSlice::UnresolvedValueReference(reference),
        ])
        .expect("old unresolved projection"),
        unresolved_projection
    );
    assert_eq!(
        old.unresolved_value_reference(old.revision(), reference)
            .expect("old unresolved state")
            .requested_name
            .as_ref(),
        "value"
    );

    let completed = workspace
        .apply(Transaction {
            base_revision: resolved.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::Call {
                            callee: function,
                            type_arguments: Vec::new(),
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("fill main after resolution");
    assert_eq!(completed.snapshot.state(), ProgramState::Complete);
    assert_eq!(run_i64(&completed.snapshot), 42);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn hole_refinement_keeps_unresolved_value_reference_revision_consistent() {
    let (mut workspace, _, _, _, function_hole, main_hole) = create_source_free_declarations(207);
    let introduced = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::IntroduceUnresolvedValueReference {
                target: function_hole.node(),
                requested_name: "value".to_owned(),
            }],
        })
        .expect("introduce reference before hole refinement");
    let reference = introduced
        .snapshot
        .unresolved_value_references()
        .next()
        .expect("unresolved reference")
        .id;
    let refined = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::RefineHole {
                hole: main_hole,
                expected_type: Some(SemanticType::I64),
                goal: "finish main".to_owned(),
            }],
        })
        .expect("refine unrelated hole metadata");
    assert!(Arc::ptr_eq(
        &introduced.snapshot.program,
        &refined.snapshot.program
    ));
    assert_eq!(
        refined
            .snapshot
            .unresolved_value_reference(refined.snapshot.revision(), reference)
            .expect("revision-updated unresolved state")
            .revision,
        refined.snapshot.revision()
    );
    refined
        .snapshot
        .check_consistency()
        .expect("consistent mixed incomplete metadata");
}

#[test]
fn unresolved_value_reference_candidates_are_typed_ordered_paginated_and_revision_bound() {
    let mut workspace = Workspace::empty_deterministic(202).expect("candidate workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "earlier".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "discarded".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateFunction {
                    name: "read".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![
                        ParameterDraft {
                            name: "requested".to_owned(),
                            ty: DeclarationType::I64,
                        },
                        ParameterDraft {
                            name: "zeta".to_owned(),
                            ty: DeclarationType::I64,
                        },
                        ParameterDraft {
                            name: "alpha".to_owned(),
                            ty: DeclarationType::I64,
                        },
                        ParameterDraft {
                            name: "beta".to_owned(),
                            ty: DeclarationType::I64,
                        },
                        ParameterDraft {
                            name: "flag".to_owned(),
                            ty: DeclarationType::Bool,
                        },
                        ParameterDraft {
                            name: "owned".to_owned(),
                            ty: DeclarationType::ByteVector,
                        },
                    ],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateFunction {
                    name: "other".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "other-value".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create candidate declarations");
    let earlier = entity_named(&created.snapshot, EntityKind::Function, "earlier");
    let read = entity_named(&created.snapshot, EntityKind::Function, "read");
    let other = entity_named(&created.snapshot, EntityKind::Function, "other");
    let requested = entity_named(&created.snapshot, EntityKind::Parameter, "requested");
    let other_parameter = entity_named(&created.snapshot, EntityKind::Parameter, "other-value");
    let flag = entity_named(&created.snapshot, EntityKind::Parameter, "flag");
    let owned = entity_named(&created.snapshot, EntityKind::Parameter, "owned");
    let read_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == read)
        .expect("read hole")
        .id;
    let other_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == other)
        .expect("other hole")
        .id;
    let introduced = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![
                Edit::IntroduceUnresolvedValueReference {
                    target: read_hole.node(),
                    requested_name: "requested".to_owned(),
                },
                Edit::IntroduceUnresolvedValueReference {
                    target: other_hole.node(),
                    requested_name: "other-value".to_owned(),
                },
            ],
        })
        .expect("introduce candidate references");
    let read_reference = introduced
        .snapshot
        .unresolved_value_references()
        .find(|state| state.owner == read)
        .expect("read unresolved reference")
        .id;
    let other_reference = introduced
        .snapshot
        .unresolved_value_references()
        .find(|state| state.owner == other)
        .expect("other unresolved reference")
        .id;
    let mut duplicate_record = (*introduced.snapshot).clone();
    let mut duplicate_records = duplicate_record
        .unresolved_value_references
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    duplicate_records[1] = duplicate_records[0].clone();
    duplicate_record.unresolved_value_references = duplicate_records.into();
    assert!(duplicate_record.check_consistency().is_err());
    let mut invalid_visibility = (*introduced.snapshot).clone();
    let mut visibility_records = invalid_visibility
        .unresolved_value_references
        .iter()
        .cloned()
        .collect::<Vec<_>>();
    let read_record = visibility_records
        .iter_mut()
        .find(|record| record.state.owner == read)
        .expect("read visibility record");
    let mut visible = read_record.state.visible_entities.to_vec();
    visible.push(other_parameter);
    visible.sort_unstable();
    visible.dedup();
    read_record.state.visible_entities = visible.into();
    invalid_visibility.unresolved_value_references = visibility_records.into();
    assert!(invalid_visibility.check_consistency().is_err());
    let request = PageRequest::new(2).expect("candidate page");
    let first = introduced
        .snapshot
        .unresolved_value_reference_candidates(
            introduced.snapshot.revision(),
            read_reference,
            request,
            None,
        )
        .expect("first candidate page");
    assert_eq!(first.items[0].entity, requested);
    assert!(first.items[0].exact_name_match);
    assert_eq!(
        introduced
            .snapshot
            .node(read_reference.node())
            .expect("unresolved choice node")
            .kind,
        NodeKind::UnresolvedValueReference
    );
    assert!(!introduced
        .snapshot
        .references()
        .iter()
        .any(|edge| edge.site == read_reference.node()));
    assert!(matches!(
        crate::compile_snapshot(&introduced.snapshot),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));
    let derived_multiple = introduced
        .snapshot
        .unresolved_value_reference_candidates(
            introduced.snapshot.revision(),
            read_reference,
            PageRequest::new(1).expect("single-candidate page"),
            None,
        )
        .expect("derive multiple-candidate choice");
    assert_eq!(derived_multiple.items.len(), 1);
    assert!(derived_multiple.continuation.is_some());
    for candidate in &first.items {
        let mut branch =
            Workspace::new((*introduced.snapshot).clone()).expect("candidate choice branch");
        let resolved = branch
            .apply(Transaction {
                base_revision: introduced.snapshot.revision(),
                edits: vec![Edit::ResolveUnresolvedValueReference {
                    reference: read_reference,
                    target: candidate.entity,
                }],
            })
            .expect("resolve one plausible candidate explicitly");
        assert_eq!(
            resolved
                .snapshot
                .node(read_reference.node())
                .expect("explicitly resolved choice")
                .kind,
            NodeKind::Load
        );
        assert!(resolved
            .snapshot
            .references()
            .iter()
            .any(|edge| { edge.site == read_reference.node() && edge.target == candidate.entity }));
    }
    let first_cursor = first.continuation.clone().expect("candidate continuation");
    assert!(matches!(
        introduced.snapshot.unresolved_value_reference_candidates(
            introduced.snapshot.revision(),
            other_reference,
            request,
            Some(&first_cursor),
        ),
        Err(WorkspaceError::InvalidContinuation(_))
    ));
    let mut names = first
        .items
        .iter()
        .map(|candidate| candidate.name.to_string())
        .collect::<Vec<_>>();
    let mut continuation = first.continuation;
    while let Some(cursor) = continuation {
        let page = introduced
            .snapshot
            .unresolved_value_reference_candidates(
                introduced.snapshot.revision(),
                read_reference,
                request,
                Some(&cursor),
            )
            .expect("next candidate page");
        names.extend(
            page.items
                .iter()
                .map(|candidate| candidate.name.to_string()),
        );
        continuation = page.continuation;
    }
    assert_eq!(names, ["requested", "alpha", "beta", "zeta"]);
    assert_eq!(names.iter().collect::<HashSet<_>>().len(), names.len());
    let initial_candidates = introduced
        .snapshot
        .unresolved_value_reference_candidates(
            introduced.snapshot.revision(),
            read_reference,
            PageRequest::new(16).expect("candidate page"),
            None,
        )
        .expect("all candidates")
        .items;
    assert!(!initial_candidates.iter().any(|candidate| {
        candidate.entity == flag
            || candidate.entity == owned
            || candidate.kind == EntityKind::Function
    }));
    let private_address = |snapshot: &WorkspaceSnapshot, entity: EntityId| {
        let index = snapshot
            .indexes
            .entity_lookup
            .get(&entity)
            .copied()
            .expect("candidate private address lookup");
        snapshot.indexes.entity_addresses[index]
    };
    let requested_before_compaction = private_address(&introduced.snapshot, requested);

    let old = Arc::clone(&introduced.snapshot);
    let compacted = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::DeleteEntity { entity: earlier }],
        })
        .expect("delete earlier declaration and compact private bindings");
    assert_ne!(
        private_address(&compacted.snapshot, requested),
        requested_before_compaction
    );
    let compacted_candidates = compacted
        .snapshot
        .unresolved_value_reference_candidates(
            compacted.snapshot.revision(),
            read_reference,
            PageRequest::new(16).expect("candidate page"),
            None,
        )
        .expect("candidates after private compaction")
        .items;
    assert_eq!(
        compacted_candidates
            .iter()
            .map(|candidate| candidate.entity)
            .collect::<Vec<_>>(),
        initial_candidates
            .iter()
            .map(|candidate| candidate.entity)
            .collect::<Vec<_>>()
    );

    let expanded = workspace
        .apply(Transaction {
            base_revision: compacted.snapshot.revision(),
            edits: vec![Edit::CreateFunction {
                name: "later".to_owned(),
                type_parameters: Vec::new(),
                parameters: vec![ParameterDraft {
                    name: "requested".to_owned(),
                    ty: DeclarationType::I64,
                }],
                return_type: DeclarationType::I64,
            }],
        })
        .expect("create out-of-scope matching declaration");
    assert_eq!(
        expanded
            .snapshot
            .unresolved_value_reference_candidates(
                expanded.snapshot.revision(),
                read_reference,
                PageRequest::new(16).expect("candidate page"),
                None,
            )
            .expect("scope-honest candidates after declaration creation")
            .items
            .iter()
            .map(|candidate| candidate.entity)
            .collect::<Vec<_>>(),
        initial_candidates
            .iter()
            .map(|candidate| candidate.entity)
            .collect::<Vec<_>>()
    );

    let renamed = workspace
        .apply(Transaction {
            base_revision: expanded.snapshot.revision(),
            edits: vec![Edit::RenameEntity {
                entity: requested,
                new_name: "renamed".to_owned(),
            }],
        })
        .expect("rename candidate");
    assert!(matches!(
        renamed.snapshot.unresolved_value_reference_candidates(
            renamed.snapshot.revision(),
            read_reference,
            request,
            Some(&first_cursor),
        ),
        Err(WorkspaceError::InvalidContinuation(_))
    ));
    let state = renamed
        .snapshot
        .unresolved_value_reference(renamed.snapshot.revision(), read_reference)
        .expect("renamed reference state");
    assert_eq!(state.requested_name.as_ref(), "requested");
    let renamed_candidates = renamed
        .snapshot
        .unresolved_value_reference_candidates(
            renamed.snapshot.revision(),
            read_reference,
            PageRequest::new(16).expect("candidate page"),
            None,
        )
        .expect("renamed candidates")
        .items;
    assert_eq!(
        renamed_candidates
            .iter()
            .map(|candidate| candidate.name.as_ref())
            .collect::<Vec<_>>(),
        ["alpha", "beta", "renamed", "zeta"]
    );
    assert!(renamed_candidates
        .iter()
        .all(|candidate| !candidate.exact_name_match));
    assert_eq!(
        old.unresolved_value_reference_candidates(
            old.revision(),
            read_reference,
            PageRequest::new(16).expect("candidate page"),
            None,
        )
        .expect("old candidates")
        .items[0]
            .name
            .as_ref(),
        "requested"
    );
    let resolved = workspace
        .apply(Transaction {
            base_revision: renamed.snapshot.revision(),
            edits: vec![Edit::ResolveUnresolvedValueReference {
                reference: read_reference,
                target: requested,
            }],
        })
        .expect("resolve explicitly after candidate rename");
    assert_eq!(
        resolved
            .snapshot
            .node(read_reference.node())
            .expect("renamed resolved load")
            .kind,
        NodeKind::Load
    );
}

#[test]
fn many_unresolved_value_reference_candidates_paginate_without_loss_or_duplicates() {
    const COUNT: usize = 257;
    let mut parameters = Vec::new();
    parameters
        .try_reserve(COUNT + 2)
        .expect("parameter allocation");
    for index in 0..COUNT {
        parameters.push(ParameterDraft {
            name: format!("binding{index:03}"),
            ty: DeclarationType::I64,
        });
    }
    parameters.push(ParameterDraft {
        name: "incompatible".to_owned(),
        ty: DeclarationType::Bool,
    });
    parameters.push(ParameterDraft {
        name: "affine".to_owned(),
        ty: DeclarationType::ByteVector,
    });
    let mut workspace = Workspace::empty_deterministic(206).expect("many-candidate workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateFunction {
                name: "many".to_owned(),
                type_parameters: Vec::new(),
                parameters,
                return_type: DeclarationType::I64,
            }],
        })
        .expect("create many candidates");
    let hole = created
        .snapshot
        .holes()
        .next()
        .expect("many-candidate hole")
        .id;
    let introduced = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::IntroduceUnresolvedValueReference {
                target: hole.node(),
                requested_name: "binding127".to_owned(),
            }],
        })
        .expect("introduce many-candidate reference");
    let reference = introduced
        .snapshot
        .unresolved_value_references()
        .next()
        .expect("many-candidate reference")
        .id;
    let collect = || {
        let mut continuation = None;
        let mut candidates = Vec::new();
        loop {
            let page = introduced
                .snapshot
                .unresolved_value_reference_candidates(
                    introduced.snapshot.revision(),
                    reference,
                    PageRequest::new(17).expect("candidate page"),
                    continuation.as_ref(),
                )
                .expect("candidate page");
            candidates.extend(page.items);
            continuation = page.continuation;
            if continuation.is_none() {
                break;
            }
        }
        candidates
    };
    let first = collect();
    let second = collect();
    assert_eq!(first, second);
    assert_eq!(first.len(), COUNT);
    assert_eq!(first[0].name.as_ref(), "binding127");
    assert!(first[0].exact_name_match);
    assert!(first[1..]
        .windows(2)
        .all(|pair| pair[0].name <= pair[1].name));
    assert_eq!(
        first
            .iter()
            .map(|candidate| candidate.entity)
            .collect::<HashSet<_>>()
            .len(),
        COUNT
    );
}

#[test]
fn unresolved_value_reference_failures_are_atomic_and_retry_ids_are_stable() {
    let mut workspace = Workspace::empty_deterministic(203).expect("atomic workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateProduct {
                    name: "record".to_owned(),
                    fields: vec![ProductFieldDraft {
                        name: "field".to_owned(),
                        ty: SemanticType::I64,
                    }],
                },
                Edit::CreateFunction {
                    name: "read".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![
                        ParameterDraft {
                            name: "value".to_owned(),
                            ty: DeclarationType::I64,
                        },
                        ParameterDraft {
                            name: "flag".to_owned(),
                            ty: DeclarationType::Bool,
                        },
                        ParameterDraft {
                            name: "owned".to_owned(),
                            ty: DeclarationType::ByteVector,
                        },
                    ],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateFunction {
                    name: "unrelated".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "other-value".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create atomic declarations");
    let read = entity_named(&created.snapshot, EntityKind::Function, "read");
    let unrelated = entity_named(&created.snapshot, EntityKind::Function, "unrelated");
    let value = entity_named(&created.snapshot, EntityKind::Parameter, "value");
    let flag = entity_named(&created.snapshot, EntityKind::Parameter, "flag");
    let owned = entity_named(&created.snapshot, EntityKind::Parameter, "owned");
    let invisible = entity_named(&created.snapshot, EntityKind::Parameter, "other-value");
    let field = entity_named(&created.snapshot, EntityKind::ProductField, "field");
    let read_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == read)
        .expect("read hole")
        .id;
    let main_hole = created
        .snapshot
        .holes()
        .find(|hole| {
            created
                .snapshot
                .entity(hole.owner)
                .is_ok_and(|owner| owner.kind == EntityKind::Main)
        })
        .expect("main hole")
        .id;
    let introduced = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::IntroduceUnresolvedValueReference {
                target: read_hole.node(),
                requested_name: "value".to_owned(),
            }],
        })
        .expect("introduce atomic reference");
    let reference = introduced
        .snapshot
        .unresolved_value_references()
        .next()
        .expect("unresolved reference")
        .id;
    let foreign_snapshot =
        import_source(SCALAR, "foreign-resolution.lkjscript").expect("foreign workspace");
    let foreign = foreign_snapshot.entities()[0].id;
    assert!(matches!(
        introduced.snapshot.unresolved_value_reference(
            introduced.snapshot.revision(),
            UnresolvedValueReferenceId(foreign_snapshot.nodes()[0].id),
        ),
        Err(WorkspaceError::ForeignNamespace(_))
    ));
    assert!(matches!(
        introduced.snapshot.unresolved_value_reference(
            introduced.snapshot.revision(),
            UnresolvedValueReferenceId(main_hole.node()),
        ),
        Err(WorkspaceError::WrongEntityKind { .. })
    ));
    assert!(matches!(
        introduced
            .snapshot
            .unresolved_value_reference(created.snapshot.revision(), reference),
        Err(WorkspaceError::StaleRevision)
    ));
    let before = workspace.current();
    let before_projection = before
        .project(&[ProjectionSlice::UnresolvedValueReference(reference)])
        .expect("atomic projection");
    let before_candidates = before
        .unresolved_value_reference_candidates(
            before.revision(),
            reference,
            PageRequest::new(16).expect("candidate page"),
            None,
        )
        .expect("atomic candidates");
    let failures = vec![
        vec![Edit::ResolveUnresolvedValueReference {
            reference,
            target: foreign,
        }],
        vec![Edit::ResolveUnresolvedValueReference {
            reference,
            target: read,
        }],
        vec![Edit::ResolveUnresolvedValueReference {
            reference,
            target: field,
        }],
        vec![Edit::ResolveUnresolvedValueReference {
            reference,
            target: invisible,
        }],
        vec![Edit::ResolveUnresolvedValueReference {
            reference,
            target: flag,
        }],
        vec![Edit::ResolveUnresolvedValueReference {
            reference,
            target: owned,
        }],
        vec![
            Edit::ResolveUnresolvedValueReference {
                reference,
                target: value,
            },
            Edit::ResolveUnresolvedValueReference {
                reference,
                target: value,
            },
        ],
        vec![
            Edit::ResolveUnresolvedValueReference {
                reference,
                target: value,
            },
            Edit::DeleteEntity { entity: read },
        ],
        vec![Edit::ResolveUnresolvedValueReference {
            reference: UnresolvedValueReferenceId(main_hole.node()),
            target: value,
        }],
        vec![Edit::IntroduceUnresolvedValueReference {
            target: main_hole.node(),
            requested_name: String::new(),
        }],
        vec![Edit::IntroduceUnresolvedValueReference {
            target: main_hole.node(),
            requested_name: "not valid!".to_owned(),
        }],
    ];
    for edits in failures {
        assert!(workspace
            .apply(Transaction {
                base_revision: before.revision(),
                edits,
            })
            .is_err());
        let current = workspace.current();
        assert!(Arc::ptr_eq(&before, &current));
        assert_eq!(current.diagnostics(), before.diagnostics());
        assert_eq!(
            current
                .unresolved_value_reference_candidates(
                    current.revision(),
                    reference,
                    PageRequest::new(16).expect("candidate page"),
                    None,
                )
                .expect("unchanged atomic candidates"),
            before_candidates
        );
        assert_eq!(
            current.completeness_blockers(),
            before.completeness_blockers()
        );
        assert_eq!(
            current
                .project(&[ProjectionSlice::UnresolvedValueReference(reference)])
                .expect("unchanged atomic projection"),
            before_projection
        );
    }

    let mut control = Workspace::new((*before).clone()).expect("retry control workspace");
    let create_retry = || Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::CreateFunction {
            name: "retry".to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::I64,
        }],
    };
    let after_failures = workspace
        .apply(create_retry())
        .expect("create after failures");
    let control_created = control.apply(create_retry()).expect("create control");
    assert_eq!(after_failures.diff, control_created.diff);

    let mut stale_target_workspace =
        Workspace::new((*before).clone()).expect("stale target workspace");
    let deleted = stale_target_workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::DeleteEntity { entity: unrelated }],
        })
        .expect("delete unrelated target");
    let before_stale_target = stale_target_workspace.current();
    assert!(matches!(
        stale_target_workspace.apply(Transaction {
            base_revision: deleted.snapshot.revision(),
            edits: vec![Edit::ResolveUnresolvedValueReference {
                reference,
                target: unrelated,
            }],
        }),
        Err(WorkspaceError::StaleIdentity(_))
    ));
    assert!(Arc::ptr_eq(
        &before_stale_target,
        &stale_target_workspace.current()
    ));

    let mut resolved_workspace = Workspace::new((*before).clone()).expect("resolved workspace");
    let resolved = resolved_workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::ResolveUnresolvedValueReference {
                reference,
                target: value,
            }],
        })
        .expect("resolve for stale checks");
    let resolved_before_failure = resolved_workspace.current();
    assert!(matches!(
        resolved_workspace.apply(Transaction {
            base_revision: resolved.snapshot.revision(),
            edits: vec![Edit::ResolveUnresolvedValueReference {
                reference,
                target: value,
            }],
        }),
        Err(WorkspaceError::WrongEntityKind { .. })
    ));
    assert!(Arc::ptr_eq(
        &resolved_before_failure,
        &resolved_workspace.current()
    ));
    assert!(matches!(
        resolved_workspace.apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: reference.node(),
                draft: ExpressionDraft::scalar_i64(0),
            }],
        }),
        Err(WorkspaceError::StaleRevision)
    ));
}

#[test]
fn resolved_value_reference_converges_with_direct_copy_load_authoring() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let (mut direct_workspace, direct_function, direct_parameter, _, direct_hole, direct_main_hole) =
        create_source_free_declarations(204);
    let direct = fill_source_free_identity(
        &mut direct_workspace,
        direct_function,
        direct_parameter,
        direct_hole,
        direct_main_hole,
    );

    let (
        mut resolved_workspace,
        resolved_function,
        resolved_parameter,
        _,
        resolved_hole,
        resolved_main_hole,
    ) = create_source_free_declarations(204);
    let introduced = resolved_workspace
        .apply(Transaction {
            base_revision: resolved_workspace.current().revision(),
            edits: vec![Edit::IntroduceUnresolvedValueReference {
                target: resolved_hole.node(),
                requested_name: "value".to_owned(),
            }],
        })
        .expect("introduce convergence reference");
    let reference = introduced
        .snapshot
        .unresolved_value_references()
        .next()
        .expect("convergence reference")
        .id;
    let resolved_function_body = resolved_workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::ResolveUnresolvedValueReference {
                reference,
                target: resolved_parameter,
            }],
        })
        .expect("resolve convergence reference");
    let resolved = resolved_workspace
        .apply(Transaction {
            base_revision: resolved_function_body.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: resolved_main_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::Call {
                            callee: resolved_function,
                            type_arguments: Vec::new(),
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("complete resolved convergence program")
        .snapshot;

    assert_eq!(direct.entities(), resolved.entities());
    assert_eq!(direct.nodes(), resolved.nodes());
    assert_eq!(direct.containment(), resolved.containment());
    assert_eq!(direct.references(), resolved.references());
    assert_eq!(direct.calls(), resolved.calls());
    assert_eq!(direct.dependencies(), resolved.dependencies());
    for (direct_node, resolved_node) in direct.nodes().iter().zip(resolved.nodes()) {
        assert_eq!(direct_node.id, resolved_node.id);
        let left = direct
            .node_semantics(direct.revision(), direct_node.id)
            .expect("direct node semantics");
        let right = resolved
            .node_semantics(resolved.revision(), resolved_node.id)
            .expect("resolved node semantics");
        assert_eq!(left.kind, right.kind);
        assert_eq!(left.actual, right.actual);
        assert_eq!(left.expected, right.expected);
        assert_eq!(left.operation, right.operation);
        assert_eq!(left.effects, right.effects);
    }
    let direct_executable = crate::compile_snapshot(&direct).expect("compile direct load");
    let resolved_executable = crate::compile_snapshot(&resolved).expect("compile resolved load");
    assert_eq!(
        direct_executable.memory_plan().obligations,
        resolved_executable.memory_plan().obligations
    );
    assert_eq!(
        direct_executable.bytecode().main().code,
        resolved_executable.bytecode().main().code
    );
    assert_eq!(
        direct_executable
            .bytecode()
            .protos()
            .iter()
            .map(|function| &function.code)
            .collect::<Vec<_>>(),
        resolved_executable
            .bytecode()
            .protos()
            .iter()
            .map(|function| &function.code)
            .collect::<Vec<_>>()
    );
    assert_eq!(run_i64(&direct), 42);
    assert_eq!(run_i64(&resolved), 42);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn unresolved_value_reference_follows_replacement_rename_and_deletion_lifecycle() {
    let mut workspace = Workspace::empty_deterministic(205).expect("lifecycle workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "read".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "value".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateFunction {
                    name: "unrelated".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create lifecycle declarations");
    let read = entity_named(&created.snapshot, EntityKind::Function, "read");
    let unrelated = entity_named(&created.snapshot, EntityKind::Function, "unrelated");
    let hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == read)
        .expect("read hole")
        .id;
    let introduced = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::IntroduceUnresolvedValueReference {
                target: hole.node(),
                requested_name: "value".to_owned(),
            }],
        })
        .expect("introduce lifecycle reference");
    let reference = introduced
        .snapshot
        .unresolved_value_references()
        .next()
        .expect("lifecycle reference")
        .id;
    let renamed = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::RenameEntity {
                entity: read,
                new_name: "reader".to_owned(),
            }],
        })
        .expect("rename unresolved owner");
    let renamed_state = renamed
        .snapshot
        .unresolved_value_reference(renamed.snapshot.revision(), reference)
        .expect("state after owner rename");
    assert_eq!(renamed_state.id, reference);
    assert_eq!(renamed_state.owner, read);
    assert_eq!(renamed_state.requested_name.as_ref(), "value");
    let preserved = workspace
        .apply(Transaction {
            base_revision: renamed.snapshot.revision(),
            edits: vec![Edit::DeleteEntity { entity: unrelated }],
        })
        .expect("delete unrelated function");
    assert_eq!(
        preserved
            .snapshot
            .unresolved_value_reference(preserved.snapshot.revision(), reference)
            .expect("preserved unresolved reference")
            .id,
        reference
    );
    let old = Arc::clone(&preserved.snapshot);

    let mut replacement_workspace = Workspace::new((*old).clone()).expect("replacement workspace");
    let replaced = replacement_workspace
        .apply(Transaction {
            base_revision: old.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: reference.node(),
                draft: ExpressionDraft::scalar_i64(7),
            }],
        })
        .expect("replace unresolved reference");
    assert_eq!(
        replaced
            .snapshot
            .node(reference.node())
            .expect("replacement root")
            .kind,
        NodeKind::Literal
    );
    assert!(replaced
        .snapshot
        .unresolved_value_references()
        .next()
        .is_none());
    assert!(matches!(
        replaced
            .snapshot
            .unresolved_value_reference(replaced.snapshot.revision(), reference,),
        Err(WorkspaceError::WrongEntityKind { .. })
    ));

    let mut hole_workspace = Workspace::new((*old).clone()).expect("hole workspace");
    let holed = hole_workspace
        .apply(Transaction {
            base_revision: old.revision(),
            edits: vec![Edit::IntroduceHole {
                target: reference.node(),
                goal: "replace unresolved intent".to_owned(),
            }],
        })
        .expect("replace unresolved reference with hole");
    assert!(holed
        .snapshot
        .unresolved_value_references()
        .next()
        .is_none());
    assert_eq!(
        holed
            .snapshot
            .holes()
            .find(|hole| hole.id.node() == reference.node())
            .expect("replacement hole")
            .kind,
        HoleKind::TypedExpression
    );

    let mut deletion_workspace = Workspace::new((*old).clone()).expect("deletion workspace");
    let deleted = deletion_workspace
        .apply(Transaction {
            base_revision: old.revision(),
            edits: vec![Edit::DeleteEntity { entity: read }],
        })
        .expect("delete unresolved owner");
    assert!(deleted.snapshot.entity(read).is_err());
    assert!(deleted.snapshot.node(reference.node()).is_err());
    assert!(deleted
        .snapshot
        .unresolved_value_references()
        .next()
        .is_none());
    assert_eq!(
        old.unresolved_value_reference(old.revision(), reference)
            .expect("old unresolved state")
            .requested_name
            .as_ref(),
        "value"
    );
}

#[test]
fn source_free_main_return_is_queryable_and_executes_without_source_work() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    crate::pipeline::reset_lowering_invocations();
    let mut workspace = Workspace::empty_deterministic(228).expect("empty return workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create return main");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let revision = created.snapshot.revision();
    let context = created
        .snapshot
        .hole_context(revision, hole)
        .expect("main hole context");
    assert_eq!(context.owner, main);
    assert_eq!(
        created
            .snapshot
            .function_signature(revision, context.owner)
            .expect("main signature")
            .result,
        SemanticType::I64
    );
    let constructors = created
        .snapshot
        .legal_constructors(
            revision,
            hole,
            PageRequest::new(64).expect("return constructor page"),
            None,
        )
        .expect("body-hole constructors")
        .items;
    assert!(constructors.contains(&LegalConstructor::Return));
    assert!(constructors.windows(2).all(|pair| pair[0] < pair[1]));
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: return_i64_draft(42),
            }],
        })
        .expect("fill main with return");

    assert_eq!(completed.snapshot.state(), ProgramState::Complete);
    let return_node = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Return)
        .expect("return node");
    assert_eq!(return_node.id, hole.node());
    assert_eq!(return_node.owner, SemanticOwner::Entity(main));
    let facts = completed
        .snapshot
        .node_semantics(completed.snapshot.revision(), return_node.id)
        .expect("return semantics");
    assert_eq!(facts.actual, SemanticType::Never);
    assert_eq!(facts.expected, Some(SemanticType::I64));
    assert!(facts.effects.contains(EffectSummary::MAY_DIVERGE));
    assert!(!facts.effects.contains(EffectSummary::MAY_EXIT));
    let value = completed
        .snapshot
        .containment()
        .iter()
        .find_map(|edge| match (edge.owner, edge.child) {
            (SemanticOwner::Node(owner), SemanticChild::Node(child)) if owner == return_node.id => {
                Some(child)
            }
            _ => None,
        })
        .expect("return value child");
    let value_facts = completed
        .snapshot
        .node_semantics(completed.snapshot.revision(), value)
        .expect("return value semantics");
    assert_eq!(value_facts.actual, SemanticType::I64);
    assert_eq!(value_facts.expected, Some(SemanticType::I64));
    assert_eq!(run_i64(&completed.snapshot), 42);
    assert_eq!(crate::pipeline::lowering_invocations(), 1);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
    let projection = completed
        .snapshot
        .project(&[ProjectionSlice::Body(main)])
        .expect("return projection");
    assert!(projection.contains("kind=return"), "{projection}");
    assert!(projection.contains("type=\"never\""), "{projection}");
}

#[test]
fn source_free_function_return_uses_its_own_declared_result_type() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let mut workspace = Workspace::empty_deterministic(229).expect("function return workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "truth".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::Bool,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create function and main");
    let function = entity_named(&created.snapshot, EntityKind::Function, "truth");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let function_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == function)
        .expect("function hole")
        .id;
    let main_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;
    let function_filled = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: function_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Bool(true),
                        DraftNode::Return {
                            value: DraftNodeId::new(0),
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("fill Boolean-returning function");
    let completed = workspace
        .apply(Transaction {
            base_revision: function_filled.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Call {
                            callee: function,
                            type_arguments: Vec::new(),
                            arguments: Vec::new(),
                        },
                        DraftNode::I64(42),
                        DraftNode::I64(0),
                        DraftNode::If {
                            condition: DraftNodeId::new(0),
                            then_branch: DraftNodeId::new(1),
                            else_branch: DraftNodeId::new(2),
                        },
                        DraftNode::Return {
                            value: DraftNodeId::new(3),
                        },
                    ],
                    DraftNodeId::new(4),
                ),
            }],
        })
        .expect("fill main using Boolean function");

    assert_eq!(run_i64(&completed.snapshot), 42);
    let call = completed
        .snapshot
        .calls()
        .iter()
        .find(|call| call.caller == main && call.callee == function)
        .expect("main-to-function call");
    assert_eq!(
        completed
            .snapshot
            .node_semantics(completed.snapshot.revision(), call.site)
            .expect("call semantics")
            .actual,
        SemanticType::Bool
    );
    let return_node = function_filled
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Return)
        .expect("function return");
    assert_eq!(
        function_filled
            .snapshot
            .node_semantics(function_filled.snapshot.revision(), return_node.id)
            .expect("function return semantics")
            .expected,
        Some(SemanticType::Bool)
    );
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn source_free_returns_join_conditionals_and_exit_while_bodies() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let mut branch_workspace =
        Workspace::empty_deterministic(231).expect("conditional return workspace");
    let branch_main = branch_workspace
        .apply(Transaction {
            base_revision: branch_workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create conditional return main");
    let branch_hole = branch_main
        .snapshot
        .holes()
        .next()
        .expect("conditional main hole")
        .id;
    let branch_complete = branch_workspace
        .apply(Transaction {
            base_revision: branch_main.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: branch_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Bool(true),
                        DraftNode::I64(1),
                        DraftNode::Return {
                            value: DraftNodeId::new(1),
                        },
                        DraftNode::I64(2),
                        DraftNode::If {
                            condition: DraftNodeId::new(0),
                            then_branch: DraftNodeId::new(2),
                            else_branch: DraftNodeId::new(3),
                        },
                    ],
                    DraftNodeId::new(4),
                ),
            }],
        })
        .expect("fill conditional return main");
    assert_eq!(run_i64(&branch_complete.snapshot), 1);
    let conditional = branch_complete
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Conditional)
        .expect("conditional node")
        .id;
    let conditional_facts = branch_complete
        .snapshot
        .node_semantics(branch_complete.snapshot.revision(), conditional)
        .expect("conditional facts");
    assert_eq!(conditional_facts.actual, SemanticType::I64);
    assert!(conditional_facts
        .effects
        .contains(EffectSummary::MAY_DIVERGE));
    let else_branch = branch_complete
        .snapshot
        .containment()
        .iter()
        .filter_map(|edge| match (edge.owner, edge.child) {
            (SemanticOwner::Node(owner), SemanticChild::Node(child)) if owner == conditional => {
                Some(child)
            }
            _ => None,
        })
        .nth(2)
        .expect("conditional else branch");
    let all_return = branch_workspace
        .apply(Transaction {
            base_revision: branch_complete.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: else_branch,
                draft: return_i64_draft(2),
            }],
        })
        .expect("replace the other branch with return");
    assert_eq!(
        all_return
            .snapshot
            .node_semantics(all_return.snapshot.revision(), conditional)
            .expect("all-return conditional facts")
            .actual,
        SemanticType::Never
    );
    assert_eq!(run_i64(&all_return.snapshot), 1);
    let introduced = branch_workspace
        .apply(Transaction {
            base_revision: all_return.snapshot.revision(),
            edits: vec![Edit::IntroduceHole {
                target: else_branch,
                goal: "choose the else control result".to_owned(),
            }],
        })
        .expect("introduce valid nested return hole");
    let nested_hole = introduced
        .snapshot
        .holes()
        .find(|hole| hole.id.node() == else_branch)
        .expect("nested return hole");
    assert_eq!(nested_hole.expected_type, SemanticType::I64);
    assert!(introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            nested_hole.id,
            PageRequest::new(64).expect("nested return constructor page"),
            None,
        )
        .expect("nested return constructors")
        .items
        .contains(&LegalConstructor::Return));

    let mut while_workspace = Workspace::empty_deterministic(232).expect("while return workspace");
    let while_main = while_workspace
        .apply(Transaction {
            base_revision: while_workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create while return main");
    let while_hole = while_main
        .snapshot
        .holes()
        .next()
        .expect("while main hole")
        .id;
    let owner = DraftBindingId::new(0);
    let while_complete = while_workspace
        .apply(Transaction {
            base_revision: while_main.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: while_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(2),
                        DraftNode::Operation {
                            operation: crate::Operation::ByteVectorNew,
                            arguments: vec![DraftNodeId::new(0)],
                        },
                        DraftNode::Bool(true),
                        DraftNode::I64(7),
                        DraftNode::Return {
                            value: DraftNodeId::new(3),
                        },
                        DraftNode::While {
                            condition: DraftNodeId::new(2),
                            body: vec![DraftNodeId::new(4)],
                        },
                        DraftNode::I64(0),
                        DraftNode::Sequence(vec![DraftNodeId::new(5), DraftNodeId::new(6)]),
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: owner,
                                name: "owner".to_owned(),
                                value: DraftNodeId::new(1),
                            }],
                            body: DraftNodeId::new(7),
                        },
                    ],
                    DraftNodeId::new(8),
                ),
            }],
        })
        .expect("fill while return main");
    assert_eq!(run_i64(&while_complete.snapshot), 7);
    let executable =
        crate::compile_snapshot(&while_complete.snapshot).expect("compile while return cleanup");
    assert_eq!(
        executable
            .memory_plan()
            .obligations
            .iter()
            .filter(|obligation| {
                obligation.kind == crate::memory_plan::MemoryObligationKind::DropWholeValue
            })
            .count(),
        1
    );
    let while_node = while_complete
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::While)
        .expect("while node")
        .id;
    let return_node = while_complete
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Return)
        .expect("while-body return");
    assert_eq!(return_node.owner, SemanticOwner::Node(while_node));

    let mut mutable_workspace =
        Workspace::empty_deterministic(236).expect("mutable-local return workspace");
    let mutable_main = mutable_workspace
        .apply(Transaction {
            base_revision: mutable_workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create mutable-local return main");
    let mutable_hole = mutable_main
        .snapshot
        .holes()
        .next()
        .expect("mutable-local main hole")
        .id;
    let local = DraftBindingId::new(0);
    let mutable_complete = mutable_workspace
        .apply(Transaction {
            base_revision: mutable_main.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: mutable_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(0),
                        DraftNode::I64(7),
                        DraftNode::Return {
                            value: DraftNodeId::new(1),
                        },
                        DraftNode::MutableLocal {
                            binding: local,
                            name: "counter".to_owned(),
                            ty: SemanticType::I64,
                            initial: DraftNodeId::new(0),
                            body: DraftNodeId::new(2),
                        },
                    ],
                    DraftNodeId::new(3),
                ),
            }],
        })
        .expect("fill mutable-local return main");
    assert_eq!(run_i64(&mutable_complete.snapshot), 7);
    assert!(mutable_complete
        .snapshot
        .nodes()
        .iter()
        .any(|node| node.kind == NodeKind::MutableLocal));
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn return_replacement_and_hole_lifecycle_preserve_only_surviving_identity() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let mut workspace = Workspace::empty_deterministic(234).expect("return lifecycle workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create return lifecycle main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let original = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: return_i64_draft(42),
            }],
        })
        .expect("publish original return")
        .snapshot;
    let return_node = original
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Return)
        .expect("original return")
        .id;
    let original_value = original
        .containment()
        .iter()
        .find_map(|edge| match (edge.owner, edge.child) {
            (SemanticOwner::Node(owner), SemanticChild::Node(child)) if owner == return_node => {
                Some(child)
            }
            _ => None,
        })
        .expect("original return value");

    let replaced = workspace
        .apply(Transaction {
            base_revision: original.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: original_value,
                draft: ExpressionDraft::scalar_i64(7),
            }],
        })
        .expect("replace the return value directly");
    assert_eq!(
        replaced
            .snapshot
            .node(return_node)
            .expect("surviving return")
            .kind,
        NodeKind::Return
    );
    let replacement_value = replaced
        .snapshot
        .containment()
        .iter()
        .find_map(|edge| match (edge.owner, edge.child) {
            (SemanticOwner::Node(owner), SemanticChild::Node(child)) if owner == return_node => {
                Some(child)
            }
            _ => None,
        })
        .expect("replacement return value");
    assert_eq!(replacement_value, original_value);
    assert!(replaced.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::ExpressionReplaced {
            node,
            old_kind: NodeKind::Literal,
            new_kind: NodeKind::Literal,
        } if *node == original_value
    )));
    assert_eq!(run_i64(&original), 42);
    assert_eq!(run_i64(&replaced.snapshot), 7);

    let introduced = workspace
        .apply(Transaction {
            base_revision: replaced.snapshot.revision(),
            edits: vec![Edit::IntroduceHole {
                target: return_node,
                goal: "choose the early result".to_owned(),
            }],
        })
        .expect("replace return with a hole");
    assert_eq!(introduced.snapshot.state(), ProgramState::Incomplete);
    assert_eq!(
        introduced
            .snapshot
            .node(return_node)
            .expect("hole keeps targeted root")
            .kind,
        NodeKind::Hole
    );
    assert!(introduced.snapshot.node(replacement_value).is_err());
    assert!(introduced.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::HoleIntroduced { hole } if *hole == HoleId(return_node)
    )));
    assert!(matches!(
        crate::compile_snapshot(&introduced.snapshot),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));

    let refilled = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: HoleId(return_node),
                draft: return_i64_draft(9),
            }],
        })
        .expect("refill return hole");
    assert_eq!(
        refilled
            .snapshot
            .node(return_node)
            .expect("refilled return root")
            .kind,
        NodeKind::Return
    );
    assert!(refilled.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::HoleFilled { hole } if *hole == HoleId(return_node)
    )));
    assert_eq!(run_i64(&refilled.snapshot), 9);
    assert_eq!(run_i64(&original), 42);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn return_replacement_recomputes_control_ancestor_types() {
    let mut workspace = Workspace::empty_deterministic(237).expect("return retyping workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create return retyping main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let outer = DraftBindingId::new(0);
    let mutable = DraftBindingId::new(1);
    let original = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Unit,
                        DraftNode::I64(0),
                        DraftNode::Unit,
                        DraftNode::I64(1),
                        DraftNode::Sequence(vec![DraftNodeId::new(2), DraftNodeId::new(3)]),
                        DraftNode::MutableLocal {
                            binding: mutable,
                            name: "counter".to_owned(),
                            ty: SemanticType::I64,
                            initial: DraftNodeId::new(1),
                            body: DraftNodeId::new(4),
                        },
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: outer,
                                name: "scope".to_owned(),
                                value: DraftNodeId::new(0),
                            }],
                            body: DraftNodeId::new(5),
                        },
                    ],
                    DraftNodeId::new(6),
                ),
            }],
        })
        .expect("publish nested control body")
        .snapshot;
    let sequence = original
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Sequence)
        .expect("sequence")
        .id;
    let target = original
        .containment()
        .iter()
        .filter_map(|edge| match (edge.owner, edge.child) {
            (SemanticOwner::Node(owner), SemanticChild::Node(child)) if owner == sequence => {
                Some(child)
            }
            _ => None,
        })
        .next_back()
        .expect("final sequence value");
    let mutable_node = original.node(sequence).expect("sequence node").owner;
    let SemanticOwner::Node(mutable_node) = mutable_node else {
        panic!("sequence must be inside mutable local")
    };
    let let_owner = original
        .node(mutable_node)
        .expect("mutable-local node")
        .owner;
    let SemanticOwner::Node(let_node) = let_owner else {
        panic!("mutable local must be inside let")
    };

    let replaced = workspace
        .apply(Transaction {
            base_revision: original.revision(),
            edits: vec![Edit::ReplaceExpression {
                target,
                draft: return_i64_draft(7),
            }],
        })
        .expect("replace final value with return");
    for node in [sequence, mutable_node, let_node, target] {
        assert_eq!(
            replaced
                .snapshot
                .node_semantics(replaced.snapshot.revision(), node)
                .expect("retyped control node")
                .actual,
            SemanticType::Never
        );
    }
    assert_eq!(
        replaced
            .snapshot
            .node(target)
            .expect("replacement root")
            .kind,
        NodeKind::Return
    );
    assert_eq!(run_i64(&original), 1);
    assert_eq!(run_i64(&replaced.snapshot), 7);
    assert!(replaced.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::ExpressionReplaced {
            node,
            old_kind: NodeKind::Literal,
            new_kind: NodeKind::Return,
        } if *node == target
    )));
    assert_eq!(
        replaced
            .snapshot
            .node_semantics(replaced.snapshot.revision(), target)
            .expect("divergent replacement expectation")
            .expected,
        Some(SemanticType::I64)
    );

    let restored = workspace
        .apply(Transaction {
            base_revision: replaced.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target,
                draft: ExpressionDraft::scalar_i64(9),
            }],
        })
        .expect("replace return with an ordinary reachable value");
    for node in [sequence, mutable_node, let_node, target] {
        assert_eq!(
            restored
                .snapshot
                .node_semantics(restored.snapshot.revision(), node)
                .expect("restored control node")
                .actual,
            SemanticType::I64
        );
    }
    assert_eq!(
        restored.snapshot.node(target).expect("restored root").kind,
        NodeKind::Literal
    );
    assert_eq!(run_i64(&replaced.snapshot), 7);
    assert_eq!(run_i64(&restored.snapshot), 9);
}

#[test]
fn invalid_return_drafts_are_structured_atomic_and_retry_stable() {
    fn created_main(seed: u64) -> (Workspace, Arc<WorkspaceSnapshot>, HoleId) {
        let mut workspace = Workspace::empty_deterministic(seed).expect("empty return workspace");
        let created = workspace
            .apply(Transaction {
                base_revision: workspace.current().revision(),
                edits: vec![Edit::CreateMain {
                    return_type: SemanticType::I64,
                }],
            })
            .expect("create return main");
        let hole = created.snapshot.holes().next().expect("main hole").id;
        (workspace, created.snapshot, hole)
    }

    let (mut workspace, published, hole) = created_main(233);
    let projection = published.project(&[]).expect("published projection");
    let wrong_type = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::Bool(true),
                    DraftNode::Return {
                        value: DraftNodeId::new(0),
                    },
                ],
                DraftNodeId::new(1),
            ),
        }],
    });
    assert!(matches!(
        wrong_type,
        Err(WorkspaceError::TypeMismatch { expected, actual })
            if *expected == SemanticType::I64 && *actual == SemanticType::Bool
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let invalid = [
        ExpressionDraft::new(
            vec![
                DraftNode::I64(7),
                DraftNode::Return {
                    value: DraftNodeId::new(0),
                },
                DraftNode::Return {
                    value: DraftNodeId::new(1),
                },
            ],
            DraftNodeId::new(2),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(7),
                DraftNode::Return {
                    value: DraftNodeId::new(0),
                },
                DraftNode::I64(8),
                DraftNode::Sequence(vec![DraftNodeId::new(1), DraftNodeId::new(2)]),
            ],
            DraftNodeId::new(3),
        ),
        ExpressionDraft::new(
            vec![DraftNode::Return {
                value: DraftNodeId::new(99),
            }],
            DraftNodeId::new(0),
        ),
        ExpressionDraft::new(
            vec![DraftNode::Return {
                value: DraftNodeId::new(0),
            }],
            DraftNodeId::new(0),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(7),
                DraftNode::Return {
                    value: DraftNodeId::new(0),
                },
                DraftNode::Sequence(vec![DraftNodeId::new(1), DraftNodeId::new(1)]),
            ],
            DraftNodeId::new(2),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(7),
                DraftNode::I64(8),
                DraftNode::Return {
                    value: DraftNodeId::new(0),
                },
            ],
            DraftNodeId::new(2),
        ),
    ];
    for draft in invalid {
        assert!(matches!(
            workspace.apply(Transaction {
                base_revision: published.revision(),
                edits: vec![Edit::FillHole { hole, draft }],
            }),
            Err(WorkspaceError::InvalidDraft(_))
        ));
        assert!(Arc::ptr_eq(&published, &workspace.current()));
        assert_eq!(workspace.current().revision(), published.revision());
        assert_eq!(
            workspace
                .current()
                .project(&[])
                .expect("current projection"),
            projection
        );
    }

    let valid = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: return_i64_draft(7),
            }],
        })
        .expect("valid retry");
    let (mut control, control_published, control_hole) = created_main(233);
    let control_valid = control
        .apply(Transaction {
            base_revision: control_published.revision(),
            edits: vec![Edit::FillHole {
                hole: control_hole,
                draft: return_i64_draft(7),
            }],
        })
        .expect("control retry");
    assert_eq!(valid.snapshot.entities(), control_valid.snapshot.entities());
    assert_eq!(valid.snapshot.nodes(), control_valid.snapshot.nodes());
    assert_eq!(valid.diff, control_valid.diff);
    assert_eq!(run_i64(&valid.snapshot), 7);

    let (mut replacement_workspace, replacement_created, replacement_hole) = created_main(235);
    let sequence = replacement_workspace
        .apply(Transaction {
            base_revision: replacement_created.revision(),
            edits: vec![Edit::FillHole {
                hole: replacement_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Unit,
                        DraftNode::I64(1),
                        DraftNode::Sequence(vec![DraftNodeId::new(0), DraftNodeId::new(1)]),
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("publish replaceable sequence")
        .snapshot;
    let first = sequence
        .nodes()
        .iter()
        .find(|node| {
            node.kind == NodeKind::Literal
                && sequence
                    .node_semantics(sequence.revision(), node.id)
                    .is_ok_and(|facts| facts.actual == SemanticType::Unit)
        })
        .expect("first sequence expression")
        .id;
    let replacement = replacement_workspace.apply(Transaction {
        base_revision: sequence.revision(),
        edits: vec![Edit::ReplaceExpression {
            target: first,
            draft: return_i64_draft(7),
        }],
    });
    assert!(matches!(replacement, Err(WorkspaceError::Validation(_))));
    assert!(Arc::ptr_eq(&sequence, &replacement_workspace.current()));
}

#[test]
fn unreachable_return_replacement_rejects_while_an_unrelated_hole_remains() {
    let mut workspace = Workspace::empty_deterministic(241).expect("incomplete return workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "unfinished".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::Unit,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create incomplete function and main");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let main_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;
    let direct = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(7),
                        DraftNode::Return {
                            value: DraftNodeId::new(0),
                        },
                        DraftNode::Unit,
                        DraftNode::Sequence(vec![DraftNodeId::new(1), DraftNodeId::new(2)]),
                    ],
                    DraftNodeId::new(3),
                ),
            }],
        })
        .expect_err("unreachable direct fill must fail");
    assert!(
        matches!(&direct, WorkspaceError::InvalidDraft(message) if message.contains("after a divergent expression")),
        "{direct:?}"
    );
    assert_eq!(workspace.current().revision(), created.snapshot.revision());

    let published = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Unit,
                        DraftNode::I64(1),
                        DraftNode::Sequence(vec![DraftNodeId::new(0), DraftNodeId::new(1)]),
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("publish main while function remains incomplete")
        .snapshot;
    assert_eq!(published.state(), ProgramState::Incomplete);
    let sequence = published
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Sequence)
        .expect("main sequence")
        .id;
    let first = published
        .containment()
        .iter()
        .find_map(|edge| match (edge.owner, edge.child) {
            (SemanticOwner::Node(owner), SemanticChild::Node(child)) if owner == sequence => {
                Some(child)
            }
            _ => None,
        })
        .expect("first sequence child");
    let projection = published.project(&[]).expect("incomplete projection");
    let mut query_workspace =
        Workspace::new((*published).clone()).expect("incomplete query workspace");
    let introduced = query_workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::IntroduceHole {
                target: first,
                goal: "choose the first sequence expression".to_owned(),
            }],
        })
        .expect("introduce non-tail sequence hole");
    let first_hole = introduced
        .snapshot
        .holes()
        .find(|hole| hole.id.node() == first)
        .expect("non-tail sequence hole")
        .id;
    assert!(!introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            first_hole,
            PageRequest::new(64).expect("non-tail constructor page"),
            None,
        )
        .expect("non-tail constructors")
        .items
        .contains(&LegalConstructor::Return));

    let invalid = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::ReplaceExpression {
            target: first,
            draft: return_i64_draft(7),
        }],
    });
    assert!(matches!(invalid, Err(WorkspaceError::Validation(_))));
    assert!(Arc::ptr_eq(&published, &workspace.current()));
    assert_eq!(workspace.current().revision(), published.revision());
    assert_eq!(
        workspace
            .current()
            .project(&[])
            .expect("current projection"),
        projection
    );
}

#[test]
fn return_call_references_reject_foreign_stale_and_deleted_entities_atomically() {
    fn create_workspace(seed: u64) -> (Workspace, Arc<WorkspaceSnapshot>, EntityId, HoleId) {
        let mut workspace =
            Workspace::empty_deterministic(seed).expect("return reference workspace");
        let created = workspace
            .apply(Transaction {
                base_revision: workspace.current().revision(),
                edits: vec![
                    Edit::CreateFunction {
                        name: "value".to_owned(),
                        type_parameters: Vec::new(),
                        parameters: Vec::new(),
                        return_type: DeclarationType::I64,
                    },
                    Edit::CreateMain {
                        return_type: SemanticType::I64,
                    },
                ],
            })
            .expect("create callable and main");
        let function = entity_named(&created.snapshot, EntityKind::Function, "value");
        let main = entity_named(&created.snapshot, EntityKind::Main, "main");
        let main_hole = created
            .snapshot
            .holes()
            .find(|hole| hole.owner == main)
            .expect("main hole")
            .id;
        (workspace, created.snapshot, function, main_hole)
    }

    fn return_call_draft(callee: EntityId) -> ExpressionDraft {
        ExpressionDraft::new(
            vec![
                DraftNode::Call {
                    callee,
                    type_arguments: Vec::new(),
                    arguments: Vec::new(),
                },
                DraftNode::Return {
                    value: DraftNodeId::new(0),
                },
            ],
            DraftNodeId::new(1),
        )
    }

    let (mut workspace, created, function, main_hole) = create_workspace(239);
    let deleted_in_batch = workspace.apply(Transaction {
        base_revision: created.revision(),
        edits: vec![
            Edit::DeleteEntity { entity: function },
            Edit::FillHole {
                hole: main_hole,
                draft: return_call_draft(function),
            },
        ],
    });
    assert!(matches!(
        deleted_in_batch,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&created, &workspace.current()));

    let deleted = workspace
        .apply(Transaction {
            base_revision: created.revision(),
            edits: vec![Edit::DeleteEntity { entity: function }],
        })
        .expect("delete callable")
        .snapshot;
    let stale = workspace.apply(Transaction {
        base_revision: deleted.revision(),
        edits: vec![Edit::FillHole {
            hole: main_hole,
            draft: return_call_draft(function),
        }],
    });
    assert!(matches!(stale, Err(WorkspaceError::StaleIdentity(_))));
    assert!(Arc::ptr_eq(&deleted, &workspace.current()));

    let (_, _, foreign_function, _) = create_workspace(240);
    let foreign = workspace.apply(Transaction {
        base_revision: deleted.revision(),
        edits: vec![Edit::FillHole {
            hole: main_hole,
            draft: return_call_draft(foreign_function),
        }],
    });
    assert!(matches!(foreign, Err(WorkspaceError::ForeignNamespace(_))));
    assert!(Arc::ptr_eq(&deleted, &workspace.current()));

    let retried = workspace
        .apply(Transaction {
            base_revision: deleted.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: return_i64_draft(7),
            }],
        })
        .expect("retry after invalid return references");
    let (mut control, control_created, control_function, control_main_hole) = create_workspace(239);
    let control_deleted = control
        .apply(Transaction {
            base_revision: control_created.revision(),
            edits: vec![Edit::DeleteEntity {
                entity: control_function,
            }],
        })
        .expect("control delete");
    let control_completed = control
        .apply(Transaction {
            base_revision: control_deleted.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: control_main_hole,
                draft: return_i64_draft(7),
            }],
        })
        .expect("control completion");
    assert_eq!(
        retried.snapshot.entities(),
        control_completed.snapshot.entities()
    );
    assert_eq!(retried.snapshot.nodes(), control_completed.snapshot.nodes());
    assert_eq!(retried.diff, control_completed.diff);
    assert_eq!(run_i64(&retried.snapshot), 7);
}

#[test]
fn source_free_imperative_counted_loop_executes_and_matches_imported_semantics() {
    const SOURCE: &str =
        include_str!("../../../lkjscript-app/tests/fixtures/imperative-counted-loop.lkjscript");
    let namespace = WorkspaceNamespace::deterministic(220);
    let imported = importer::import_source_with_namespace(
        SOURCE,
        "workspace-imperative-equivalence.lkjscript",
        namespace,
    )
    .expect("import imperative control fixture");

    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let mut workspace = Workspace::empty_deterministic(220).expect("empty imperative workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create imperative main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let complete = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: counted_loop_draft(100),
            }],
        })
        .expect("fill counted loop");

    assert_eq!(complete.snapshot.state(), ProgramState::Complete);
    assert_eq!(run_i64(&complete.snapshot), 100);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
    assert_eq!(run_i64(&imported), 100);
    assert_eq!(
        canonical_workspace_observation(&imported),
        canonical_workspace_observation(&complete.snapshot)
    );

    let counter = entity_named(&complete.snapshot, EntityKind::MutableLocal, "counter");
    let references = complete
        .snapshot
        .references_to(
            complete.snapshot.revision(),
            counter,
            PageRequest::new(16).expect("reference page"),
            None,
        )
        .expect("counter references");
    assert_eq!(references.items.len(), 4);
    for kind in [
        NodeKind::MutableLocal,
        NodeKind::Sequence,
        NodeKind::While,
        NodeKind::SetLocal,
        NodeKind::Load,
        NodeKind::Operation,
    ] {
        assert!(complete
            .snapshot
            .nodes()
            .iter()
            .any(|node| node.kind == kind));
    }
    let main = entity_named(&complete.snapshot, EntityKind::Main, "main");
    let projection = complete
        .snapshot
        .project(&[
            ProjectionSlice::Entity(counter),
            ProjectionSlice::Body(main),
        ])
        .expect("imperative projection");
    for fact in ["mutable-local", "set-local", "while"] {
        assert!(projection.contains(fact), "missing {fact}: {projection}");
    }
    assert_eq!(
        complete
            .snapshot
            .entity_type(complete.snapshot.revision(), counter)
            .expect("counter type")
            .declared,
        Some(SemanticType::I64)
    );
    let less = complete
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Operation)
        .expect("less operation")
        .id;
    let less_facts = complete
        .snapshot
        .node_semantics(complete.snapshot.revision(), less)
        .expect("less operation facts");
    assert_eq!(less_facts.operation, Some(crate::Operation::Less));
    assert_eq!(less_facts.actual, SemanticType::Bool);
    assert!(less_facts.effects.is_pure());
    let while_node = complete
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::While)
        .expect("while node");
    let while_facts = complete
        .snapshot
        .node_semantics(complete.snapshot.revision(), while_node.id)
        .expect("while facts");
    assert!(while_facts.effects.contains(EffectSummary::MAY_DIVERGE));
    assert!(while_facts.effects.contains(EffectSummary::MUTATES_LOCAL));
    assert!(projection.contains("type=\"i64\""), "{projection}");
    assert!(projection.contains("operation=less-than"), "{projection}");
    assert!(projection.contains("mutates-local"), "{projection}");

    let sequence = complete
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Sequence)
        .expect("sequence node")
        .id;
    let children: Vec<_> = complete
        .snapshot
        .containment()
        .iter()
        .filter_map(|edge| match (edge.owner, edge.child) {
            (SemanticOwner::Node(owner), SemanticChild::Node(child)) if owner == sequence => {
                Some(complete.snapshot.node(child).expect("sequence child").kind)
            }
            _ => None,
        })
        .collect();
    assert_eq!(children, [NodeKind::While, NodeKind::Load]);

    let introduced = workspace
        .apply(Transaction {
            base_revision: complete.snapshot.revision(),
            edits: vec![Edit::IntroduceHole {
                target: less,
                goal: "choose the loop comparison".to_owned(),
            }],
        })
        .expect("introduce comparison hole");
    let comparison_hole = introduced.snapshot.holes().next().expect("comparison hole");
    let constructors = introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            comparison_hole.id,
            PageRequest::new(64).expect("comparison constructor page"),
            None,
        )
        .expect("comparison constructors")
        .items;
    assert!(constructors.contains(&LegalConstructor::Operation(crate::Operation::Less)));
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn empty_sequence_is_unit_pure_and_executable() {
    let mut workspace = Workspace::empty_deterministic(227).expect("empty sequence workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::Unit,
            }],
        })
        .expect("create unit main");
    let hole = created.snapshot.holes().next().expect("unit hole").id;
    let complete = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![DraftNode::Sequence(Vec::new())],
                    DraftNodeId::new(0),
                ),
            }],
        })
        .expect("fill empty sequence");
    let sequence = complete.snapshot.nodes()[0].id;
    let facts = complete
        .snapshot
        .node_semantics(complete.snapshot.revision(), sequence)
        .expect("empty sequence facts");
    assert_eq!(facts.kind, NodeKind::Sequence);
    assert_eq!(facts.actual, SemanticType::Unit);
    assert!(facts.effects.is_pure());
    assert!(complete
        .snapshot
        .containment()
        .iter()
        .all(|edge| { edge.owner != SemanticOwner::Node(sequence) }));
    let executable = crate::compile_snapshot(&complete.snapshot).expect("compile empty sequence");
    assert!(matches!(
        run_chunk(
            executable.bytecode(),
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        ),
        ExecutionOutcome::Returned(_)
    ));
}

#[test]
fn imperative_draft_scope_kind_type_and_operation_failures_are_atomic() {
    let mut workspace = Workspace::empty_deterministic(221).expect("failure workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "f".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create failure subjects");
    let published = created.snapshot;
    let hole = published
        .holes()
        .find(|hole| {
            published
                .entity(hole.owner)
                .is_ok_and(|entity| entity.kind == EntityKind::Main)
        })
        .expect("main hole")
        .id;
    let function = entity_named(&published, EntityKind::Function, "f");
    let local = DraftBindingId::new(0);
    let failures = vec![
        ExpressionDraft::new(
            vec![
                DraftNode::Load(DraftBindingRef::Local(local)),
                DraftNode::I64(0),
                DraftNode::MutableLocal {
                    binding: local,
                    name: "self-reference".to_owned(),
                    ty: SemanticType::I64,
                    initial: DraftNodeId::new(0),
                    body: DraftNodeId::new(1),
                },
            ],
            DraftNodeId::new(2),
        ),
        ExpressionDraft::new(
            vec![DraftNode::Load(DraftBindingRef::Local(local))],
            DraftNodeId::new(0),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(1),
                DraftNode::SetLocal {
                    target: DraftBindingRef::Local(local),
                    value: DraftNodeId::new(0),
                },
            ],
            DraftNodeId::new(1),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(0),
                DraftNode::I64(1),
                DraftNode::SetLocal {
                    target: DraftBindingRef::Local(local),
                    value: DraftNodeId::new(1),
                },
                DraftNode::Let {
                    bindings: vec![LocalDraft {
                        binding: local,
                        name: "immutable".to_owned(),
                        value: DraftNodeId::new(0),
                    }],
                    body: DraftNodeId::new(2),
                },
            ],
            DraftNodeId::new(3),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(1),
                DraftNode::SetLocal {
                    target: DraftBindingRef::Entity(function),
                    value: DraftNodeId::new(0),
                },
            ],
            DraftNodeId::new(1),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(0),
                DraftNode::Bool(true),
                DraftNode::SetLocal {
                    target: DraftBindingRef::Local(local),
                    value: DraftNodeId::new(1),
                },
                DraftNode::MutableLocal {
                    binding: local,
                    name: "wrong-type".to_owned(),
                    ty: SemanticType::I64,
                    initial: DraftNodeId::new(0),
                    body: DraftNodeId::new(2),
                },
            ],
            DraftNodeId::new(3),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(0),
                DraftNode::While {
                    condition: DraftNodeId::new(0),
                    body: Vec::new(),
                },
            ],
            DraftNodeId::new(1),
        ),
        ExpressionDraft::new(
            vec![DraftNode::Operation {
                operation: crate::Operation::Less,
                arguments: Vec::new(),
            }],
            DraftNodeId::new(0),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(0),
                DraftNode::Bool(false),
                DraftNode::Operation {
                    operation: crate::Operation::Less,
                    arguments: vec![DraftNodeId::new(0), DraftNodeId::new(1)],
                },
            ],
            DraftNodeId::new(2),
        ),
    ];
    for draft in failures {
        assert!(workspace
            .apply(Transaction {
                base_revision: published.revision(),
                edits: vec![Edit::FillHole { hole, draft }],
            })
            .is_err());
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }

    let wrong_kind = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(1),
                        DraftNode::SetLocal {
                            target: DraftBindingRef::Entity(function),
                            value: DraftNodeId::new(0),
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect_err("function cannot be an assignment target");
    assert!(matches!(
        wrong_kind,
        WorkspaceError::WrongEntityKind {
            operation,
            expected,
            actual: SemanticKind::Entity(EntityKind::Function),
        } if operation.as_ref() == "set-local" && expected.as_ref() == "mutable local"
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let wrong_type = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(0),
                        DraftNode::Bool(true),
                        DraftNode::SetLocal {
                            target: DraftBindingRef::Local(local),
                            value: DraftNodeId::new(1),
                        },
                        DraftNode::MutableLocal {
                            binding: local,
                            name: "wrong-type".to_owned(),
                            ty: SemanticType::I64,
                            initial: DraftNodeId::new(0),
                            body: DraftNodeId::new(2),
                        },
                    ],
                    DraftNodeId::new(3),
                ),
            }],
        })
        .expect_err("assignment type mismatch");
    assert!(matches!(
        wrong_type,
        WorkspaceError::TypeMismatch { expected, actual }
            if *expected == SemanticType::I64 && *actual == SemanticType::Bool
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let wrong_initializer = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Bool(true),
                        DraftNode::I64(0),
                        DraftNode::MutableLocal {
                            binding: local,
                            name: "wrong-initializer".to_owned(),
                            ty: SemanticType::I64,
                            initial: DraftNodeId::new(0),
                            body: DraftNodeId::new(1),
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect_err("mutable initializer type mismatch");
    assert!(matches!(
        wrong_initializer,
        WorkspaceError::TypeMismatch { expected, actual }
            if *expected == SemanticType::I64 && *actual == SemanticType::Bool
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let owner = DraftBindingId::new(10);
    let borrowed = DraftBindingId::new(11);
    let restricted_storage = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(1),
                        DraftNode::Operation {
                            operation: crate::Operation::ByteVectorNew,
                            arguments: vec![DraftNodeId::new(0)],
                        },
                        DraftNode::BorrowShared(DraftBindingRef::Local(owner)),
                        DraftNode::I64(0),
                        DraftNode::MutableLocal {
                            binding: borrowed,
                            name: "borrowed-storage".to_owned(),
                            ty: SemanticType::ByteSlice,
                            initial: DraftNodeId::new(2),
                            body: DraftNodeId::new(3),
                        },
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: owner,
                                name: "owner".to_owned(),
                                value: DraftNodeId::new(1),
                            }],
                            body: DraftNodeId::new(4),
                        },
                    ],
                    DraftNodeId::new(5),
                ),
            }],
        })
        .expect_err("lexical reference cannot occupy mutable storage");
    assert!(matches!(
        restricted_storage,
        WorkspaceError::InvalidDraft(message)
            if message.as_ref() == "lexical references cannot occupy mutable local storage"
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let non_boolean = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(0),
                        DraftNode::While {
                            condition: DraftNodeId::new(0),
                            body: Vec::new(),
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect_err("while condition must be Boolean");
    assert!(matches!(
        non_boolean,
        WorkspaceError::TypeMismatch { expected, actual }
            if *expected == SemanticType::Bool && *actual == SemanticType::I64
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let duplicate = DraftBindingId::new(7);
    let duplicate_failure = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::I64(0),
                    DraftNode::I64(1),
                    DraftNode::Load(DraftBindingRef::Local(duplicate)),
                    DraftNode::Let {
                        bindings: vec![LocalDraft {
                            binding: duplicate,
                            name: "same".to_owned(),
                            value: DraftNodeId::new(1),
                        }],
                        body: DraftNodeId::new(2),
                    },
                    DraftNode::MutableLocal {
                        binding: duplicate,
                        name: "same".to_owned(),
                        ty: SemanticType::I64,
                        initial: DraftNodeId::new(0),
                        body: DraftNodeId::new(3),
                    },
                ],
                DraftNodeId::new(4),
            ),
        }],
    });
    assert!(duplicate_failure.is_err());
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let mut control = Workspace::new((*published).clone()).expect("retry control");
    let fill = |workspace: &mut Workspace| {
        workspace
            .apply(Transaction {
                base_revision: published.revision(),
                edits: vec![Edit::FillHole {
                    hole,
                    draft: counted_loop_draft(3),
                }],
            })
            .expect("fill valid mutable loop after failures")
    };
    let retried = fill(&mut workspace);
    let clean = fill(&mut control);
    assert_eq!(retried.diff, clean.diff);
    assert_eq!(retried.snapshot.entities(), clean.snapshot.entities());
    assert_eq!(retried.snapshot.nodes(), clean.snapshot.nodes());

    let mut parameter_workspace = Workspace::empty_deterministic(222).expect("parameter workspace");
    let created = parameter_workspace
        .apply(Transaction {
            base_revision: parameter_workspace.current().revision(),
            edits: vec![Edit::CreateFunction {
                name: "parameter-target".to_owned(),
                type_parameters: Vec::new(),
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: DeclarationType::I64,
                }],
                return_type: DeclarationType::Unit,
            }],
        })
        .expect("create parameter target");
    let published = created.snapshot;
    let parameter = entity_named(&published, EntityKind::Parameter, "value");
    let hole = published.holes().next().expect("function hole").id;
    let failure = parameter_workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::I64(1),
                    DraftNode::SetLocal {
                        target: DraftBindingRef::Entity(parameter),
                        value: DraftNodeId::new(0),
                    },
                ],
                DraftNodeId::new(1),
            ),
        }],
    });
    assert!(matches!(
        failure,
        Err(WorkspaceError::WrongEntityKind {
            operation,
            expected,
            actual: SemanticKind::Entity(EntityKind::Parameter),
        }) if operation.as_ref() == "set-local" && expected.as_ref() == "mutable local"
    ));
    assert!(Arc::ptr_eq(&published, &parameter_workspace.current()));

    let mut scope_workspace = Workspace::empty_deterministic(228).expect("scope workspace");
    let created = scope_workspace
        .apply(Transaction {
            base_revision: scope_workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "left".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "left-value".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::Unit,
                },
                Edit::CreateFunction {
                    name: "right".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "right-value".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::Unit,
                },
            ],
        })
        .expect("create disjoint scopes");
    let left = entity_named(&created.snapshot, EntityKind::Function, "left");
    let right_parameter = entity_named(&created.snapshot, EntityKind::Parameter, "right-value");
    let left_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == left)
        .expect("left hole")
        .id;
    let published = created.snapshot;
    let invisible = scope_workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole: left_hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::I64(1),
                    DraftNode::SetLocal {
                        target: DraftBindingRef::Entity(right_parameter),
                        value: DraftNodeId::new(0),
                    },
                ],
                DraftNodeId::new(1),
            ),
        }],
    });
    assert!(matches!(
        invisible,
        Err(WorkspaceError::InvisibleEntity {
            operation,
            entity,
            reason,
        }) if operation.as_ref() == "set-local"
            && *entity == right_parameter
            && reason.as_ref().contains("lexical visibility")
    ));
    assert!(Arc::ptr_eq(&published, &scope_workspace.current()));
}

#[test]
fn consistency_rejects_set_local_targeting_immutable_hir_storage() {
    let immutable = DraftBindingId::new(0);
    let mutable = DraftBindingId::new(1);
    let draft = ExpressionDraft::new(
        vec![
            DraftNode::I64(0),
            DraftNode::I64(0),
            DraftNode::I64(1),
            DraftNode::SetLocal {
                target: DraftBindingRef::Local(mutable),
                value: DraftNodeId::new(2),
            },
            DraftNode::Load(DraftBindingRef::Local(mutable)),
            DraftNode::Sequence(vec![DraftNodeId::new(3), DraftNodeId::new(4)]),
            DraftNode::MutableLocal {
                binding: mutable,
                name: "mutable".to_owned(),
                ty: SemanticType::I64,
                initial: DraftNodeId::new(1),
                body: DraftNodeId::new(5),
            },
            DraftNode::Let {
                bindings: vec![LocalDraft {
                    binding: immutable,
                    name: "immutable".to_owned(),
                    value: DraftNodeId::new(0),
                }],
                body: DraftNodeId::new(6),
            },
        ],
        DraftNodeId::new(7),
    );
    let mut workspace = Workspace::empty_deterministic(233).expect("HIR validation workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create HIR validation main");
    let hole = created.snapshot.holes().next().expect("validation hole").id;
    let complete = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole { hole, draft }],
        })
        .expect("fill HIR validation body");
    let snapshot = complete.snapshot;
    drop(workspace);
    let mut malformed = match Arc::try_unwrap(snapshot) {
        Ok(snapshot) => snapshot,
        Err(_) => panic!("complete snapshot must be uniquely owned"),
    };
    let program = Arc::get_mut(&mut malformed.program).expect("unique semantic program");
    let body = &mut program.main.as_mut().expect("main").body;
    let crate::hir::ExprKind::Let { bindings, body } = &mut body.kind else {
        panic!("expected let root");
    };
    let immutable_binding = bindings[0].binding;
    let immutable_slot = bindings[0].slot;
    let crate::hir::ExprKind::MutableLocal { body, .. } = &mut body.kind else {
        panic!("expected mutable local");
    };
    let crate::hir::ExprKind::Do(expressions) = &mut body.kind else {
        panic!("expected sequence");
    };
    let crate::hir::ExprKind::SetLocal { target, slot, .. } = &mut expressions[0].kind else {
        panic!("expected assignment");
    };
    *target = immutable_binding;
    *slot = immutable_slot;
    let error = malformed
        .check_consistency()
        .expect_err("immutable HIR assignment target must fail closed");
    assert!(
        error.to_string().contains("set-local target kind"),
        "{error}"
    );
}

#[test]
fn consistency_rejects_non_unit_while_hir_type() {
    let mut workspace = Workspace::empty_deterministic(234).expect("while validation workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create while validation main");
    let hole = created.snapshot.holes().next().expect("while hole").id;
    let complete = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: counted_loop_draft(1),
            }],
        })
        .expect("fill while validation body");
    let snapshot = complete.snapshot;
    drop(workspace);
    let mut malformed = match Arc::try_unwrap(snapshot) {
        Ok(snapshot) => snapshot,
        Err(_) => panic!("complete snapshot must be uniquely owned"),
    };
    let program = Arc::get_mut(&mut malformed.program).expect("unique semantic program");
    let body = &mut program.main.as_mut().expect("main").body;
    let crate::hir::ExprKind::MutableLocal { body, .. } = &mut body.kind else {
        panic!("expected mutable root");
    };
    let crate::hir::ExprKind::Do(expressions) = &mut body.kind else {
        panic!("expected sequence");
    };
    assert!(matches!(
        expressions[0].kind,
        crate::hir::ExprKind::While { .. }
    ));
    expressions[0].ty = crate::Type::I64;
    let error = malformed
        .check_consistency()
        .expect_err("non-unit while HIR must fail closed");
    assert!(
        error
            .to_string()
            .contains("expression kind and type facts are inconsistent"),
        "{error}"
    );
}

#[test]
fn adjacent_nested_while_ids_are_distinct_and_sequence_order_is_observable() {
    let local = DraftBindingId::new(0);
    let mut nodes = Vec::new();
    let initial = push_draft_node(&mut nodes, DraftNode::I64(0));
    let false_one = push_draft_node(&mut nodes, DraftNode::Bool(false));
    let ninety_nine = push_draft_node(&mut nodes, DraftNode::I64(99));
    let skipped_set = push_draft_node(
        &mut nodes,
        DraftNode::SetLocal {
            target: DraftBindingRef::Local(local),
            value: ninety_nine,
        },
    );
    let first = push_draft_node(
        &mut nodes,
        DraftNode::While {
            condition: false_one,
            body: vec![skipped_set],
        },
    );
    let false_nested = push_draft_node(&mut nodes, DraftNode::Bool(false));
    let nested = push_draft_node(
        &mut nodes,
        DraftNode::While {
            condition: false_nested,
            body: Vec::new(),
        },
    );
    let false_outer = push_draft_node(&mut nodes, DraftNode::Bool(false));
    let outer = push_draft_node(
        &mut nodes,
        DraftNode::While {
            condition: false_outer,
            body: vec![nested],
        },
    );
    let one = push_draft_node(&mut nodes, DraftNode::I64(1));
    let set_one = push_draft_node(
        &mut nodes,
        DraftNode::SetLocal {
            target: DraftBindingRef::Local(local),
            value: one,
        },
    );
    let two = push_draft_node(&mut nodes, DraftNode::I64(2));
    let set_two = push_draft_node(
        &mut nodes,
        DraftNode::SetLocal {
            target: DraftBindingRef::Local(local),
            value: two,
        },
    );
    let result = push_draft_node(&mut nodes, DraftNode::Load(DraftBindingRef::Local(local)));
    let sequence = push_draft_node(
        &mut nodes,
        DraftNode::Sequence(vec![set_one, set_two, first, outer, result]),
    );
    let root = push_draft_node(
        &mut nodes,
        DraftNode::MutableLocal {
            binding: local,
            name: "ordered".to_owned(),
            ty: SemanticType::I64,
            initial,
            body: sequence,
        },
    );
    let mut workspace = Workspace::empty_deterministic(223).expect("loop workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create loop main");
    let hole = created.snapshot.holes().next().expect("loop hole").id;
    let complete = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(nodes, root),
            }],
        })
        .expect("fill adjacent loops");
    assert_eq!(run_i64(&complete.snapshot), 2);

    let mut ids = std::collections::BTreeSet::new();
    let body = &complete.snapshot.program.main.as_ref().expect("main").body;
    let mut pending = vec![body];
    while let Some(expression) = pending.pop() {
        if let crate::hir::ExprKind::While { loop_id, .. } = &expression.kind {
            ids.insert(loop_id.raw());
        }
        crate::hir::for_each_expression_child(expression, &mut |child| pending.push(child));
    }
    assert_eq!(ids.len(), 3);
}

#[test]
fn affine_mutable_reinitialization_moves_and_cleans_up_exactly_once() {
    let local = DraftBindingId::new(0);
    let mut workspace = Workspace::empty_deterministic(229).expect("affine mutable workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create affine main");
    let hole = created.snapshot.holes().next().expect("affine hole").id;
    let published = created.snapshot;
    let live_overwrite = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::I64(1),
                    DraftNode::Operation {
                        operation: crate::Operation::ByteVectorNew,
                        arguments: vec![DraftNodeId::new(0)],
                    },
                    DraftNode::I64(4),
                    DraftNode::Operation {
                        operation: crate::Operation::ByteVectorNew,
                        arguments: vec![DraftNodeId::new(2)],
                    },
                    DraftNode::SetLocal {
                        target: DraftBindingRef::Local(local),
                        value: DraftNodeId::new(3),
                    },
                    DraftNode::I64(0),
                    DraftNode::Sequence(vec![DraftNodeId::new(4), DraftNodeId::new(5)]),
                    DraftNode::MutableLocal {
                        binding: local,
                        name: "buffer".to_owned(),
                        ty: SemanticType::ByteVector,
                        initial: DraftNodeId::new(1),
                        body: DraftNodeId::new(6),
                    },
                ],
                DraftNodeId::new(7),
            ),
        }],
    });
    assert!(matches!(
        live_overwrite,
        Err(WorkspaceError::Validation(message))
            if message.as_ref() == "affine assignment is only reinitialization after move or drop"
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let draft = ExpressionDraft::new(
        vec![
            DraftNode::I64(1),
            DraftNode::Operation {
                operation: crate::Operation::ByteVectorNew,
                arguments: vec![DraftNodeId::new(0)],
            },
            DraftNode::Move(DraftBindingRef::Local(local)),
            DraftNode::I64(4),
            DraftNode::Operation {
                operation: crate::Operation::ByteVectorNew,
                arguments: vec![DraftNodeId::new(3)],
            },
            DraftNode::SetLocal {
                target: DraftBindingRef::Local(local),
                value: DraftNodeId::new(4),
            },
            DraftNode::Move(DraftBindingRef::Local(local)),
            DraftNode::I64(4),
            DraftNode::Sequence(vec![
                DraftNodeId::new(2),
                DraftNodeId::new(5),
                DraftNodeId::new(6),
                DraftNodeId::new(7),
            ]),
            DraftNode::MutableLocal {
                binding: local,
                name: "buffer".to_owned(),
                ty: SemanticType::ByteVector,
                initial: DraftNodeId::new(1),
                body: DraftNodeId::new(8),
            },
        ],
        DraftNodeId::new(9),
    );
    let complete = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole { hole, draft }],
        })
        .expect("fill affine mutable body");
    complete
        .snapshot
        .check_consistency()
        .expect("validate affine mutable storage");
    let executable = crate::compile_snapshot(&complete.snapshot).expect("compile affine mutable");
    assert!(executable
        .memory_plan()
        .obligations
        .iter()
        .any(|obligation| obligation.kind
            == crate::memory_plan::MemoryObligationKind::DropWholeValue));
    let outcome = run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(outcome.cleanup_failures().is_none());
    assert!(matches!(
        outcome,
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(4)
    ));
    let buffer = entity_named(&complete.snapshot, EntityKind::MutableLocal, "buffer");
    assert_eq!(
        complete
            .snapshot
            .entity_type(complete.snapshot.revision(), buffer)
            .expect("buffer type")
            .declared,
        Some(SemanticType::ByteVector)
    );
}

#[test]
fn unit_hole_legal_constructors_expose_imperative_forms_and_visible_mutable_set() {
    let local = DraftBindingId::new(0);
    let mut workspace = Workspace::empty_deterministic(224).expect("constructor workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create constructor main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let complete = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(0),
                        DraftNode::Unit,
                        DraftNode::Load(DraftBindingRef::Local(local)),
                        DraftNode::Sequence(vec![DraftNodeId::new(1), DraftNodeId::new(2)]),
                        DraftNode::MutableLocal {
                            binding: local,
                            name: "target".to_owned(),
                            ty: SemanticType::I64,
                            initial: DraftNodeId::new(0),
                            body: DraftNodeId::new(3),
                        },
                    ],
                    DraftNodeId::new(4),
                ),
            }],
        })
        .expect("fill constructor subject");
    let target = entity_named(&complete.snapshot, EntityKind::MutableLocal, "target");
    let unit = complete
        .snapshot
        .nodes()
        .iter()
        .find(|node| {
            node.kind == NodeKind::Literal && {
                complete
                    .snapshot
                    .node_type(complete.snapshot.revision(), node.id)
                    .is_ok_and(|facts| facts.actual == SemanticType::Unit)
            }
        })
        .expect("unit node")
        .id;
    let introduced = workspace
        .apply(Transaction {
            base_revision: complete.snapshot.revision(),
            edits: vec![Edit::IntroduceHole {
                target: unit,
                goal: "perform one local action".to_owned(),
            }],
        })
        .expect("introduce unit hole");
    let hole = introduced.snapshot.holes().next().expect("unit hole");
    assert!(hole.visible_entities.contains(&target));
    let constructors = introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            hole.id,
            PageRequest::new(64).expect("constructor page"),
            None,
        )
        .expect("unit constructors")
        .items;
    for constructor in [
        LegalConstructor::Sequence,
        LegalConstructor::MutableLocal,
        LegalConstructor::While,
        LegalConstructor::SetLocal(target),
    ] {
        assert!(
            constructors.contains(&constructor),
            "missing {constructor:?}"
        );
    }
    assert!(constructors
        .iter()
        .filter_map(|constructor| match constructor {
            LegalConstructor::SetLocal(entity) => Some(*entity),
            _ => None,
        })
        .all(|entity| entity == target));
}

#[test]
fn wide_imperative_draft_visits_each_node_once_on_a_small_stack() {
    std::thread::Builder::new()
        .name("workspace-wide-imperative".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| {
            let local = DraftBindingId::new(0);
            let mut nodes = Vec::new();
            let initial = push_draft_node(&mut nodes, DraftNode::I64(0));
            let mut body = Vec::new();
            for value in 1..=512_i64 {
                let value = push_draft_node(&mut nodes, DraftNode::I64(value));
                let set = push_draft_node(
                    &mut nodes,
                    DraftNode::SetLocal {
                        target: DraftBindingRef::Local(local),
                        value,
                    },
                );
                body.push(set);
            }
            let result =
                push_draft_node(&mut nodes, DraftNode::Load(DraftBindingRef::Local(local)));
            body.push(result);
            let sequence = push_draft_node(&mut nodes, DraftNode::Sequence(body));
            let root = push_draft_node(
                &mut nodes,
                DraftNode::MutableLocal {
                    binding: local,
                    name: "wide".to_owned(),
                    ty: SemanticType::I64,
                    initial,
                    body: sequence,
                },
            );
            let node_count = u64::try_from(nodes.len()).expect("node count");
            let mut workspace = Workspace::empty_deterministic(225).expect("wide workspace");
            let created = workspace
                .apply(Transaction {
                    base_revision: workspace.current().revision(),
                    edits: vec![Edit::CreateMain {
                        return_type: SemanticType::I64,
                    }],
                })
                .expect("create wide main");
            let hole = created.snapshot.holes().next().expect("wide hole").id;
            super::transaction::reset_draft_imperative_node_visits();
            let complete = workspace
                .apply(Transaction {
                    base_revision: created.snapshot.revision(),
                    edits: vec![Edit::FillHole {
                        hole,
                        draft: ExpressionDraft::new(nodes, root),
                    }],
                })
                .expect("fill wide imperative draft");
            assert_eq!(
                super::transaction::draft_imperative_node_visits(),
                (node_count, node_count)
            );
            assert_eq!(run_i64(&complete.snapshot), 512);
        })
        .expect("spawn wide imperative thread")
        .join()
        .expect("wide imperative thread completes");
}

#[test]
fn wide_stable_assignment_targets_use_one_callable_location_scan() {
    fn run(width: usize, seed: u64) -> (u64, u64) {
        let mut workspace = Workspace::empty_deterministic(seed).expect("stable target workspace");
        let created = workspace
            .apply(Transaction {
                base_revision: workspace.current().revision(),
                edits: vec![Edit::CreateMain {
                    return_type: SemanticType::I64,
                }],
            })
            .expect("create stable target main");
        let hole = created
            .snapshot
            .holes()
            .next()
            .expect("stable target hole")
            .id;
        let baseline = workspace
            .apply(Transaction {
                base_revision: created.snapshot.revision(),
                edits: vec![Edit::FillHole {
                    hole,
                    draft: counted_loop_draft(1),
                }],
            })
            .expect("fill stable target baseline");
        let target = entity_named(&baseline.snapshot, EntityKind::MutableLocal, "counter");
        let sequence = baseline
            .snapshot
            .nodes()
            .iter()
            .find(|node| node.kind == NodeKind::Sequence)
            .expect("replaceable sequence")
            .id;
        let mut nodes = Vec::new();
        let mut expressions = Vec::new();
        for value in 1..=width {
            let value = push_draft_node(
                &mut nodes,
                DraftNode::I64(i64::try_from(value).expect("assignment value")),
            );
            expressions.push(push_draft_node(
                &mut nodes,
                DraftNode::SetLocal {
                    target: DraftBindingRef::Entity(target),
                    value,
                },
            ));
        }
        expressions.push(push_draft_node(
            &mut nodes,
            DraftNode::Load(DraftBindingRef::Entity(target)),
        ));
        let root = push_draft_node(&mut nodes, DraftNode::Sequence(expressions));
        super::transaction::reset_binding_location_work();
        let replaced = workspace
            .apply(Transaction {
                base_revision: baseline.snapshot.revision(),
                edits: vec![Edit::ReplaceExpression {
                    target: sequence,
                    draft: ExpressionDraft::new(nodes, root),
                }],
            })
            .expect("replace with stable target assignments");
        assert_eq!(
            run_i64(&replaced.snapshot),
            i64::try_from(width).expect("result width")
        );
        super::transaction::binding_location_work()
    }

    let narrow = run(1, 230);
    let wide = run(2_000, 231);
    assert_eq!(narrow.0, wide.0, "callable scan must not scale with uses");
    assert_eq!(narrow.1, 2);
    assert_eq!(wide.1, 2_001);
}

#[test]
fn mutable_local_identity_survives_rename_and_recreation_is_fresh() {
    let mut workspace = Workspace::empty_deterministic(226).expect("mutable identity workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create mutable identity main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let complete = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: counted_loop_draft(2),
            }],
        })
        .expect("fill mutable identity subject");
    let local = entity_named(&complete.snapshot, EntityKind::MutableLocal, "counter");
    let reference_targets = complete
        .snapshot
        .references()
        .iter()
        .filter(|edge| edge.target == local)
        .count();
    let renamed = workspace
        .apply(Transaction {
            base_revision: complete.snapshot.revision(),
            edits: vec![Edit::RenameEntity {
                entity: local,
                new_name: "renamed".to_owned(),
            }],
        })
        .expect("rename mutable local");
    assert_eq!(
        renamed
            .snapshot
            .entity(local)
            .expect("renamed local")
            .name
            .as_ref(),
        "renamed"
    );
    assert_eq!(
        renamed
            .snapshot
            .references()
            .iter()
            .filter(|edge| edge.target == local)
            .count(),
        reference_targets
    );
    assert!(renamed.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::EntityRenamed { entity, old_name, new_name }
            if *entity == local && old_name.as_ref() == "counter" && new_name.as_ref() == "renamed"
    )));
    let old = renamed.snapshot.clone();
    let root = renamed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::MutableLocal)
        .expect("mutable root")
        .id;
    let removed = workspace
        .apply(Transaction {
            base_revision: renamed.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: root,
                draft: ExpressionDraft::scalar_i64(9),
            }],
        })
        .expect("remove mutable definition");
    assert!(removed.snapshot.entity(local).is_err());
    assert!(removed.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::EntityDeleted {
            entity,
            kind: EntityKind::MutableLocal,
            ..
        } if *entity == local
    )));
    assert_eq!(run_i64(&old), 2);
    assert_eq!(
        old.entity(local).expect("old local").name.as_ref(),
        "renamed"
    );
    let published = removed.snapshot;
    let stale_target = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::ReplaceExpression {
            target: root,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::I64(1),
                    DraftNode::SetLocal {
                        target: DraftBindingRef::Entity(local),
                        value: DraftNodeId::new(0),
                    },
                    DraftNode::I64(9),
                    DraftNode::Sequence(vec![DraftNodeId::new(1), DraftNodeId::new(2)]),
                ],
                DraftNodeId::new(3),
            ),
        }],
    });
    assert!(matches!(
        stale_target,
        Err(WorkspaceError::StaleIdentity(_))
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let recreated = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: root,
                draft: {
                    let mut draft = counted_loop_draft(3);
                    if let Some(DraftNode::MutableLocal { name, .. }) = draft.nodes.last_mut() {
                        *name = "renamed".to_owned();
                    }
                    draft
                },
            }],
        })
        .expect("recreate mutable definition");
    let fresh = entity_named(&recreated.snapshot, EntityKind::MutableLocal, "renamed");
    assert_ne!(fresh, local);
    assert!(recreated.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::EntityCreated {
            entity,
            kind: EntityKind::MutableLocal,
            name,
        } if *entity == fresh && name.as_ref() == "renamed"
    )));
    assert_eq!(run_i64(&recreated.snapshot), 3);
    assert_eq!(run_i64(&old), 2);
}

#[test]
fn imported_mutable_entity_load_and_set_are_usable_only_in_lexical_body() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "var/\nname/\nx\n/name\ntype/\ni64\n/type\n0\nx\n/var\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "workspace-stable-mutable-reference.lkjscript",
        WorkspaceNamespace::deterministic(227),
    )
    .expect("import stable mutable subject");
    let mutable = entity_named(&snapshot, EntityKind::MutableLocal, "x");
    let root = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::MutableLocal)
        .expect("mutable root")
        .id;
    let initial = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Literal)
        .expect("initial literal")
        .id;
    let body = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Load)
        .expect("mutable body load")
        .id;
    let mut workspace = Workspace::new(snapshot).expect("imported mutable workspace");
    let edited = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::ReplaceExpression {
                target: body,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(5),
                        DraftNode::SetLocal {
                            target: DraftBindingRef::Entity(mutable),
                            value: DraftNodeId::new(0),
                        },
                        DraftNode::Load(DraftBindingRef::Entity(mutable)),
                        DraftNode::Sequence(vec![DraftNodeId::new(1), DraftNodeId::new(2)]),
                    ],
                    DraftNodeId::new(3),
                ),
            }],
        })
        .expect("use stable mutable entity in body");
    assert_eq!(run_i64(&edited.snapshot), 5);
    let published = edited.snapshot;

    for (target, draft) in [
        (
            initial,
            ExpressionDraft::new(
                vec![DraftNode::Load(DraftBindingRef::Entity(mutable))],
                DraftNodeId::new(0),
            ),
        ),
        (
            root,
            ExpressionDraft::new(
                vec![
                    DraftNode::I64(1),
                    DraftNode::SetLocal {
                        target: DraftBindingRef::Entity(mutable),
                        value: DraftNodeId::new(0),
                    },
                ],
                DraftNodeId::new(1),
            ),
        ),
    ] {
        assert!(workspace
            .apply(Transaction {
                base_revision: published.revision(),
                edits: vec![Edit::ReplaceExpression { target, draft }],
            })
            .is_err());
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }
}

#[test]
fn source_free_generic_function_creation_is_exact_and_executes_without_source_work() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    crate::pipeline::reset_lowering_invocations();
    let mut workspace = Workspace::empty_deterministic(200).expect("empty workspace");
    let binder = DraftTypeParameterId::new(7);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "identity".to_owned(),
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
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create source-free generic identity");
    let function = entity_named(&created.snapshot, EntityKind::Function, "identity");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let signature = created
        .snapshot
        .function_signature(created.snapshot.revision(), function)
        .expect("generic signature");
    assert_eq!(signature.type_parameters.len(), 1);
    let parameter = signature.type_parameters[0].id;
    assert_eq!(signature.type_parameters[0].owner, function);
    assert_eq!(signature.type_parameters[0].name.as_ref(), "t");
    assert_eq!(
        signature.type_parameters[0].bounds,
        [TypeParameterBoundView {
            parameter,
            trait_identity: SemanticTrait::Builtin(BuiltinTrait::Copy),
        }]
    );
    assert_eq!(signature.parameters.len(), 1);
    assert_eq!(
        signature.parameters[0].ty,
        SemanticType::TypeParameter(parameter)
    );
    assert_eq!(signature.result, SemanticType::TypeParameter(parameter));
    let value = signature.parameters[0].entity;
    let function_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == function)
        .expect("generic body hole");
    assert_eq!(
        function_hole.expected_type,
        SemanticType::TypeParameter(parameter)
    );
    assert!(function_hole.visible_entities.contains(&parameter));
    let function_hole_id = function_hole.id;
    let main_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main body hole")
        .id;
    assert!(matches!(
        crate::compile_snapshot(&created.snapshot),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));
    assert_eq!(crate::pipeline::lowering_invocations(), 0);
    for expected in [function, parameter, value, main] {
        assert!(created.diff.entries.iter().any(|entry| matches!(
            entry,
            SemanticDiffEntry::EntityCreated { entity, .. } if *entity == expected
        )));
    }
    let projection = created
        .snapshot
        .project(&[
            ProjectionSlice::Entity(function),
            ProjectionSlice::Body(function),
        ])
        .expect("generic declaration projection");
    assert!(projection.contains("type-parameter"));
    assert!(projection.contains("bound trait=builtin:copy"));
    assert!(projection.contains("type-parameter("));

    let body = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: function_hole_id,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Load(DraftBindingRef::Entity(value)),
                        DraftNode::Return {
                            value: DraftNodeId::new(0),
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("fill generic identity body");
    let completed = workspace
        .apply(Transaction {
            base_revision: body.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::Call {
                            callee: function,
                            type_arguments: vec![TypeArgumentDraft {
                                parameter,
                                argument: SemanticType::I64,
                            }],
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("call source-free generic identity");
    let call = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("generic call")
        .id;
    let view = completed
        .snapshot
        .call_instantiation(completed.snapshot.revision(), call)
        .expect("generic call view");
    assert_eq!(
        view.type_arguments,
        [TypeArgumentView {
            parameter,
            argument: SemanticType::I64,
        }]
    );
    assert_eq!(view.parameters, [SemanticType::I64]);
    assert_eq!(view.result, SemanticType::I64);
    assert_eq!(view.witnesses.len(), 1);
    assert_eq!(view.witnesses[0].parameter, parameter);
    assert_eq!(
        view.witnesses[0].trait_identity,
        SemanticTrait::Builtin(BuiltinTrait::Copy)
    );
    assert_eq!(view.witnesses[0].kind, TraitWitnessKindView::AutoTrait);
    assert!(view.effects.is_known());
    assert!(view.effects.contains(EffectSummary::MAY_DIVERGE));
    assert!(completed
        .snapshot
        .calls()
        .iter()
        .any(|edge| edge.caller == main && edge.callee == function && edge.site == call));
    assert!(completed
        .snapshot
        .references()
        .iter()
        .any(|edge| edge.target == value));
    assert!(completed
        .snapshot
        .dependencies()
        .iter()
        .any(|edge| edge.dependent == main && edge.dependency == function));
    assert_eq!(run_i64(&completed.snapshot), 42);
    assert_eq!(crate::pipeline::lowering_invocations(), 1);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn source_free_generic_declaration_preserves_binder_and_bound_order_in_nested_types() {
    let mut workspace = Workspace::empty_deterministic(199).expect("empty workspace");
    let nominal = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateProduct {
                name: "context".to_owned(),
                fields: vec![ProductFieldDraft {
                    name: "value".to_owned(),
                    ty: SemanticType::I64,
                }],
            }],
        })
        .expect("create stable nominal context");
    let product = entity_named(&nominal.snapshot, EntityKind::Product, "context");
    let first = DraftTypeParameterId::new(9);
    let second = DraftTypeParameterId::new(2);
    let created = workspace
        .apply(Transaction {
            base_revision: nominal.snapshot.revision(),
            edits: vec![Edit::CreateFunction {
                name: "nested".to_owned(),
                type_parameters: vec![
                    TypeParameterDraft {
                        id: first,
                        name: "first".to_owned(),
                        bounds: vec![
                            SemanticTrait::Builtin(BuiltinTrait::Copy),
                            SemanticTrait::Builtin(BuiltinTrait::Send),
                        ],
                    },
                    TypeParameterDraft {
                        id: second,
                        name: "second".to_owned(),
                        bounds: vec![SemanticTrait::Builtin(BuiltinTrait::Sync)],
                    },
                ],
                parameters: vec![
                    ParameterDraft {
                        name: "value".to_owned(),
                        ty: DeclarationType::Enum {
                            constructor: SemanticEnum::Builtin(BuiltinEnum::Result),
                            arguments: vec![
                                DeclarationType::List(Box::new(
                                    DeclarationType::DraftTypeParameter(first),
                                )),
                                DeclarationType::Enum {
                                    constructor: SemanticEnum::Builtin(BuiltinEnum::Option),
                                    arguments: vec![DeclarationType::DraftTypeParameter(second)],
                                },
                            ],
                        },
                    },
                    ParameterDraft {
                        name: "context".to_owned(),
                        ty: DeclarationType::Product(product),
                    },
                ],
                return_type: DeclarationType::Function {
                    parameters: vec![DeclarationType::DraftTypeParameter(second)],
                    result: Box::new(DeclarationType::DraftTypeParameter(first)),
                },
            }],
        })
        .expect("create nested generic declaration");
    let function = entity_named(&created.snapshot, EntityKind::Function, "nested");
    let signature = created
        .snapshot
        .function_signature(created.snapshot.revision(), function)
        .expect("nested signature");
    assert_eq!(
        signature
            .type_parameters
            .iter()
            .map(|parameter| parameter.name.as_ref())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
    let first = signature.type_parameters[0].id;
    let second = signature.type_parameters[1].id;
    assert_eq!(
        signature.type_parameters[0]
            .bounds
            .iter()
            .map(|bound| bound.trait_identity)
            .collect::<Vec<_>>(),
        [
            SemanticTrait::Builtin(BuiltinTrait::Copy),
            SemanticTrait::Builtin(BuiltinTrait::Send),
        ]
    );
    assert_eq!(
        signature.type_parameters[1].bounds[0].trait_identity,
        SemanticTrait::Builtin(BuiltinTrait::Sync)
    );
    assert_eq!(
        signature.parameters[0].ty,
        SemanticType::Enum {
            constructor: SemanticEnum::Builtin(BuiltinEnum::Result),
            arguments: vec![
                SemanticType::List(Box::new(SemanticType::TypeParameter(first))),
                SemanticType::Enum {
                    constructor: SemanticEnum::Builtin(BuiltinEnum::Option),
                    arguments: vec![SemanticType::TypeParameter(second)],
                },
            ],
        }
    );
    assert_eq!(signature.parameters[1].ty, SemanticType::Product(product));
    assert_eq!(
        signature.result,
        SemanticType::Function {
            parameters: vec![SemanticType::TypeParameter(second)],
            result: Box::new(SemanticType::TypeParameter(first)),
        }
    );
    assert!(created
        .snapshot
        .dependencies()
        .iter()
        .any(|edge| edge.dependent == function && edge.dependency == product));
}

#[test]
fn malformed_generic_declarations_are_structured_atomic_and_retry_stable() {
    let seed = 198;
    let mut workspace = Workspace::empty_deterministic(seed).expect("empty workspace");
    let before = workspace.current();
    let projection = before.project(&[]).expect("empty projection");
    let duplicate = DraftTypeParameterId::new(0);
    let failure = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::CreateFunction {
            name: "identity".to_owned(),
            type_parameters: vec![
                TypeParameterDraft {
                    id: duplicate,
                    name: "t".to_owned(),
                    bounds: Vec::new(),
                },
                TypeParameterDraft {
                    id: duplicate,
                    name: "u".to_owned(),
                    bounds: Vec::new(),
                },
            ],
            parameters: vec![ParameterDraft {
                name: "value".to_owned(),
                ty: DeclarationType::DraftTypeParameter(duplicate),
            }],
            return_type: DeclarationType::DraftTypeParameter(duplicate),
        }],
    });
    assert_eq!(
        failure.expect_err("duplicate draft binder rejects"),
        WorkspaceError::DuplicateDraftTypeParameter {
            parameter: duplicate,
        }
    );
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    assert_eq!(workspace.current().revision(), before.revision());
    assert_eq!(workspace.current().diagnostics(), before.diagnostics());
    assert_eq!(
        workspace.current().project(&[]).expect("projection"),
        projection
    );

    let declared = DraftTypeParameterId::new(1);
    let unknown = DraftTypeParameterId::new(2);
    let failure = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::CreateFunction {
            name: "identity".to_owned(),
            type_parameters: vec![TypeParameterDraft {
                id: declared,
                name: "t".to_owned(),
                bounds: Vec::new(),
            }],
            parameters: vec![ParameterDraft {
                name: "value".to_owned(),
                ty: DeclarationType::DraftTypeParameter(unknown),
            }],
            return_type: DeclarationType::DraftTypeParameter(declared),
        }],
    });
    assert_eq!(
        failure.expect_err("unknown draft binder rejects"),
        WorkspaceError::UnknownDraftTypeParameter { parameter: unknown }
    );
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let same_name = DraftTypeParameterId::new(3);
    let duplicate_name = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::CreateFunction {
            name: "duplicate-name".to_owned(),
            type_parameters: vec![
                TypeParameterDraft {
                    id: declared,
                    name: "t".to_owned(),
                    bounds: Vec::new(),
                },
                TypeParameterDraft {
                    id: same_name,
                    name: "t".to_owned(),
                    bounds: Vec::new(),
                },
            ],
            parameters: vec![ParameterDraft {
                name: "value".to_owned(),
                ty: DeclarationType::DraftTypeParameter(declared),
            }],
            return_type: DeclarationType::DraftTypeParameter(same_name),
        }],
    });
    assert_eq!(
        duplicate_name.expect_err("duplicate binder name rejects"),
        WorkspaceError::DuplicateTypeParameterName {
            first: declared,
            duplicate: same_name,
        }
    );
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let unused = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::CreateFunction {
            name: "unused".to_owned(),
            type_parameters: vec![TypeParameterDraft {
                id: declared,
                name: "t".to_owned(),
                bounds: Vec::new(),
            }],
            parameters: Vec::new(),
            return_type: DeclarationType::Unit,
        }],
    });
    assert_eq!(
        unused.expect_err("unused binder rejects"),
        WorkspaceError::UnusedDraftTypeParameter {
            parameter: declared,
        }
    );
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    for invalid in [
        Edit::CreateFunction {
            name: "invalid-binder".to_owned(),
            type_parameters: vec![TypeParameterDraft {
                id: declared,
                name: String::new(),
                bounds: Vec::new(),
            }],
            parameters: Vec::new(),
            return_type: DeclarationType::DraftTypeParameter(declared),
        },
        Edit::CreateFunction {
            name: "invalid-value".to_owned(),
            type_parameters: vec![TypeParameterDraft {
                id: declared,
                name: "t".to_owned(),
                bounds: Vec::new(),
            }],
            parameters: vec![ParameterDraft {
                name: String::new(),
                ty: DeclarationType::DraftTypeParameter(declared),
            }],
            return_type: DeclarationType::DraftTypeParameter(declared),
        },
    ] {
        let invalid_name = workspace.apply(Transaction {
            base_revision: before.revision(),
            edits: vec![invalid],
        });
        assert!(matches!(
            invalid_name,
            Err(WorkspaceError::InvalidTransaction(_))
        ));
        assert!(Arc::ptr_eq(&before, &workspace.current()));
    }

    let corrected = Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::CreateFunction {
            name: "identity".to_owned(),
            type_parameters: vec![TypeParameterDraft {
                id: declared,
                name: "t".to_owned(),
                bounds: vec![SemanticTrait::Builtin(BuiltinTrait::Copy)],
            }],
            parameters: vec![ParameterDraft {
                name: "value".to_owned(),
                ty: DeclarationType::DraftTypeParameter(declared),
            }],
            return_type: DeclarationType::DraftTypeParameter(declared),
        }],
    };
    let retry = workspace.apply(corrected.clone()).expect("corrected retry");
    let mut control = Workspace::empty_deterministic(seed).expect("control workspace");
    let control = control.apply(corrected).expect("control creation");
    assert_eq!(retry.snapshot.entities(), control.snapshot.entities());
    assert_eq!(retry.snapshot.nodes(), control.snapshot.nodes());
    assert_eq!(
        retry.snapshot.holes().collect::<Vec<_>>(),
        control.snapshot.holes().collect::<Vec<_>>()
    );

    let published = workspace.current();
    let conflict = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::CreateFunction {
            name: "identity".to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::Unit,
        }],
    });
    assert!(matches!(
        conflict,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));
}

#[test]
fn generic_declaration_trait_identities_fail_closed_before_publication() {
    let mut workspace = Workspace::empty_deterministic(192).expect("empty workspace");
    let main_created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create wrong-kind identity");
    let main = entity_named(&main_created.snapshot, EntityKind::Main, "main");
    let before = workspace.current();
    let parameter = DraftTypeParameterId::new(0);
    let bounded = |bound| Edit::CreateFunction {
        name: "bounded".to_owned(),
        type_parameters: vec![TypeParameterDraft {
            id: parameter,
            name: "t".to_owned(),
            bounds: vec![bound],
        }],
        parameters: vec![ParameterDraft {
            name: "value".to_owned(),
            ty: DeclarationType::DraftTypeParameter(parameter),
        }],
        return_type: DeclarationType::DraftTypeParameter(parameter),
    };
    let wrong_kind = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![bounded(SemanticTrait::Entity(main))],
    });
    assert!(matches!(
        wrong_kind,
        Err(WorkspaceError::WrongEntityKind { .. })
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let foreign_workspace = Workspace::empty_deterministic(191).expect("foreign workspace");
    let foreign = foreign_workspace.current().namespace();
    let foreign_entity = EntityId::new(foreign, main.slot(), main.generation());
    let foreign_failure = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![bounded(SemanticTrait::Entity(foreign_entity))],
    });
    assert!(matches!(
        foreign_failure,
        Err(WorkspaceError::ForeignNamespace(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let stale_entity = EntityId::new(
        main.namespace(),
        main.slot(),
        main.generation().checked_add(1).expect("stale generation"),
    );
    let stale_failure = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![bounded(SemanticTrait::Entity(stale_entity))],
    });
    assert!(matches!(
        stale_failure,
        Err(WorkspaceError::StaleIdentity(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let duplicate = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::CreateFunction {
            name: "bounded".to_owned(),
            type_parameters: vec![TypeParameterDraft {
                id: parameter,
                name: "t".to_owned(),
                bounds: vec![
                    SemanticTrait::Builtin(BuiltinTrait::Copy),
                    SemanticTrait::Builtin(BuiltinTrait::Copy),
                ],
            }],
            parameters: vec![ParameterDraft {
                name: "value".to_owned(),
                ty: DeclarationType::DraftTypeParameter(parameter),
            }],
            return_type: DeclarationType::DraftTypeParameter(parameter),
        }],
    });
    assert_eq!(
        duplicate.expect_err("duplicate bound rejects"),
        WorkspaceError::DuplicateTypeParameterBound {
            parameter,
            trait_identity: SemanticTrait::Builtin(BuiltinTrait::Copy),
        }
    );
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let unsupported = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![bounded(SemanticTrait::Builtin(BuiltinTrait::Clone))],
    });
    assert!(matches!(
        unsupported,
        Err(WorkspaceError::UnsupportedEdit { .. })
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
}

#[test]
fn generic_declaration_type_identities_and_shapes_fail_closed_atomically() {
    let mut workspace = Workspace::empty_deterministic(189).expect("empty workspace");
    let draft = DraftTypeParameterId::new(0);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "owner".to_owned(),
                    type_parameters: vec![TypeParameterDraft {
                        id: draft,
                        name: "t".to_owned(),
                        bounds: Vec::new(),
                    }],
                    parameters: vec![ParameterDraft {
                        name: "value".to_owned(),
                        ty: DeclarationType::DraftTypeParameter(draft),
                    }],
                    return_type: DeclarationType::DraftTypeParameter(draft),
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create stable identities");
    let owner = entity_named(&created.snapshot, EntityKind::Function, "owner");
    let stable_parameter = created
        .snapshot
        .function_signature(created.snapshot.revision(), owner)
        .expect("owner signature")
        .type_parameters[0]
        .id;
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let before = workspace.current();
    let declaration = |name: &str, parameter_type, return_type| Edit::CreateFunction {
        name: name.to_owned(),
        type_parameters: Vec::new(),
        parameters: vec![ParameterDraft {
            name: "value".to_owned(),
            ty: parameter_type,
        }],
        return_type,
    };

    let wrong_owner = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![declaration(
            "wrong-owner",
            DeclarationType::TypeParameter(stable_parameter),
            DeclarationType::Unit,
        )],
    });
    assert!(matches!(
        wrong_owner,
        Err(WorkspaceError::WrongTypeParameterOwner {
            parameter,
            actual: Some(actual),
            ..
        }) if parameter.as_ref() == &stable_parameter && actual.as_ref() == &owner
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let wrong_kind = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![declaration(
            "wrong-kind",
            DeclarationType::TypeParameter(main),
            DeclarationType::Unit,
        )],
    });
    assert!(matches!(
        wrong_kind,
        Err(WorkspaceError::WrongEntityKind { .. })
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let stale = EntityId::new(
        main.namespace(),
        main.slot(),
        main.generation().checked_add(1).expect("stale generation"),
    );
    let stale_failure = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![declaration(
            "stale-type",
            DeclarationType::TypeParameter(stale),
            DeclarationType::Unit,
        )],
    });
    assert!(matches!(
        stale_failure,
        Err(WorkspaceError::StaleIdentity(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let foreign_namespace = Workspace::empty_deterministic(188)
        .expect("foreign workspace")
        .current()
        .namespace();
    let foreign = EntityId::new(foreign_namespace, main.slot(), main.generation());
    let foreign_failure = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![declaration(
            "foreign-type",
            DeclarationType::TypeParameter(foreign),
            DeclarationType::Unit,
        )],
    });
    assert!(matches!(
        foreign_failure,
        Err(WorkspaceError::ForeignNamespace(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let invalid_arity = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![declaration(
            "bad-arity",
            DeclarationType::Enum {
                constructor: SemanticEnum::Builtin(BuiltinEnum::Option),
                arguments: Vec::new(),
            },
            DeclarationType::Unit,
        )],
    });
    assert!(matches!(
        invalid_arity,
        Err(WorkspaceError::InvalidSemanticType { .. })
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let reference_result = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![declaration(
            "reference-result",
            DeclarationType::Unit,
            DeclarationType::ByteSlice,
        )],
    });
    assert!(matches!(
        reference_result,
        Err(WorkspaceError::UnsupportedEdit { operation, .. })
            if operation.as_ref() == "create-declaration"
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
}

#[test]
fn source_free_generic_declaration_uses_imported_trait_and_explicit_implementation() {
    let source = concat!(
        "trait/\nname/\nmarked\n/name\n/trait\n",
        "product/\nname/\nkeep\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n",
        "impl/\ntrait/\nmarked\n/trait\nfor/\nproduct\nkeep\n/for\n/impl\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n0\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "source-free-explicit-bound.lkjscript",
        WorkspaceNamespace::deterministic(190),
    )
    .expect("import explicit implementation context");
    let named = |kind, suffix: &str| {
        snapshot
            .entities()
            .iter()
            .find(|entity| entity.kind == kind && entity.name.ends_with(suffix))
            .unwrap_or_else(|| panic!("missing {kind:?} ending in {suffix}"))
            .id
    };
    let product = named(EntityKind::Product, ":keep");
    let field = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::ProductField && entity.owner == Some(product))
        .expect("product field")
        .id;
    let trait_entity = named(EntityKind::Trait, ":marked");
    let implementation = named(EntityKind::Implementation, ":keep");
    let main = named(EntityKind::Main, "main");
    let main_root = snapshot
        .nodes()
        .iter()
        .find(|node| node.owner == SemanticOwner::Entity(main))
        .expect("main root")
        .id;
    let mut workspace = Workspace::new(snapshot).expect("workspace");
    let draft_parameter = DraftTypeParameterId::new(0);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateFunction {
                name: "source-marked".to_owned(),
                type_parameters: vec![TypeParameterDraft {
                    id: draft_parameter,
                    name: "t".to_owned(),
                    bounds: vec![SemanticTrait::Entity(trait_entity)],
                }],
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: DeclarationType::DraftTypeParameter(draft_parameter),
                }],
                return_type: DeclarationType::DraftTypeParameter(draft_parameter),
            }],
        })
        .expect("create source-free explicitly bounded function");
    let function = entity_named(&created.snapshot, EntityKind::Function, "source-marked");
    let signature = created
        .snapshot
        .function_signature(created.snapshot.revision(), function)
        .expect("source-free explicit signature");
    let type_parameter = signature.type_parameters[0].id;
    let value_parameter = signature.parameters[0].entity;
    assert_eq!(
        signature.type_parameters[0].bounds[0].trait_identity,
        SemanticTrait::Entity(trait_entity)
    );
    let hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == function)
        .expect("source-free body hole")
        .id;
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![
                Edit::FillHole {
                    hole,
                    draft: ExpressionDraft::new(
                        vec![DraftNode::Load(DraftBindingRef::Entity(value_parameter))],
                        DraftNodeId::new(0),
                    ),
                },
                Edit::ReplaceExpression {
                    target: main_root,
                    draft: ExpressionDraft::new(
                        vec![
                            DraftNode::I64(42),
                            DraftNode::ProductValue {
                                product,
                                fields: vec![DraftFieldValue {
                                    field,
                                    value: DraftNodeId::new(0),
                                }],
                            },
                            DraftNode::Call {
                                callee: function,
                                type_arguments: vec![TypeArgumentDraft {
                                    parameter: type_parameter,
                                    argument: SemanticType::Product(product),
                                }],
                                arguments: vec![DraftNodeId::new(1)],
                            },
                            DraftNode::ProductField {
                                field,
                                value: DraftNodeId::new(2),
                            },
                        ],
                        DraftNodeId::new(3),
                    ),
                },
            ],
        })
        .expect("call source-free explicit-bound function");
    let call = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("source-free explicit call")
        .id;
    let view = completed
        .snapshot
        .call_instantiation(completed.snapshot.revision(), call)
        .expect("explicit witness view");
    assert_eq!(view.witnesses[0].parameter, type_parameter);
    assert_eq!(
        view.witnesses[0].trait_identity,
        SemanticTrait::Entity(trait_entity)
    );
    assert_eq!(
        view.witnesses[0].kind,
        TraitWitnessKindView::Explicit(implementation)
    );
    assert_eq!(run_i64(&completed.snapshot), 42);
}

#[test]
fn source_free_bound_rejection_names_the_created_binder_and_is_atomic() {
    let mut workspace = Workspace::empty_deterministic(197).expect("empty workspace");
    let binder = DraftTypeParameterId::new(0);
    let declarations = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateEnum {
                    name: "choice".to_owned(),
                    variants: vec![EnumVariantDraft {
                        name: "only".to_owned(),
                        fields: Vec::new(),
                    }],
                },
                Edit::CreateFunction {
                    name: "copy-value".to_owned(),
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
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create bounded declaration");
    let function = entity_named(&declarations.snapshot, EntityKind::Function, "copy-value");
    let parameter = declarations
        .snapshot
        .function_signature(declarations.snapshot.revision(), function)
        .expect("bounded signature")
        .type_parameters[0]
        .id;
    let enumeration = entity_named(&declarations.snapshot, EntityKind::Enum, "choice");
    let variant = declarations
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::EnumVariant && entity.owner == Some(enumeration))
        .expect("enum variant")
        .id;
    let main = entity_named(&declarations.snapshot, EntityKind::Main, "main");
    let hole = declarations
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;
    let before = workspace.current();
    let failure = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::FillHole {
            hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::EnumValue {
                        variant,
                        fields: Vec::new(),
                    },
                    DraftNode::Call {
                        callee: function,
                        type_arguments: vec![TypeArgumentDraft {
                            parameter,
                            argument: SemanticType::Enum {
                                constructor: SemanticEnum::Entity(enumeration),
                                arguments: Vec::new(),
                            },
                        }],
                        arguments: vec![DraftNodeId::new(0)],
                    },
                ],
                DraftNodeId::new(1),
            ),
        }],
    });
    assert!(matches!(
        failure,
        Err(WorkspaceError::UnsatisfiedTraitBound {
            parameter: failed,
            trait_identity,
            ..
        }) if *failed == parameter
            && *trait_identity == SemanticTrait::Builtin(BuiltinTrait::Copy)
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
}

#[test]
fn imported_generic_signature_and_explicit_workspace_call_are_exact_and_execute() {
    let source = concat!(
        "def/\nname/\nidentity\n/name\nfn/\nforall/\nt\n/forall\n",
        "sig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "identity/\n41\n/identity\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "generic-explicit-call.lkjscript",
        WorkspaceNamespace::deterministic(201),
    )
    .expect("import generic identity");
    let function = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function && entity.name.ends_with(":identity"))
        .expect("identity function")
        .id;
    let signature = snapshot
        .function_signature(snapshot.revision(), function)
        .expect("generic signature");
    assert_eq!(signature.type_parameters.len(), 1);
    let parameter = signature.type_parameters[0].id;
    assert_eq!(signature.type_parameters[0].owner, function);
    assert_eq!(
        signature.parameters[0].ty,
        SemanticType::TypeParameter(parameter)
    );
    assert_eq!(signature.result, SemanticType::TypeParameter(parameter));
    assert_eq!(
        snapshot.entity(parameter).expect("type parameter").kind,
        EntityKind::TypeParameter
    );

    let call = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("imported call")
        .id;
    let imported_call = snapshot
        .call_instantiation(snapshot.revision(), call)
        .expect("imported instantiation");
    assert_eq!(
        imported_call.type_arguments,
        vec![TypeArgumentView {
            parameter,
            argument: SemanticType::I64,
        }]
    );
    assert_eq!(imported_call.result, SemanticType::I64);

    let mut workspace = Workspace::new(snapshot).expect("generic workspace");
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let edited = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::ReplaceExpression {
                target: call,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(41),
                        DraftNode::Call {
                            callee: function,
                            type_arguments: vec![TypeArgumentDraft {
                                parameter,
                                argument: SemanticType::I64,
                            }],
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("replace with explicit generic call");
    assert_eq!(run_i64(&edited.snapshot), 41);
    let exact = edited
        .snapshot
        .call_instantiation(edited.snapshot.revision(), call)
        .expect("explicit instantiation");
    assert_eq!(exact.revision, edited.snapshot.revision());
    assert_ne!(exact.revision, imported_call.revision);
    assert_eq!(exact.site, imported_call.site);
    assert_eq!(exact.callee, imported_call.callee);
    assert_eq!(exact.type_arguments, imported_call.type_arguments);
    assert_eq!(exact.parameters, imported_call.parameters);
    assert_eq!(exact.result, imported_call.result);
    assert_eq!(exact.witnesses, imported_call.witnesses);
    assert_eq!(exact.effects, imported_call.effects);
    let projection = edited
        .snapshot
        .project(&[ProjectionSlice::Call(call)])
        .expect("call projection");
    assert_eq!(
        projection,
        edited
            .snapshot
            .project(&[ProjectionSlice::Call(call)])
            .expect("deterministic call projection")
    );
    assert!(projection.contains("generic=true"), "{projection}");
    assert!(projection.contains("type-argument"), "{projection}");
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn generic_calls_accept_nested_builtin_enum_types_without_source_round_trips() {
    let source = concat!(
        "def/\nname/\nidentity\n/name\nfn/\nforall/\nt\n/forall\n",
        "sig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\n",
        "def/\nname/\nwrapper\n/name\nfn/\nsig/\ninputs/\noption/\ni64\n/option\n/inputs\n",
        "output/\noption/\ni64\n/option\n/output\n/sig\nparams/\nvalue\noption/\ni64\n/option\n/params\n",
        "identity/\nvalue\n/identity\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\noption/\ni64\n/option\n/output\n/sig\n",
        "wrapper/\nsome/\n1\n/some\n/wrapper\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "generic-builtin-enum.lkjscript",
        WorkspaceNamespace::deterministic(209),
    )
    .expect("import builtin generic call");
    let function = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("identity function")
        .id;
    let parameter = snapshot
        .function_signature(snapshot.revision(), function)
        .expect("identity signature")
        .type_parameters[0]
        .id;
    let wrapper = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function && entity.name.ends_with(":wrapper"))
        .expect("wrapper function")
        .id;
    let wrapper_parameter = snapshot
        .function_signature(snapshot.revision(), wrapper)
        .expect("wrapper signature")
        .parameters[0]
        .entity;
    let call = snapshot
        .calls()
        .iter()
        .find(|call| call.callee == function)
        .expect("generic call")
        .site;
    let option_i64 = SemanticType::Enum {
        constructor: SemanticEnum::Builtin(BuiltinEnum::Option),
        arguments: vec![SemanticType::I64],
    };
    assert_eq!(
        snapshot
            .call_instantiation(snapshot.revision(), call)
            .expect("imported option instantiation")
            .type_arguments,
        vec![TypeArgumentView {
            parameter,
            argument: option_i64.clone(),
        }]
    );

    let mut workspace = Workspace::new(snapshot).expect("builtin generic workspace");
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let replaced = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::ReplaceExpression {
                target: call,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Load(DraftBindingRef::Entity(wrapper_parameter)),
                        DraftNode::Call {
                            callee: function,
                            type_arguments: vec![TypeArgumentDraft {
                                parameter,
                                argument: option_i64.clone(),
                            }],
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("replace builtin generic call");
    let exact = replaced
        .snapshot
        .call_instantiation(replaced.snapshot.revision(), call)
        .expect("source-free option instantiation");
    assert_eq!(exact.parameters, vec![option_i64.clone()]);
    assert_eq!(exact.result, option_i64);
    assert!(exact.effects.is_known());
    assert!(exact.effects.is_pure());
    let projection = replaced
        .snapshot
        .project(&[ProjectionSlice::Call(call)])
        .expect("builtin call projection");
    assert!(projection.contains("effects=[pure]"), "{projection}");
    crate::compile_snapshot(&replaced.snapshot).expect("compile builtin generic call");
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn every_builtin_enum_identity_round_trips_in_nested_semantic_types() {
    let option = SemanticType::Enum {
        constructor: SemanticEnum::Builtin(BuiltinEnum::Option),
        arguments: vec![SemanticType::List(Box::new(SemanticType::I64))],
    };
    let system_error = SemanticType::Enum {
        constructor: SemanticEnum::Builtin(BuiltinEnum::SystemError),
        arguments: Vec::new(),
    };
    let result = SemanticType::Enum {
        constructor: SemanticEnum::Builtin(BuiltinEnum::Result),
        arguments: vec![option, system_error.clone()],
    };
    let numeric_error = SemanticType::Enum {
        constructor: SemanticEnum::Builtin(BuiltinEnum::NumericError),
        arguments: Vec::new(),
    };
    let utf8_error = SemanticType::Enum {
        constructor: SemanticEnum::Builtin(BuiltinEnum::Utf8Error),
        arguments: Vec::new(),
    };
    let expected = vec![result, numeric_error, utf8_error, system_error];
    let mut workspace = Workspace::empty_deterministic(210).expect("builtin type workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateFunction {
                name: "builtins".to_owned(),
                type_parameters: Vec::new(),
                parameters: expected
                    .iter()
                    .enumerate()
                    .map(|(index, ty)| ParameterDraft {
                        name: format!("value-{index}"),
                        ty: DeclarationType::try_from(ty).expect("declaration type"),
                    })
                    .collect(),
                return_type: DeclarationType::Unit,
            }],
        })
        .expect("create builtin signature");
    let function = entity_named(&created.snapshot, EntityKind::Function, "builtins");
    let signature = created
        .snapshot
        .function_signature(created.snapshot.revision(), function)
        .expect("query builtin signature");
    assert_eq!(
        signature
            .parameters
            .iter()
            .map(|parameter| parameter.ty.clone())
            .collect::<Vec<_>>(),
        expected
    );
    created
        .snapshot
        .check_consistency()
        .expect("validate builtin signature");
}

#[test]
fn generic_call_effects_are_exact_machine_readable_and_projected_by_name() {
    let source = concat!(
        "def/\nname/\nchecked\n/name\nfn/\nforall/\nt\n/forall\n",
        "sig/\ninputs/\nt\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\ndivide/\n8\n2\n/divide\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "checked/\n1\n/checked\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "generic-call-effects.lkjscript",
        WorkspaceNamespace::deterministic(211),
    )
    .expect("import effectful generic call");
    let call = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("generic call")
        .id;
    let exact = snapshot
        .call_instantiation(snapshot.revision(), call)
        .expect("effectful instantiation");
    assert!(exact.effects.is_known());
    assert!(!exact.effects.is_pure());
    assert!(exact.effects.contains(EffectSummary::MAY_TRAP));
    assert!(!exact.effects.contains(EffectSummary::HOST_IO));
    let projection = snapshot
        .project(&[ProjectionSlice::Call(call)])
        .expect("effectful call projection");
    assert!(projection.contains("effects=[may-trap]"), "{projection}");
    assert_eq!(run_i64(&snapshot), 4);
}

#[test]
fn explicit_generic_calls_derive_auto_witnesses_and_fail_atomically_when_unsatisfied() {
    let source = concat!(
        "enum/\nname/\nchoice\n/name\nvariants/\nvariant/\nname/\none\n/name\n",
        "fields/\n/fields\n/variant\n/variants\n/enum\n",
        "def/\nname/\ncopy-value\n/name\nfn/\nforall/\nt\n/forall\n",
        "bounds/\nbound/\nt\ncopy\n/bound\n/bounds\n",
        "sig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "copy-value/\n1\n/copy-value\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "generic-auto-witness.lkjscript",
        WorkspaceNamespace::deterministic(203),
    )
    .expect("import Copy-bound call");
    let function = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function && entity.name.ends_with(":copy-value"))
        .expect("copy function")
        .id;
    let signature = snapshot
        .function_signature(snapshot.revision(), function)
        .expect("copy signature");
    let parameter = signature.type_parameters[0].id;
    assert_eq!(
        signature.type_parameters[0].bounds[0].trait_identity,
        SemanticTrait::Builtin(BuiltinTrait::Copy)
    );
    let call = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("copy call")
        .id;
    let queried = snapshot
        .call_instantiation(snapshot.revision(), call)
        .expect("copy instantiation");
    assert_eq!(queried.witnesses.len(), 1);
    assert_eq!(queried.witnesses[0].parameter, parameter);
    assert_eq!(queried.witnesses[0].kind, TraitWitnessKindView::AutoTrait);

    let enumeration = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Enum && entity.name.ends_with(":choice"))
        .expect("choice enum")
        .id;
    let variant = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::EnumVariant && entity.owner == Some(enumeration))
        .expect("choice variant")
        .id;
    let mut workspace = Workspace::new(snapshot).expect("copy workspace");
    let edited = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::ReplaceExpression {
                target: call,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(2),
                        DraftNode::Call {
                            callee: function,
                            type_arguments: vec![TypeArgumentDraft {
                                parameter,
                                argument: SemanticType::I64,
                            }],
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("publish explicit auto-witness call");
    let edited_call = edited
        .snapshot
        .call_instantiation(edited.snapshot.revision(), call)
        .expect("query edited auto witness");
    assert_eq!(edited_call.witnesses, queried.witnesses);
    assert_eq!(run_i64(&edited.snapshot), 2);
    let before = workspace.current();
    let error = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: call,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::EnumValue {
                            variant,
                            fields: Vec::new(),
                        },
                        DraftNode::Call {
                            callee: function,
                            type_arguments: vec![TypeArgumentDraft {
                                parameter,
                                argument: SemanticType::Enum {
                                    constructor: SemanticEnum::Entity(enumeration),
                                    arguments: Vec::new(),
                                },
                            }],
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect_err("user enum does not satisfy Copy");
    assert!(matches!(
        error,
        WorkspaceError::UnsatisfiedTraitBound {
            parameter: failed,
            argument,
            ..
        } if failed.as_ref() == &parameter && matches!(argument.as_ref(), SemanticType::Enum { .. })
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    assert_eq!(run_i64(&before), 2);
}

#[test]
fn unsatisfied_bound_reports_the_exact_binder_when_traits_repeat() {
    let source = concat!(
        "enum/\nname/\nchoice\n/name\nvariants/\nvariant/\nname/\none\n/name\n",
        "fields/\n/fields\n/variant\n/variants\n/enum\n",
        "def/\nname/\nfirst\n/name\nfn/\nforall/\nt\nu\n/forall\n",
        "bounds/\nbound/\nt\ncopy\n/bound\nbound/\nu\ncopy\n/bound\n/bounds\n",
        "sig/\ninputs/\nt\nu\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nleft\nt\nright\nu\n/params\nleft\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "first/\n1\n2\n/first\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "generic-repeated-bound.lkjscript",
        WorkspaceNamespace::deterministic(212),
    )
    .expect("import repeated-bound call");
    let function = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("generic function")
        .id;
    let parameters = snapshot
        .function_signature(snapshot.revision(), function)
        .expect("generic signature")
        .type_parameters;
    assert_eq!(parameters.len(), 2);
    let enumeration = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Enum)
        .expect("choice enum")
        .id;
    let variant = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::EnumVariant)
        .expect("choice variant")
        .id;
    let call = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("generic call")
        .id;
    let mut workspace = Workspace::new(snapshot).expect("repeated-bound workspace");
    let edited = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::ReplaceExpression {
                target: call,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(3),
                        DraftNode::I64(4),
                        DraftNode::Call {
                            callee: function,
                            type_arguments: vec![
                                TypeArgumentDraft {
                                    parameter: parameters[1].id,
                                    argument: SemanticType::I64,
                                },
                                TypeArgumentDraft {
                                    parameter: parameters[0].id,
                                    argument: SemanticType::I64,
                                },
                            ],
                            arguments: vec![DraftNodeId::new(0), DraftNodeId::new(1)],
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("publish repeated auto-trait witnesses");
    let exact = edited
        .snapshot
        .call_instantiation(edited.snapshot.revision(), call)
        .expect("repeated witness call");
    assert_eq!(
        exact
            .type_arguments
            .iter()
            .map(|argument| argument.parameter)
            .collect::<Vec<_>>(),
        vec![parameters[0].id, parameters[1].id]
    );
    assert_eq!(exact.witnesses.len(), 2);
    assert!(exact
        .witnesses
        .iter()
        .all(|witness| witness.kind == TraitWitnessKindView::AutoTrait));
    assert_eq!(run_i64(&edited.snapshot), 3);
    let before = workspace.current();
    let error = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::ReplaceExpression {
                target: call,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(1),
                        DraftNode::EnumValue {
                            variant,
                            fields: Vec::new(),
                        },
                        DraftNode::Call {
                            callee: function,
                            type_arguments: vec![
                                TypeArgumentDraft {
                                    parameter: parameters[0].id,
                                    argument: SemanticType::I64,
                                },
                                TypeArgumentDraft {
                                    parameter: parameters[1].id,
                                    argument: SemanticType::Enum {
                                        constructor: SemanticEnum::Entity(enumeration),
                                        arguments: Vec::new(),
                                    },
                                },
                            ],
                            arguments: vec![DraftNodeId::new(0), DraftNodeId::new(1)],
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect_err("second Copy bound must fail");
    assert!(matches!(
        error,
        WorkspaceError::UnsatisfiedTraitBound { parameter, .. }
            if parameter.as_ref() == &parameters[1].id
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    assert_eq!(run_i64(&before), 3);
}

#[test]
fn hir_consistency_rejects_malformed_generic_binders_bounds_and_substitutions() {
    let source = concat!(
        "def/\nname/\ncopy-value\n/name\nfn/\nforall/\nt\n/forall\n",
        "bounds/\nbound/\nt\ncopy\n/bound\n/bounds\n",
        "sig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "copy-value/\n1\n/copy-value\n/main\n",
    );
    let snapshot =
        import_source(source, "malformed-generic-hir.lkjscript").expect("import valid generic HIR");
    let valid = snapshot.validated_complete_hir().expect("validated HIR");

    let mut unknown_parameter = valid.clone();
    unknown_parameter.functions[0].bounds[0].parameter = "missing".to_owned();
    assert!(super::validate::program(&unknown_parameter)
        .expect_err("bound parameter must be declared")
        .to_string()
        .contains("undeclared type parameter"));

    let mut duplicate_bound = valid.clone();
    let repeated_bound = duplicate_bound.functions[0].bounds[0].clone();
    duplicate_bound.functions[0].bounds.push(repeated_bound);
    assert!(super::validate::program(&duplicate_bound)
        .expect_err("duplicate bound must reject")
        .to_string()
        .contains("duplicated"));

    let mut stale_trait = valid.clone();
    stale_trait.functions[0].bounds[0].trait_id = crate::hir::TraitId::new(u64::MAX);
    assert!(super::validate::program(&stale_trait)
        .expect_err("stale trait identity must reject")
        .to_string()
        .contains("trait bound identity is stale"));

    let mut duplicate_binder = valid.clone();
    let binding = duplicate_binder.functions[0]
        .binding
        .index()
        .expect("binding index");
    let crate::Type::Forall { vars, .. } = &mut duplicate_binder.bindings[binding].ty else {
        panic!("generic function binding")
    };
    vars.push(vars[0].clone());
    assert!(super::validate::program(&duplicate_binder)
        .expect_err("duplicate binder must reject")
        .to_string()
        .contains("type parameter is duplicated"));

    let mut forwarded = valid;
    let crate::hir::ExprKind::Call {
        instantiation: Some(instantiation),
        ..
    } = &mut forwarded.main.body.kind
    else {
        panic!("generic main call")
    };
    instantiation.substitutions[0].ty = crate::Type::Param("t".to_owned());
    assert!(super::validate::program(&forwarded)
        .expect_err("unresolved substitution must reject")
        .to_string()
        .contains("current transport route"));
}

#[test]
fn type_argument_only_replacement_has_an_exact_semantic_diff() {
    let source = concat!(
        "def/\nname/\ndiscard\n/name\nfn/\nforall/\nt\n/forall\n",
        "sig/\ninputs/\nt\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\n7\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "discard/\n1\n/discard\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "generic-call-diff.lkjscript",
        WorkspaceNamespace::deterministic(202),
    )
    .expect("import generic discard");
    let function = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function && entity.name.ends_with(":discard"))
        .expect("discard function")
        .id;
    let parameter = snapshot
        .function_signature(snapshot.revision(), function)
        .expect("discard signature")
        .type_parameters[0]
        .id;
    let call = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("discard call")
        .id;
    let mut workspace = Workspace::new(snapshot).expect("discard workspace");
    let changed = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::ReplaceExpression {
                target: call,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Bool(true),
                        DraftNode::Call {
                            callee: function,
                            type_arguments: vec![TypeArgumentDraft {
                                parameter,
                                argument: SemanticType::Bool,
                            }],
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("change only generic instantiation semantics");
    assert_eq!(run_i64(&changed.snapshot), 7);
    let entry = changed
        .diff
        .entries
        .iter()
        .find_map(|entry| match entry {
            SemanticDiffEntry::CallInstantiationChanged { old, new, .. } => Some((old, new)),
            _ => None,
        })
        .expect("call instantiation diff");
    assert_eq!(entry.0.type_arguments[0].argument, SemanticType::I64);
    assert_eq!(entry.1.type_arguments[0].argument, SemanticType::Bool);
}

#[test]
fn generic_call_identity_and_substitution_failures_are_structured_and_atomic() {
    let source = concat!(
        "def/\nname/\nidentity\n/name\nfn/\nforall/\nt\n/forall\n",
        "sig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\n",
        "def/\nname/\nother\n/name\nfn/\nforall/\nu\n/forall\n",
        "sig/\ninputs/\nu\n/inputs\noutput/\nu\n/output\n/sig\n",
        "params/\nvalue\nu\n/params\nvalue\n/fn\n/def\n",
        "def/\nname/\nplain\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\n",
        "output/\ni64\n/output\n/sig\nparams/\nvalue\ni64\n/params\nvalue\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "identity/\n1\n/identity\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "generic-call-errors.lkjscript",
        WorkspaceNamespace::deterministic(204),
    )
    .expect("import generic call fixture");
    let function = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function && entity.name.ends_with(":identity"))
        .expect("identity function")
        .id;
    let other = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function && entity.name.ends_with(":other"))
        .expect("other function")
        .id;
    let plain = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function && entity.name.ends_with(":plain"))
        .expect("plain function")
        .id;
    let parameter = snapshot
        .function_signature(snapshot.revision(), function)
        .expect("identity signature")
        .type_parameters[0]
        .id;
    let other_parameter = snapshot
        .function_signature(snapshot.revision(), other)
        .expect("other signature")
        .type_parameters[0]
        .id;
    let call = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("generic call")
        .id;
    let foreign = importer::import_source_with_namespace(
        source,
        "foreign-generic-call-errors.lkjscript",
        WorkspaceNamespace::deterministic(205),
    )
    .expect("import foreign fixture");
    let foreign_parameter = foreign
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::TypeParameter)
        .expect("foreign parameter")
        .id;
    let stale = EntityId::new(snapshot.namespace(), u64::MAX, 1);
    let mut workspace = Workspace::new(snapshot).expect("generic error workspace");
    let before = workspace.current();
    {
        let mut replace = |type_arguments: Vec<TypeArgumentDraft>, argument: DraftNode| {
            workspace.apply(Transaction {
                base_revision: before.revision(),
                edits: vec![Edit::ReplaceExpression {
                    target: call,
                    draft: ExpressionDraft::new(
                        vec![
                            argument,
                            DraftNode::Call {
                                callee: function,
                                type_arguments,
                                arguments: vec![DraftNodeId::new(0)],
                            },
                        ],
                        DraftNodeId::new(1),
                    ),
                }],
            })
        };

        assert!(matches!(
            replace(Vec::new(), DraftNode::I64(1)),
            Err(WorkspaceError::MissingTypeArgument { parameter: missing }) if missing == parameter
        ));
        assert!(matches!(
            replace(
                vec![
                    TypeArgumentDraft {
                        parameter,
                        argument: SemanticType::I64,
                    },
                    TypeArgumentDraft {
                        parameter,
                        argument: SemanticType::I64,
                    },
                ],
                DraftNode::I64(1),
            ),
            Err(WorkspaceError::DuplicateTypeArgument { parameter: duplicate }) if duplicate == parameter
        ));
        assert!(matches!(
            replace(
                vec![TypeArgumentDraft {
                    parameter: other_parameter,
                    argument: SemanticType::I64,
                }],
                DraftNode::I64(1),
            ),
            Err(WorkspaceError::WrongTypeParameterOwner { parameter: failed, expected, .. })
                if failed.as_ref() == &other_parameter && expected.as_ref() == &function
        ));
        assert!(matches!(
            replace(
                vec![TypeArgumentDraft {
                    parameter: foreign_parameter,
                    argument: SemanticType::I64,
                }],
                DraftNode::I64(1),
            ),
            Err(WorkspaceError::ForeignNamespace(_))
        ));
        assert!(matches!(
            replace(
                vec![TypeArgumentDraft {
                    parameter: function,
                    argument: SemanticType::I64,
                }],
                DraftNode::I64(1),
            ),
            Err(WorkspaceError::WrongEntityKind { .. })
        ));
        assert!(matches!(
            replace(
                vec![TypeArgumentDraft {
                    parameter: stale,
                    argument: SemanticType::I64,
                }],
                DraftNode::I64(1),
            ),
            Err(WorkspaceError::StaleIdentity(_))
        ));
        assert!(matches!(
            replace(
                vec![TypeArgumentDraft {
                    parameter,
                    argument: SemanticType::Bool,
                }],
                DraftNode::I64(1),
            ),
            Err(WorkspaceError::TypeMismatch { expected, actual })
                if expected.as_ref() == &SemanticType::Bool && actual.as_ref() == &SemanticType::I64
        ));
        assert!(matches!(
            replace(
                vec![TypeArgumentDraft {
                    parameter,
                    argument: SemanticType::TypeParameter(parameter),
                }],
                DraftNode::I64(1),
            ),
            Err(WorkspaceError::WrongTypeParameterOwner { .. })
        ));
    }
    assert!(matches!(
        workspace.apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: call,
                draft: ExpressionDraft::new(
                    vec![DraftNode::Call {
                        callee: function,
                        type_arguments: vec![TypeArgumentDraft {
                            parameter,
                            argument: SemanticType::I64,
                        }],
                        arguments: Vec::new(),
                    }],
                    DraftNodeId::new(0),
                ),
            }],
        }),
        Err(WorkspaceError::CallArity {
            expected: 1,
            actual: 0,
            ..
        })
    ));
    assert!(matches!(
        workspace.apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: call,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(1),
                        DraftNode::Call {
                            callee: plain,
                            type_arguments: vec![TypeArgumentDraft {
                                parameter,
                                argument: SemanticType::I64,
                            }],
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        }),
        Err(WorkspaceError::UnexpectedTypeArgument)
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    assert_eq!(run_i64(&before), 1);
}

#[test]
fn generic_binder_identities_survive_compaction_and_follow_function_lifecycle() {
    let source = concat!(
        "def/\nname/\nremove\n/name\nfn/\nforall/\nt\n/forall\n",
        "sig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\n",
        "def/\nname/\nkeep\n/name\nfn/\nforall/\nu\n/forall\n",
        "sig/\ninputs/\nu\n/inputs\noutput/\nu\n/output\n/sig\n",
        "params/\nvalue\nu\n/params\nvalue\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "keep/\n8\n/keep\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "generic-binder-lifecycle.lkjscript",
        WorkspaceNamespace::deterministic(208),
    )
    .expect("import generic lifecycle fixture");
    let named_function = |name: &str| {
        snapshot
            .entities()
            .iter()
            .find(|entity| {
                entity.kind == EntityKind::Function && entity.name.ends_with(&format!(":{name}"))
            })
            .expect("named generic function")
            .id
    };
    let remove = named_function("remove");
    let keep = named_function("keep");
    let removed_parameter = snapshot
        .function_signature(snapshot.revision(), remove)
        .expect("removed signature")
        .type_parameters[0]
        .id;
    let kept_parameter = snapshot
        .function_signature(snapshot.revision(), keep)
        .expect("kept signature")
        .type_parameters[0]
        .id;
    let main = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Main)
        .expect("main")
        .id;
    let call = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("retained generic call")
        .id;
    let old = snapshot.clone();
    let mut workspace = Workspace::new(snapshot).expect("generic lifecycle workspace");
    let compacted = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::DeleteEntity { entity: remove }],
        })
        .expect("delete earlier generic function");
    assert!(compacted.snapshot.entity(remove).is_err());
    assert!(compacted.snapshot.entity(removed_parameter).is_err());
    assert_eq!(
        compacted.snapshot.entity(keep).expect("kept function").id,
        keep
    );
    assert_eq!(
        compacted
            .snapshot
            .entity(kept_parameter)
            .expect("kept type parameter")
            .id,
        kept_parameter
    );
    assert_eq!(
        compacted
            .snapshot
            .function_signature(compacted.snapshot.revision(), keep)
            .expect("compacted signature")
            .type_parameters[0]
            .id,
        kept_parameter
    );
    assert_eq!(
        compacted
            .snapshot
            .call_instantiation(compacted.snapshot.revision(), call)
            .expect("compacted instantiation")
            .type_arguments[0]
            .parameter,
        kept_parameter
    );
    assert_eq!(run_i64(&compacted.snapshot), 8);
    assert_eq!(run_i64(&old), 8);

    let before_block = workspace.current();
    assert!(workspace
        .apply(Transaction {
            base_revision: before_block.revision(),
            edits: vec![Edit::DeleteEntity { entity: keep }],
        })
        .is_err());
    assert!(Arc::ptr_eq(&before_block, &workspace.current()));
    let deleted = workspace
        .apply(Transaction {
            base_revision: before_block.revision(),
            edits: vec![
                Edit::DeleteEntity { entity: keep },
                Edit::DeleteEntity { entity: main },
            ],
        })
        .expect("delete generic function with dependent entry point");
    assert!(deleted.snapshot.entity(keep).is_err());
    assert!(deleted.snapshot.entity(kept_parameter).is_err());
    assert!(deleted.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::EntityDeleted { entity, .. } if *entity == kept_parameter
    )));
    assert_eq!(
        deleted.snapshot.completeness_blockers(),
        &[CompletenessBlocker::MissingEntryPoint]
    );
    assert_eq!(run_i64(&old), 8);
}

#[test]
fn source_free_generic_binders_survive_rename_compaction_and_follow_deletion() {
    let mut workspace = Workspace::empty_deterministic(209).expect("empty workspace");
    let binder = DraftTypeParameterId::new(0);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "remove".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::I64,
                },
                Edit::CreateFunction {
                    name: "keep".to_owned(),
                    type_parameters: vec![TypeParameterDraft {
                        id: binder,
                        name: "t".to_owned(),
                        bounds: Vec::new(),
                    }],
                    parameters: vec![ParameterDraft {
                        name: "value".to_owned(),
                        ty: DeclarationType::DraftTypeParameter(binder),
                    }],
                    return_type: DeclarationType::DraftTypeParameter(binder),
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create lifecycle declarations");
    let remove = entity_named(&created.snapshot, EntityKind::Function, "remove");
    let keep = entity_named(&created.snapshot, EntityKind::Function, "keep");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let signature = created
        .snapshot
        .function_signature(created.snapshot.revision(), keep)
        .expect("generic signature");
    let type_parameter = signature.type_parameters[0].id;
    let value = signature.parameters[0].entity;
    let hole_for = |owner| {
        created
            .snapshot
            .holes()
            .find(|hole| hole.owner == owner)
            .expect("owned hole")
            .id
    };
    let remove_hole = hole_for(remove);
    let keep_hole = hole_for(keep);
    let main_hole = hole_for(main);
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![
                Edit::FillHole {
                    hole: remove_hole,
                    draft: ExpressionDraft::scalar_i64(0),
                },
                Edit::FillHole {
                    hole: keep_hole,
                    draft: ExpressionDraft::new(
                        vec![DraftNode::Load(DraftBindingRef::Entity(value))],
                        DraftNodeId::new(0),
                    ),
                },
                Edit::FillHole {
                    hole: main_hole,
                    draft: ExpressionDraft::new(
                        vec![
                            DraftNode::I64(42),
                            DraftNode::Call {
                                callee: keep,
                                type_arguments: vec![TypeArgumentDraft {
                                    parameter: type_parameter,
                                    argument: SemanticType::I64,
                                }],
                                arguments: vec![DraftNodeId::new(0)],
                            },
                        ],
                        DraftNodeId::new(1),
                    ),
                },
            ],
        })
        .expect("complete lifecycle fixture");
    let call = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("generic call")
        .id;
    assert_eq!(run_i64(&completed.snapshot), 42);

    let renamed = workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![Edit::RenameEntity {
                entity: keep,
                new_name: "kept".to_owned(),
            }],
        })
        .expect("rename generic function");
    assert_eq!(
        renamed
            .snapshot
            .function_signature(renamed.snapshot.revision(), keep)
            .expect("renamed signature")
            .type_parameters[0]
            .id,
        type_parameter
    );
    assert_eq!(run_i64(&renamed.snapshot), 42);

    let old = renamed.snapshot.clone();
    let compacted = workspace
        .apply(Transaction {
            base_revision: renamed.snapshot.revision(),
            edits: vec![Edit::DeleteEntity { entity: remove }],
        })
        .expect("compact earlier function");
    assert_eq!(
        compacted.snapshot.entity(keep).expect("kept function").id,
        keep
    );
    assert_eq!(
        compacted
            .snapshot
            .entity(type_parameter)
            .expect("kept binder")
            .id,
        type_parameter
    );
    assert_eq!(
        compacted
            .snapshot
            .call_instantiation(compacted.snapshot.revision(), call)
            .expect("kept call")
            .type_arguments[0]
            .parameter,
        type_parameter
    );
    assert_eq!(run_i64(&compacted.snapshot), 42);

    let before_failure = workspace.current();
    assert!(workspace
        .apply(Transaction {
            base_revision: before_failure.revision(),
            edits: vec![Edit::DeleteEntity { entity: keep }],
        })
        .is_err());
    assert!(Arc::ptr_eq(&before_failure, &workspace.current()));
    let deleted = workspace
        .apply(Transaction {
            base_revision: before_failure.revision(),
            edits: vec![
                Edit::DeleteEntity { entity: main },
                Edit::DeleteEntity { entity: keep },
            ],
        })
        .expect("delete dependency-closed generic function");
    assert!(deleted.snapshot.entity(type_parameter).is_err());
    assert_eq!(
        old.function_signature(old.revision(), keep)
            .expect("old generic signature")
            .type_parameters[0]
            .id,
        type_parameter
    );

    let recreated = workspace
        .apply(Transaction {
            base_revision: deleted.snapshot.revision(),
            edits: vec![Edit::CreateFunction {
                name: "kept".to_owned(),
                type_parameters: vec![TypeParameterDraft {
                    id: binder,
                    name: "t".to_owned(),
                    bounds: Vec::new(),
                }],
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: DeclarationType::DraftTypeParameter(binder),
                }],
                return_type: DeclarationType::DraftTypeParameter(binder),
            }],
        })
        .expect("recreate generic function");
    let recreated_function = entity_named(&recreated.snapshot, EntityKind::Function, "kept");
    let recreated_parameter = recreated
        .snapshot
        .function_signature(recreated.snapshot.revision(), recreated_function)
        .expect("recreated signature")
        .type_parameters[0]
        .id;
    assert_ne!(recreated_function, keep);
    assert_ne!(recreated_parameter, type_parameter);
}

#[test]
fn imported_and_source_free_programs_converge_semantically() {
    let imported = import_source(FUNCTION_PROGRAM_42, "workspace-convergence.lkjscript")
        .expect("import equivalent program");
    let (mut workspace, function, parameter, _main, function_hole, main_hole) =
        create_source_free_declarations(31);
    let source_free = fill_source_free_identity(
        &mut workspace,
        function,
        parameter,
        function_hole,
        main_hole,
    );

    let observations = |snapshot: &WorkspaceSnapshot| {
        let mut entities = snapshot
            .entities()
            .iter()
            .map(|entity| {
                (
                    entity.kind,
                    entity
                        .name
                        .rsplit(':')
                        .next()
                        .unwrap_or(&entity.name)
                        .to_owned(),
                )
            })
            .collect::<Vec<_>>();
        entities.sort();
        let node_kinds = snapshot
            .nodes()
            .iter()
            .map(|node| {
                (
                    node.kind,
                    snapshot
                        .node_type(snapshot.revision(), node.id)
                        .expect("node type")
                        .actual
                        .to_string(),
                )
            })
            .collect::<Vec<_>>();
        let function_signatures = snapshot
            .program
            .functions
            .iter()
            .map(|function| {
                snapshot
                    .program
                    .binding(function.binding)
                    .expect("function binding")
                    .ty
                    .clone()
            })
            .collect::<Vec<_>>();
        (
            entities,
            function_signatures,
            snapshot
                .program
                .main
                .as_ref()
                .expect("complete main")
                .return_type
                .clone(),
            node_kinds,
            snapshot.calls().len(),
            snapshot.references().len(),
            snapshot.diagnostics().to_vec(),
        )
    };
    assert_eq!(observations(&imported), observations(&source_free));
    assert_eq!(run_i64(&imported), 42);
    assert_eq!(run_i64(&source_free), 42);
}

#[test]
fn imported_and_source_free_generic_declarations_converge_through_bytecode_and_vm() {
    let source = concat!(
        "def/\nname/\nidentity\n/name\nfn/\nforall/\nt\n/forall\n",
        "bounds/\nbound/\nt\ncopy\n/bound\n/bounds\n",
        "sig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "identity/\n42\n/identity\n/main\n",
    );
    let imported = importer::import_source_with_namespace(
        source,
        "generic-declaration-convergence.lkjscript",
        WorkspaceNamespace::deterministic(196),
    )
    .expect("import generic declaration");

    let mut workspace = Workspace::empty_deterministic(195).expect("empty workspace");
    let draft_parameter = DraftTypeParameterId::new(0);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "identity".to_owned(),
                    type_parameters: vec![TypeParameterDraft {
                        id: draft_parameter,
                        name: "t".to_owned(),
                        bounds: vec![SemanticTrait::Builtin(BuiltinTrait::Copy)],
                    }],
                    parameters: vec![ParameterDraft {
                        name: "value".to_owned(),
                        ty: DeclarationType::DraftTypeParameter(draft_parameter),
                    }],
                    return_type: DeclarationType::DraftTypeParameter(draft_parameter),
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create generic declaration");
    let function = entity_named(&created.snapshot, EntityKind::Function, "identity");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let signature = created
        .snapshot
        .function_signature(created.snapshot.revision(), function)
        .expect("source-free signature");
    let type_parameter = signature.type_parameters[0].id;
    let value = signature.parameters[0].entity;
    let function_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == function)
        .expect("function hole")
        .id;
    let main_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;
    let complete = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![
                Edit::FillHole {
                    hole: function_hole,
                    draft: ExpressionDraft::new(
                        vec![DraftNode::Load(DraftBindingRef::Entity(value))],
                        DraftNodeId::new(0),
                    ),
                },
                Edit::FillHole {
                    hole: main_hole,
                    draft: ExpressionDraft::new(
                        vec![
                            DraftNode::I64(42),
                            DraftNode::Call {
                                callee: function,
                                type_arguments: vec![TypeArgumentDraft {
                                    parameter: type_parameter,
                                    argument: SemanticType::I64,
                                }],
                                arguments: vec![DraftNodeId::new(0)],
                            },
                        ],
                        DraftNodeId::new(1),
                    ),
                },
            ],
        })
        .expect("complete source-free generic program");

    let imported_function = imported
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function && entity.name.ends_with(":identity"))
        .expect("imported identity")
        .id;
    let imported_signature = imported
        .function_signature(imported.revision(), imported_function)
        .expect("imported signature");
    assert_eq!(
        imported_signature.type_parameters.len(),
        signature.type_parameters.len()
    );
    assert_eq!(
        imported_signature.type_parameters[0].name,
        signature.type_parameters[0].name
    );
    assert_eq!(
        imported_signature.type_parameters[0].bounds[0].trait_identity,
        signature.type_parameters[0].bounds[0].trait_identity
    );
    assert!(matches!(
        imported_signature.parameters[0].ty,
        SemanticType::TypeParameter(id) if id == imported_signature.type_parameters[0].id
    ));
    assert!(matches!(
        signature.parameters[0].ty,
        SemanticType::TypeParameter(id) if id == type_parameter
    ));

    let imported_call = imported
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("imported call")
        .id;
    let source_free_call = complete
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("source-free call")
        .id;
    let imported_view = imported
        .call_instantiation(imported.revision(), imported_call)
        .expect("imported call view");
    let source_free_view = complete
        .snapshot
        .call_instantiation(complete.snapshot.revision(), source_free_call)
        .expect("source-free call view");
    assert_eq!(
        imported_view.type_arguments[0].argument,
        source_free_view.type_arguments[0].argument
    );
    assert_eq!(imported_view.parameters, source_free_view.parameters);
    assert_eq!(imported_view.result, source_free_view.result);
    assert_eq!(
        imported_view.witnesses[0].trait_identity,
        source_free_view.witnesses[0].trait_identity
    );
    assert_eq!(
        imported_view.witnesses[0].kind,
        source_free_view.witnesses[0].kind
    );
    assert_eq!(imported_view.effects, source_free_view.effects);
    assert_eq!(imported.calls().len(), complete.snapshot.calls().len());
    assert_eq!(
        imported.dependencies().len(),
        complete.snapshot.dependencies().len()
    );

    let imported_executable = crate::compile_snapshot(&imported).expect("compile imported generic");
    let source_free_executable =
        crate::compile_snapshot(&complete.snapshot).expect("compile source-free generic");
    assert_eq!(
        imported_executable.bytecode().main().code,
        source_free_executable.bytecode().main().code
    );
    assert_eq!(imported_executable.bytecode().protos().len(), 1);
    assert_eq!(source_free_executable.bytecode().protos().len(), 1);
    assert_eq!(
        imported_executable.bytecode().protos()[0].code,
        source_free_executable.bytecode().protos()[0].code
    );
    assert_eq!(run_i64(&imported), 42);
    assert_eq!(run_i64(&complete.snapshot), 42);
}

#[test]
fn failed_source_free_creation_and_drafts_are_atomic_and_ids_are_retry_stable() {
    crate::source::reset_parser_invocation_count();
    let mut workspace = Workspace::empty_deterministic(32).expect("empty workspace");
    let before = workspace.current();
    let before_projection = before.project(&[]).expect("projection");
    let failure = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![
            Edit::CreateFunction {
                name: "identity".to_owned(),
                type_parameters: Vec::new(),
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: DeclarationType::I64,
                }],
                return_type: DeclarationType::I64,
            },
            Edit::CreateMain {
                return_type: SemanticType::I64,
            },
            Edit::CreateMain {
                return_type: SemanticType::I64,
            },
        ],
    });
    assert!(matches!(
        failure,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    assert_eq!(
        workspace.current().project(&[]).expect("projection"),
        before_projection
    );
    assert!(workspace.current().entities().is_empty());
    assert!(workspace.current().nodes().is_empty());

    let (control, ..) = create_source_free_declarations(32);
    let retry = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "identity".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "value".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("retry creation");
    assert_eq!(retry.snapshot.entities(), control.current().entities());
    assert_eq!(retry.snapshot.nodes(), control.current().nodes());

    let function = retry
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("function")
        .id;
    let parameter = retry
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Parameter)
        .expect("parameter")
        .id;
    let main = retry
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Main)
        .expect("main")
        .id;
    let main_hole = retry
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;
    let published = workspace.current();
    let duplicate_function = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::CreateFunction {
            name: "identity".to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::I64,
        }],
    });
    assert!(matches!(
        duplicate_function,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let reserved_function = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::CreateFunction {
            name: "main".to_owned(),
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            return_type: DeclarationType::I64,
        }],
    });
    assert!(matches!(
        reserved_function,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let unsupported_signature = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::CreateFunction {
            name: "borrowed".to_owned(),
            type_parameters: Vec::new(),
            parameters: vec![ParameterDraft {
                name: "value".to_owned(),
                ty: DeclarationType::ByteVector,
            }],
            return_type: DeclarationType::ByteSlice,
        }],
    });
    assert!(matches!(
        unsupported_signature,
        Err(WorkspaceError::UnsupportedEdit { .. })
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let duplicate_parameters = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::CreateFunction {
            name: "other".to_owned(),
            type_parameters: Vec::new(),
            parameters: vec![
                ParameterDraft {
                    name: "value".to_owned(),
                    ty: DeclarationType::I64,
                },
                ParameterDraft {
                    name: "value".to_owned(),
                    ty: DeclarationType::I64,
                },
            ],
            return_type: DeclarationType::I64,
        }],
    });
    assert!(matches!(
        duplicate_parameters,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let invisible = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole: main_hole,
            draft: ExpressionDraft::new(
                vec![DraftNode::Load(DraftBindingRef::Entity(parameter))],
                DraftNodeId::new(0),
            ),
        }],
    });
    assert!(matches!(
        invisible,
        Err(WorkspaceError::InvisibleEntity {
            operation,
            entity,
            reason,
        }) if operation.as_ref() == "binding reference"
            && *entity == parameter
            && reason.as_ref().contains("lexical visibility")
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let wrong_arity = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole: main_hole,
            draft: ExpressionDraft::new(
                vec![DraftNode::Call {
                    callee: function,
                    type_arguments: Vec::new(),
                    arguments: Vec::new(),
                }],
                DraftNodeId::new(0),
            ),
        }],
    });
    assert!(matches!(wrong_arity, Err(WorkspaceError::CallArity { .. })));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let wrong_type = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole: main_hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::Bool(true),
                    DraftNode::Call {
                        callee: function,
                        type_arguments: Vec::new(),
                        arguments: vec![DraftNodeId::new(0)],
                    },
                ],
                DraftNodeId::new(1),
            ),
        }],
    });
    assert!(matches!(
        wrong_type,
        Err(WorkspaceError::TypeMismatch { .. })
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let stale = EntityId::new(
        retry.snapshot.namespace(),
        function.slot(),
        function.generation().checked_add(1).expect("generation"),
    );
    let stale_call = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole: main_hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::I64(42),
                    DraftNode::Call {
                        callee: stale,
                        type_arguments: Vec::new(),
                        arguments: vec![DraftNodeId::new(0)],
                    },
                ],
                DraftNodeId::new(1),
            ),
        }],
    });
    assert!(matches!(stale_call, Err(WorkspaceError::StaleIdentity(_))));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let foreign = Workspace::empty_deterministic(33)
        .expect("foreign workspace")
        .current();
    let foreign_id = EntityId::new(foreign.namespace(), 0, 1);
    let foreign_call = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole: main_hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::I64(42),
                    DraftNode::Call {
                        callee: foreign_id,
                        type_arguments: Vec::new(),
                        arguments: vec![DraftNodeId::new(0)],
                    },
                ],
                DraftNodeId::new(1),
            ),
        }],
    });
    assert!(matches!(
        foreign_call,
        Err(WorkspaceError::ForeignNamespace(_))
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let cyclic = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole: main_hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::I64(0),
                    DraftNode::If {
                        condition: DraftNodeId::new(1),
                        then_branch: DraftNodeId::new(0),
                        else_branch: DraftNodeId::new(0),
                    },
                ],
                DraftNodeId::new(1),
            ),
        }],
    });
    assert!(matches!(cyclic, Err(WorkspaceError::InvalidDraft(_))));
    assert!(Arc::ptr_eq(&published, &workspace.current()));
    assert_eq!(workspace.current().entities(), published.entities());
    assert_eq!(workspace.current().nodes(), published.nodes());
    assert_eq!(
        workspace.current().holes().cloned().collect::<Vec<_>>(),
        published.holes().cloned().collect::<Vec<_>>()
    );
    assert_eq!(workspace.current().diagnostics(), published.diagnostics());
    assert_eq!(
        workspace
            .current()
            .project(&[])
            .expect("current projection"),
        published.project(&[]).expect("published projection")
    );
    assert_eq!(crate::source::parser_invocation_count(), 0);
}

#[test]
fn callable_deletion_is_dependency_closed_compacts_and_preserves_survivors() {
    let mut workspace = Workspace::empty_deterministic(33).expect("deletion workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "f".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "f-value".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateFunction {
                    name: "g".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "g-value".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create call chain");
    let f = entity_named(&created.snapshot, EntityKind::Function, "f");
    let g = entity_named(&created.snapshot, EntityKind::Function, "g");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let f_parameter = entity_named(&created.snapshot, EntityKind::Parameter, "f-value");
    let g_parameter = entity_named(&created.snapshot, EntityKind::Parameter, "g-value");
    let hole_for = |owner| {
        created
            .snapshot
            .holes()
            .find(|hole| hole.owner == owner)
            .expect("callable hole")
            .id
    };
    workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole: hole_for(f),
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Load(DraftBindingRef::Entity(f_parameter)),
                        DraftNode::Load(DraftBindingRef::Local(DraftBindingId::new(0))),
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: DraftBindingId::new(0),
                                name: "f-local".to_owned(),
                                value: DraftNodeId::new(0),
                            }],
                            body: DraftNodeId::new(1),
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("fill f");
    workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole: hole_for(g),
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Load(DraftBindingRef::Entity(g_parameter)),
                        DraftNode::Call {
                            callee: f,
                            type_arguments: Vec::new(),
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("fill g");
    let complete = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole: hole_for(main),
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::Call {
                            callee: g,
                            type_arguments: Vec::new(),
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("fill main");
    assert_eq!(run_i64(&complete.snapshot), 42);
    let g_root = complete
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.owner == SemanticOwner::Entity(g))
        .expect("g root")
        .id;
    let main_call = complete
        .snapshot
        .calls()
        .iter()
        .find(|edge| edge.caller == main)
        .expect("main call")
        .site;
    let f_local = entity_named(&complete.snapshot, EntityKind::ImmutableLocal, "f-local");
    let f_nodes: Vec<_> = complete
        .snapshot
        .nodes()
        .iter()
        .filter(|node| node.owner == SemanticOwner::Entity(f))
        .map(|node| node.id)
        .collect();
    let f_root = *f_nodes.first().expect("f root");
    let before = workspace.current();

    let retained_dependency = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::DeleteEntity { entity: f }],
    });
    assert!(matches!(
        retained_dependency,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    let duplicate = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![
            Edit::DeleteEntity { entity: f },
            Edit::DeleteEntity { entity: f },
        ],
    });
    assert!(matches!(
        duplicate,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    let wrong_kind = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![Edit::DeleteEntity {
            entity: g_parameter,
        }],
    });
    assert!(matches!(
        wrong_kind,
        Err(WorkspaceError::UnsupportedEdit { .. })
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    let rename_and_delete = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![
            Edit::RenameEntity {
                entity: f,
                new_name: "renamed-f".to_owned(),
            },
            Edit::DeleteEntity { entity: f },
        ],
    });
    assert!(matches!(
        rename_and_delete,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    let delete_and_descendant_edit = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![
            Edit::DeleteEntity { entity: f },
            Edit::ReplaceExpression {
                target: f_root,
                draft: ExpressionDraft::scalar_i64(1),
            },
        ],
    });
    assert!(matches!(
        delete_and_descendant_edit,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    let stale_f = EntityId::new(
        before.namespace(),
        f.slot(),
        f.generation().checked_add(1).expect("stale generation"),
    );
    assert!(matches!(
        workspace.apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::DeleteEntity { entity: stale_f }],
        }),
        Err(WorkspaceError::StaleIdentity(_))
    ));
    let foreign = Workspace::empty_deterministic(137)
        .expect("foreign workspace")
        .current();
    let foreign_entity = EntityId::new(foreign.namespace(), 0, 1);
    assert!(matches!(
        workspace.apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::DeleteEntity {
                entity: foreign_entity,
            }],
        }),
        Err(WorkspaceError::ForeignNamespace(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    let newly_reintroduced = workspace.apply(Transaction {
        base_revision: before.revision(),
        edits: vec![
            Edit::DeleteEntity { entity: f },
            Edit::ReplaceExpression {
                target: g_root,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Load(DraftBindingRef::Entity(g_parameter)),
                        DraftNode::Call {
                            callee: f,
                            type_arguments: Vec::new(),
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            },
        ],
    });
    assert!(matches!(
        newly_reintroduced,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

    let deleted = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![
                Edit::DeleteEntity { entity: f },
                Edit::ReplaceExpression {
                    target: g_root,
                    draft: ExpressionDraft::new(
                        vec![DraftNode::Load(DraftBindingRef::Entity(g_parameter))],
                        DraftNodeId::new(0),
                    ),
                },
            ],
        })
        .expect("remove call and delete f atomically");
    assert_eq!(
        deleted
            .snapshot
            .definition(deleted.snapshot.revision(), g)
            .expect("g")
            .id,
        g
    );
    assert_eq!(
        deleted
            .snapshot
            .definition(deleted.snapshot.revision(), main)
            .expect("main")
            .id,
        main
    );
    assert_eq!(
        deleted
            .snapshot
            .definition(deleted.snapshot.revision(), g_parameter)
            .expect("g parameter")
            .id,
        g_parameter
    );
    assert_eq!(
        deleted.snapshot.node(g_root).expect("stable g root").id,
        g_root
    );
    assert_eq!(
        deleted
            .snapshot
            .node(main_call)
            .expect("stable main call")
            .id,
        main_call
    );
    assert!(deleted.snapshot.entity(f).is_err());
    assert!(deleted.snapshot.entity(f_parameter).is_err());
    assert!(deleted.snapshot.entity(f_local).is_err());
    for node in f_nodes {
        assert!(deleted.snapshot.node(node).is_err());
    }
    assert_eq!(
        before.entity(f).expect("old function remains queryable").id,
        f
    );
    assert_eq!(run_i64(&before), 42);
    assert_eq!(run_i64(&deleted.snapshot), 42);
    for (index, binding) in deleted.snapshot.program.bindings.iter().enumerate() {
        assert_eq!(
            binding.id.raw(),
            u64::try_from(index).expect("binding index")
        );
    }
    assert!(deleted.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::EntityDeleted { entity, .. } if *entity == f
    )));
    assert!(deleted.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::EntityDeleted { entity, .. } if *entity == f_parameter
    )));

    let with_h = workspace
        .apply(Transaction {
            base_revision: deleted.snapshot.revision(),
            edits: vec![Edit::CreateFunction {
                name: "h".to_owned(),
                type_parameters: Vec::new(),
                parameters: vec![ParameterDraft {
                    name: "h-value".to_owned(),
                    ty: DeclarationType::I64,
                }],
                return_type: DeclarationType::I64,
            }],
        })
        .expect("reuse callable tombstones");
    let h = entity_named(&with_h.snapshot, EntityKind::Function, "h");
    let h_parameter = entity_named(&with_h.snapshot, EntityKind::Parameter, "h-value");
    let removed_ids = [f, f_parameter, f_local];
    assert!([h, h_parameter].into_iter().any(|created| {
        removed_ids.iter().any(|removed| {
            created.slot() == removed.slot() && created.generation() != removed.generation()
        })
    }));
    let h_hole = with_h
        .snapshot
        .holes()
        .find(|hole| hole.owner == h)
        .expect("h hole")
        .id;
    let without_h = workspace
        .apply(Transaction {
            base_revision: with_h.snapshot.revision(),
            edits: vec![Edit::DeleteEntity { entity: h }],
        })
        .expect("delete callable with active hole");
    assert!(without_h.snapshot.entity(h).is_err());
    assert!(without_h.snapshot.node(h_hole.node()).is_err());
    assert!(without_h.snapshot.holes().all(|hole| hole.owner != h));
    assert!(without_h.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::DescendantDeleted {
            node,
            kind: NodeKind::Hole,
            ..
        } if *node == h_hole.node()
    )));

    let without_main = workspace
        .apply(Transaction {
            base_revision: without_h.snapshot.revision(),
            edits: vec![Edit::DeleteEntity { entity: main }],
        })
        .expect("delete main");
    assert_eq!(without_main.snapshot.state(), ProgramState::Incomplete);
    assert_eq!(
        without_main.snapshot.completeness_blockers(),
        &[CompletenessBlocker::MissingEntryPoint]
    );
    assert!(without_main.snapshot.entity(main).is_err());
    assert!(without_main.snapshot.node(main_call).is_err());
    assert!(without_main.snapshot.program.main.is_none());
    assert_eq!(without_main.snapshot.diagnostics().len(), 1);
    assert_eq!(
        without_main.snapshot.diagnostics()[0].code.as_ref(),
        "workspace.missing-entry-point"
    );
    let recreated = workspace
        .apply(Transaction {
            base_revision: without_main.snapshot.revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("recreate main");
    let new_main = entity_named(&recreated.snapshot, EntityKind::Main, "main");
    assert_eq!(new_main.slot(), main.slot());
    assert_ne!(new_main.generation(), main.generation());
}

#[test]
fn callable_compaction_preserves_a_later_function_hole_and_old_snapshot() {
    let mut workspace = Workspace::empty_deterministic(142).expect("hole relocation workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "remove".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "removed-parameter".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateFunction {
                    name: "retain".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "retained-parameter".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
            ],
        })
        .expect("create incomplete functions");
    let removed = entity_named(&created.snapshot, EntityKind::Function, "remove");
    let retained = entity_named(&created.snapshot, EntityKind::Function, "retain");
    let retained_parameter = entity_named(
        &created.snapshot,
        EntityKind::Parameter,
        "retained-parameter",
    );
    let removed_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == removed)
        .expect("removed hole")
        .id;
    let retained_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == retained)
        .expect("retained hole")
        .id;
    let old = created.snapshot;
    let deleted = workspace
        .apply(Transaction {
            base_revision: old.revision(),
            edits: vec![Edit::DeleteEntity { entity: removed }],
        })
        .expect("delete earlier incomplete function");
    assert!(deleted.snapshot.entity(removed).is_err());
    assert!(deleted.snapshot.node(removed_hole.node()).is_err());
    assert_eq!(
        deleted.snapshot.entity(retained).expect("retained").id,
        retained
    );
    assert_eq!(
        deleted
            .snapshot
            .entity(retained_parameter)
            .expect("retained parameter")
            .id,
        retained_parameter
    );
    let hole = deleted
        .snapshot
        .hole_context(deleted.snapshot.revision(), retained_hole)
        .expect("retained hole context");
    assert_eq!(hole.id, retained_hole);
    assert_eq!(hole.owner, retained);
    assert!(hole.visible_entities.contains(&retained_parameter));
    assert_eq!(old.entity(removed).expect("old function").id, removed);
    assert_eq!(
        old.hole_context(old.revision(), removed_hole)
            .expect("old hole")
            .id,
        removed_hole
    );
}

#[test]
fn imported_function_deletion_is_parser_free_and_uses_the_same_lifecycle() {
    let snapshot = importer::import_source_with_namespace(
        FUNCTION_PROGRAM,
        "workspace-delete-imported.lkjscript",
        WorkspaceNamespace::deterministic(134),
    )
    .expect("import function program");
    let function = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("imported function")
        .id;
    let main = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Main)
        .expect("imported main")
        .id;
    let main_root = snapshot
        .nodes()
        .iter()
        .find(|node| node.owner == SemanticOwner::Entity(main))
        .expect("main root")
        .id;
    let mut workspace = Workspace::new(snapshot).expect("imported workspace");
    crate::source::reset_parser_invocation_count();
    let deleted = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::DeleteEntity { entity: function },
                Edit::ReplaceExpression {
                    target: main_root,
                    draft: ExpressionDraft::scalar_i64(42),
                },
            ],
        })
        .expect("delete imported function without source");
    assert!(deleted.snapshot.entity(function).is_err());
    assert_eq!(run_i64(&deleted.snapshot), 42);
    assert_eq!(crate::source::parser_invocation_count(), 0);

    let (
        mut source_free,
        source_free_function,
        parameter,
        source_free_main,
        function_hole,
        main_hole,
    ) = create_source_free_declarations(139);
    let source_free_complete = fill_source_free_identity(
        &mut source_free,
        source_free_function,
        parameter,
        function_hole,
        main_hole,
    );
    let source_free_root = source_free_complete
        .nodes()
        .iter()
        .find(|node| node.owner == SemanticOwner::Entity(source_free_main))
        .expect("source-free main root")
        .id;
    let source_free_deleted = source_free
        .apply(Transaction {
            base_revision: source_free_complete.revision(),
            edits: vec![
                Edit::ReplaceExpression {
                    target: source_free_root,
                    draft: ExpressionDraft::scalar_i64(42),
                },
                Edit::DeleteEntity {
                    entity: source_free_function,
                },
            ],
        })
        .expect("delete source-free function");
    assert_eq!(
        canonical_workspace_observation(&deleted.snapshot),
        canonical_workspace_observation(&source_free_deleted.snapshot)
    );
    assert_eq!(run_i64(&source_free_deleted.snapshot), 42);
    assert_eq!(crate::source::parser_invocation_count(), 0);
}

#[test]
fn imported_mutable_local_subtree_removal_compacts_without_reparsing() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "var/\nname/\nx\n/name\ntype/\ni64\n/type\n1\nx\n/var\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "workspace-remove-mutable.lkjscript",
        WorkspaceNamespace::deterministic(136),
    )
    .expect("import mutable local");
    let local = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::MutableLocal)
        .expect("mutable local")
        .id;
    let node = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::MutableLocal)
        .expect("mutable-local node")
        .id;
    let mut workspace = Workspace::new(snapshot).expect("mutable workspace");
    crate::source::reset_parser_invocation_count();
    let replaced = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::ReplaceExpression {
                target: node,
                draft: ExpressionDraft::scalar_i64(9),
            }],
        })
        .expect("remove mutable local");
    assert!(replaced.snapshot.entity(local).is_err());
    assert_eq!(replaced.snapshot.node(node).expect("stable root").id, node);
    assert_eq!(
        replaced
            .snapshot
            .program
            .main
            .as_ref()
            .expect("main")
            .local_count,
        0
    );
    assert_eq!(run_i64(&replaced.snapshot), 9);
    assert_eq!(crate::source::parser_invocation_count(), 0);
}

#[test]
fn source_free_mutable_initializer_compacts_before_local_activation() {
    let mut workspace = Workspace::empty_deterministic(232).expect("mutable compaction workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "remove".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::I64,
                },
                Edit::CreateFunction {
                    name: "retain".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "input".to_owned(),
                        ty: DeclarationType::I64,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create compaction declarations");
    let remove = entity_named(&created.snapshot, EntityKind::Function, "remove");
    let retain = entity_named(&created.snapshot, EntityKind::Function, "retain");
    let input = entity_named(&created.snapshot, EntityKind::Parameter, "input");
    let retain_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == retain)
        .expect("retain hole")
        .id;
    let filled_retain = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: retain_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Load(DraftBindingRef::Entity(input)),
                        DraftNode::Load(DraftBindingRef::Local(DraftBindingId::new(0))),
                        DraftNode::MutableLocal {
                            binding: DraftBindingId::new(0),
                            name: "retained-local".to_owned(),
                            ty: SemanticType::I64,
                            initial: DraftNodeId::new(0),
                            body: DraftNodeId::new(1),
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("fill retained function");
    let local = entity_named(
        &filled_retain.snapshot,
        EntityKind::MutableLocal,
        "retained-local",
    );
    let main = entity_named(&filled_retain.snapshot, EntityKind::Main, "main");
    let main_hole = filled_retain
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;
    let filled_main = workspace
        .apply(Transaction {
            base_revision: filled_retain.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(7),
                        DraftNode::Call {
                            callee: retain,
                            type_arguments: Vec::new(),
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("fill compaction main");
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let compacted = workspace
        .apply(Transaction {
            base_revision: filled_main.snapshot.revision(),
            edits: vec![Edit::DeleteEntity { entity: remove }],
        })
        .expect("delete earlier function and compact bindings");
    assert_eq!(compacted.snapshot.state(), ProgramState::Complete);
    assert_eq!(
        compacted
            .snapshot
            .entity(retain)
            .expect("retained function")
            .id,
        retain
    );
    assert_eq!(
        compacted
            .snapshot
            .entity(input)
            .expect("retained parameter")
            .id,
        input
    );
    assert_eq!(
        compacted.snapshot.entity(local).expect("retained local").id,
        local
    );
    assert_eq!(run_i64(&compacted.snapshot), 7);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn declarations_created_in_separate_revisions_refresh_hole_scope_and_keep_ids() {
    let mut function_first = Workspace::empty_deterministic(34).expect("function-first workspace");
    let function_created = function_first
        .apply(Transaction {
            base_revision: function_first.current().revision(),
            edits: vec![Edit::CreateFunction {
                name: "identity".to_owned(),
                type_parameters: Vec::new(),
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: DeclarationType::I64,
                }],
                return_type: DeclarationType::I64,
            }],
        })
        .expect("create function before main");
    let function = function_created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("function entity")
        .id;
    let parameter = function_created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Parameter)
        .expect("parameter entity")
        .id;
    let function_hole = function_created
        .snapshot
        .holes()
        .next()
        .expect("function hole")
        .id;
    assert_eq!(
        function_created.snapshot.completeness_blockers()[0],
        CompletenessBlocker::MissingEntryPoint
    );
    let main_created = function_first
        .apply(Transaction {
            base_revision: function_created.snapshot.revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create main after function");
    assert_eq!(
        main_created
            .snapshot
            .definition(main_created.snapshot.revision(), function)
            .expect("stable function")
            .id,
        function
    );
    assert_eq!(
        main_created
            .snapshot
            .definition(main_created.snapshot.revision(), parameter)
            .expect("stable parameter")
            .id,
        parameter
    );
    assert_eq!(
        main_created
            .snapshot
            .hole_context(main_created.snapshot.revision(), function_hole)
            .expect("stable function hole")
            .id,
        function_hole
    );

    let mut main_first = Workspace::empty_deterministic(35).expect("main-first workspace");
    let main_created = main_first
        .apply(Transaction {
            base_revision: main_first.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create main before function");
    let main_hole = main_created.snapshot.holes().next().expect("main hole").id;
    assert!(main_created
        .snapshot
        .hole_context(main_created.snapshot.revision(), main_hole)
        .expect("initial main context")
        .visible_entities
        .is_empty());
    let function_created = main_first
        .apply(Transaction {
            base_revision: main_created.snapshot.revision(),
            edits: vec![Edit::CreateFunction {
                name: "identity".to_owned(),
                type_parameters: Vec::new(),
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: DeclarationType::I64,
                }],
                return_type: DeclarationType::I64,
            }],
        })
        .expect("create function after main");
    let later_function = function_created
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("later function")
        .id;
    let refreshed = function_created
        .snapshot
        .hole_context(function_created.snapshot.revision(), main_hole)
        .expect("refreshed main context");
    assert!(refreshed.visible_entities.contains(&later_function));
    assert!(function_created
        .snapshot
        .legal_constructors(
            function_created.snapshot.revision(),
            main_hole,
            PageRequest::new(16).expect("page"),
            None,
        )
        .expect("main constructors")
        .items
        .contains(&LegalConstructor::Call(later_function)));
}

#[test]
fn tombstoned_node_generations_survive_snapshot_reopening() {
    let snapshot = importer::import_source_with_namespace(
        CONDITIONAL,
        "workspace-reopen.lkjscript",
        WorkspaceNamespace::deterministic(36),
    )
    .expect("conditional import");
    let root = snapshot.nodes()[0].id;
    let deleted = snapshot.nodes()[1..]
        .iter()
        .map(|node| node.id)
        .collect::<Vec<_>>();
    let mut workspace = Workspace::new(snapshot).expect("workspace");
    let holed = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::IntroduceHole {
                target: root,
                goal: "replace the conditional".to_owned(),
            }],
        })
        .expect("remove descendants with a real hole");
    for id in &deleted {
        assert!(holed.snapshot.node(*id).is_err());
    }

    let reopened_snapshot = (*holed.snapshot).clone();
    let hole = reopened_snapshot.holes().next().expect("hole").id;
    let mut reopened = Workspace::new(reopened_snapshot).expect("reopen snapshot");
    let filled = reopened
        .apply(Transaction {
            base_revision: reopened.current().revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Bool(true),
                        DraftNode::I64(1),
                        DraftNode::I64(2),
                        DraftNode::If {
                            condition: DraftNodeId::new(0),
                            then_branch: DraftNodeId::new(1),
                            else_branch: DraftNodeId::new(2),
                        },
                    ],
                    DraftNodeId::new(3),
                ),
            }],
        })
        .expect("fill reopened hole");
    assert_eq!(filled.snapshot.node(root).expect("root identity").id, root);
    for id in &deleted {
        assert!(filled.snapshot.node(*id).is_err());
    }
    assert!(filled
        .snapshot
        .nodes()
        .iter()
        .filter(|node| node.id != root)
        .all(|node| node.id.generation() > 1));
}

#[test]
fn removing_attachments_preserves_semantic_identity_and_revision() {
    let snapshot = import_source(SCALAR, "workspace-attachments.lkjscript")
        .expect("import snapshot with attachments");
    let detached = snapshot.without_attachments();
    assert!(snapshot.attachments().is_some());
    assert!(detached.attachments().is_none());
    assert_eq!(snapshot.namespace(), detached.namespace());
    assert_eq!(snapshot.revision(), detached.revision());
    assert_eq!(snapshot.entities(), detached.entities());
    assert_eq!(snapshot.nodes(), detached.nodes());
}

#[test]
fn concise_projection_is_deterministic_attachment_independent_and_exact() {
    let namespace = WorkspaceNamespace::deterministic(13);
    let first =
        importer::import_source_with_namespace(SCALAR, "workspace-projection.lkjscript", namespace)
            .expect("first projection import");
    let formatted_source = format!(";; formatting attachment only\n\n{SCALAR}\n");
    let formatted = importer::import_source_with_namespace(
        &formatted_source,
        "workspace-projection.lkjscript",
        namespace,
    )
    .expect("formatted projection import");
    assert_ne!(
        first.attachments().expect("first attachment").files()[0].exact_source_sha256(),
        formatted
            .attachments()
            .expect("formatted attachment")
            .files()[0]
            .exact_source_sha256()
    );
    assert_eq!(first.entities(), formatted.entities());
    assert_eq!(first.nodes(), formatted.nodes());

    let main = first.entities()[0].id;
    let root = first.nodes()[0].id;
    let selection = [
        ProjectionSlice::Entity(main),
        ProjectionSlice::Body(main),
        ProjectionSlice::Type(root),
    ];
    let expected = concat!(
        "workspace revision=1 state=complete\n",
        "entity e0g1 kind=main name=\"main\" owner=- type=\"i64\"\n",
        "body e0g1 name=\"main\"\n",
        "  node n0g1 kind=literal type=\"i64\" expected=\"i64\" operation=- effects=[pure]\n",
        "type n0g1 actual=\"i64\" expected=\"i64\" operation=- effects=[pure]\n",
    );
    assert_eq!(
        first.project(&selection).expect("first projection"),
        expected
    );
    assert_eq!(
        formatted.project(&selection).expect("formatted projection"),
        expected
    );
    assert_eq!(
        first
            .without_attachments()
            .project(&selection)
            .expect("detached projection"),
        expected
    );
}

#[test]
fn atomic_rename_replace_queries_and_direct_compile_use_no_parser() {
    let snapshot = importer::import_source_with_namespace(
        FUNCTION_PROGRAM,
        "workspace-edit.lkjscript",
        WorkspaceNamespace::deterministic(21),
    )
    .expect("import function program");
    let function = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("function entity")
        .id;
    let parameter = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Parameter)
        .expect("parameter entity")
        .id;
    let call = snapshot.calls()[0];
    assert_eq!(
        snapshot
            .references_to(
                snapshot.revision(),
                function,
                PageRequest::new(8).expect("page"),
                None,
            )
            .expect("references")
            .items
            .len(),
        1
    );
    assert_eq!(
        snapshot
            .callers_of(
                snapshot.revision(),
                function,
                PageRequest::new(8).expect("page"),
                None,
            )
            .expect("callers")
            .items[0],
        call
    );
    assert_eq!(
        snapshot
            .node_type(snapshot.revision(), call.site)
            .expect("type")
            .actual,
        SemanticType::I64
    );
    assert_eq!(
        snapshot
            .references_to(
                snapshot.revision(),
                parameter,
                PageRequest::new(8).expect("page"),
                None,
            )
            .expect("parameter references")
            .items
            .len(),
        1
    );

    let root = snapshot
        .nodes()
        .iter()
        .find(|node| node.owner == SemanticOwner::Entity(snapshot.entities()[0].id))
        .expect("main root")
        .id;
    let draft = ExpressionDraft::new(
        vec![
            DraftNode::I64(42),
            DraftNode::Call {
                callee: function,
                type_arguments: Vec::new(),
                arguments: vec![DraftNodeId::new(0)],
            },
        ],
        DraftNodeId::new(1),
    );
    let unchanged_parameter = parameter;
    crate::source::reset_parser_invocation_count();
    let mut workspace = Workspace::new(snapshot).expect("workspace");
    let outcome = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::RenameEntity {
                    entity: function,
                    new_name: "answer".to_owned(),
                },
                Edit::ReplaceExpression {
                    target: root,
                    draft,
                },
            ],
        })
        .expect("atomic semantic transaction");
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(
        outcome
            .snapshot
            .definition(outcome.snapshot.revision(), function)
            .expect("renamed")
            .name
            .as_ref(),
        "answer"
    );
    assert_eq!(
        outcome
            .snapshot
            .search_entities(
                outcome.snapshot.revision(),
                "ANS",
                PageRequest::new(4).expect("page"),
                None,
            )
            .expect("search")
            .items[0]
            .id,
        function
    );
    assert_eq!(
        outcome
            .snapshot
            .callees_of(
                outcome.snapshot.revision(),
                outcome.snapshot.entities()[0].id,
                PageRequest::new(4).expect("page"),
                None,
            )
            .expect("callees")
            .items[0]
            .callee,
        function
    );
    assert_eq!(
        outcome
            .snapshot
            .definition(outcome.snapshot.revision(), unchanged_parameter)
            .expect("stable parameter")
            .id,
        unchanged_parameter
    );
    assert_eq!(
        outcome
            .snapshot
            .node(root)
            .expect("stable replacement root")
            .id,
        root
    );
    assert_eq!(
        outcome
            .snapshot
            .program
            .main
            .as_ref()
            .expect("main")
            .body
            .origin,
        crate::hir::Origin::Semantic
    );
    assert_eq!(run_i64(&outcome.snapshot), 42);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert!(outcome.diff.entries.iter().any(|entry| matches!(entry, SemanticDiffEntry::EntityRenamed { entity, .. } if *entity == function)));
    assert!(outcome.diff.entries.iter().any(
        |entry| matches!(entry, SemanticDiffEntry::ExpressionReplaced { node, .. } if *node == root)
    ));
}

#[test]
fn typed_hole_is_queryable_refinable_not_executable_and_fill_preserves_root() {
    let snapshot = importer::import_source_with_namespace(
        FUNCTION_PROGRAM,
        "workspace-hole.lkjscript",
        WorkspaceNamespace::deterministic(22),
    )
    .expect("import function program");
    let main = snapshot.entities()[0].id;
    let function = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Function)
        .expect("function")
        .id;
    assert!(snapshot
        .dependencies()
        .iter()
        .any(|edge| edge.dependent == main && edge.dependency == function));
    let root = snapshot.nodes()[0].id;
    let mut workspace = Workspace::new(snapshot).expect("workspace");
    let introduced = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::IntroduceHole {
                target: root,
                goal: "compute the result".to_owned(),
            }],
        })
        .expect("introduce hole");
    assert_eq!(introduced.snapshot.state(), ProgramState::Incomplete);
    assert!(matches!(
        introduced
            .snapshot
            .program
            .main
            .as_ref()
            .expect("main")
            .body
            .kind,
        crate::hir::ExprKind::Hole
    ));
    assert_eq!(
        introduced
            .snapshot
            .program
            .main
            .as_ref()
            .expect("main")
            .body
            .origin,
        crate::hir::Origin::Semantic
    );
    assert!(!introduced
        .snapshot
        .dependencies()
        .iter()
        .any(|edge| edge.dependent == main && edge.dependency == function));
    let hole = introduced.snapshot.holes().next().expect("hole").id;
    let context = introduced
        .snapshot
        .hole_context(introduced.snapshot.revision(), hole)
        .expect("hole context");
    assert_eq!(context.expected_type, SemanticType::I64);
    let projection = introduced
        .snapshot
        .project(&[
            ProjectionSlice::Entity(main),
            ProjectionSlice::Body(main),
            ProjectionSlice::Type(root),
            ProjectionSlice::References(function),
            ProjectionSlice::Hole(hole),
        ])
        .expect("incomplete projection");
    assert!(projection.starts_with("workspace revision=2 state=incomplete\n"));
    assert!(projection.contains(
        "node n0g1 kind=hole type=\"i64\" expected=\"i64\" operation=- effects=[unknown] [HOLE]"
    ));
    assert!(projection.contains(
        "type n0g1 actual=\"i64\" expected=\"i64\" operation=- effects=[unknown] [HOLE]"
    ));
    assert!(projection.contains(" count=0\n"));
    assert!(projection.contains("hole n0g1 [HOLE] expected=\"i64\""));
    assert!(projection.contains("goal=\"compute the result\""));
    assert_eq!(introduced.snapshot.diagnostics().len(), 1);
    assert_eq!(
        introduced
            .snapshot
            .diagnostic_page(
                introduced.snapshot.revision(),
                PageRequest::new(1).expect("page"),
                None,
            )
            .expect("diagnostics")
            .items
            .len(),
        1
    );
    assert!(introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            hole,
            PageRequest::new(8).expect("page"),
            None,
        )
        .expect("legal constructors")
        .items
        .contains(&LegalConstructor::I64Literal));
    let failure =
        crate::compile_snapshot(&introduced.snapshot).expect_err("incomplete is not executable");
    match failure {
        crate::CompileSnapshotError::Incomplete(error) => assert!(error.blockers.iter().any(
            |blocker| matches!(blocker, CompletenessBlocker::TypedHole { hole: blocked, .. } if *blocked == hole)
        )),
        other => panic!("unexpected compile failure: {other}"),
    }

    let refined = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::RefineHole {
                hole,
                expected_type: Some(SemanticType::I64),
                goal: "return nine".to_owned(),
            }],
        })
        .expect("refine hole");
    assert_eq!(
        refined
            .snapshot
            .hole_context(refined.snapshot.revision(), hole)
            .expect("refined context")
            .goal
            .as_ref(),
        "return nine"
    );
    crate::source::reset_parser_invocation_count();
    let filled = workspace
        .apply(Transaction {
            base_revision: refined.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::scalar_i64(9),
            }],
        })
        .expect("fill hole");
    assert_eq!(filled.snapshot.state(), ProgramState::Complete);
    assert_eq!(filled.snapshot.node(root).expect("preserved root").id, root);
    assert_eq!(run_i64(&filled.snapshot), 9);
    assert_eq!(crate::source::parser_invocation_count(), 0);
}

#[test]
fn failed_transaction_stale_revision_foreign_id_and_cursor_leave_snapshot_unchanged() {
    let snapshot = importer::import_source_with_namespace(
        FUNCTION_PROGRAM,
        "workspace-atomic.lkjscript",
        WorkspaceNamespace::deterministic(23),
    )
    .expect("import function program");
    let root = snapshot.nodes().iter().find(|node| matches!(node.owner, SemanticOwner::Entity(entity) if entity == snapshot.entities()[0].id)).expect("main root").id;
    let old_revision = snapshot.revision();
    let first_page = snapshot
        .entity_page(old_revision, PageRequest::new(2).expect("page"), None)
        .expect("first page");
    let cursor = first_page.continuation.clone().expect("continuation");
    let mut workspace = Workspace::new(snapshot).expect("workspace");
    let before = workspace.current();
    let failure = workspace.apply(Transaction {
        base_revision: old_revision,
        edits: vec![Edit::ReplaceExpression {
            target: root,
            draft: ExpressionDraft::scalar_bool(true),
        }],
    });
    assert!(matches!(failure, Err(WorkspaceError::TypeMismatch { .. })));
    assert!(Arc::ptr_eq(&before, &workspace.current()));
    assert_eq!(workspace.current().revision(), old_revision);
    let committed = workspace
        .apply(Transaction {
            base_revision: old_revision,
            edits: vec![Edit::ReplaceExpression {
                target: root,
                draft: ExpressionDraft::scalar_i64(7),
            }],
        })
        .expect("commit");
    assert!(matches!(
        workspace.apply(Transaction {
            base_revision: old_revision,
            edits: vec![Edit::ReplaceExpression {
                target: root,
                draft: ExpressionDraft::scalar_i64(8)
            }],
        }),
        Err(WorkspaceError::StaleRevision)
    ));
    assert!(committed
        .snapshot
        .entity_page(
            committed.snapshot.revision(),
            PageRequest::new(2).expect("page"),
            Some(&cursor)
        )
        .is_err());

    let foreign = import_source(SCALAR, "workspace-foreign.lkjscript").expect("foreign");
    assert!(matches!(
        committed
            .snapshot
            .definition(committed.snapshot.revision(), foreign.entities()[0].id),
        Err(WorkspaceError::ForeignNamespace(_))
    ));
}

#[test]
fn overlapping_structural_edits_and_reserved_function_rename_are_atomic() {
    let imported = importer::import_source_with_namespace(
        CONDITIONAL,
        "workspace-overlap.lkjscript",
        WorkspaceNamespace::deterministic(43),
    )
    .expect("import conditional");
    let mut workspace = Workspace::new(imported).expect("workspace");
    let before = workspace.current();
    let main = before
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Main)
        .expect("main")
        .id;
    let root = before
        .nodes()
        .iter()
        .find(|node| node.owner == SemanticOwner::Entity(main))
        .expect("root")
        .id;
    let condition = before
        .nodes()
        .iter()
        .find(|node| {
            node.owner == SemanticOwner::Node(root)
                && before
                    .node_type(before.revision(), node.id)
                    .expect("condition type")
                    .actual
                    == SemanticType::Bool
        })
        .expect("condition")
        .id;
    let before_projection = before.project(&[]).expect("projection");

    for edits in [
        vec![
            Edit::ReplaceExpression {
                target: root,
                draft: ExpressionDraft::new(vec![DraftNode::I64(7)], DraftNodeId::new(0)),
            },
            Edit::ReplaceExpression {
                target: condition,
                draft: ExpressionDraft::new(vec![DraftNode::Bool(false)], DraftNodeId::new(0)),
            },
        ],
        vec![
            Edit::ReplaceExpression {
                target: condition,
                draft: ExpressionDraft::new(vec![DraftNode::Bool(false)], DraftNodeId::new(0)),
            },
            Edit::ReplaceExpression {
                target: root,
                draft: ExpressionDraft::new(vec![DraftNode::I64(7)], DraftNodeId::new(0)),
            },
        ],
    ] {
        let error = workspace
            .apply(Transaction {
                base_revision: before.revision(),
                edits,
            })
            .expect_err("reject overlapping subtree edits");
        assert!(error.to_string().contains("disjoint expression subtrees"));
        assert!(Arc::ptr_eq(&workspace.current(), &before));
        assert_eq!(workspace.current().revision(), before.revision());
        assert_eq!(
            workspace
                .current()
                .project(&[])
                .expect("projection after failure"),
            before_projection
        );
    }

    let (mut source_free, function, _, _, _, _) = create_source_free_declarations(44);
    let source_free_before = source_free.current();
    let error = source_free
        .apply(Transaction {
            base_revision: source_free_before.revision(),
            edits: vec![Edit::RenameEntity {
                entity: function,
                new_name: "main".to_owned(),
            }],
        })
        .expect_err("reject reserved global name");
    assert!(error.to_string().contains("exists or is reserved"));
    assert!(Arc::ptr_eq(&source_free.current(), &source_free_before));
}

#[test]
fn disjoint_batch_edits_preserve_roots_when_earlier_subtrees_change_size() {
    let snapshot = importer::import_source_with_namespace(
        CONDITIONAL,
        "workspace-disjoint-shift.lkjscript",
        WorkspaceNamespace::deterministic(74),
    )
    .expect("conditional import");
    let root = snapshot.nodes()[0].id;
    let condition = snapshot.nodes()[1].id;
    let alternative = snapshot.nodes()[3].id;
    let mut workspace = Workspace::new(snapshot).expect("workspace");
    let edited = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::ReplaceExpression {
                    target: condition,
                    draft: ExpressionDraft::new(
                        vec![
                            DraftNode::If {
                                condition: DraftNodeId::new(1),
                                then_branch: DraftNodeId::new(2),
                                else_branch: DraftNodeId::new(3),
                            },
                            DraftNode::Bool(true),
                            DraftNode::Bool(true),
                            DraftNode::Bool(false),
                        ],
                        DraftNodeId::new(0),
                    ),
                },
                Edit::ReplaceExpression {
                    target: alternative,
                    draft: ExpressionDraft::scalar_i64(9),
                },
            ],
        })
        .expect("apply size-shifting disjoint edits");
    assert_eq!(edited.snapshot.node(root).expect("root").id, root);
    assert_eq!(
        edited
            .snapshot
            .node(condition)
            .expect("condition root preserved")
            .kind,
        NodeKind::Conditional
    );
    assert_eq!(
        edited
            .snapshot
            .node_type(edited.snapshot.revision(), alternative)
            .expect("alternative root preserved")
            .actual,
        SemanticType::I64
    );
}

#[test]
fn replacement_descendants_do_not_inherit_identity_from_fingerprints() {
    let snapshot = importer::import_source_with_namespace(
        CONDITIONAL,
        "workspace-reorder.lkjscript",
        WorkspaceNamespace::deterministic(25),
    )
    .expect("conditional import");
    assert_eq!(snapshot.nodes().len(), 4);
    let root = snapshot.nodes()[0].id;
    let old_one = snapshot.nodes()[2].id;
    let old_two = snapshot.nodes()[3].id;
    let old_revision = snapshot.revision();
    let draft = ExpressionDraft::new(
        vec![
            DraftNode::Bool(true),
            DraftNode::I64(2),
            DraftNode::I64(1),
            DraftNode::If {
                condition: DraftNodeId::new(0),
                then_branch: DraftNodeId::new(1),
                else_branch: DraftNodeId::new(2),
            },
        ],
        DraftNodeId::new(3),
    );
    let mut workspace = Workspace::new(snapshot).expect("workspace");
    let edited = workspace
        .apply(Transaction {
            base_revision: old_revision,
            edits: vec![Edit::ReplaceExpression {
                target: root,
                draft,
            }],
        })
        .expect("reorder branches");
    assert!(edited.snapshot.node(old_one).is_err());
    assert!(edited.snapshot.node(old_two).is_err());
    assert_eq!(edited.snapshot.node(root).expect("root").id, root);
    assert!(edited
        .snapshot
        .nodes()
        .iter()
        .skip(1)
        .all(|node| node.id != old_one && node.id != old_two));

    let holed = workspace
        .apply(Transaction {
            base_revision: edited.snapshot.revision(),
            edits: vec![Edit::IntroduceHole {
                target: root,
                goal: "choose a value".to_owned(),
            }],
        })
        .expect("replace tree with hole");
    assert!(holed.snapshot.node(old_one).is_err());
    let hole = holed.snapshot.holes().next().expect("hole").id;
    let filled = workspace
        .apply(Transaction {
            base_revision: holed.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::scalar_i64(3),
            }],
        })
        .expect("fill tree hole");
    assert_eq!(filled.snapshot.node(root).expect("root preserved").id, root);
    assert!(filled.snapshot.node(old_one).is_err());
}

fn deep_if_draft(depth: usize) -> ExpressionDraft {
    let mut nodes = Vec::new();
    nodes
        .try_reserve(
            depth
                .checked_mul(3)
                .and_then(|count| count.checked_add(2))
                .expect("draft geometry"),
        )
        .expect("draft allocation");
    nodes.push(DraftNode::I64(1));
    let mut expression = DraftNodeId::new(0);
    for _ in 0..depth {
        let condition = DraftNodeId::new(u64::try_from(nodes.len()).expect("condition id"));
        nodes.push(DraftNode::Bool(true));
        let alternative = DraftNodeId::new(u64::try_from(nodes.len()).expect("alternative id"));
        nodes.push(DraftNode::I64(0));
        let next = DraftNodeId::new(u64::try_from(nodes.len()).expect("if id"));
        nodes.push(DraftNode::If {
            condition,
            then_branch: expression,
            else_branch: alternative,
        });
        expression = next;
    }
    let returned = DraftNodeId::new(u64::try_from(nodes.len()).expect("return id"));
    nodes.push(DraftNode::Return { value: expression });
    ExpressionDraft::new(nodes, returned)
}

fn run_source_free_deep(depth: usize, seed: u64) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("empty workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create main");
    let main = created.snapshot.entities()[0].id;
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: deep_if_draft(depth),
            }],
        })
        .expect("fill deep main");
    assert_eq!(completed.snapshot.nodes().len(), depth * 3 + 2);
    let projection = completed
        .snapshot
        .project(&[ProjectionSlice::Body(main)])
        .expect("deep body projection");
    assert_eq!(projection.matches("node n").count(), depth * 3 + 2);
    assert_eq!(run_i64(&completed.snapshot), 1);

    let deepest_value = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| {
            node.kind == NodeKind::Literal
                && completed
                    .snapshot
                    .node_semantics(completed.snapshot.revision(), node.id)
                    .is_ok_and(|facts| facts.actual == SemanticType::I64)
        })
        .expect("deepest branch value")
        .id;
    let holed = workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![Edit::IntroduceHole {
                target: deepest_value,
                goal: "choose the deepest branch result".to_owned(),
            }],
        })
        .expect("introduce deep result hole");
    let deep_hole = holed
        .snapshot
        .holes()
        .find(|hole| hole.id.node() == deepest_value)
        .expect("deep result hole")
        .id;
    assert!(holed
        .snapshot
        .legal_constructors(
            holed.snapshot.revision(),
            deep_hole,
            PageRequest::new(64).expect("deep constructor page"),
            None,
        )
        .expect("query deep constructors")
        .items
        .contains(&LegalConstructor::Return));

    let mut unresolved_workspace =
        Workspace::new((*completed.snapshot).clone()).expect("deep unresolved workspace");
    let unresolved = unresolved_workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![Edit::IntroduceUnresolvedValueReference {
                target: deepest_value,
                requested_name: "missing".to_owned(),
            }],
        })
        .expect("introduce deep unresolved reference");
    let reference = unresolved
        .snapshot
        .unresolved_value_references()
        .next()
        .expect("deep unresolved reference")
        .id;
    assert!(unresolved
        .snapshot
        .unresolved_value_reference_candidates(
            unresolved.snapshot.revision(),
            reference,
            PageRequest::new(8).expect("deep candidate page"),
            None,
        )
        .expect("deep candidate query")
        .items
        .is_empty());
    let projection = unresolved
        .snapshot
        .project(&[
            ProjectionSlice::Body(main),
            ProjectionSlice::UnresolvedValueReference(reference),
        ])
        .expect("deep unresolved projection");
    assert!(projection.contains("[UNRESOLVED] intent=copy-load"));
    assert!(matches!(
        crate::compile_snapshot(&unresolved.snapshot),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));
    let replaced = unresolved_workspace
        .apply(Transaction {
            base_revision: unresolved.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: reference.node(),
                draft: ExpressionDraft::scalar_i64(1),
            }],
        })
        .expect("replace deep unresolved reference");
    assert_eq!(run_i64(&replaced.snapshot), 1);
}

#[test]
fn source_free_index_root_resolution_is_one_lookup_per_node() {
    for (seed, depth) in [(40, 32_usize), (41, 64), (42, 128)] {
        let mut workspace = Workspace::empty_deterministic(seed).expect("empty workspace");
        let created = workspace
            .apply(Transaction {
                base_revision: workspace.current().revision(),
                edits: vec![Edit::CreateMain {
                    return_type: SemanticType::I64,
                }],
            })
            .expect("create main");
        let hole = created.snapshot.holes().next().expect("main hole").id;
        super::index::reset_root_address_lookups();
        let completed = workspace
            .apply(Transaction {
                base_revision: created.snapshot.revision(),
                edits: vec![Edit::FillHole {
                    hole,
                    draft: deep_if_draft(depth),
                }],
            })
            .expect("fill generated body");
        assert_eq!(
            super::index::root_address_lookups(),
            u64::try_from(completed.snapshot.nodes().len()).expect("node count fits u64")
        );
    }
}

fn run_deep_semantic_type_boundary(depth: usize, seed: u64) {
    let mut ty = SemanticType::I64;
    for _ in 0..depth {
        ty = SemanticType::List(Box::new(ty));
    }
    let cloned = ty.clone();
    assert_eq!(cloned, ty);
    let display = cloned.to_string();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&cloned, &mut hasher);
    let _hash = std::hash::Hasher::finish(&hasher);
    assert!(display.ends_with("i64"));
    assert_eq!(display.matches("list ").count(), depth);

    let mut workspace = Workspace::empty_deterministic(seed).expect("empty type workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateFunction {
                name: "deep-type".to_owned(),
                type_parameters: Vec::new(),
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: DeclarationType::try_from(&ty).expect("declaration type"),
                }],
                return_type: DeclarationType::try_from(&ty).expect("declaration type"),
            }],
        })
        .expect("publish deep semantic type");
    let function = entity_named(&created.snapshot, EntityKind::Function, "deep-type");
    let signature = created
        .snapshot
        .function_signature(created.snapshot.revision(), function)
        .expect("query deep signature");
    assert_eq!(signature.parameters[0].ty, ty);
    assert_eq!(signature.result, ty);
    created
        .snapshot
        .check_consistency()
        .expect("validate deep semantic type");
    let projection = created.snapshot.project(&[]).expect("project deep blocker");
    assert_eq!(projection.matches("list ").count(), depth);
}

fn run_deep_generic_declaration_type_boundary(depth: usize, seed: u64) {
    let parameter = DraftTypeParameterId::new(0);
    let mut ty = DeclarationType::DraftTypeParameter(parameter);
    for _ in 0..depth {
        ty = DeclarationType::List(Box::new(ty));
    }
    let cloned = ty.clone();
    assert_eq!(cloned, ty);
    let debug = format!("{cloned:?}");
    assert_eq!(debug.matches("list ").count(), depth);

    let mut workspace = Workspace::empty_deterministic(seed).expect("empty generic type workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateFunction {
                name: "deep-generic".to_owned(),
                type_parameters: vec![TypeParameterDraft {
                    id: parameter,
                    name: "t".to_owned(),
                    bounds: Vec::new(),
                }],
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: cloned,
                }],
                return_type: ty,
            }],
        })
        .expect("publish deep generic declaration type");
    let function = entity_named(&created.snapshot, EntityKind::Function, "deep-generic");
    let signature = created
        .snapshot
        .function_signature(created.snapshot.revision(), function)
        .expect("query deep generic signature");
    let stable_parameter = signature.type_parameters[0].id;
    let mut expected = SemanticType::TypeParameter(stable_parameter);
    for _ in 0..depth {
        expected = SemanticType::List(Box::new(expected));
    }
    assert_eq!(signature.parameters[0].ty, expected);
    assert_eq!(signature.result, expected);
    let projection = created
        .snapshot
        .project(&[
            ProjectionSlice::Entity(function),
            ProjectionSlice::Body(function),
        ])
        .expect("project deep generic declaration");
    assert!(projection.contains("type-parameter"));
    assert!(projection.matches("list ").count() >= depth);
}

#[test]
fn modest_generic_declaration_types_are_stack_safe() {
    std::thread::Builder::new()
        .name("workspace-modest-generic-declaration-type".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_deep_generic_declaration_type_boundary(256, 194))
        .expect("spawn generic declaration type boundary")
        .join()
        .expect("generic declaration type boundary completes");
}

#[test]
#[ignore = "20k-level generic declaration type small-stack stress geometry"]
fn twenty_thousand_level_generic_declaration_types_are_stack_safe() {
    std::thread::Builder::new()
        .name("workspace-deep-generic-declaration-type".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_deep_generic_declaration_type_boundary(20_000, 193))
        .expect("spawn deep generic declaration type boundary")
        .join()
        .expect("deep generic declaration type boundary completes");
}

#[test]
fn modest_semantic_type_operations_are_stack_safe() {
    std::thread::Builder::new()
        .name("workspace-modest-semantic-type".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_deep_semantic_type_boundary(256, 206))
        .expect("spawn semantic type boundary")
        .join()
        .expect("semantic type boundary completes");
}

#[test]
#[ignore = "20k-level type-only public boundary small-stack stress geometry"]
fn twenty_thousand_level_semantic_type_operations_are_stack_safe() {
    std::thread::Builder::new()
        .name("workspace-deep-semantic-type".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_deep_semantic_type_boundary(20_000, 207))
        .expect("spawn semantic type boundary")
        .join()
        .expect("semantic type boundary completes");
}

#[test]
fn modest_source_free_depth_compiles_executes_and_drops_on_small_stack() {
    std::thread::Builder::new()
        .name("workspace-modest-source-free".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_source_free_deep(128, 26))
        .expect("spawn small-stack edit")
        .join()
        .expect("small-stack edit completes");
}

#[test]
#[ignore = "20k-node locked-release source-free small-stack stress geometry"]
fn twenty_thousand_level_source_free_compile_execute_and_drop_on_small_stack() {
    std::thread::Builder::new()
        .name("workspace-deep-source-free".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_source_free_deep(20_000, 27))
        .expect("spawn small-stack edit")
        .join()
        .expect("small-stack edit completes");
}

fn deep_match_draft(
    depth: usize,
    some: EntityId,
    none: EntityId,
    value_field: EntityId,
) -> ExpressionDraft {
    assert!(depth > 0, "deep match geometry requires one match");
    let some_pattern = || {
        PatternDraft::new(
            vec![
                DraftPatternNode::Wildcard,
                DraftPatternNode::EnumVariant {
                    variant: some,
                    fields: vec![DraftPatternField {
                        field: value_field,
                        pattern: DraftPatternNodeId::new(0),
                    }],
                },
            ],
            DraftPatternNodeId::new(1),
        )
    };
    let none_pattern = || {
        PatternDraft::new(
            vec![DraftPatternNode::EnumVariant {
                variant: none,
                fields: Vec::new(),
            }],
            DraftPatternNodeId::new(0),
        )
    };
    let mut nodes = Vec::new();
    nodes
        .try_reserve(
            depth
                .checked_mul(4)
                .and_then(|count| count.checked_add(1))
                .expect("match draft geometry"),
        )
        .expect("match draft allocation");
    nodes.push(DraftNode::I64(1));
    nodes.push(DraftNode::EnumValue {
        variant: some,
        fields: vec![DraftFieldValue {
            field: value_field,
            value: DraftNodeId::new(0),
        }],
    });
    let mut scrutinee = DraftNodeId::new(1);
    for level in 1..depth {
        let payload = DraftNodeId::new(u64::try_from(nodes.len()).expect("payload id"));
        nodes.push(DraftNode::I64(i64::try_from(level).expect("payload value")));
        let some_value = DraftNodeId::new(u64::try_from(nodes.len()).expect("some value id"));
        nodes.push(DraftNode::EnumValue {
            variant: some,
            fields: vec![DraftFieldValue {
                field: value_field,
                value: payload,
            }],
        });
        let none_value = DraftNodeId::new(u64::try_from(nodes.len()).expect("none value id"));
        nodes.push(DraftNode::EnumValue {
            variant: none,
            fields: Vec::new(),
        });
        let next = DraftNodeId::new(u64::try_from(nodes.len()).expect("match id"));
        nodes.push(DraftNode::Match {
            scrutinee,
            arms: vec![
                MatchArmDraft {
                    pattern: some_pattern(),
                    body: some_value,
                },
                MatchArmDraft {
                    pattern: none_pattern(),
                    body: none_value,
                },
            ],
        });
        scrutinee = next;
    }
    let one = DraftNodeId::new(u64::try_from(nodes.len()).expect("result id"));
    nodes.push(DraftNode::I64(1));
    let zero = DraftNodeId::new(u64::try_from(nodes.len()).expect("alternative id"));
    nodes.push(DraftNode::I64(0));
    let root = DraftNodeId::new(u64::try_from(nodes.len()).expect("root match id"));
    nodes.push(DraftNode::Match {
        scrutinee,
        arms: vec![
            MatchArmDraft {
                pattern: some_pattern(),
                body: one,
            },
            MatchArmDraft {
                pattern: none_pattern(),
                body: zero,
            },
        ],
    });
    ExpressionDraft::new(nodes, root)
}

fn run_source_free_deep_match(depth: usize, seed: u64) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("deep match workspace");
    let (_choice, some, none, value_field) = create_choice(&mut workspace);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create deep match main");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    super::index::reset_root_address_lookups();
    super::transaction::reset_pattern_lowering_node_visits();
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: deep_match_draft(depth, some, none, value_field),
            }],
        })
        .expect("fill deep match main");
    assert_eq!(completed.snapshot.nodes().len(), depth * 4 + 1);
    assert_eq!(completed.snapshot.program.match_plans.len(), depth);
    assert_eq!(
        super::transaction::pattern_lowering_node_visits(),
        u64::try_from(depth * 3).expect("pattern visit count")
    );
    assert_eq!(
        super::index::root_address_lookups(),
        u64::try_from(depth * 4 + 1).expect("lookup count")
    );
    assert!(completed
        .snapshot
        .program
        .match_plans
        .iter()
        .all(|plan| plan.tests.len() == 2
            && plan.projections.is_empty()
            && plan.bindings.is_empty()));
    let site = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Match)
        .expect("deep match site")
        .id;
    assert_eq!(
        completed
            .snapshot
            .match_view(completed.snapshot.revision(), site)
            .expect("deep match view")
            .arms
            .len(),
        2
    );
    let projection = completed
        .snapshot
        .project(&[ProjectionSlice::Body(main), ProjectionSlice::Match(site)])
        .expect("deep match projection");
    assert_eq!(projection.matches("kind=match ").count(), depth);
    crate::codegen::reset_nonowned_structural_work();
    let executable = crate::compile_snapshot(&completed.snapshot).expect("compile deep match");
    let ssa = executable.ssa().program();
    let expected_edges = ssa
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .map(|block| match block.terminator {
            lkjscript_ir::Terminator::Branch { .. } => 1_u64,
            lkjscript_ir::Terminator::ConditionalBranch { .. } => 2,
            lkjscript_ir::Terminator::Return(_)
            | lkjscript_ir::Terminator::Trap { .. }
            | lkjscript_ir::Terminator::Exit { .. }
            | lkjscript_ir::Terminator::Outcome { .. } => 0,
        })
        .sum::<u64>();
    assert_eq!(
        crate::codegen::nonowned_structural_work(),
        (
            u64::try_from(ssa.functions.len()).expect("SSA function count"),
            expected_edges,
        )
    );
    assert!(matches!(
        run_chunk(
            executable.bytecode(),
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        ),
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(1)
    ));
}

#[test]
fn modest_source_free_enum_match_depth_compiles_executes_and_drops_on_small_stack() {
    std::thread::Builder::new()
        .name("workspace-modest-source-free-match".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_source_free_deep_match(128, 84))
        .expect("spawn small-stack match edit")
        .join()
        .expect("small-stack match edit completes");
}

#[test]
#[ignore = "20k-level locked-release source-free enum-match small-stack stress geometry"]
fn twenty_thousand_level_source_free_enum_matches_compile_execute_and_drop_on_small_stack() {
    std::thread::Builder::new()
        .name("workspace-deep-source-free-match".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_source_free_deep_match(20_000, 85))
        .expect("spawn small-stack match edit")
        .join()
        .expect("small-stack match edit completes");
}

#[test]
fn pagination_and_semantic_diff_are_deterministic() {
    let first = importer::import_source_with_namespace(
        FUNCTION_PROGRAM,
        "workspace-deterministic.lkjscript",
        WorkspaceNamespace::deterministic(24),
    )
    .expect("first import");
    let second = importer::import_source_with_namespace(
        FUNCTION_PROGRAM,
        "workspace-deterministic.lkjscript",
        WorkspaceNamespace::deterministic(24),
    )
    .expect("second import");
    let collect = |snapshot: &WorkspaceSnapshot| {
        let request = PageRequest::new(2).expect("page");
        let mut cursor = None;
        let mut ids = Vec::new();
        loop {
            let page = snapshot
                .entity_page(snapshot.revision(), request, cursor.as_ref())
                .expect("entity page");
            ids.extend(page.items.iter().map(|entity| entity.id));
            cursor = page.continuation;
            if cursor.is_none() {
                break;
            }
        }
        ids
    };
    assert_eq!(collect(&first), collect(&second));

    let edit = |snapshot: WorkspaceSnapshot| {
        let root = snapshot.nodes()[0].id;
        let revision = snapshot.revision();
        let mut workspace = Workspace::new(snapshot).expect("workspace");
        workspace
            .apply(Transaction {
                base_revision: revision,
                edits: vec![Edit::ReplaceExpression {
                    target: root,
                    draft: ExpressionDraft::scalar_i64(5),
                }],
            })
            .expect("edit")
            .diff
    };
    assert_eq!(edit(first), edit(second));
}

fn entity_named(snapshot: &WorkspaceSnapshot, kind: EntityKind, name: &str) -> EntityId {
    snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == kind && entity.name.as_ref() == name)
        .unwrap_or_else(|| panic!("missing {kind:?} {name}"))
        .id
}

fn create_pair(workspace: &mut Workspace) -> (EntityId, EntityId, EntityId) {
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateProduct {
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
            }],
        })
        .expect("create pair product");
    (
        entity_named(&created.snapshot, EntityKind::Product, "pair"),
        entity_named(&created.snapshot, EntityKind::ProductField, "left"),
        entity_named(&created.snapshot, EntityKind::ProductField, "right"),
    )
}

fn create_choice(workspace: &mut Workspace) -> (EntityId, EntityId, EntityId, EntityId) {
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateEnum {
                name: "choice".to_owned(),
                variants: vec![
                    EnumVariantDraft {
                        name: "some".to_owned(),
                        fields: vec![EnumFieldDraft {
                            name: "value".to_owned(),
                            ty: SemanticType::I64,
                        }],
                    },
                    EnumVariantDraft {
                        name: "none".to_owned(),
                        fields: Vec::new(),
                    },
                ],
            }],
        })
        .expect("create choice enum");
    (
        entity_named(&created.snapshot, EntityKind::Enum, "choice"),
        entity_named(&created.snapshot, EntityKind::EnumVariant, "some"),
        entity_named(&created.snapshot, EntityKind::EnumVariant, "none"),
        entity_named(&created.snapshot, EntityKind::EnumField, "value"),
    )
}

#[test]
fn product_deletion_cascades_fields_compacts_dense_ids_and_preserves_survivors() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let mut workspace = Workspace::empty_deterministic(150).expect("product deletion workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateProduct {
                    name: "remove-first".to_owned(),
                    fields: vec![ProductFieldDraft {
                        name: "first-value".to_owned(),
                        ty: SemanticType::I64,
                    }],
                },
                Edit::CreateProduct {
                    name: "remove-middle".to_owned(),
                    fields: vec![ProductFieldDraft {
                        name: "middle-value".to_owned(),
                        ty: SemanticType::Bool,
                    }],
                },
                Edit::CreateProduct {
                    name: "keep".to_owned(),
                    fields: vec![ProductFieldDraft {
                        name: "kept-value".to_owned(),
                        ty: SemanticType::I64,
                    }],
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create products");
    let remove_first = entity_named(&created.snapshot, EntityKind::Product, "remove-first");
    let first_field = entity_named(&created.snapshot, EntityKind::ProductField, "first-value");
    let remove_middle = entity_named(&created.snapshot, EntityKind::Product, "remove-middle");
    let middle_field = entity_named(&created.snapshot, EntityKind::ProductField, "middle-value");
    let keep = entity_named(&created.snapshot, EntityKind::Product, "keep");
    let kept_field = entity_named(&created.snapshot, EntityKind::ProductField, "kept-value");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;

    let unchanged = workspace.current();
    for invalid in [
        Transaction {
            base_revision: unchanged.revision(),
            edits: vec![Edit::DeleteEntity {
                entity: first_field,
            }],
        },
        Transaction {
            base_revision: unchanged.revision(),
            edits: vec![
                Edit::DeleteEntity {
                    entity: remove_first,
                },
                Edit::DeleteEntity {
                    entity: remove_first,
                },
            ],
        },
        Transaction {
            base_revision: unchanged.revision(),
            edits: vec![
                Edit::DeleteEntity {
                    entity: remove_first,
                },
                Edit::CreateProduct {
                    name: "remove-first".to_owned(),
                    fields: Vec::new(),
                },
            ],
        },
    ] {
        assert!(workspace.apply(invalid).is_err());
        assert!(Arc::ptr_eq(&unchanged, &workspace.current()));
    }

    let completed = workspace
        .apply(Transaction {
            base_revision: unchanged.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::ProductValue {
                            product: keep,
                            fields: vec![DraftFieldValue {
                                field: kept_field,
                                value: DraftNodeId::new(0),
                            }],
                        },
                        DraftNode::ProductField {
                            field: kept_field,
                            value: DraftNodeId::new(1),
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("construct and project retained product");
    assert_eq!(run_i64(&completed.snapshot), 42);
    let main_nodes: Vec<_> = completed
        .snapshot
        .nodes()
        .iter()
        .filter(|node| {
            node.owner == SemanticOwner::Entity(main)
                || matches!(node.owner, SemanticOwner::Node(_))
        })
        .map(|node| node.id)
        .collect();
    let main_root = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.owner == SemanticOwner::Entity(main))
        .expect("main root")
        .id;
    let old_definition = completed.snapshot.program.products[2].clone();
    let old = completed.snapshot;

    let deleted = workspace
        .apply(Transaction {
            base_revision: old.revision(),
            edits: vec![
                Edit::DeleteEntity {
                    entity: remove_middle,
                },
                Edit::DeleteEntity {
                    entity: remove_first,
                },
            ],
        })
        .expect("delete two earlier products");
    assert_eq!(run_i64(&deleted.snapshot), 42);
    assert_eq!(deleted.snapshot.program.products.len(), 1);
    assert_eq!(deleted.snapshot.program.products[0].id.raw(), 0);
    assert_eq!(deleted.snapshot.program.products[0].name, "keep");
    assert_eq!(
        deleted.snapshot.program.products[0].identity,
        old_definition.identity
    );
    assert_eq!(
        deleted.snapshot.program.products[0].fields[0].identity,
        old_definition.fields[0].identity
    );
    assert_eq!(
        deleted.snapshot.program.products[0].fields[0].source_order,
        old_definition.fields[0].source_order
    );
    assert_eq!(
        deleted.snapshot.entity(keep).expect("retained product").id,
        keep
    );
    assert_eq!(
        deleted
            .snapshot
            .entity(kept_field)
            .expect("retained product field")
            .id,
        kept_field
    );
    for node in &main_nodes {
        assert_eq!(
            deleted.snapshot.node(*node).expect("retained node").id,
            *node
        );
    }
    for removed in [remove_first, first_field, remove_middle, middle_field] {
        assert!(deleted.snapshot.entity(removed).is_err());
        assert_eq!(old.entity(removed).expect("old entity remains").id, removed);
        assert!(deleted.diff.entries.iter().any(|entry| matches!(
            entry,
            SemanticDiffEntry::EntityDeleted { entity, .. } if *entity == removed
        )));
    }
    assert!(deleted
        .snapshot
        .project(&[ProjectionSlice::Entity(remove_first)])
        .is_err());
    assert!(deleted.snapshot.dependencies().iter().all(|edge| ![
        remove_first,
        first_field,
        remove_middle,
        middle_field
    ]
    .contains(&edge.dependency)));
    assert_eq!(run_i64(&old), 42);

    let recreated = workspace
        .apply(Transaction {
            base_revision: deleted.snapshot.revision(),
            edits: vec![Edit::CreateProduct {
                name: "remove-first".to_owned(),
                fields: vec![ProductFieldDraft {
                    name: "recreated-value".to_owned(),
                    ty: SemanticType::I64,
                }],
            }],
        })
        .expect("recreate a deleted product name later");
    let recreated_product = entity_named(&recreated.snapshot, EntityKind::Product, "remove-first");
    assert_ne!(recreated_product, remove_first);
    assert!(recreated.snapshot.entity(remove_first).is_err());

    let blocked = workspace.apply(Transaction {
        base_revision: recreated.snapshot.revision(),
        edits: vec![Edit::DeleteEntity { entity: keep }],
    });
    assert!(matches!(
        blocked,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    let before_final = workspace.current();
    let final_state = workspace
        .apply(Transaction {
            base_revision: before_final.revision(),
            edits: vec![
                Edit::DeleteEntity { entity: keep },
                Edit::ReplaceExpression {
                    target: main_root,
                    draft: ExpressionDraft::scalar_i64(42),
                },
            ],
        })
        .expect("remove final body dependency and product atomically");
    assert!(final_state.snapshot.entity(keep).is_err());
    assert!(final_state.snapshot.entity(kept_field).is_err());
    assert_eq!(
        final_state
            .snapshot
            .node(main_root)
            .expect("replacement root identity")
            .id,
        main_root
    );
    assert_eq!(run_i64(&final_state.snapshot), 42);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn enum_deletion_cascades_members_and_preserves_stable_nominal_layout_identity() {
    let mut workspace = Workspace::empty_deterministic(151).expect("enum deletion workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateEnum {
                    name: "remove-enum".to_owned(),
                    variants: vec![EnumVariantDraft {
                        name: "removed-variant".to_owned(),
                        fields: vec![EnumFieldDraft {
                            name: "removed-field".to_owned(),
                            ty: SemanticType::I64,
                        }],
                    }],
                },
                Edit::CreateEnum {
                    name: "middle-enum".to_owned(),
                    variants: vec![EnumVariantDraft {
                        name: "middle-variant".to_owned(),
                        fields: Vec::new(),
                    }],
                },
                Edit::CreateEnum {
                    name: "kept-enum".to_owned(),
                    variants: vec![EnumVariantDraft {
                        name: "kept-variant".to_owned(),
                        fields: vec![EnumFieldDraft {
                            name: "kept-field".to_owned(),
                            ty: SemanticType::Bool,
                        }],
                    }],
                },
                Edit::CreateMain {
                    return_type: SemanticType::Bool,
                },
            ],
        })
        .expect("create enums");
    let removed = entity_named(&created.snapshot, EntityKind::Enum, "remove-enum");
    let removed_variant = entity_named(
        &created.snapshot,
        EntityKind::EnumVariant,
        "removed-variant",
    );
    let removed_field = entity_named(&created.snapshot, EntityKind::EnumField, "removed-field");
    let middle = entity_named(&created.snapshot, EntityKind::Enum, "middle-enum");
    let middle_variant = entity_named(&created.snapshot, EntityKind::EnumVariant, "middle-variant");
    let kept = entity_named(&created.snapshot, EntityKind::Enum, "kept-enum");
    let kept_variant = entity_named(&created.snapshot, EntityKind::EnumVariant, "kept-variant");
    let kept_field = entity_named(&created.snapshot, EntityKind::EnumField, "kept-field");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("enum main hole")
        .id;
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Bool(true),
                        DraftNode::EnumValue {
                            variant: kept_variant,
                            fields: vec![DraftFieldValue {
                                field: kept_field,
                                value: DraftNodeId::new(0),
                            }],
                        },
                        DraftNode::EnumIsVariant {
                            variant: kept_variant,
                            value: DraftNodeId::new(1),
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("complete retained enum program");
    assert!(run_bool(&completed.snapshot));
    let retained_nodes: Vec<_> = completed
        .snapshot
        .nodes()
        .iter()
        .map(|node| node.id)
        .collect();
    let old_definition = completed
        .snapshot
        .program
        .enums
        .iter()
        .find(|definition| definition.name == "kept-enum")
        .expect("kept enum definition")
        .clone();
    let old = completed.snapshot;

    for member in [removed_variant, removed_field] {
        let direct_member = workspace.apply(Transaction {
            base_revision: old.revision(),
            edits: vec![Edit::DeleteEntity { entity: member }],
        });
        assert!(matches!(
            direct_member,
            Err(WorkspaceError::UnsupportedEdit { .. })
        ));
        assert!(Arc::ptr_eq(&old, &workspace.current()));
    }
    let redundant = workspace.apply(Transaction {
        base_revision: old.revision(),
        edits: vec![
            Edit::DeleteEntity { entity: removed },
            Edit::DeleteEntity {
                entity: removed_field,
            },
        ],
    });
    assert!(matches!(
        redundant,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&old, &workspace.current()));
    let same_name = workspace.apply(Transaction {
        base_revision: old.revision(),
        edits: vec![
            Edit::DeleteEntity { entity: removed },
            Edit::CreateEnum {
                name: "remove-enum".to_owned(),
                variants: vec![EnumVariantDraft {
                    name: "replacement".to_owned(),
                    fields: Vec::new(),
                }],
            },
        ],
    });
    assert!(matches!(
        same_name,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&old, &workspace.current()));

    let deleted = workspace
        .apply(Transaction {
            base_revision: old.revision(),
            edits: vec![
                Edit::DeleteEntity { entity: middle },
                Edit::DeleteEntity { entity: removed },
            ],
        })
        .expect("delete earlier enums");
    assert!(run_bool(&deleted.snapshot));
    for node in retained_nodes {
        assert_eq!(
            deleted.snapshot.node(node).expect("retained enum node").id,
            node
        );
    }
    assert_eq!(deleted.snapshot.program.enums.len(), 1);
    let retained = &deleted.snapshot.program.enums[0];
    assert_eq!(retained.id, old_definition.id);
    assert_eq!(retained.layout, old_definition.layout);
    assert_eq!(retained.variants[0].id, old_definition.variants[0].id);
    assert_eq!(
        retained.variants[0].fields[0].id,
        old_definition.variants[0].fields[0].id
    );
    for survivor in [kept, kept_variant, kept_field] {
        assert_eq!(
            deleted.snapshot.entity(survivor).expect("survivor").id,
            survivor
        );
    }
    for removed_entity in [
        removed,
        removed_variant,
        removed_field,
        middle,
        middle_variant,
    ] {
        assert!(deleted.snapshot.entity(removed_entity).is_err());
        assert_eq!(
            old.entity(removed_entity).expect("old enum entity").id,
            removed_entity
        );
        assert!(deleted.diff.entries.iter().any(|entry| matches!(
            entry,
            SemanticDiffEntry::EntityDeleted { entity, .. } if *entity == removed_entity
        )));
    }
    assert_eq!(
        deleted
            .snapshot
            .entity_type(deleted.snapshot.revision(), kept)
            .expect("retained enum type")
            .declared,
        Some(SemanticType::Enum {
            constructor: SemanticEnum::Entity(kept),
            arguments: Vec::new()
        })
    );
    let recreated = workspace
        .apply(Transaction {
            base_revision: deleted.snapshot.revision(),
            edits: vec![Edit::CreateEnum {
                name: "remove-enum".to_owned(),
                variants: vec![EnumVariantDraft {
                    name: "replacement".to_owned(),
                    fields: Vec::new(),
                }],
            }],
        })
        .expect("recreate enum name later");
    let recreated_id = entity_named(&recreated.snapshot, EntityKind::Enum, "remove-enum");
    assert_ne!(recreated_id, removed);
    assert!(recreated.snapshot.entity(removed).is_err());
}

#[test]
fn nominal_creation_and_deletion_share_one_compaction_and_forced_identity_boundary() {
    let mut workspace = Workspace::empty_deterministic(156).expect("mixed nominal workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateProduct {
                    name: "old-product".to_owned(),
                    fields: vec![ProductFieldDraft {
                        name: "old-product-field".to_owned(),
                        ty: SemanticType::I64,
                    }],
                },
                Edit::CreateEnum {
                    name: "old-enum".to_owned(),
                    variants: vec![EnumVariantDraft {
                        name: "old-variant".to_owned(),
                        fields: Vec::new(),
                    }],
                },
            ],
        })
        .expect("create old nominals");
    let old_product = entity_named(&created.snapshot, EntityKind::Product, "old-product");
    let old_enum = entity_named(&created.snapshot, EntityKind::Enum, "old-enum");
    let published = created.snapshot;
    let apply = |creation_first: bool| {
        let deletions = vec![
            Edit::DeleteEntity {
                entity: old_product,
            },
            Edit::DeleteEntity { entity: old_enum },
        ];
        let creations = vec![
            Edit::CreateProduct {
                name: "new-product".to_owned(),
                fields: vec![ProductFieldDraft {
                    name: "new-product-field".to_owned(),
                    ty: SemanticType::Bool,
                }],
            },
            Edit::CreateEnum {
                name: "new-enum".to_owned(),
                variants: vec![EnumVariantDraft {
                    name: "new-variant".to_owned(),
                    fields: vec![EnumFieldDraft {
                        name: "new-enum-field".to_owned(),
                        ty: SemanticType::I64,
                    }],
                }],
            },
        ];
        let edits = if creation_first {
            creations.into_iter().chain(deletions).collect()
        } else {
            deletions.into_iter().chain(creations).collect()
        };
        let mut workspace = Workspace::new((*published).clone()).expect("ordered workspace");
        workspace
            .apply(Transaction {
                base_revision: published.revision(),
                edits,
            })
            .expect("mixed nominal lifecycle")
    };
    let first = apply(true);
    let second = apply(false);
    assert_eq!(first.diff, second.diff);
    assert_eq!(first.snapshot.entities(), second.snapshot.entities());
    assert!(first.snapshot.entity(old_product).is_err());
    assert!(first.snapshot.entity(old_enum).is_err());
    assert_eq!(first.snapshot.program.products.len(), 1);
    assert_eq!(first.snapshot.program.products[0].id.raw(), 0);
    assert_eq!(first.snapshot.program.products[0].name, "new-product");
    assert_eq!(first.snapshot.program.enums.len(), 1);
    assert_eq!(first.snapshot.program.enums[0].name, "new-enum");
    let new_product = entity_named(&first.snapshot, EntityKind::Product, "new-product");
    let new_enum = entity_named(&first.snapshot, EntityKind::Enum, "new-enum");
    assert_eq!(
        first
            .snapshot
            .entity_type(first.snapshot.revision(), new_product)
            .expect("new product type")
            .declared,
        Some(SemanticType::Product(new_product))
    );
    assert_eq!(
        first
            .snapshot
            .entity_type(first.snapshot.revision(), new_enum)
            .expect("new enum type")
            .declared,
        Some(SemanticType::Enum {
            constructor: SemanticEnum::Entity(new_enum),
            arguments: Vec::new()
        })
    );
}

#[test]
fn nominal_deletion_dependencies_use_the_final_staged_state_in_any_edit_order() {
    let (workspace, choice, some, none, value_field) = source_free_choice_match(152);
    let published = workspace.current();
    let main = entity_named(&published, EntityKind::Main, "main");
    let match_node = published
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Match)
        .expect("match node")
        .id;
    let mut blocked_workspace = Workspace::new((*published).clone()).expect("blocked workspace");
    let blocked_before = blocked_workspace.current();
    let blocked = blocked_workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::DeleteEntity { entity: choice }],
    });
    let message = blocked
        .expect_err("surviving match blocks enum deletion")
        .to_string();
    assert!(message.contains("cannot delete enum"), "{message}");
    assert!(message.contains("surviving main"), "{message}");
    assert!(Arc::ptr_eq(&blocked_before, &blocked_workspace.current()));

    let apply_order = |deletion_first: bool| {
        let mut workspace = Workspace::new((*published).clone()).expect("ordered workspace");
        let replacement = Edit::ReplaceExpression {
            target: match_node,
            draft: ExpressionDraft::scalar_i64(42),
        };
        let deletion = Edit::DeleteEntity { entity: choice };
        let edits = if deletion_first {
            vec![deletion, replacement]
        } else {
            vec![replacement, deletion]
        };
        workspace
            .apply(Transaction {
                base_revision: published.revision(),
                edits,
            })
            .expect("dependency-closed enum deletion")
    };
    let first = apply_order(true);
    let second = apply_order(false);
    assert_eq!(first.diff, second.diff);
    assert_eq!(first.snapshot.entities(), second.snapshot.entities());
    assert_eq!(
        first.snapshot.dependencies(),
        second.snapshot.dependencies()
    );
    let mut removed_reference_targets: Vec<_> = first
        .diff
        .entries
        .iter()
        .filter_map(|entry| match entry {
            SemanticDiffEntry::ReferenceRewired {
                site,
                old_target: Some(target),
                new_target: None,
            } if *site == match_node => Some(*target),
            _ => None,
        })
        .collect();
    removed_reference_targets.sort_unstable();
    let mut expected_reference_targets = vec![choice, some, none, value_field];
    expected_reference_targets.sort_unstable();
    assert_eq!(removed_reference_targets, expected_reference_targets);
    assert_eq!(run_i64(&first.snapshot), 42);
    for removed in [choice, some, none, value_field] {
        assert!(first.snapshot.entity(removed).is_err());
    }
    assert!(first.snapshot.program.match_plans.is_empty());
    assert!(first
        .snapshot
        .references()
        .iter()
        .all(|edge| { ![choice, some, none, value_field].contains(&edge.target) }));
    assert!(first
        .snapshot
        .project(&[ProjectionSlice::Body(main)])
        .expect("surviving projection")
        .contains("kind=literal"));

    let mut hole_workspace = Workspace::empty_deterministic(153).expect("nominal hole workspace");
    let (hole_choice, ..) = create_choice(&mut hole_workspace);
    let with_main = hole_workspace
        .apply(Transaction {
            base_revision: hole_workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::Enum {
                    constructor: SemanticEnum::Entity(hole_choice),
                    arguments: Vec::new(),
                },
            }],
        })
        .expect("create enum-typed main hole");
    let hole_main = entity_named(&with_main.snapshot, EntityKind::Main, "main");
    assert!(with_main
        .snapshot
        .dependencies()
        .iter()
        .any(|edge| { edge.dependent == hole_main && edge.dependency == hole_choice }));
    let stable = hole_workspace.current();
    assert!(matches!(
        hole_workspace.apply(Transaction {
            base_revision: stable.revision(),
            edits: vec![Edit::DeleteEntity {
                entity: hole_choice,
            }],
        }),
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&stable, &hole_workspace.current()));
    let closed = hole_workspace
        .apply(Transaction {
            base_revision: stable.revision(),
            edits: vec![
                Edit::DeleteEntity {
                    entity: hole_choice,
                },
                Edit::DeleteEntity { entity: hole_main },
            ],
        })
        .expect("delete enum and enum-typed hole owner");
    assert_eq!(
        closed.snapshot.completeness_blockers(),
        &[CompletenessBlocker::MissingEntryPoint]
    );
}

#[test]
fn imported_enum_deletion_preserves_prelude_identities_and_never_reparses() {
    let snapshot = import_source(
        &imported_choice_match_source(),
        "imported-enum-delete.lkjscript",
    )
    .expect("import enum match");
    assert_eq!(run_i64(&snapshot), 42);
    let choice = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Enum && entity.name.ends_with(":choice"))
        .expect("imported choice")
        .id;
    let imported_member = |kind, suffix: &str| {
        snapshot
            .entities()
            .iter()
            .find(|entity| entity.kind == kind && entity.name.ends_with(suffix))
            .unwrap_or_else(|| panic!("missing imported {kind:?} ending in {suffix}"))
            .id
    };
    let some = imported_member(EntityKind::EnumVariant, ":some");
    let none = imported_member(EntityKind::EnumVariant, ":none");
    let value = imported_member(EntityKind::EnumField, "value");
    let match_node = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Match)
        .expect("imported match")
        .id;
    let prelude: Vec<_> = snapshot
        .program
        .enums
        .iter()
        .filter(|definition| definition.origin == crate::hir::Origin::Builtin)
        .map(|definition| (definition.id, definition.layout.clone()))
        .collect();
    assert!(!prelude.is_empty());
    let old = snapshot.clone();
    let mut workspace = Workspace::new(snapshot).expect("imported enum workspace");
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let deleted = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::DeleteEntity { entity: choice },
                Edit::ReplaceExpression {
                    target: match_node,
                    draft: ExpressionDraft::scalar_i64(42),
                },
            ],
        })
        .expect("delete imported enum after removing match");
    assert_eq!(run_i64(&deleted.snapshot), 42);
    let retained: Vec<_> = deleted
        .snapshot
        .program
        .enums
        .iter()
        .map(|definition| (definition.id, definition.layout.clone()))
        .collect();
    assert_eq!(retained, prelude);
    for removed in [choice, some, none, value] {
        assert!(deleted.snapshot.entity(removed).is_err());
        assert_eq!(old.entity(removed).expect("old enum member").id, removed);
    }
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn nominal_signature_and_field_dependencies_require_dependency_closed_batch_deletion() {
    let mut workspace = Workspace::empty_deterministic(155).expect("nominal dependency workspace");
    let root_created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateProduct {
                name: "dependency-root".to_owned(),
                fields: Vec::new(),
            }],
        })
        .expect("create dependency root");
    let root = entity_named(
        &root_created.snapshot,
        EntityKind::Product,
        "dependency-root",
    );
    let dependents = workspace
        .apply(Transaction {
            base_revision: root_created.snapshot.revision(),
            edits: vec![
                Edit::CreateProduct {
                    name: "product-dependent".to_owned(),
                    fields: vec![ProductFieldDraft {
                        name: "product-reference".to_owned(),
                        ty: SemanticType::Product(root),
                    }],
                },
                Edit::CreateEnum {
                    name: "enum-dependent".to_owned(),
                    variants: vec![EnumVariantDraft {
                        name: "holding".to_owned(),
                        fields: vec![EnumFieldDraft {
                            name: "enum-reference".to_owned(),
                            ty: SemanticType::Product(root),
                        }],
                    }],
                },
                Edit::CreateFunction {
                    name: "function-dependent".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "input".to_owned(),
                        ty: DeclarationType::Product(root),
                    }],
                    return_type: DeclarationType::Product(root),
                },
            ],
        })
        .expect("create nominal dependents");
    let product = entity_named(
        &dependents.snapshot,
        EntityKind::Product,
        "product-dependent",
    );
    let enumeration = entity_named(&dependents.snapshot, EntityKind::Enum, "enum-dependent");
    let function = entity_named(
        &dependents.snapshot,
        EntityKind::Function,
        "function-dependent",
    );
    let published = dependents.snapshot;
    let mut blocked_workspace = Workspace::new((*published).clone()).expect("blocked workspace");
    let before = blocked_workspace.current();
    let error = blocked_workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::DeleteEntity { entity: root }],
        })
        .expect_err("surviving field and signature dependencies block deletion")
        .to_string();
    assert!(error.contains("dependency-root"), "{error}");
    assert!(
        error.contains("field type") || error.contains("signature type"),
        "{error}"
    );
    assert!(Arc::ptr_eq(&before, &blocked_workspace.current()));

    let delete = |reverse: bool| {
        let mut workspace = Workspace::new((*published).clone()).expect("ordered workspace");
        let mut edits = vec![
            Edit::DeleteEntity { entity: root },
            Edit::DeleteEntity { entity: product },
            Edit::DeleteEntity {
                entity: enumeration,
            },
            Edit::DeleteEntity { entity: function },
        ];
        if reverse {
            edits.reverse();
        }
        workspace
            .apply(Transaction {
                base_revision: published.revision(),
                edits,
            })
            .expect("delete complete nominal dependency closure")
    };
    let forward = delete(false);
    let reverse = delete(true);
    assert_eq!(forward.diff, reverse.diff);
    assert_eq!(forward.snapshot.entities(), reverse.snapshot.entities());
    for removed in [root, product, enumeration, function] {
        assert!(forward.snapshot.entity(removed).is_err());
    }
    assert_eq!(
        forward.snapshot.completeness_blockers(),
        &[CompletenessBlocker::MissingEntryPoint]
    );
}

#[test]
fn imported_product_deletion_cascades_implementation_and_remaps_surviving_witness() {
    let source = concat!(
        "trait/\nname/\nmarked\n/name\n/trait\n",
        "product/\nname/\nremove\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n",
        "product/\nname/\nkeep\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n",
        "impl/\ntrait/\nmarked\n/trait\nfor/\nproduct\nremove\n/for\n/impl\n",
        "impl/\ntrait/\nmarked\n/trait\nfor/\nproduct\nkeep\n/for\n/impl\n",
        "def/\nname/\nkeep-marked\n/name\nfn/\nforall/\nt\n/forall\nbounds/\nbound/\nt\nmarked\n/bound\n/bounds\nsig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\nparams/\nvalue\nt\n/params\nvalue\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nfield/\nkeep-marked/\nproduct-value/\nkeep\nfield/\nvalue\n42\n/field\n/product-value\n/keep-marked\nvalue\n/field\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "nominal-implementation-delete.lkjscript",
        WorkspaceNamespace::deterministic(154),
    )
    .expect("import implemented products");
    let blocked_source = source.replacen("product-value/\nkeep", "product-value/\nremove", 1);
    let blocked_snapshot = importer::import_source_with_namespace(
        &blocked_source,
        "nominal-implementation-blocked.lkjscript",
        WorkspaceNamespace::deterministic(157),
    )
    .expect("import call witnessing the removed implementation");
    let blocked_product = blocked_snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Product && entity.name.ends_with(":remove"))
        .expect("blocked removed product")
        .id;
    let mut blocked_workspace = Workspace::new(blocked_snapshot).expect("blocked impl workspace");
    let blocked_error = blocked_workspace
        .apply(Transaction {
            base_revision: blocked_workspace.current().revision(),
            edits: vec![Edit::DeleteEntity {
                entity: blocked_product,
            }],
        })
        .expect_err("surviving explicit witness must block implementation cascade")
        .to_string();
    assert!(
        blocked_error.contains("explicit implementation witness"),
        "{blocked_error}"
    );
    assert_eq!(run_i64(&snapshot), 42);
    let qualified = |kind, suffix: &str| {
        snapshot
            .entities()
            .iter()
            .find(|entity| entity.kind == kind && entity.name.ends_with(suffix))
            .unwrap_or_else(|| panic!("missing {kind:?} ending in {suffix}"))
            .id
    };
    let remove = qualified(EntityKind::Product, ":remove");
    let keep = qualified(EntityKind::Product, ":keep");
    let trait_entity = qualified(EntityKind::Trait, ":marked");
    let removed_impl = qualified(EntityKind::Implementation, ":remove");
    let kept_impl = qualified(EntityKind::Implementation, ":keep");
    let function = qualified(EntityKind::Function, ":keep-marked");
    let type_parameter = snapshot
        .function_signature(snapshot.revision(), function)
        .expect("generic marked signature")
        .type_parameters[0]
        .id;
    let keep_field = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::ProductField && entity.owner == Some(keep))
        .expect("keep product field")
        .id;
    let call_node = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Call)
        .expect("generic call node")
        .id;
    let old = snapshot.clone();
    let old_call = old
        .call_instantiation(old.revision(), call_node)
        .expect("public generic call witness");
    assert_eq!(
        old_call.type_arguments[0].argument,
        SemanticType::Product(keep)
    );
    assert_eq!(
        old_call.witnesses[0].kind,
        TraitWitnessKindView::Explicit(kept_impl)
    );
    let mut forged = old.validated_complete_hir().expect("complete imported HIR");
    let crate::hir::ExprKind::ProductField { value, .. } = &mut forged.main.body.kind else {
        panic!("main retains product projection")
    };
    let crate::hir::ExprKind::Call {
        instantiation: Some(instantiation),
        ..
    } = &mut value.kind
    else {
        panic!("main retains generic call")
    };
    let explicit = instantiation
        .witnesses
        .iter_mut()
        .find(|witness| matches!(witness.kind, crate::hir::TraitWitnessKind::Explicit(_)))
        .expect("explicit witness");
    explicit.kind = crate::hir::TraitWitnessKind::Explicit(crate::hir::ImplId::new(0));
    let alias_error = super::validate::program(&forged)
        .expect_err("in-range witness for a different product must reject")
        .to_string();
    assert!(alias_error.contains("not canonical"), "{alias_error}");

    let mut workspace = Workspace::new(snapshot).expect("implemented product workspace");
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let edited = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::ReplaceExpression {
                target: call_node,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::ProductValue {
                            product: keep,
                            fields: vec![DraftFieldValue {
                                field: keep_field,
                                value: DraftNodeId::new(0),
                            }],
                        },
                        DraftNode::Call {
                            callee: function,
                            type_arguments: vec![TypeArgumentDraft {
                                parameter: type_parameter,
                                argument: SemanticType::Product(keep),
                            }],
                            arguments: vec![DraftNodeId::new(1)],
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("publish explicit implementation witness call");
    assert_eq!(run_i64(&edited.snapshot), 42);
    let edited_call = edited
        .snapshot
        .call_instantiation(edited.snapshot.revision(), call_node)
        .expect("source-free explicit witness");
    assert_eq!(edited_call.type_arguments, old_call.type_arguments);
    assert_eq!(edited_call.parameters, old_call.parameters);
    assert_eq!(edited_call.result, old_call.result);
    assert_eq!(edited_call.witnesses, old_call.witnesses);
    let deleted = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::DeleteEntity { entity: remove }],
        })
        .expect("delete implemented product");
    assert_eq!(run_i64(&deleted.snapshot), 42);
    assert!(deleted.snapshot.entity(remove).is_err());
    assert!(deleted.snapshot.entity(removed_impl).is_err());
    for survivor in [keep, kept_impl, trait_entity] {
        assert_eq!(
            deleted.snapshot.entity(survivor).expect("survivor").id,
            survivor
        );
    }
    assert_eq!(
        deleted.snapshot.node(call_node).expect("call node").id,
        call_node
    );
    assert_eq!(deleted.snapshot.program.products[0].id.raw(), 0);
    assert!(deleted.snapshot.program.products[0].name.ends_with(":keep"));
    assert_eq!(deleted.snapshot.program.implementations.len(), 1);
    assert_eq!(deleted.snapshot.program.implementations[0].id.raw(), 0);
    assert_eq!(deleted.snapshot.program.implementations[0].product.raw(), 0);
    let main = deleted.snapshot.program.main.as_ref().expect("main");
    let mut pending = vec![&main.body];
    let mut explicit = None;
    while let Some(expression) = pending.pop() {
        if let crate::hir::ExprKind::Call {
            instantiation: Some(instantiation),
            ..
        } = &expression.kind
        {
            explicit = instantiation
                .witnesses
                .iter()
                .find_map(|witness| match witness.kind {
                    crate::hir::TraitWitnessKind::Explicit(implementation) => Some(implementation),
                    crate::hir::TraitWitnessKind::AutoTrait => None,
                });
        }
        crate::hir::for_each_expression_child(expression, &mut |child| pending.push(child));
    }
    assert_eq!(explicit.expect("explicit witness").raw(), 0);
    let public_after = deleted
        .snapshot
        .call_instantiation(deleted.snapshot.revision(), call_node)
        .expect("remapped public call witness");
    assert_eq!(
        public_after.witnesses[0].kind,
        TraitWitnessKindView::Explicit(kept_impl)
    );
    assert_eq!(public_after.type_arguments, old_call.type_arguments);
    assert!(deleted.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::EntityDeleted { entity, .. } if *entity == removed_impl
    )));
    assert_eq!(
        old.entity(removed_impl).expect("old implementation").id,
        removed_impl
    );
    assert_eq!(run_i64(&old), 42);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn imported_product_pattern_and_value_survive_earlier_product_compaction() {
    let source = concat!(
        "product/\nname/\nremove\n/name\nfields/\n/fields\n/product\n",
        "product/\nname/\npair\n/name\nfields/\n",
        "field/\nname/\nleft\n/name\ntype/\nbool\n/type\n/field\n",
        "field/\nname/\nright\n/name\ntype/\nbool\n/type\n/field\n/fields\n/product\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "match/\nproduct-value/\npair\n",
        "field/\nleft\ntrue\n/field\nfield/\nright\nfalse\n/field\n/product-value\n",
        "arms/\narm/\nproduct-pattern/\ntype/\nproduct\npair\n/type\nfields/\n",
        "product-field-pattern/\nname/\nleft\n/name\nwildcard/\n/wildcard\n/product-field-pattern\n",
        "product-field-pattern/\nname/\nright\n/name\nwildcard/\n/wildcard\n/product-field-pattern\n",
        "/fields\n/product-pattern\n42\n/arm\n/arms\n/match\n/main\n",
    );
    let snapshot = importer::import_source_with_namespace(
        source,
        "nominal-product-pattern-delete.lkjscript",
        WorkspaceNamespace::deterministic(158),
    )
    .expect("import product pattern program");
    assert_eq!(run_i64(&snapshot), 42);
    let remove = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Product && entity.name.ends_with(":remove"))
        .expect("removed product")
        .id;
    let pair = snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Product && entity.name.ends_with(":pair"))
        .expect("retained product")
        .id;
    let match_node = snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Match)
        .expect("product match node")
        .id;
    let old = snapshot.clone();
    let mut workspace = Workspace::new(snapshot).expect("product pattern workspace");
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let deleted = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::DeleteEntity { entity: remove }],
        })
        .expect("delete earlier product and remap pattern");
    assert_eq!(run_i64(&deleted.snapshot), 42);
    assert_eq!(run_i64(&old), 42);
    assert_eq!(
        deleted.snapshot.entity(pair).expect("retained pair").id,
        pair
    );
    assert_eq!(
        deleted
            .snapshot
            .node(match_node)
            .expect("retained product match node")
            .id,
        match_node
    );
    assert_eq!(deleted.snapshot.program.products.len(), 1);
    assert_eq!(deleted.snapshot.program.products[0].id.raw(), 0);
    let crate::hir::MatchPattern::Product { product, .. } =
        &deleted.snapshot.program.match_plans[0].arms[0].pattern
    else {
        panic!("retained product pattern changed kind")
    };
    assert_eq!(product.raw(), 0);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn source_free_nominal_declarations_publish_stable_children_types_and_dependencies() {
    let mut workspace = Workspace::empty_deterministic(50).expect("empty workspace");
    let (pair, left, right) = create_pair(&mut workspace);
    let pair_snapshot = workspace.current();
    assert_eq!(
        pair_snapshot
            .entity_type(pair_snapshot.revision(), pair)
            .expect("product type")
            .declared,
        Some(SemanticType::Product(pair))
    );
    assert_eq!(
        pair_snapshot
            .entity_type(pair_snapshot.revision(), left)
            .expect("field type")
            .declared,
        Some(SemanticType::I64)
    );
    assert!(pair_snapshot.containment().iter().any(|edge| {
        edge.owner == SemanticOwner::Entity(pair) && edge.child == SemanticChild::Entity(left)
    }));
    assert!(pair_snapshot.containment().iter().any(|edge| {
        edge.owner == SemanticOwner::Entity(pair) && edge.child == SemanticChild::Entity(right)
    }));

    let (choice, some, none, value) = create_choice(&mut workspace);
    let declarations = workspace.current();
    assert_eq!(
        declarations
            .definition(declarations.revision(), pair)
            .expect("stable pair")
            .id,
        pair
    );
    assert_eq!(
        declarations
            .entity_type(declarations.revision(), choice)
            .expect("enum type")
            .declared,
        Some(SemanticType::Enum {
            constructor: SemanticEnum::Entity(choice),
            arguments: Vec::new()
        })
    );
    for child in [some, none] {
        assert!(declarations.containment().iter().any(|edge| {
            edge.owner == SemanticOwner::Entity(choice)
                && edge.child == SemanticChild::Entity(child)
        }));
    }
    assert!(declarations.containment().iter().any(|edge| {
        edge.owner == SemanticOwner::Entity(some) && edge.child == SemanticChild::Entity(value)
    }));

    let function = workspace
        .apply(Transaction {
            base_revision: declarations.revision(),
            edits: vec![Edit::CreateFunction {
                name: "keep-pair".to_owned(),
                type_parameters: Vec::new(),
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: DeclarationType::Product(pair),
                }],
                return_type: DeclarationType::Product(pair),
            }],
        })
        .expect("create nominal function");
    let keep = entity_named(&function.snapshot, EntityKind::Function, "keep-pair");
    let signature = function
        .snapshot
        .function_signature(function.snapshot.revision(), keep)
        .expect("structured nominal signature");
    assert_eq!(signature.parameters.len(), 1);
    assert_eq!(signature.parameters[0].ty, SemanticType::Product(pair));
    assert_eq!(signature.result, SemanticType::Product(pair));
    assert!(function
        .snapshot
        .dependencies()
        .iter()
        .any(|edge| { edge.dependent == keep && edge.dependency == pair }));
    let projection = function
        .snapshot
        .project(&[
            ProjectionSlice::Entity(pair),
            ProjectionSlice::Entity(left),
            ProjectionSlice::Entity(choice),
            ProjectionSlice::Entity(some),
            ProjectionSlice::Entity(value),
        ])
        .expect("nominal projection");
    for kind in [
        "product",
        "product-field",
        "enum",
        "enum-variant",
        "enum-field",
    ] {
        assert!(projection.contains(&format!("kind={kind}")), "{projection}");
    }
}

#[test]
fn invalid_nominal_declarations_are_atomic_and_forced_ids_are_retry_stable() {
    let mut workspace = Workspace::empty_deterministic(51).expect("empty workspace");
    let before = workspace.current();
    let cases = vec![
        Edit::CreateProduct {
            name: String::new(),
            fields: Vec::new(),
        },
        Edit::CreateProduct {
            name: "pair".to_owned(),
            fields: vec![
                ProductFieldDraft {
                    name: "value".to_owned(),
                    ty: SemanticType::I64,
                },
                ProductFieldDraft {
                    name: "value".to_owned(),
                    ty: SemanticType::I64,
                },
            ],
        },
        Edit::CreateProduct {
            name: "add".to_owned(),
            fields: Vec::new(),
        },
        Edit::CreateProduct {
            name: "owned".to_owned(),
            fields: vec![ProductFieldDraft {
                name: "value".to_owned(),
                ty: SemanticType::ByteVector,
            }],
        },
        Edit::CreateEnum {
            name: "empty".to_owned(),
            variants: Vec::new(),
        },
        Edit::CreateEnum {
            name: "duplicate".to_owned(),
            variants: vec![
                EnumVariantDraft {
                    name: "same".to_owned(),
                    fields: Vec::new(),
                },
                EnumVariantDraft {
                    name: "same".to_owned(),
                    fields: Vec::new(),
                },
            ],
        },
        Edit::CreateEnum {
            name: "duplicate-fields".to_owned(),
            variants: vec![EnumVariantDraft {
                name: "one".to_owned(),
                fields: vec![
                    EnumFieldDraft {
                        name: "value".to_owned(),
                        ty: SemanticType::I64,
                    },
                    EnumFieldDraft {
                        name: "value".to_owned(),
                        ty: SemanticType::I64,
                    },
                ],
            }],
        },
        Edit::CreateEnum {
            name: "owned-enum".to_owned(),
            variants: vec![EnumVariantDraft {
                name: "one".to_owned(),
                fields: vec![EnumFieldDraft {
                    name: "value".to_owned(),
                    ty: SemanticType::ByteSlice,
                }],
            }],
        },
    ];
    for edit in cases {
        let result = workspace.apply(Transaction {
            base_revision: before.revision(),
            edits: vec![edit],
        });
        assert!(result.is_err());
        assert!(Arc::ptr_eq(&before, &workspace.current()));
    }
    let mut control = Workspace::empty_deterministic(51).expect("control workspace");
    let control_ids = {
        create_pair(&mut control);
        control.current().entities().to_vec()
    };
    create_pair(&mut workspace);
    assert_eq!(workspace.current().entities(), control_ids);
    let published_pair = workspace.current();
    let duplicate_declaration = workspace.apply(Transaction {
        base_revision: published_pair.revision(),
        edits: vec![Edit::CreateProduct {
            name: "pair".to_owned(),
            fields: Vec::new(),
        }],
    });
    assert!(matches!(
        duplicate_declaration,
        Err(WorkspaceError::InvalidTransaction(_))
    ));
    assert!(Arc::ptr_eq(&published_pair, &workspace.current()));

    let mut foreign = Workspace::empty_deterministic(52).expect("foreign workspace");
    let (foreign_pair, ..) = create_pair(&mut foreign);
    let published = workspace.current();
    let result = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::CreateProduct {
            name: "foreign-field".to_owned(),
            fields: vec![ProductFieldDraft {
                name: "value".to_owned(),
                ty: SemanticType::Product(foreign_pair),
            }],
        }],
    });
    assert!(matches!(result, Err(WorkspaceError::ForeignNamespace(_))));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let local_pair = entity_named(&published, EntityKind::Product, "pair");
    let wrong_kind = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::CreateProduct {
            name: "wrong-kind".to_owned(),
            fields: vec![ProductFieldDraft {
                name: "value".to_owned(),
                ty: SemanticType::Enum {
                    constructor: SemanticEnum::Entity(local_pair),
                    arguments: Vec::new(),
                },
            }],
        }],
    });
    assert!(matches!(
        wrong_kind,
        Err(WorkspaceError::WrongEntityKind { .. })
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let stale = EntityId::new(published.namespace(), u64::MAX, 1);
    let stale_type = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::CreateProduct {
            name: "stale-field".to_owned(),
            fields: vec![ProductFieldDraft {
                name: "value".to_owned(),
                ty: SemanticType::Product(stale),
            }],
        }],
    });
    assert!(matches!(stale_type, Err(WorkspaceError::StaleIdentity(_))));
    assert!(Arc::ptr_eq(&published, &workspace.current()));
}

#[test]
fn malformed_nominal_value_identities_and_fields_are_atomic() {
    let mut workspace = Workspace::empty_deterministic(70).expect("nominal draft workspace");
    let (pair, left, right) = create_pair(&mut workspace);
    let other_product = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateProduct {
                name: "single".to_owned(),
                fields: vec![ProductFieldDraft {
                    name: "other-product-field".to_owned(),
                    ty: SemanticType::I64,
                }],
            }],
        })
        .expect("create second product");
    let other_product_field = entity_named(
        &other_product.snapshot,
        EntityKind::ProductField,
        "other-product-field",
    );
    let (choice, some, _none, value) = create_choice(&mut workspace);
    let other_enum = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateEnum {
                name: "alternate".to_owned(),
                variants: vec![EnumVariantDraft {
                    name: "alternate-variant".to_owned(),
                    fields: vec![EnumFieldDraft {
                        name: "other-enum-field".to_owned(),
                        ty: SemanticType::I64,
                    }],
                }],
            }],
        })
        .expect("create second enum");
    let other_enum_field = entity_named(
        &other_enum.snapshot,
        EntityKind::EnumField,
        "other-enum-field",
    );
    let main = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create nominal draft main");
    let published = main.snapshot;
    let hole = published.holes().next().expect("main hole").id;
    let stale = EntityId::new(published.namespace(), u64::MAX, 1);
    let drafts = vec![
        ExpressionDraft::new(
            vec![
                DraftNode::I64(1),
                DraftNode::ProductValue {
                    product: pair,
                    fields: vec![DraftFieldValue {
                        field: left,
                        value: DraftNodeId::new(0),
                    }],
                },
            ],
            DraftNodeId::new(1),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(1),
                DraftNode::I64(2),
                DraftNode::ProductValue {
                    product: pair,
                    fields: vec![
                        DraftFieldValue {
                            field: left,
                            value: DraftNodeId::new(0),
                        },
                        DraftFieldValue {
                            field: left,
                            value: DraftNodeId::new(1),
                        },
                    ],
                },
            ],
            DraftNodeId::new(2),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(1),
                DraftNode::Bool(true),
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
            ],
            DraftNodeId::new(2),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(1),
                DraftNode::I64(2),
                DraftNode::ProductValue {
                    product: pair,
                    fields: vec![
                        DraftFieldValue {
                            field: left,
                            value: DraftNodeId::new(0),
                        },
                        DraftFieldValue {
                            field: other_product_field,
                            value: DraftNodeId::new(1),
                        },
                    ],
                },
            ],
            DraftNodeId::new(2),
        ),
        ExpressionDraft::new(
            vec![DraftNode::ProductValue {
                product: choice,
                fields: Vec::new(),
            }],
            DraftNodeId::new(0),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(1),
                DraftNode::EnumValue {
                    variant: some,
                    fields: vec![DraftFieldValue {
                        field: other_enum_field,
                        value: DraftNodeId::new(0),
                    }],
                },
            ],
            DraftNodeId::new(1),
        ),
        ExpressionDraft::new(
            vec![DraftNode::EnumValue {
                variant: pair,
                fields: Vec::new(),
            }],
            DraftNodeId::new(0),
        ),
        ExpressionDraft::new(
            vec![DraftNode::ProductValue {
                product: stale,
                fields: Vec::new(),
            }],
            DraftNodeId::new(0),
        ),
    ];
    for draft in drafts {
        let failure = workspace.apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole { hole, draft }],
        });
        assert!(failure.is_err());
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }

    assert_eq!(
        published
            .definition(published.revision(), value)
            .expect("original enum field")
            .owner,
        Some(some)
    );
}

#[test]
fn nominal_holes_report_compatible_stable_constructors() {
    let mut product_workspace = Workspace::empty_deterministic(72).expect("product workspace");
    let (pair, left, right) = create_pair(&mut product_workspace);
    let created = product_workspace
        .apply(Transaction {
            base_revision: product_workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::Product(pair),
            }],
        })
        .expect("create product main");
    let hole = created.snapshot.holes().next().expect("product hole").id;
    let completed = product_workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(1),
                        DraftNode::I64(2),
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
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("construct product");
    let target = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Product)
        .expect("product node")
        .id;
    let introduced = product_workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![Edit::IntroduceHole {
                target,
                goal: "construct the pair".to_owned(),
            }],
        })
        .expect("introduce product hole");
    let product_hole = introduced.snapshot.holes().next().expect("product hole");
    assert!(introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            product_hole.id,
            PageRequest::new(16).expect("page"),
            None,
        )
        .expect("product constructors")
        .items
        .contains(&LegalConstructor::Product(pair)));

    let mut enum_workspace = Workspace::empty_deterministic(73).expect("enum workspace");
    let (choice, some, none, value) = create_choice(&mut enum_workspace);
    let created = enum_workspace
        .apply(Transaction {
            base_revision: enum_workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::Enum {
                    constructor: SemanticEnum::Entity(choice),
                    arguments: Vec::new(),
                },
            }],
        })
        .expect("create enum main");
    let hole = created.snapshot.holes().next().expect("enum hole").id;
    let completed = enum_workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(7),
                        DraftNode::EnumValue {
                            variant: some,
                            fields: vec![DraftFieldValue {
                                field: value,
                                value: DraftNodeId::new(0),
                            }],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("construct enum");
    let target = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Enum)
        .expect("enum node")
        .id;
    let introduced = enum_workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![Edit::IntroduceHole {
                target,
                goal: "construct the choice".to_owned(),
            }],
        })
        .expect("introduce enum hole");
    let enum_hole = introduced.snapshot.holes().next().expect("enum hole");
    let constructors = introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            enum_hole.id,
            PageRequest::new(16).expect("page"),
            None,
        )
        .expect("enum constructors")
        .items;
    assert!(constructors.contains(&LegalConstructor::EnumVariant(some)));
    assert!(constructors.contains(&LegalConstructor::EnumVariant(none)));
}

#[test]
fn source_free_product_and_enum_locals_compile_and_execute() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let mut product_workspace = Workspace::empty_deterministic(53).expect("product workspace");
    let (pair, left, right) = create_pair(&mut product_workspace);
    let main = product_workspace
        .apply(Transaction {
            base_revision: product_workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create product main");
    let hole = main.snapshot.holes().next().expect("main hole").id;
    let local = DraftBindingId::new(0);
    let completed = product_workspace
        .apply(Transaction {
            base_revision: main.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(20),
                        DraftNode::I64(22),
                        DraftNode::ProductValue {
                            product: pair,
                            fields: vec![
                                DraftFieldValue {
                                    field: right,
                                    value: DraftNodeId::new(1),
                                },
                                DraftFieldValue {
                                    field: left,
                                    value: DraftNodeId::new(0),
                                },
                            ],
                        },
                        DraftNode::Load(DraftBindingRef::Local(local)),
                        DraftNode::ProductField {
                            field: left,
                            value: DraftNodeId::new(3),
                        },
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: local,
                                name: "pair-value".to_owned(),
                                value: DraftNodeId::new(2),
                            }],
                            body: DraftNodeId::new(4),
                        },
                    ],
                    DraftNodeId::new(5),
                ),
            }],
        })
        .expect("construct and project product local");
    assert_eq!(run_i64(&completed.snapshot), 20);
    let product_local = entity_named(
        &completed.snapshot,
        EntityKind::ImmutableLocal,
        "pair-value",
    );
    assert!(completed
        .snapshot
        .references()
        .iter()
        .any(|edge| edge.target == left));
    assert!(completed
        .snapshot
        .references()
        .iter()
        .any(|edge| edge.target == right));
    assert!(completed
        .diff
        .entries
        .iter()
        .any(|entry| matches!(entry, SemanticDiffEntry::EntityCreated { entity, .. } if *entity == product_local)));

    let mut enum_workspace = Workspace::empty_deterministic(54).expect("enum workspace");
    let (choice, some, _none, value_field) = create_choice(&mut enum_workspace);
    let main = enum_workspace
        .apply(Transaction {
            base_revision: enum_workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create enum main");
    let hole = main.snapshot.holes().next().expect("main hole").id;
    let local = DraftBindingId::new(0);
    let completed = enum_workspace
        .apply(Transaction {
            base_revision: main.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(7),
                        DraftNode::EnumValue {
                            variant: some,
                            fields: vec![DraftFieldValue {
                                field: value_field,
                                value: DraftNodeId::new(0),
                            }],
                        },
                        DraftNode::Load(DraftBindingRef::Local(local)),
                        DraftNode::EnumIsVariant {
                            variant: some,
                            value: DraftNodeId::new(2),
                        },
                        DraftNode::I64(1),
                        DraftNode::I64(0),
                        DraftNode::If {
                            condition: DraftNodeId::new(3),
                            then_branch: DraftNodeId::new(4),
                            else_branch: DraftNodeId::new(5),
                        },
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: local,
                                name: "choice-value".to_owned(),
                                value: DraftNodeId::new(1),
                            }],
                            body: DraftNodeId::new(6),
                        },
                    ],
                    DraftNodeId::new(7),
                ),
            }],
        })
        .expect("construct and test enum local");
    assert_eq!(run_i64(&completed.snapshot), 1);
    assert!(completed
        .snapshot
        .references()
        .iter()
        .any(|edge| edge.target == choice));
    assert!(completed
        .snapshot
        .references()
        .iter()
        .any(|edge| edge.target == some));
    assert!(completed
        .snapshot
        .references()
        .iter()
        .any(|edge| edge.target == value_field));
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn source_free_enum_payload_match_compiles_and_executes() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let mut workspace = Workspace::empty_deterministic(80).expect("match workspace");
    let (choice, some, none, value_field) = create_choice(&mut workspace);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create match main");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let payload = DraftBindingId::new(0);
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::EnumValue {
                            variant: some,
                            fields: vec![DraftFieldValue {
                                field: value_field,
                                value: DraftNodeId::new(0),
                            }],
                        },
                        DraftNode::Load(DraftBindingRef::Local(payload)),
                        DraftNode::I64(0),
                        DraftNode::Match {
                            scrutinee: DraftNodeId::new(1),
                            arms: vec![
                                MatchArmDraft {
                                    pattern: PatternDraft::new(
                                        vec![
                                            DraftPatternNode::Binding {
                                                binding: payload,
                                                name: "value".to_owned(),
                                            },
                                            DraftPatternNode::EnumVariant {
                                                variant: some,
                                                fields: vec![DraftPatternField {
                                                    field: value_field,
                                                    pattern: DraftPatternNodeId::new(0),
                                                }],
                                            },
                                        ],
                                        DraftPatternNodeId::new(1),
                                    ),
                                    body: DraftNodeId::new(2),
                                },
                                MatchArmDraft {
                                    pattern: PatternDraft::new(
                                        vec![DraftPatternNode::EnumVariant {
                                            variant: none,
                                            fields: Vec::new(),
                                        }],
                                        DraftPatternNodeId::new(0),
                                    ),
                                    body: DraftNodeId::new(3),
                                },
                            ],
                        },
                    ],
                    DraftNodeId::new(4),
                ),
            }],
        })
        .expect("fill main with exhaustive enum match");

    assert_eq!(run_i64(&completed.snapshot), 42);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
    assert!(completed.snapshot.attachments().is_none());
    let binding = entity_named(&completed.snapshot, EntityKind::ImmutableLocal, "value");
    assert!(completed
        .diff
        .entries
        .iter()
        .any(|entry| matches!(entry, SemanticDiffEntry::EntityCreated { entity, .. } if *entity == binding)));
    assert!(completed
        .snapshot
        .entities()
        .iter()
        .all(|entity| !entity.name.starts_with("$match")));
    assert!(completed
        .snapshot
        .search_entities(
            completed.snapshot.revision(),
            "$match",
            PageRequest::new(16).expect("search page"),
            None,
        )
        .expect("hidden match search")
        .items
        .is_empty());
    assert!(completed
        .snapshot
        .search_entities(
            completed.snapshot.revision(),
            "value",
            PageRequest::new(16).expect("search page"),
            None,
        )
        .expect("payload search")
        .items
        .iter()
        .any(|entity| entity.id == binding));
    let site = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Match)
        .expect("semantic match node")
        .id;
    let view = completed
        .snapshot
        .match_view(completed.snapshot.revision(), site)
        .expect("match view");
    assert_eq!(
        view,
        completed
            .snapshot
            .match_view(completed.snapshot.revision(), site)
            .expect("repeat match view")
    );
    assert!(matches!(
        completed
            .snapshot
            .match_view(created.snapshot.revision(), site),
        Err(WorkspaceError::StaleRevision)
    ));
    let literal = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Literal)
        .expect("literal node")
        .id;
    assert!(matches!(
        completed
            .snapshot
            .match_view(completed.snapshot.revision(), literal),
        Err(WorkspaceError::WrongEntityKind { .. })
    ));
    assert!(view.exhaustive);
    assert_eq!(view.arms.len(), 2);
    assert!(view.arms[0].patterns.iter().any(|pattern| {
        matches!(pattern.kind, MatchPatternKindView::Binding { binding: found } if found == binding)
    }));
    for target in [choice, some, none, value_field, binding] {
        assert!(completed
            .snapshot
            .references()
            .iter()
            .any(|edge| edge.target == target));
    }
    assert!(completed
        .snapshot
        .dependencies()
        .iter()
        .any(|edge| edge.dependent == main && edge.dependency == choice));
    let projection = completed
        .snapshot
        .project(&[ProjectionSlice::Body(main), ProjectionSlice::Match(site)])
        .expect("source-free match projection");
    assert_eq!(
        projection,
        completed
            .snapshot
            .project(&[ProjectionSlice::Body(main), ProjectionSlice::Match(site)])
            .expect("repeat source-free match projection")
    );
    assert!(projection.contains("kind=match"), "{projection}");
    assert!(projection.contains("kind=enum-variant"), "{projection}");
    assert!(projection.contains("kind=binding"), "{projection}");
    assert!(!projection.contains("$match"), "{projection}");
    assert_eq!(
        completed.snapshot.program.match_plans[0].origin,
        crate::hir::Origin::Semantic
    );
    let complete = completed
        .snapshot
        .validated_complete_hir()
        .expect("derive complete HIR");
    let mut enum_values = 0_usize;
    let mut pending = vec![&complete.main.body];
    while let Some(expression) = pending.pop() {
        assert!(!matches!(
            expression.kind,
            crate::hir::ExprKind::Match { .. }
        ));
        enum_values += usize::from(matches!(
            expression.kind,
            crate::hir::ExprKind::EnumValue { .. }
        ));
        crate::hir::for_each_expression_child(expression, &mut |child| pending.push(child));
    }
    assert_eq!(
        enum_values, 1,
        "the match scrutinee is lowered exactly once"
    );
    let mut stale_projection = complete.clone();
    let crate::hir::MatchPattern::Variant { fields, .. } =
        &mut stale_projection.match_plans[0].arms[0].pattern
    else {
        panic!("payload arm remains an enum pattern")
    };
    fields[0].projection = None;
    let projection_error = super::validate::program(&stale_projection)
        .expect_err("non-wildcard payload pattern requires projection metadata")
        .to_string();
    assert!(
        projection_error.contains("wildcard/projection"),
        "{projection_error}"
    );
    let mut stale_field_type = complete.clone();
    let crate::hir::MatchPattern::Variant { fields, .. } =
        &mut stale_field_type.match_plans[0].arms[0].pattern
    else {
        panic!("payload arm remains an enum pattern")
    };
    fields[0].projection = None;
    fields[0].pattern = crate::hir::MatchPattern::Wildcard {
        ty: crate::Type::Bool,
    };
    let field_type_error = super::validate::program(&stale_field_type)
        .expect_err("nested payload pattern uses its declared field type")
        .to_string();
    assert!(
        field_type_error.contains("pattern type"),
        "{field_type_error}"
    );
    let mut builtin_origin = complete.clone();
    builtin_origin.match_plans[0].origin = crate::hir::Origin::Builtin;
    let builtin_error = crate::analyze::verify_match_plans(&builtin_origin)
        .expect_err("ordinary match plans reject builtin provenance")
        .to_string();
    assert!(builtin_error.contains("origin"), "{builtin_error}");
    let mut stale_origin = complete;
    stale_origin.match_plans[0].origin =
        crate::hir::Origin::Source(crate::hir::SourceId::new(u64::MAX));
    let stale_error = super::validate::program(&stale_origin)
        .expect_err("match plans reject stale source provenance")
        .to_string();
    assert!(
        stale_error.contains("origin") || stale_error.contains("source"),
        "{stale_error}"
    );
}

#[test]
fn source_free_enum_payload_match_has_semantic_memory_places_and_executes() {
    let (workspace, ..) = source_free_choice_match(89);
    let snapshot = workspace.current();
    let executable = crate::compile_snapshot(&snapshot).expect("compile payload cleanup match");
    let semantic_places = executable
        .memory_plan()
        .entries
        .iter()
        .filter(|entry| {
            matches!(
                entry.subject,
                crate::memory_plan::MemorySubject::Place { .. }
            ) && entry.origin.source == crate::memory_plan::MemorySourceOrigin::Semantic
        })
        .count();
    assert!(
        semantic_places >= 3,
        "scrutinee, projection, and payload places"
    );
    let outcome = run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(outcome.cleanup_failures().is_none());
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(42)));
}

fn imported_choice_match_source() -> String {
    concat!(
        "enum/\nname/\nchoice\n/name\nvariants/\n",
        "variant/\nname/\nsome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\n",
        "type/\ni64\n/type\n/variant-field\n/fields\n/variant\n",
        "variant/\nname/\nnone\n/name\nfields/\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\n",
        "variant-value/\ntype/\nchoice/\n/choice\n/type\nvariant/\nsome\n/variant\n",
        "fields/\nvariant-field/\nname/\nvalue\n/name\n42\n/variant-field\n/fields\n/variant-value\n",
        "arms/\narm/\nvariant-pattern/\ntype/\nchoice/\n/choice\n/type\n",
        "variant/\nsome\n/variant\nfields/\nvariant-field-pattern/\nname/\nvalue\n/name\n",
        "binding/\nname/\nx\n/name\n/binding\n/variant-field-pattern\n/fields\n",
        "/variant-pattern\nx\n/arm\n",
        "arm/\nvariant-pattern/\ntype/\nchoice/\n/choice\n/type\nvariant/\nnone\n/variant\n",
        "fields/\n/fields\n/variant-pattern\n0\n/arm\n/arms\n/match\n/main\n",
    )
    .to_owned()
}

fn source_free_choice_match(seed: u64) -> (Workspace, EntityId, EntityId, EntityId, EntityId) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("match workspace");
    let (choice, some, none, value_field) = create_choice(&mut workspace);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create match main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let payload = DraftBindingId::new(0);
    workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::EnumValue {
                            variant: some,
                            fields: vec![DraftFieldValue {
                                field: value_field,
                                value: DraftNodeId::new(0),
                            }],
                        },
                        DraftNode::Load(DraftBindingRef::Local(payload)),
                        DraftNode::I64(0),
                        DraftNode::Match {
                            scrutinee: DraftNodeId::new(1),
                            arms: vec![
                                MatchArmDraft {
                                    pattern: PatternDraft::new(
                                        vec![
                                            DraftPatternNode::Binding {
                                                binding: payload,
                                                name: "x".to_owned(),
                                            },
                                            DraftPatternNode::EnumVariant {
                                                variant: some,
                                                fields: vec![DraftPatternField {
                                                    field: value_field,
                                                    pattern: DraftPatternNodeId::new(0),
                                                }],
                                            },
                                        ],
                                        DraftPatternNodeId::new(1),
                                    ),
                                    body: DraftNodeId::new(2),
                                },
                                MatchArmDraft {
                                    pattern: PatternDraft::new(
                                        vec![DraftPatternNode::EnumVariant {
                                            variant: none,
                                            fields: Vec::new(),
                                        }],
                                        DraftPatternNodeId::new(0),
                                    ),
                                    body: DraftNodeId::new(3),
                                },
                            ],
                        },
                    ],
                    DraftNodeId::new(4),
                ),
            }],
        })
        .expect("construct source-free match");
    (workspace, choice, some, none, value_field)
}

#[test]
fn return_replacement_refreshes_match_arm_and_result_types() {
    let (mut workspace, ..) = source_free_choice_match(238);
    let original = workspace.current();
    let match_node = original
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Match)
        .expect("match node")
        .id;
    let view = original
        .match_view(original.revision(), match_node)
        .expect("original match view");
    assert_eq!(view.arms.len(), 2);
    let replaced = workspace
        .apply(Transaction {
            base_revision: original.revision(),
            edits: vec![
                Edit::ReplaceExpression {
                    target: view.arms[0].body,
                    draft: return_i64_draft(7),
                },
                Edit::ReplaceExpression {
                    target: view.arms[1].body,
                    draft: return_i64_draft(8),
                },
            ],
        })
        .expect("replace both match arms with returns");
    let updated = replaced
        .snapshot
        .match_view(replaced.snapshot.revision(), match_node)
        .expect("updated match view");
    assert_eq!(updated.result, SemanticType::Never);
    assert!(updated.arms.iter().all(|arm| {
        replaced
            .snapshot
            .node_semantics(replaced.snapshot.revision(), arm.body)
            .is_ok_and(|facts| {
                facts.kind == NodeKind::Return
                    && facts.actual == SemanticType::Never
                    && facts.expected == Some(SemanticType::I64)
            })
    }));
    assert_eq!(run_i64(&replaced.snapshot), 7);

    let restored = workspace
        .apply(Transaction {
            base_revision: replaced.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: updated.arms[0].body,
                draft: ExpressionDraft::scalar_i64(9),
            }],
        })
        .expect("restore one reachable match arm");
    let restored_view = restored
        .snapshot
        .match_view(restored.snapshot.revision(), match_node)
        .expect("restored match view");
    assert_eq!(restored_view.result, SemanticType::I64);
    assert_eq!(
        restored
            .snapshot
            .node_semantics(restored.snapshot.revision(), restored_view.arms[0].body)
            .expect("restored arm facts")
            .actual,
        SemanticType::I64
    );
    assert_eq!(run_i64(&replaced.snapshot), 7);
    assert_eq!(run_i64(&restored.snapshot), 9);
}

#[test]
fn source_free_match_removal_prunes_payload_bindings_plans_and_holes_cleanly() {
    let (mut replacement_workspace, ..) = source_free_choice_match(135);
    let published = replacement_workspace.current();
    let payload = entity_named(&published, EntityKind::ImmutableLocal, "x");
    let match_node = published
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Match)
        .expect("match node")
        .id;
    let mut hole_workspace = Workspace::new((*published).clone()).expect("match hole workspace");

    let replaced = replacement_workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: match_node,
                draft: ExpressionDraft::scalar_i64(5),
            }],
        })
        .expect("replace semantic match");
    assert!(replaced.snapshot.entity(payload).is_err());
    assert!(replaced.snapshot.program.match_plans.is_empty());
    assert!(replaced.snapshot.program.bindings.is_empty());
    assert_eq!(
        replaced
            .snapshot
            .node(match_node)
            .expect("stable match root")
            .id,
        match_node
    );
    assert!(replaced
        .snapshot
        .references()
        .iter()
        .all(|edge| edge.target != payload));
    assert!(replaced
        .snapshot
        .dependencies()
        .iter()
        .all(|edge| edge.dependency != payload));
    assert_eq!(run_i64(&replaced.snapshot), 5);

    let introduced = hole_workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::IntroduceHole {
                target: match_node,
                goal: "replace the complete match".to_owned(),
            }],
        })
        .expect("hole semantic match");
    assert!(introduced.snapshot.entity(payload).is_err());
    assert!(introduced.snapshot.program.match_plans.is_empty());
    assert!(introduced.snapshot.program.bindings.is_empty());
    let match_hole = introduced.snapshot.holes().next().expect("match hole");
    assert_eq!(match_hole.id.node(), match_node);
    assert!(!match_hole.visible_entities.contains(&payload));
    assert!(!introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            match_hole.id,
            PageRequest::new(32).expect("page"),
            None,
        )
        .expect("former-match constructors")
        .items
        .contains(&LegalConstructor::Load(payload)));
    assert!(introduced
        .snapshot
        .match_view(introduced.snapshot.revision(), match_node)
        .is_err());
    let filled = hole_workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: HoleId(match_node),
                draft: ExpressionDraft::scalar_i64(6),
            }],
        })
        .expect("fill former match");
    assert_eq!(run_i64(&filled.snapshot), 6);
}

fn choice_match_draft(some: EntityId, none: EntityId, value_field: EntityId) -> ExpressionDraft {
    let payload = DraftBindingId::new(0);
    ExpressionDraft::new(
        vec![
            DraftNode::I64(42),
            DraftNode::EnumValue {
                variant: some,
                fields: vec![DraftFieldValue {
                    field: value_field,
                    value: DraftNodeId::new(0),
                }],
            },
            DraftNode::Load(DraftBindingRef::Local(payload)),
            DraftNode::I64(0),
            DraftNode::Match {
                scrutinee: DraftNodeId::new(1),
                arms: vec![
                    MatchArmDraft {
                        pattern: PatternDraft::new(
                            vec![
                                DraftPatternNode::Binding {
                                    binding: payload,
                                    name: "x".to_owned(),
                                },
                                DraftPatternNode::EnumVariant {
                                    variant: some,
                                    fields: vec![DraftPatternField {
                                        field: value_field,
                                        pattern: DraftPatternNodeId::new(0),
                                    }],
                                },
                            ],
                            DraftPatternNodeId::new(1),
                        ),
                        body: DraftNodeId::new(2),
                    },
                    MatchArmDraft {
                        pattern: PatternDraft::new(
                            vec![DraftPatternNode::EnumVariant {
                                variant: none,
                                fields: Vec::new(),
                            }],
                            DraftPatternNodeId::new(0),
                        ),
                        body: DraftNodeId::new(3),
                    },
                ],
            },
        ],
        DraftNodeId::new(4),
    )
}

fn choice_match_with_patterns(
    some: EntityId,
    value_field: EntityId,
    patterns: Vec<PatternDraft>,
) -> ExpressionDraft {
    let mut nodes = Vec::new();
    nodes.push(DraftNode::I64(42));
    nodes.push(DraftNode::EnumValue {
        variant: some,
        fields: vec![DraftFieldValue {
            field: value_field,
            value: DraftNodeId::new(0),
        }],
    });
    let mut arms = Vec::new();
    for (index, pattern) in patterns.into_iter().enumerate() {
        let body = DraftNodeId::new(u64::try_from(nodes.len()).expect("pattern-case body id"));
        nodes.push(DraftNode::I64(
            i64::try_from(index).expect("pattern-case body"),
        ));
        arms.push(MatchArmDraft { pattern, body });
    }
    let root = DraftNodeId::new(u64::try_from(nodes.len()).expect("pattern-case root id"));
    nodes.push(DraftNode::Match {
        scrutinee: DraftNodeId::new(1),
        arms,
    });
    ExpressionDraft::new(nodes, root)
}

#[test]
fn imported_and_source_free_enum_payload_matches_converge() {
    let imported = import_source(
        &imported_choice_match_source(),
        "choice-match-convergence.lkjscript",
    )
    .expect("import payload match");
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let (workspace, ..) = source_free_choice_match(81);
    let source_free = workspace.current();
    assert_eq!(
        canonical_workspace_observation(&imported),
        canonical_workspace_observation(&source_free)
    );
    assert_eq!(run_i64(&imported), 42);
    assert_eq!(run_i64(&source_free), 42);
    for snapshot in [&imported, &source_free] {
        assert!(snapshot
            .entities()
            .iter()
            .all(|entity| !entity.name.starts_with("$match")));
    }
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
    assert!(matches!(
        imported.program.match_plans[0].origin,
        crate::hir::Origin::Source(_)
    ));
    assert_eq!(
        source_free.program.match_plans[0].origin,
        crate::hir::Origin::Semantic
    );
    let imported_executable = crate::compile_snapshot(&imported).expect("compile imported match");
    let source_free_executable =
        crate::compile_snapshot(&source_free).expect("compile source-free match");
    let obligation_kinds = |program: &crate::ExecutableProgram| {
        program
            .memory_plan()
            .obligations
            .iter()
            .map(|obligation| obligation.kind)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        obligation_kinds(&imported_executable),
        obligation_kinds(&source_free_executable)
    );
    assert_eq!(
        imported_executable.bytecode().main().code,
        source_free_executable.bytecode().main().code
    );
}

#[test]
fn semantic_match_edit_inside_imported_main_keeps_mixed_origins_honest() {
    let source = concat!(
        "enum/\nname/\nchoice\n/name\nvariants/\n",
        "variant/\nname/\nsome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\n",
        "type/\ni64\n/type\n/variant-field\n/fields\n/variant\n",
        "variant/\nname/\nnone\n/name\nfields/\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n0\n/main\n",
    );
    let imported = import_source(source, "mixed-origin-match.lkjscript")
        .expect("import source-backed match host");
    let imported_origin = imported
        .program
        .main
        .as_ref()
        .expect("imported main")
        .origin;
    assert!(matches!(imported_origin, crate::hir::Origin::Source(_)));
    let find = |kind, name: &str| {
        imported
            .entities()
            .iter()
            .find(|entity| {
                entity.kind == kind
                    && entity
                        .name
                        .rsplit(':')
                        .next()
                        .is_some_and(|item| item == name)
            })
            .expect("imported match entity")
            .id
    };
    let some = find(EntityKind::EnumVariant, "some");
    let none = find(EntityKind::EnumVariant, "none");
    let value_field = find(EntityKind::EnumField, "value");
    let target = imported
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Literal)
        .expect("imported main body")
        .id;
    let mut workspace = Workspace::new(imported).expect("mixed-origin workspace");
    let introduced = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::IntroduceHole {
                target,
                goal: "replace imported body with semantic match".to_owned(),
            }],
        })
        .expect("introduce imported body hole");
    let hole = introduced
        .snapshot
        .holes()
        .next()
        .expect("imported hole")
        .id;
    let completed = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: choice_match_draft(some, none, value_field),
            }],
        })
        .expect("publish semantic match in imported main");
    assert_eq!(
        completed
            .snapshot
            .program
            .main
            .as_ref()
            .expect("main")
            .origin,
        imported_origin
    );
    assert_eq!(
        completed.snapshot.program.match_plans[0].origin,
        crate::hir::Origin::Semantic
    );
    assert_eq!(run_i64(&completed.snapshot), 42);
}

#[test]
fn source_free_match_pattern_physical_order_does_not_change_semantics() {
    let (control, ..) = source_free_choice_match(88);
    let mut workspace = Workspace::empty_deterministic(88).expect("reordered match workspace");
    let (_choice, some, none, value_field) = create_choice(&mut workspace);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create reordered match main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let payload = DraftBindingId::new(0);
    let reordered = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::EnumValue {
                            variant: some,
                            fields: vec![DraftFieldValue {
                                field: value_field,
                                value: DraftNodeId::new(0),
                            }],
                        },
                        DraftNode::Load(DraftBindingRef::Local(payload)),
                        DraftNode::I64(0),
                        DraftNode::Match {
                            scrutinee: DraftNodeId::new(1),
                            arms: vec![
                                MatchArmDraft {
                                    pattern: PatternDraft::new(
                                        vec![
                                            DraftPatternNode::EnumVariant {
                                                variant: some,
                                                fields: vec![DraftPatternField {
                                                    field: value_field,
                                                    pattern: DraftPatternNodeId::new(1),
                                                }],
                                            },
                                            DraftPatternNode::Binding {
                                                binding: payload,
                                                name: "x".to_owned(),
                                            },
                                        ],
                                        DraftPatternNodeId::new(0),
                                    ),
                                    body: DraftNodeId::new(2),
                                },
                                MatchArmDraft {
                                    pattern: PatternDraft::new(
                                        vec![DraftPatternNode::EnumVariant {
                                            variant: none,
                                            fields: Vec::new(),
                                        }],
                                        DraftPatternNodeId::new(0),
                                    ),
                                    body: DraftNodeId::new(3),
                                },
                            ],
                        },
                    ],
                    DraftNodeId::new(4),
                ),
            }],
        })
        .expect("construct physically reordered pattern");
    assert_eq!(run_i64(&reordered.snapshot), 42);
    assert_eq!(
        canonical_workspace_observation(&reordered.snapshot),
        canonical_workspace_observation(&control.current())
    );
    assert_eq!(
        crate::compile_snapshot(&reordered.snapshot)
            .expect("compile reordered match")
            .bytecode()
            .main()
            .code,
        crate::compile_snapshot(&control.current())
            .expect("compile control match")
            .bytecode()
            .main()
            .code
    );
}

#[test]
fn source_free_match_rejects_nonexhaustive_and_useless_arms_atomically() {
    let mut workspace = Workspace::empty_deterministic(83).expect("atomic match workspace");
    let (_choice, some, none, value_field) = create_choice(&mut workspace);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create atomic match main");
    let before = created.snapshot;
    let hole = before.holes().next().expect("main hole").id;
    let some_pattern = || {
        PatternDraft::new(
            vec![
                DraftPatternNode::Wildcard,
                DraftPatternNode::EnumVariant {
                    variant: some,
                    fields: vec![DraftPatternField {
                        field: value_field,
                        pattern: DraftPatternNodeId::new(0),
                    }],
                },
            ],
            DraftPatternNodeId::new(1),
        )
    };
    let none_pattern = || {
        PatternDraft::new(
            vec![DraftPatternNode::EnumVariant {
                variant: none,
                fields: Vec::new(),
            }],
            DraftPatternNodeId::new(0),
        )
    };
    let nonexhaustive = ExpressionDraft::new(
        vec![
            DraftNode::I64(42),
            DraftNode::EnumValue {
                variant: some,
                fields: vec![DraftFieldValue {
                    field: value_field,
                    value: DraftNodeId::new(0),
                }],
            },
            DraftNode::I64(1),
            DraftNode::Match {
                scrutinee: DraftNodeId::new(1),
                arms: vec![MatchArmDraft {
                    pattern: some_pattern(),
                    body: DraftNodeId::new(2),
                }],
            },
        ],
        DraftNodeId::new(3),
    );
    let duplicate = ExpressionDraft::new(
        vec![
            DraftNode::I64(42),
            DraftNode::EnumValue {
                variant: some,
                fields: vec![DraftFieldValue {
                    field: value_field,
                    value: DraftNodeId::new(0),
                }],
            },
            DraftNode::I64(1),
            DraftNode::I64(2),
            DraftNode::Match {
                scrutinee: DraftNodeId::new(1),
                arms: vec![
                    MatchArmDraft {
                        pattern: some_pattern(),
                        body: DraftNodeId::new(2),
                    },
                    MatchArmDraft {
                        pattern: some_pattern(),
                        body: DraftNodeId::new(3),
                    },
                ],
            },
        ],
        DraftNodeId::new(4),
    );
    let wildcard_then_arm = ExpressionDraft::new(
        vec![
            DraftNode::I64(42),
            DraftNode::EnumValue {
                variant: some,
                fields: vec![DraftFieldValue {
                    field: value_field,
                    value: DraftNodeId::new(0),
                }],
            },
            DraftNode::I64(1),
            DraftNode::I64(2),
            DraftNode::Match {
                scrutinee: DraftNodeId::new(1),
                arms: vec![
                    MatchArmDraft {
                        pattern: PatternDraft::wildcard(),
                        body: DraftNodeId::new(2),
                    },
                    MatchArmDraft {
                        pattern: none_pattern(),
                        body: DraftNodeId::new(3),
                    },
                ],
            },
        ],
        DraftNodeId::new(4),
    );
    let empty = ExpressionDraft::new(
        vec![
            DraftNode::I64(42),
            DraftNode::EnumValue {
                variant: some,
                fields: vec![DraftFieldValue {
                    field: value_field,
                    value: DraftNodeId::new(0),
                }],
            },
            DraftNode::Match {
                scrutinee: DraftNodeId::new(1),
                arms: Vec::new(),
            },
        ],
        DraftNodeId::new(2),
    );
    let before_projection = before
        .project(&[ProjectionSlice::Body(entity_named(
            &before,
            EntityKind::Main,
            "main",
        ))])
        .expect("before projection");
    for (draft, expected) in [
        (nonexhaustive, "nonexhaustive match"),
        (duplicate, "useless or subsumed match arm 1"),
        (wildcard_then_arm, "useless or subsumed match arm 1"),
        (empty, "match arms must not be empty"),
    ] {
        let failure = workspace.apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::FillHole { hole, draft }],
        });
        let message = failure.expect_err("invalid match must reject").to_string();
        assert!(message.contains(expected), "{message}");
        assert!(Arc::ptr_eq(&before, &workspace.current()));
        assert_eq!(workspace.current().revision(), before.revision());
        assert_eq!(
            workspace
                .current()
                .project(&[ProjectionSlice::Body(entity_named(
                    &before,
                    EntityKind::Main,
                    "main",
                ))])
                .expect("unchanged projection"),
            before_projection
        );
    }
    let retried = workspace
        .apply(Transaction {
            base_revision: before.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: choice_match_draft(some, none, value_field),
            }],
        })
        .expect("deterministic successful retry");
    let (control, ..) = source_free_choice_match(83);
    assert_eq!(retried.snapshot.entities(), control.current().entities());
    assert_eq!(retried.snapshot.nodes(), control.current().nodes());
}

#[test]
fn malformed_source_free_match_shapes_identities_and_scopes_are_atomic() {
    let mut workspace = Workspace::empty_deterministic(86).expect("malformed match workspace");
    let (choice, some, none, value_field) = create_choice(&mut workspace);
    let alternate = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateEnum {
                name: "alternate".to_owned(),
                variants: vec![EnumVariantDraft {
                    name: "alternate-variant".to_owned(),
                    fields: vec![EnumFieldDraft {
                        name: "alternate-field".to_owned(),
                        ty: SemanticType::I64,
                    }],
                }],
            }],
        })
        .expect("create alternate enum");
    let alternate_variant = entity_named(
        &alternate.snapshot,
        EntityKind::EnumVariant,
        "alternate-variant",
    );
    let alternate_field = entity_named(
        &alternate.snapshot,
        EntityKind::EnumField,
        "alternate-field",
    );
    let duo = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateEnum {
                name: "duo".to_owned(),
                variants: vec![EnumVariantDraft {
                    name: "both".to_owned(),
                    fields: vec![
                        EnumFieldDraft {
                            name: "left-value".to_owned(),
                            ty: SemanticType::I64,
                        },
                        EnumFieldDraft {
                            name: "right-value".to_owned(),
                            ty: SemanticType::I64,
                        },
                    ],
                }],
            }],
        })
        .expect("create two-field enum");
    let both = entity_named(&duo.snapshot, EntityKind::EnumVariant, "both");
    let left = entity_named(&duo.snapshot, EntityKind::EnumField, "left-value");
    let right = entity_named(&duo.snapshot, EntityKind::EnumField, "right-value");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create malformed match main");
    let published = created.snapshot;
    let hole = published.holes().next().expect("main hole").id;
    let stale = EntityId::new(published.namespace(), u64::MAX, 1);
    let mut foreign = Workspace::empty_deterministic(87).expect("foreign match workspace");
    let (_foreign_choice, foreign_some, _foreign_none, foreign_field) = create_choice(&mut foreign);
    let none_pattern = || {
        PatternDraft::new(
            vec![DraftPatternNode::EnumVariant {
                variant: none,
                fields: Vec::new(),
            }],
            DraftPatternNodeId::new(0),
        )
    };
    let malformed = vec![
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![DraftPatternNode::Wildcard, DraftPatternNode::Wildcard],
                    DraftPatternNodeId::new(0),
                ),
                none_pattern(),
            ],
        ),
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![DraftPatternNode::EnumVariant {
                        variant: some,
                        fields: vec![DraftPatternField {
                            field: value_field,
                            pattern: DraftPatternNodeId::new(0),
                        }],
                    }],
                    DraftPatternNodeId::new(0),
                ),
                none_pattern(),
            ],
        ),
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![
                        DraftPatternNode::Wildcard,
                        DraftPatternNode::EnumVariant {
                            variant: some,
                            fields: vec![
                                DraftPatternField {
                                    field: value_field,
                                    pattern: DraftPatternNodeId::new(0),
                                },
                                DraftPatternField {
                                    field: value_field,
                                    pattern: DraftPatternNodeId::new(0),
                                },
                            ],
                        },
                    ],
                    DraftPatternNodeId::new(1),
                ),
                none_pattern(),
            ],
        ),
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![DraftPatternNode::Wildcard],
                    DraftPatternNodeId::new(u64::MAX),
                ),
                none_pattern(),
            ],
        ),
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![DraftPatternNode::EnumVariant {
                        variant: some,
                        fields: vec![DraftPatternField {
                            field: value_field,
                            pattern: DraftPatternNodeId::new(u64::MAX),
                        }],
                    }],
                    DraftPatternNodeId::new(0),
                ),
                none_pattern(),
            ],
        ),
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![DraftPatternNode::EnumVariant {
                        variant: foreign_some,
                        fields: vec![DraftPatternField {
                            field: foreign_field,
                            pattern: DraftPatternNodeId::new(0),
                        }],
                    }],
                    DraftPatternNodeId::new(0),
                ),
                none_pattern(),
            ],
        ),
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![DraftPatternNode::EnumVariant {
                        variant: stale,
                        fields: Vec::new(),
                    }],
                    DraftPatternNodeId::new(0),
                ),
                none_pattern(),
            ],
        ),
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![DraftPatternNode::EnumVariant {
                        variant: choice,
                        fields: Vec::new(),
                    }],
                    DraftPatternNodeId::new(0),
                ),
                none_pattern(),
            ],
        ),
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![
                        DraftPatternNode::Wildcard,
                        DraftPatternNode::EnumVariant {
                            variant: alternate_variant,
                            fields: vec![DraftPatternField {
                                field: alternate_field,
                                pattern: DraftPatternNodeId::new(0),
                            }],
                        },
                    ],
                    DraftPatternNodeId::new(1),
                ),
                none_pattern(),
            ],
        ),
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![
                        DraftPatternNode::Wildcard,
                        DraftPatternNode::EnumVariant {
                            variant: some,
                            fields: vec![DraftPatternField {
                                field: alternate_field,
                                pattern: DraftPatternNodeId::new(0),
                            }],
                        },
                    ],
                    DraftPatternNodeId::new(1),
                ),
                none_pattern(),
            ],
        ),
        choice_match_with_patterns(
            some,
            value_field,
            vec![
                PatternDraft::new(
                    vec![DraftPatternNode::EnumVariant {
                        variant: some,
                        fields: Vec::new(),
                    }],
                    DraftPatternNodeId::new(0),
                ),
                none_pattern(),
            ],
        ),
    ];
    for draft in malformed {
        let result = workspace.apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole { hole, draft }],
        });
        assert!(result.is_err(), "malformed pattern must reject");
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }

    let foreign_pattern = PatternDraft::new(
        vec![
            DraftPatternNode::Wildcard,
            DraftPatternNode::EnumVariant {
                variant: foreign_some,
                fields: vec![DraftPatternField {
                    field: foreign_field,
                    pattern: DraftPatternNodeId::new(0),
                }],
            },
        ],
        DraftPatternNodeId::new(1),
    );
    let foreign_result = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole,
            draft: choice_match_with_patterns(
                some,
                value_field,
                vec![foreign_pattern, none_pattern()],
            ),
        }],
    });
    assert!(matches!(
        foreign_result,
        Err(WorkspaceError::ForeignNamespace(_))
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));
    let stale_result = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole,
            draft: choice_match_with_patterns(
                some,
                value_field,
                vec![
                    PatternDraft::new(
                        vec![DraftPatternNode::EnumVariant {
                            variant: stale,
                            fields: Vec::new(),
                        }],
                        DraftPatternNodeId::new(0),
                    ),
                    none_pattern(),
                ],
            ),
        }],
    });
    assert!(matches!(
        stale_result,
        Err(WorkspaceError::StaleIdentity(_))
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));
    let wrong_kind_result = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole,
            draft: choice_match_with_patterns(
                some,
                value_field,
                vec![
                    PatternDraft::new(
                        vec![DraftPatternNode::EnumVariant {
                            variant: choice,
                            fields: Vec::new(),
                        }],
                        DraftPatternNodeId::new(0),
                    ),
                    none_pattern(),
                ],
            ),
        }],
    });
    assert!(matches!(
        wrong_kind_result,
        Err(WorkspaceError::WrongEntityKind { .. })
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let pattern_with_field = |field| {
        PatternDraft::new(
            vec![
                DraftPatternNode::Wildcard,
                DraftPatternNode::EnumVariant {
                    variant: some,
                    fields: vec![DraftPatternField {
                        field,
                        pattern: DraftPatternNodeId::new(0),
                    }],
                },
            ],
            DraftPatternNodeId::new(1),
        )
    };
    for (field, expected) in [
        (foreign_field, "foreign"),
        (stale, "stale"),
        (some, "wrong-kind"),
    ] {
        let result = workspace.apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: choice_match_with_patterns(
                    some,
                    value_field,
                    vec![pattern_with_field(field), none_pattern()],
                ),
            }],
        });
        match expected {
            "foreign" => assert!(matches!(result, Err(WorkspaceError::ForeignNamespace(_)))),
            "stale" => assert!(matches!(result, Err(WorkspaceError::StaleIdentity(_)))),
            "wrong-kind" => {
                assert!(matches!(
                    result,
                    Err(WorkspaceError::WrongEntityKind { .. })
                ))
            }
            _ => unreachable!("known malformed field case"),
        }
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }

    let non_enum_match = ExpressionDraft::new(
        vec![
            DraftNode::Bool(true),
            DraftNode::I64(1),
            DraftNode::Match {
                scrutinee: DraftNodeId::new(0),
                arms: vec![MatchArmDraft {
                    pattern: PatternDraft::wildcard(),
                    body: DraftNodeId::new(1),
                }],
            },
        ],
        DraftNodeId::new(2),
    );
    let non_enum_error = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: non_enum_match,
            }],
        })
        .expect_err("unsupported source-free pattern space must reject");
    assert!(matches!(
        non_enum_error,
        WorkspaceError::UnsupportedEdit { operation, .. } if operation.as_ref() == "match"
    ));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let payload = DraftBindingId::new(0);
    let cross_arm = ExpressionDraft::new(
        vec![
            DraftNode::I64(42),
            DraftNode::EnumValue {
                variant: some,
                fields: vec![DraftFieldValue {
                    field: value_field,
                    value: DraftNodeId::new(0),
                }],
            },
            DraftNode::I64(1),
            DraftNode::Load(DraftBindingRef::Local(payload)),
            DraftNode::Match {
                scrutinee: DraftNodeId::new(1),
                arms: vec![
                    MatchArmDraft {
                        pattern: PatternDraft::new(
                            vec![
                                DraftPatternNode::Binding {
                                    binding: payload,
                                    name: "payload".to_owned(),
                                },
                                DraftPatternNode::EnumVariant {
                                    variant: some,
                                    fields: vec![DraftPatternField {
                                        field: value_field,
                                        pattern: DraftPatternNodeId::new(0),
                                    }],
                                },
                            ],
                            DraftPatternNodeId::new(1),
                        ),
                        body: DraftNodeId::new(2),
                    },
                    MatchArmDraft {
                        pattern: none_pattern(),
                        body: DraftNodeId::new(3),
                    },
                ],
            },
        ],
        DraftNodeId::new(4),
    );
    let incompatible_bodies = ExpressionDraft::new(
        vec![
            DraftNode::I64(42),
            DraftNode::EnumValue {
                variant: some,
                fields: vec![DraftFieldValue {
                    field: value_field,
                    value: DraftNodeId::new(0),
                }],
            },
            DraftNode::I64(1),
            DraftNode::Bool(false),
            DraftNode::Match {
                scrutinee: DraftNodeId::new(1),
                arms: vec![
                    MatchArmDraft {
                        pattern: PatternDraft::new(
                            vec![
                                DraftPatternNode::Wildcard,
                                DraftPatternNode::EnumVariant {
                                    variant: some,
                                    fields: vec![DraftPatternField {
                                        field: value_field,
                                        pattern: DraftPatternNodeId::new(0),
                                    }],
                                },
                            ],
                            DraftPatternNodeId::new(1),
                        ),
                        body: DraftNodeId::new(2),
                    },
                    MatchArmDraft {
                        pattern: none_pattern(),
                        body: DraftNodeId::new(3),
                    },
                ],
            },
        ],
        DraftNodeId::new(4),
    );
    for draft in [cross_arm, incompatible_bodies] {
        assert!(workspace
            .apply(Transaction {
                base_revision: published.revision(),
                edits: vec![Edit::FillHole { hole, draft }],
            })
            .is_err());
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }

    let malformed_duo_pattern =
        |left_binding: (DraftBindingId, &str), right_binding: (DraftBindingId, &str)| {
            PatternDraft::new(
                vec![
                    DraftPatternNode::Binding {
                        binding: left_binding.0,
                        name: left_binding.1.to_owned(),
                    },
                    DraftPatternNode::Binding {
                        binding: right_binding.0,
                        name: right_binding.1.to_owned(),
                    },
                    DraftPatternNode::EnumVariant {
                        variant: both,
                        fields: vec![
                            DraftPatternField {
                                field: left,
                                pattern: DraftPatternNodeId::new(0),
                            },
                            DraftPatternField {
                                field: right,
                                pattern: DraftPatternNodeId::new(1),
                            },
                        ],
                    },
                ],
                DraftPatternNodeId::new(2),
            )
        };
    for pattern in [
        malformed_duo_pattern(
            (DraftBindingId::new(0), "left"),
            (DraftBindingId::new(0), "right"),
        ),
        malformed_duo_pattern(
            (DraftBindingId::new(0), "same"),
            (DraftBindingId::new(1), "same"),
        ),
    ] {
        let draft = ExpressionDraft::new(
            vec![
                DraftNode::I64(1),
                DraftNode::I64(2),
                DraftNode::EnumValue {
                    variant: both,
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
                DraftNode::I64(0),
                DraftNode::Match {
                    scrutinee: DraftNodeId::new(2),
                    arms: vec![MatchArmDraft {
                        pattern,
                        body: DraftNodeId::new(3),
                    }],
                },
            ],
            DraftNodeId::new(4),
        );
        assert!(workspace
            .apply(Transaction {
                base_revision: published.revision(),
                edits: vec![Edit::FillHole { hole, draft }],
            })
            .is_err());
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }
}

#[test]
fn match_arm_hole_context_contains_only_its_payload_bindings() {
    let mut workspace = Workspace::empty_deterministic(82).expect("match hole workspace");
    let (_choice, some, _none, value_field) = create_choice(&mut workspace);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create match main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let payload = DraftBindingId::new(0);
    let fallback = DraftBindingId::new(1);
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(42),
                        DraftNode::EnumValue {
                            variant: some,
                            fields: vec![DraftFieldValue {
                                field: value_field,
                                value: DraftNodeId::new(0),
                            }],
                        },
                        DraftNode::Load(DraftBindingRef::Local(payload)),
                        DraftNode::I64(0),
                        DraftNode::Match {
                            scrutinee: DraftNodeId::new(1),
                            arms: vec![
                                MatchArmDraft {
                                    pattern: PatternDraft::new(
                                        vec![
                                            DraftPatternNode::Binding {
                                                binding: payload,
                                                name: "payload".to_owned(),
                                            },
                                            DraftPatternNode::EnumVariant {
                                                variant: some,
                                                fields: vec![DraftPatternField {
                                                    field: value_field,
                                                    pattern: DraftPatternNodeId::new(0),
                                                }],
                                            },
                                        ],
                                        DraftPatternNodeId::new(1),
                                    ),
                                    body: DraftNodeId::new(2),
                                },
                                MatchArmDraft {
                                    pattern: PatternDraft::new(
                                        vec![DraftPatternNode::Binding {
                                            binding: fallback,
                                            name: "fallback".to_owned(),
                                        }],
                                        DraftPatternNodeId::new(0),
                                    ),
                                    body: DraftNodeId::new(3),
                                },
                            ],
                        },
                    ],
                    DraftNodeId::new(4),
                ),
            }],
        })
        .expect("construct match with bindings in both arms");
    let payload_entity = entity_named(&completed.snapshot, EntityKind::ImmutableLocal, "payload");
    let fallback_entity = entity_named(&completed.snapshot, EntityKind::ImmutableLocal, "fallback");
    let site = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Match)
        .expect("match node")
        .id;
    let view = completed
        .snapshot
        .match_view(completed.snapshot.revision(), site)
        .expect("match view");
    let first_body = view.arms[0].body;
    let second_body = view.arms[1].body;
    let introduced = workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![
                Edit::IntroduceHole {
                    target: first_body,
                    goal: "use the payload".to_owned(),
                },
                Edit::IntroduceHole {
                    target: second_body,
                    goal: "handle the remainder".to_owned(),
                },
            ],
        })
        .expect("introduce arm holes");
    let first = introduced
        .snapshot
        .holes()
        .find(|hole| hole.context == first_body)
        .expect("first arm hole");
    let second = introduced
        .snapshot
        .holes()
        .find(|hole| hole.context == second_body)
        .expect("second arm hole");
    assert_eq!(first.visible_entities.as_ref(), &[payload_entity]);
    assert_eq!(second.visible_entities.as_ref(), &[fallback_entity]);
    let constructors = introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            first.id,
            PageRequest::new(16).expect("page"),
            None,
        )
        .expect("arm constructors")
        .items;
    assert!(constructors.contains(&LegalConstructor::Load(payload_entity)));
    assert!(!constructors.contains(&LegalConstructor::Load(fallback_entity)));
    let first_hole = first.id;
    let second_hole = second.id;
    let refilled = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![
                Edit::FillHole {
                    hole: first_hole,
                    draft: ExpressionDraft::new(
                        vec![DraftNode::Load(DraftBindingRef::Entity(payload_entity))],
                        DraftNodeId::new(0),
                    ),
                },
                Edit::FillHole {
                    hole: second_hole,
                    draft: ExpressionDraft::scalar_i64(0),
                },
            ],
        })
        .expect("refill arm holes");
    assert_eq!(run_i64(&refilled.snapshot), 42);
}

fn create_owned_workspace(seed: u64) -> (Workspace, EntityId, EntityId, HoleId, HoleId) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("owned workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "consume".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: vec![ParameterDraft {
                        name: "bytes".to_owned(),
                        ty: DeclarationType::ByteVector,
                    }],
                    return_type: DeclarationType::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create owned declarations");
    let function = entity_named(&created.snapshot, EntityKind::Function, "consume");
    let parameter = entity_named(&created.snapshot, EntityKind::Parameter, "bytes");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let function_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == function)
        .expect("function hole")
        .id;
    let main_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;
    (workspace, function, parameter, function_hole, main_hole)
}

fn fill_owned_helper(
    workspace: &mut Workspace,
    parameter: EntityId,
    hole: HoleId,
) -> Arc<WorkspaceSnapshot> {
    workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::BorrowShared(DraftBindingRef::Entity(parameter)),
                        DraftNode::Operation {
                            operation: crate::Operation::ByteSliceLength,
                            arguments: vec![DraftNodeId::new(0)],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("fill owned helper")
        .snapshot
}

fn valid_owned_main(function: EntityId) -> ExpressionDraft {
    let owner = DraftBindingId::new(0);
    ExpressionDraft::new(
        vec![
            DraftNode::Bytes(vec![1, 2, 3]),
            DraftNode::Operation {
                operation: crate::Operation::ThawBytes,
                arguments: vec![DraftNodeId::new(0)],
            },
            DraftNode::BorrowShared(DraftBindingRef::Local(owner)),
            DraftNode::Operation {
                operation: crate::Operation::ByteSliceLength,
                arguments: vec![DraftNodeId::new(2)],
            },
            DraftNode::Move(DraftBindingRef::Local(owner)),
            DraftNode::Call {
                callee: function,
                type_arguments: Vec::new(),
                arguments: vec![DraftNodeId::new(4)],
            },
            DraftNode::Operation {
                operation: crate::Operation::Add,
                arguments: vec![DraftNodeId::new(3), DraftNodeId::new(5)],
            },
            DraftNode::Let {
                bindings: vec![LocalDraft {
                    binding: owner,
                    name: "owner".to_owned(),
                    value: DraftNodeId::new(1),
                }],
                body: DraftNodeId::new(6),
            },
        ],
        DraftNodeId::new(7),
    )
}

#[test]
fn source_free_return_moves_an_affine_value_to_its_caller() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    let mut workspace = Workspace::empty_deterministic(242).expect("affine return workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "make-owner".to_owned(),
                    type_parameters: Vec::new(),
                    parameters: Vec::new(),
                    return_type: DeclarationType::ByteVector,
                },
                Edit::CreateMain {
                    return_type: SemanticType::I64,
                },
            ],
        })
        .expect("create affine-return function and main");
    let function = entity_named(&created.snapshot, EntityKind::Function, "make-owner");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let function_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == function)
        .expect("function hole")
        .id;
    let main_hole = created
        .snapshot
        .holes()
        .find(|hole| hole.owner == main)
        .expect("main hole")
        .id;
    let function_owner = DraftBindingId::new(0);
    let function_filled = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: function_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(2),
                        DraftNode::Operation {
                            operation: crate::Operation::ByteVectorNew,
                            arguments: vec![DraftNodeId::new(0)],
                        },
                        DraftNode::Move(DraftBindingRef::Local(function_owner)),
                        DraftNode::Return {
                            value: DraftNodeId::new(2),
                        },
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: function_owner,
                                name: "created".to_owned(),
                                value: DraftNodeId::new(1),
                            }],
                            body: DraftNodeId::new(3),
                        },
                    ],
                    DraftNodeId::new(4),
                ),
            }],
        })
        .expect("return moved owner from function");
    let caller_owner = DraftBindingId::new(0);
    let completed = workspace
        .apply(Transaction {
            base_revision: function_filled.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Call {
                            callee: function,
                            type_arguments: Vec::new(),
                            arguments: Vec::new(),
                        },
                        DraftNode::BorrowShared(DraftBindingRef::Local(caller_owner)),
                        DraftNode::Operation {
                            operation: crate::Operation::ByteSliceLength,
                            arguments: vec![DraftNodeId::new(1)],
                        },
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: caller_owner,
                                name: "received".to_owned(),
                                value: DraftNodeId::new(0),
                            }],
                            body: DraftNodeId::new(2),
                        },
                    ],
                    DraftNodeId::new(3),
                ),
            }],
        })
        .expect("receive and inspect returned owner");
    let executable =
        crate::compile_snapshot(&completed.snapshot).expect("compile affine early return");
    assert!(executable
        .memory_plan()
        .obligations
        .iter()
        .any(|obligation| {
            obligation.kind == crate::memory_plan::MemoryObligationKind::DropWholeValue
        }));
    assert!(executable
        .memory_plan()
        .obligations
        .iter()
        .any(|obligation| obligation.kind == crate::memory_plan::MemoryObligationKind::EndBorrow));
    let outcome = run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(outcome.cleanup_failures().is_none());
    assert!(matches!(
        outcome,
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(2)
    ));
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

fn ownership_control_return_draft() -> ExpressionDraft {
    let owner = DraftBindingId::new(0);
    ExpressionDraft::new(
        vec![
            DraftNode::I64(2),
            DraftNode::Operation {
                operation: crate::Operation::ByteVectorNew,
                arguments: vec![DraftNodeId::new(0)],
            },
            DraftNode::BorrowShared(DraftBindingRef::Local(owner)),
            DraftNode::Operation {
                operation: crate::Operation::ByteSliceLength,
                arguments: vec![DraftNodeId::new(2)],
            },
            DraftNode::I64(7),
            DraftNode::Return {
                value: DraftNodeId::new(4),
            },
            DraftNode::Sequence(vec![DraftNodeId::new(3), DraftNodeId::new(5)]),
            DraftNode::Let {
                bindings: vec![LocalDraft {
                    binding: owner,
                    name: "b".to_owned(),
                    value: DraftNodeId::new(1),
                }],
                body: DraftNodeId::new(6),
            },
        ],
        DraftNodeId::new(7),
    )
}

#[test]
fn source_free_early_return_executes_cleans_up_and_matches_imported_semantics() {
    const SOURCE: &str =
        include_str!("../../../lkjscript-app/tests/fixtures/ownership-control.lkjscript");
    let namespace = WorkspaceNamespace::deterministic(230);
    let imported = importer::import_source_with_namespace(
        SOURCE,
        "workspace-ownership-control-return.lkjscript",
        namespace,
    )
    .expect("import ownership-control return fixture");

    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    crate::pipeline::reset_lowering_invocations();
    let mut workspace = Workspace::empty_deterministic(230).expect("source-free return workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create source-free return main");
    let main = entity_named(&created.snapshot, EntityKind::Main, "main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ownership_control_return_draft(),
            }],
        })
        .expect("fill ownership-control return body");

    assert_eq!(completed.snapshot.state(), ProgramState::Complete);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
    assert_eq!(
        canonical_workspace_observation(&imported),
        canonical_workspace_observation(&completed.snapshot)
    );
    assert!(matches!(
        imported
            .program
            .main
            .as_ref()
            .expect("imported main")
            .origin,
        crate::hir::Origin::Source(_)
    ));
    assert_eq!(
        completed
            .snapshot
            .program
            .main
            .as_ref()
            .expect("source-free main")
            .origin,
        crate::hir::Origin::Semantic
    );

    let return_node = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Return)
        .expect("return node");
    let return_facts = completed
        .snapshot
        .node_semantics(completed.snapshot.revision(), return_node.id)
        .expect("return facts");
    assert_eq!(return_facts.actual, SemanticType::Never);
    assert_eq!(return_facts.expected, Some(SemanticType::I64));
    assert!(return_facts.effects.contains(EffectSummary::MAY_DIVERGE));
    let return_children = completed
        .snapshot
        .containment()
        .iter()
        .filter_map(|edge| match (edge.owner, edge.child) {
            (SemanticOwner::Node(owner), SemanticChild::Node(child)) if owner == return_node.id => {
                Some(child)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(return_children.len(), 1);
    let value_facts = completed
        .snapshot
        .node_semantics(completed.snapshot.revision(), return_children[0])
        .expect("return value facts");
    assert_eq!(value_facts.actual, SemanticType::I64);
    assert_eq!(value_facts.expected, Some(SemanticType::I64));
    let projection = completed
        .snapshot
        .project(&[ProjectionSlice::Body(main)])
        .expect("source-free return projection");
    assert!(projection.contains("kind=return"), "{projection}");
    assert!(!projection.contains("ownership-control"), "{projection}");

    let imported_executable = crate::compile_snapshot(&imported).expect("compile imported return");
    let source_free_executable =
        crate::compile_snapshot(&completed.snapshot).expect("compile source-free return");
    assert_eq!(crate::pipeline::lowering_invocations(), 2);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
    let obligation_kinds = |executable: &crate::ExecutableProgram| {
        executable
            .memory_plan()
            .obligations
            .iter()
            .map(|obligation| obligation.kind)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        obligation_kinds(&imported_executable),
        obligation_kinds(&source_free_executable)
    );
    assert_eq!(
        obligation_kinds(&source_free_executable)
            .iter()
            .filter(|kind| { **kind == crate::memory_plan::MemoryObligationKind::DropWholeValue })
            .count(),
        1
    );
    assert_eq!(
        imported_executable.bytecode().main().code,
        source_free_executable.bytecode().main().code
    );
    for executable in [&imported_executable, &source_free_executable] {
        let outcome = run_chunk(
            executable.bytecode(),
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        );
        assert!(outcome.cleanup_failures().is_none());
        assert!(matches!(
            outcome,
            ExecutionOutcome::Returned(value) if value.as_i64() == Some(7)
        ));
    }
}

#[test]
fn source_free_byte_vector_borrow_then_move_executes_and_cleans_up() {
    crate::source::reset_parser_invocation_count();
    crate::source::reset_source_load_invocation_count();
    crate::pipeline::reset_lowering_invocations();
    let (mut workspace, function, parameter, function_hole, main_hole) = create_owned_workspace(55);
    let helper = fill_owned_helper(&mut workspace, parameter, function_hole);
    assert!(matches!(
        crate::compile_snapshot(&helper),
        Err(crate::CompileSnapshotError::Incomplete(_))
    ));
    assert_eq!(crate::pipeline::lowering_invocations(), 0);
    let completed = workspace
        .apply(Transaction {
            base_revision: helper.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: valid_owned_main(function),
            }],
        })
        .expect("fill valid owned main");
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
    let executable = crate::compile_snapshot(&completed.snapshot).expect("compile owned snapshot");
    assert!(executable
        .memory_plan()
        .obligations
        .iter()
        .any(|obligation| obligation.kind == crate::memory_plan::MemoryObligationKind::EndBorrow));
    assert!(executable
        .memory_plan()
        .obligations
        .iter()
        .any(|obligation| obligation.kind
            == crate::memory_plan::MemoryObligationKind::DropWholeValue));
    let outcome = run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(outcome.cleanup_failures().is_none());
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(6)));
    assert_eq!(crate::pipeline::lowering_invocations(), 1);

    let owner = entity_named(&completed.snapshot, EntityKind::ImmutableLocal, "owner");
    let main = entity_named(&completed.snapshot, EntityKind::Main, "main");
    let root = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Let && node.owner == SemanticOwner::Entity(main))
        .expect("owned local root")
        .id;
    let removed = workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: root,
                draft: ExpressionDraft::scalar_i64(7),
            }],
        })
        .expect("remove ownership-sensitive local subtree");
    assert!(removed.snapshot.entity(owner).is_err());
    let executable =
        crate::compile_snapshot(&removed.snapshot).expect("compile after owned-local removal");
    let outcome = run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(outcome.cleanup_failures().is_none());
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(7)));
    assert_eq!(crate::pipeline::lowering_invocations(), 2);
    assert_eq!(crate::source::parser_invocation_count(), 0);
    assert_eq!(crate::source::source_load_invocation_count(), 0);
}

#[test]
fn owned_holes_report_only_truthful_move_and_borrow_candidates() {
    let (mut workspace, function, parameter, function_hole, main_hole) = create_owned_workspace(71);
    fill_owned_helper(&mut workspace, parameter, function_hole);
    let completed = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: valid_owned_main(function),
            }],
        })
        .expect("construct ownership candidates")
        .snapshot;
    let owner = entity_named(&completed, EntityKind::ImmutableLocal, "owner");
    let borrow = completed
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Borrow)
        .expect("borrow node")
        .id;
    let moved = completed
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Move)
        .expect("move node")
        .id;
    let introduced = workspace
        .apply(Transaction {
            base_revision: completed.revision(),
            edits: vec![
                Edit::IntroduceHole {
                    target: borrow,
                    goal: "borrow the owner".to_owned(),
                },
                Edit::IntroduceHole {
                    target: moved,
                    goal: "move the owner".to_owned(),
                },
            ],
        })
        .expect("introduce ownership holes");
    let page = PageRequest::new(32).expect("page");
    let borrow_hole = introduced
        .snapshot
        .holes()
        .find(|hole| hole.context == borrow)
        .expect("borrow hole");
    let borrow_constructors = introduced
        .snapshot
        .legal_constructors(introduced.snapshot.revision(), borrow_hole.id, page, None)
        .expect("borrow constructors")
        .items;
    assert!(
        borrow_constructors.contains(&LegalConstructor::BorrowShared {
            binding: owner,
            status: ConstructorStatus::RequiresOwnershipValidation,
        })
    );
    assert!(!borrow_constructors.contains(&LegalConstructor::Load(owner)));

    let move_hole = introduced
        .snapshot
        .holes()
        .find(|hole| hole.context == moved)
        .expect("move hole");
    let move_constructors = introduced
        .snapshot
        .legal_constructors(introduced.snapshot.revision(), move_hole.id, page, None)
        .expect("move constructors")
        .items;
    assert!(move_constructors.contains(&LegalConstructor::Move {
        binding: owner,
        status: ConstructorStatus::RequiresOwnershipValidation,
    }));
    assert!(!move_constructors.contains(&LegalConstructor::Load(owner)));
}

#[test]
fn source_free_bounds_failure_unwinds_owned_local_without_cleanup_failure() {
    let mut workspace = Workspace::empty_deterministic(56).expect("trap workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create trap main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let owner = DraftBindingId::new(0);
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Bytes(vec![1]),
                        DraftNode::Operation {
                            operation: crate::Operation::ThawBytes,
                            arguments: vec![DraftNodeId::new(0)],
                        },
                        DraftNode::BorrowShared(DraftBindingRef::Local(owner)),
                        DraftNode::I64(99),
                        DraftNode::Operation {
                            operation: crate::Operation::ByteSliceByteAt,
                            arguments: vec![DraftNodeId::new(2), DraftNodeId::new(3)],
                        },
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: owner,
                                name: "owner".to_owned(),
                                value: DraftNodeId::new(1),
                            }],
                            body: DraftNodeId::new(4),
                        },
                    ],
                    DraftNodeId::new(5),
                ),
            }],
        })
        .expect("compile trap ownership state");
    let executable = crate::compile_snapshot(&completed.snapshot).expect("compile trap snapshot");
    assert!(executable
        .memory_plan()
        .obligations
        .iter()
        .any(|obligation| obligation.kind
            == crate::memory_plan::MemoryObligationKind::DropWholeValue));
    assert!(!executable.bytecode().main().failure_cleanups.is_empty());
    assert!(!executable
        .bytecode()
        .main()
        .failure_cleanup_ranges
        .is_empty());
    let outcome = run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(outcome.cleanup_failures().is_none());
    match outcome {
        ExecutionOutcome::Trapped(trap) => {
            assert!(trap.as_str().contains("byte-slice-byte-at out of bounds"));
        }
        other => panic!("unexpected bounds outcome: {other:?}"),
    }
}

type CanonicalNodeObservation = (
    NodeKind,
    String,
    Option<String>,
    Option<crate::Operation>,
    u16,
);

#[derive(Debug, Eq, PartialEq)]
struct CanonicalWorkspaceObservation {
    entities: Vec<(String, EntityKind, Option<String>)>,
    nodes: Vec<CanonicalNodeObservation>,
    containment: Vec<(String, String)>,
    references: Vec<(usize, String)>,
    calls: Vec<(String, String, usize)>,
    dependencies: Vec<(String, String)>,
    effects: Vec<u16>,
    diagnostics: Vec<(String, String)>,
}

fn canonical_entity_path(snapshot: &WorkspaceSnapshot, mut entity: EntityId) -> String {
    let mut parts = Vec::new();
    loop {
        let header = snapshot.entity(entity).expect("canonical entity");
        parts.push(
            header
                .name
                .rsplit(':')
                .next()
                .unwrap_or(&header.name)
                .to_owned(),
        );
        let Some(owner) = header.owner else {
            break;
        };
        entity = owner;
    }
    parts.reverse();
    parts.join("/")
}

fn canonical_type_text(snapshot: &WorkspaceSnapshot, ty: &crate::Type) -> String {
    let mut display = ty.to_string();
    for product in &snapshot.program.products {
        let name = product.name.rsplit(':').next().unwrap_or(&product.name);
        display = display.replace(&product.name, name);
    }
    for enumeration in &snapshot.program.enums {
        let name = enumeration
            .name
            .rsplit(':')
            .next()
            .unwrap_or(&enumeration.name);
        display = display.replace(&enumeration.name, name);
    }
    display
}

fn canonical_workspace_observation(snapshot: &WorkspaceSnapshot) -> CanonicalWorkspaceObservation {
    let mut entities = snapshot
        .entities()
        .iter()
        .map(|entity| {
            let index = snapshot.indexes.entity_lookup[&entity.id];
            (
                canonical_entity_path(snapshot, entity.id),
                entity.kind,
                snapshot.indexes.entity_types[index]
                    .as_ref()
                    .map(|ty| canonical_type_text(snapshot, ty)),
            )
        })
        .collect::<Vec<_>>();
    entities.sort();
    let nodes = snapshot
        .nodes()
        .iter()
        .enumerate()
        .map(|(index, node)| {
            let facts = snapshot
                .node_semantics(snapshot.revision(), node.id)
                .expect("canonical node semantics");
            (
                node.kind,
                canonical_type_text(snapshot, &snapshot.indexes.node_actual_types[index]),
                snapshot.indexes.node_expected_types[index]
                    .as_ref()
                    .map(|ty| canonical_type_text(snapshot, ty)),
                facts.operation,
                facts.effects.bits(),
            )
        })
        .collect();
    let node_index = snapshot
        .nodes()
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id, index))
        .collect::<std::collections::HashMap<_, _>>();
    let owner = |owner: SemanticOwner| match owner {
        SemanticOwner::Entity(entity) => {
            format!("entity:{}", canonical_entity_path(snapshot, entity))
        }
        SemanticOwner::Node(node) => format!("node:{}", node_index[&node]),
    };
    let child = |child: SemanticChild| match child {
        SemanticChild::Entity(entity) => {
            format!("entity:{}", canonical_entity_path(snapshot, entity))
        }
        SemanticChild::Node(node) => format!("node:{}", node_index[&node]),
    };
    let containment = snapshot
        .containment()
        .iter()
        .map(|edge| (owner(edge.owner), child(edge.child)))
        .collect::<Vec<_>>();
    let mut references = snapshot
        .references()
        .iter()
        .map(|edge| {
            (
                node_index[&edge.site],
                canonical_entity_path(snapshot, edge.target),
            )
        })
        .collect::<Vec<_>>();
    references.sort();
    let mut calls = snapshot
        .calls()
        .iter()
        .map(|edge| {
            (
                canonical_entity_path(snapshot, edge.caller),
                canonical_entity_path(snapshot, edge.callee),
                node_index[&edge.site],
            )
        })
        .collect::<Vec<_>>();
    calls.sort();
    let mut dependencies = snapshot
        .dependencies()
        .iter()
        .map(|edge| {
            (
                canonical_entity_path(snapshot, edge.dependent),
                canonical_entity_path(snapshot, edge.dependency),
            )
        })
        .collect::<Vec<_>>();
    dependencies.sort();
    let mut effects = snapshot
        .program
        .functions
        .iter()
        .map(|function| function.summary.bits())
        .collect::<Vec<_>>();
    effects.push(
        snapshot
            .program
            .main
            .as_ref()
            .expect("canonical main")
            .body
            .effects
            .bits(),
    );
    let diagnostics = snapshot
        .diagnostics()
        .iter()
        .map(|diagnostic| (diagnostic.code.to_string(), diagnostic.message.to_string()))
        .collect();
    CanonicalWorkspaceObservation {
        entities,
        nodes,
        containment,
        references,
        calls,
        dependencies,
        effects,
        diagnostics,
    }
}

#[test]
fn imported_nominal_local_and_ownership_programs_converge() {
    let product_source = concat!(
        "product/\nname/\npair\n/name\nfields/\n",
        "field/\nname/\nleft\n/name\ntype/\ni64\n/type\n/field\n",
        "field/\nname/\nright\n/name\ntype/\ni64\n/type\n/field\n",
        "/fields\n/product\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "let/\nbind/\npair-value\nproduct-value/\npair\nfield/\nleft\n20\n/field\n",
        "field/\nright\n22\n/field\n/product-value\n/bind\n",
        "field/\npair-value\nleft\n/field\n/let\n/main\n",
    );
    let imported_product =
        import_source(product_source, "product-convergence.lkjscript").expect("import product");
    let mut product_workspace =
        Workspace::empty_deterministic(66).expect("source-free product workspace");
    let (pair, left, right) = create_pair(&mut product_workspace);
    let created = product_workspace
        .apply(Transaction {
            base_revision: product_workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create product main");
    let hole = created.snapshot.holes().next().expect("product hole").id;
    let local = DraftBindingId::new(0);
    let source_free_product = product_workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(20),
                        DraftNode::I64(22),
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
                        DraftNode::Load(DraftBindingRef::Local(local)),
                        DraftNode::ProductField {
                            field: left,
                            value: DraftNodeId::new(3),
                        },
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: local,
                                name: "pair-value".to_owned(),
                                value: DraftNodeId::new(2),
                            }],
                            body: DraftNodeId::new(4),
                        },
                    ],
                    DraftNodeId::new(5),
                ),
            }],
        })
        .expect("construct source-free product")
        .snapshot;
    assert_eq!(
        canonical_workspace_observation(&imported_product),
        canonical_workspace_observation(&source_free_product)
    );
    assert_eq!(run_i64(&imported_product), 20);
    assert_eq!(run_i64(&source_free_product), 20);

    let enum_source = concat!(
        "enum/\nname/\nchoice\n/name\nvariants/\nvariant/\nname/\nsome\n/name\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\nmain/\nsig/\ninputs/\n/inputs\n",
        "output/\nchoice/\n/choice\n/output\n/sig\nlet/\nbind/\nchoice-value\n",
        "variant-value/\ntype/\nchoice/\n/choice\n/type\nvariant/\nsome\n/variant\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\n7\n/variant-field\n/fields\n/variant-value\n",
        "/bind\nchoice-value\n/let\n/main\n",
    );
    let imported_enum =
        import_source(enum_source, "enum-convergence.lkjscript").expect("import enum");
    let mut enum_workspace =
        Workspace::empty_deterministic(67).expect("source-free enum workspace");
    let enum_created = enum_workspace
        .apply(Transaction {
            base_revision: enum_workspace.current().revision(),
            edits: vec![Edit::CreateEnum {
                name: "choice".to_owned(),
                variants: vec![EnumVariantDraft {
                    name: "some".to_owned(),
                    fields: vec![EnumFieldDraft {
                        name: "value".to_owned(),
                        ty: SemanticType::I64,
                    }],
                }],
            }],
        })
        .expect("create source-free enum");
    let choice = entity_named(&enum_created.snapshot, EntityKind::Enum, "choice");
    let some = entity_named(&enum_created.snapshot, EntityKind::EnumVariant, "some");
    let value = entity_named(&enum_created.snapshot, EntityKind::EnumField, "value");
    let main = enum_workspace
        .apply(Transaction {
            base_revision: enum_created.snapshot.revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::Enum {
                    constructor: SemanticEnum::Entity(choice),
                    arguments: Vec::new(),
                },
            }],
        })
        .expect("create enum main");
    let hole = main.snapshot.holes().next().expect("enum hole").id;
    let local = DraftBindingId::new(0);
    let source_free_enum = enum_workspace
        .apply(Transaction {
            base_revision: main.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(7),
                        DraftNode::EnumValue {
                            variant: some,
                            fields: vec![DraftFieldValue {
                                field: value,
                                value: DraftNodeId::new(0),
                            }],
                        },
                        DraftNode::Load(DraftBindingRef::Local(local)),
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: local,
                                name: "choice-value".to_owned(),
                                value: DraftNodeId::new(1),
                            }],
                            body: DraftNodeId::new(2),
                        },
                    ],
                    DraftNodeId::new(3),
                ),
            }],
        })
        .expect("construct source-free enum")
        .snapshot;
    assert_eq!(
        canonical_workspace_observation(&imported_enum),
        canonical_workspace_observation(&source_free_enum)
    );
    let enum_field = |snapshot: &WorkspaceSnapshot| {
        let executable = crate::compile_snapshot(snapshot).expect("compile enum convergence");
        match run_chunk(
            executable.bytecode(),
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        ) {
            ExecutionOutcome::Returned(value) => value.enum_field_i64(0),
            outcome => panic!("unexpected enum outcome: {outcome:?}"),
        }
    };
    assert_eq!(enum_field(&imported_enum), Some(7));
    assert_eq!(enum_field(&source_free_enum), Some(7));

    let ownership_source = concat!(
        "def/\nname/\nconsume\n/name\nfn/\nsig/\ninputs/\nbyte-vector\n/inputs\n",
        "output/\ni64\n/output\n/sig\nparams/\nbytes\nbyte-vector\n/params\n",
        "byte-slice-length/\nborrow/\nbytes\n/borrow\n/byte-slice-length\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nowner\n",
        "thaw-bytes/\nbytes-literal/\n010203\n/bytes-literal\n/thaw-bytes\n/bind\n",
        "add/\nbyte-slice-length/\nborrow/\nowner\n/borrow\n/byte-slice-length\n",
        "consume/\nmove/\nowner\n/move\n/consume\n/add\n/let\n/main\n",
    );
    let imported_ownership = import_source(ownership_source, "ownership-convergence.lkjscript")
        .expect("import ownership");
    let (mut ownership_workspace, function, parameter, function_hole, main_hole) =
        create_owned_workspace(68);
    fill_owned_helper(&mut ownership_workspace, parameter, function_hole);
    let source_free_ownership = ownership_workspace
        .apply(Transaction {
            base_revision: ownership_workspace.current().revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: valid_owned_main(function),
            }],
        })
        .expect("construct source-free ownership")
        .snapshot;
    assert_eq!(
        canonical_workspace_observation(&imported_ownership),
        canonical_workspace_observation(&source_free_ownership)
    );
    for snapshot in [&imported_ownership, &source_free_ownership] {
        let executable = crate::compile_snapshot(snapshot).expect("compile ownership convergence");
        assert!(executable.memory_plan().obligations.iter().any(
            |obligation| obligation.kind == crate::memory_plan::MemoryObligationKind::EndBorrow
        ));
        let outcome = run_chunk(
            executable.bytecode(),
            &ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        );
        assert!(outcome.cleanup_failures().is_none());
        assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(6)));
    }
}

#[test]
fn generic_enum_holes_do_not_advertise_unavailable_source_free_constructors() {
    let source = concat!(
        "enum/\nname/\nremove\n/name\nvariants/\nvariant/\nname/\none\n/name\n",
        "fields/\n/fields\n/variant\n/variants\n/enum\n",
        "enum/\nname/\nmaybe\n/name\nforall/\nt\n/forall\nvariants/\n",
        "variant/\nname/\nsome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\n",
        "type/\nt\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nmaybe/\ni64\n/maybe\n/output\n/sig\n",
        "variant-value/\ntype/\nmaybe/\ni64\n/maybe\n/type\nvariant/\nsome\n/variant\n",
        "fields/\nvariant-field/\nname/\nvalue\n/name\n1\n/variant-field\n/fields\n",
        "/variant-value\n/main\n",
    );
    let imported = import_source(source, "generic-hole.lkjscript").expect("import generic enum");
    let target = imported.nodes()[0].id;
    let removed = imported
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Enum && entity.name.ends_with(":remove"))
        .expect("earlier enum")
        .id;
    let mut workspace = Workspace::new(imported).expect("generic workspace");
    let introduced = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::IntroduceHole {
                target,
                goal: "reconstruct the generic enum".to_owned(),
            }],
        })
        .expect("introduce generic hole");
    let hole = introduced.snapshot.holes().next().expect("generic hole");
    let enumeration = introduced
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Enum && entity.name.ends_with(":maybe"))
        .expect("generic enum entity")
        .id;
    let type_parameter = introduced
        .snapshot
        .entities()
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::TypeParameter && entity.owner == Some(enumeration)
        })
        .expect("generic enum binder")
        .id;
    let expected = SemanticType::Enum {
        constructor: SemanticEnum::Entity(enumeration),
        arguments: vec![SemanticType::I64],
    };
    assert_eq!(hole.expected_type, expected);
    let old = introduced.snapshot.clone();
    let compacted = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::DeleteEntity { entity: removed }],
        })
        .expect("delete earlier enum");
    assert_eq!(
        compacted
            .snapshot
            .entity(enumeration)
            .expect("generic enum survivor")
            .id,
        enumeration
    );
    assert_eq!(
        compacted
            .snapshot
            .entity(type_parameter)
            .expect("generic enum binder survivor")
            .id,
        type_parameter
    );
    let compacted_hole = compacted.snapshot.holes().next().expect("compacted hole");
    assert_eq!(compacted_hole.id, hole.id);
    assert_eq!(compacted_hole.expected_type, expected);
    assert_eq!(
        old.entity(type_parameter).expect("old binder").id,
        type_parameter
    );
    assert!(compacted
        .snapshot
        .legal_constructors(
            compacted.snapshot.revision(),
            compacted_hole.id,
            PageRequest::new(16).expect("page"),
            None,
        )
        .expect("generic constructors")
        .items
        .iter()
        .all(|constructor| !matches!(constructor, LegalConstructor::EnumVariant(_))));
    let main = compacted
        .snapshot
        .entities()
        .iter()
        .find(|entity| entity.kind == EntityKind::Main)
        .expect("generic enum main")
        .id;
    let deleted = workspace
        .apply(Transaction {
            base_revision: compacted.snapshot.revision(),
            edits: vec![
                Edit::DeleteEntity {
                    entity: enumeration,
                },
                Edit::DeleteEntity { entity: main },
            ],
        })
        .expect("delete generic enum with its dependent entry point");
    assert!(deleted.snapshot.entity(enumeration).is_err());
    assert!(deleted.snapshot.entity(type_parameter).is_err());
    assert_eq!(
        old.entity(type_parameter).expect("old binder").id,
        type_parameter
    );
}

fn invalid_owned_main(function: EntityId, kind: &str) -> ExpressionDraft {
    let owner = DraftBindingId::new(0);
    match kind {
        "load" => ExpressionDraft::new(
            vec![
                DraftNode::Bytes(vec![1]),
                DraftNode::Operation {
                    operation: crate::Operation::ThawBytes,
                    arguments: vec![DraftNodeId::new(0)],
                },
                DraftNode::Load(DraftBindingRef::Local(owner)),
                DraftNode::Call {
                    callee: function,
                    type_arguments: Vec::new(),
                    arguments: vec![DraftNodeId::new(2)],
                },
                DraftNode::Let {
                    bindings: vec![LocalDraft {
                        binding: owner,
                        name: "owner".to_owned(),
                        value: DraftNodeId::new(1),
                    }],
                    body: DraftNodeId::new(3),
                },
            ],
            DraftNodeId::new(4),
        ),
        "double-move" => ExpressionDraft::new(
            vec![
                DraftNode::Bytes(vec![1]),
                DraftNode::Operation {
                    operation: crate::Operation::ThawBytes,
                    arguments: vec![DraftNodeId::new(0)],
                },
                DraftNode::Move(DraftBindingRef::Local(owner)),
                DraftNode::Call {
                    callee: function,
                    type_arguments: Vec::new(),
                    arguments: vec![DraftNodeId::new(2)],
                },
                DraftNode::Move(DraftBindingRef::Local(owner)),
                DraftNode::Call {
                    callee: function,
                    type_arguments: Vec::new(),
                    arguments: vec![DraftNodeId::new(4)],
                },
                DraftNode::Operation {
                    operation: crate::Operation::Add,
                    arguments: vec![DraftNodeId::new(3), DraftNodeId::new(5)],
                },
                DraftNode::Let {
                    bindings: vec![LocalDraft {
                        binding: owner,
                        name: "owner".to_owned(),
                        value: DraftNodeId::new(1),
                    }],
                    body: DraftNodeId::new(6),
                },
            ],
            DraftNodeId::new(7),
        ),
        "borrowed" => {
            let reference = DraftBindingId::new(1);
            ExpressionDraft::new(
                vec![
                    DraftNode::Bytes(vec![1]),
                    DraftNode::Operation {
                        operation: crate::Operation::ThawBytes,
                        arguments: vec![DraftNodeId::new(0)],
                    },
                    DraftNode::BorrowShared(DraftBindingRef::Local(owner)),
                    DraftNode::Move(DraftBindingRef::Local(owner)),
                    DraftNode::Call {
                        callee: function,
                        type_arguments: Vec::new(),
                        arguments: vec![DraftNodeId::new(3)],
                    },
                    DraftNode::Load(DraftBindingRef::Local(reference)),
                    DraftNode::Operation {
                        operation: crate::Operation::ByteSliceLength,
                        arguments: vec![DraftNodeId::new(5)],
                    },
                    DraftNode::Operation {
                        operation: crate::Operation::Add,
                        arguments: vec![DraftNodeId::new(4), DraftNodeId::new(6)],
                    },
                    DraftNode::Let {
                        bindings: vec![LocalDraft {
                            binding: reference,
                            name: "reference".to_owned(),
                            value: DraftNodeId::new(2),
                        }],
                        body: DraftNodeId::new(7),
                    },
                    DraftNode::Let {
                        bindings: vec![LocalDraft {
                            binding: owner,
                            name: "owner".to_owned(),
                            value: DraftNodeId::new(1),
                        }],
                        body: DraftNodeId::new(8),
                    },
                ],
                DraftNodeId::new(9),
            )
        }
        _ => unreachable!("invalid ownership case"),
    }
}

#[test]
fn invalid_source_free_ownership_fails_before_publication() {
    for (offset, kind) in ["load", "double-move", "borrowed"].into_iter().enumerate() {
        let (mut workspace, function, parameter, function_hole, main_hole) =
            create_owned_workspace(60 + u64::try_from(offset).expect("seed"));
        fill_owned_helper(&mut workspace, parameter, function_hole);
        let published = workspace.current();
        let failure = workspace.apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole {
                hole: main_hole,
                draft: invalid_owned_main(function, kind),
            }],
        });
        assert!(failure.is_err(), "{kind} must fail");
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }
}

#[test]
fn nested_hole_visibility_is_exact_for_source_free_locals() {
    let mut workspace = Workspace::empty_deterministic(64).expect("scope workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create scope main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let local = DraftBindingId::new(0);
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(7),
                        DraftNode::Load(DraftBindingRef::Local(local)),
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: local,
                                name: "visible".to_owned(),
                                value: DraftNodeId::new(0),
                            }],
                            body: DraftNodeId::new(1),
                        },
                        DraftNode::I64(0),
                        DraftNode::Operation {
                            operation: crate::Operation::Add,
                            arguments: vec![DraftNodeId::new(2), DraftNodeId::new(3)],
                        },
                    ],
                    DraftNodeId::new(4),
                ),
            }],
        })
        .expect("fill local body");
    let local_entity = entity_named(&completed.snapshot, EntityKind::ImmutableLocal, "visible");
    let let_node = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Let)
        .expect("local let")
        .id;
    let load = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Load)
        .expect("local load")
        .id;
    let operation = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Operation)
        .expect("outer operation")
        .id;
    let outside = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Literal && node.owner == SemanticOwner::Node(operation))
        .expect("outside scalar")
        .id;

    let mut unresolved_workspace =
        Workspace::new((*completed.snapshot).clone()).expect("unresolved scope workspace");
    let unresolved = unresolved_workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![
                Edit::IntroduceUnresolvedValueReference {
                    target: load,
                    requested_name: "visible".to_owned(),
                },
                Edit::IntroduceUnresolvedValueReference {
                    target: outside,
                    requested_name: "visible".to_owned(),
                },
            ],
        })
        .expect("introduce scoped unresolved references");
    let nested_reference = unresolved
        .snapshot
        .unresolved_value_references()
        .find(|state| state.context == load)
        .expect("nested unresolved reference")
        .id;
    let outside_reference = unresolved
        .snapshot
        .unresolved_value_references()
        .find(|state| state.context == outside)
        .expect("outside unresolved reference")
        .id;
    assert_eq!(
        unresolved
            .snapshot
            .unresolved_value_reference_candidates(
                unresolved.snapshot.revision(),
                nested_reference,
                PageRequest::new(16).expect("candidate page"),
                None,
            )
            .expect("nested candidates")
            .items[0]
            .entity,
        local_entity
    );
    assert!(unresolved
        .snapshot
        .unresolved_value_reference_candidates(
            unresolved.snapshot.revision(),
            outside_reference,
            PageRequest::new(16).expect("candidate page"),
            None,
        )
        .expect("outside candidates")
        .items
        .is_empty());
    let mut local_resolution_workspace =
        Workspace::new((*unresolved.snapshot).clone()).expect("local resolution workspace");
    let local_resolved = local_resolution_workspace
        .apply(Transaction {
            base_revision: unresolved.snapshot.revision(),
            edits: vec![Edit::ResolveUnresolvedValueReference {
                reference: nested_reference,
                target: local_entity,
            }],
        })
        .expect("resolve nested local candidate");
    assert!(local_resolved
        .snapshot
        .references()
        .iter()
        .any(|edge| { edge.site == nested_reference.node() && edge.target == local_entity }));
    let local_complete = local_resolution_workspace
        .apply(Transaction {
            base_revision: local_resolved.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: outside_reference.node(),
                draft: ExpressionDraft::scalar_i64(0),
            }],
        })
        .expect("complete nested local resolution");
    assert_eq!(run_i64(&local_complete.snapshot), 7);

    let pruned = unresolved_workspace
        .apply(Transaction {
            base_revision: unresolved.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: let_node,
                draft: ExpressionDraft::scalar_i64(0),
            }],
        })
        .expect("replace unresolved ancestor");
    assert!(pruned.snapshot.node(nested_reference.node()).is_err());
    assert!(matches!(
        pruned
            .snapshot
            .unresolved_value_reference(pruned.snapshot.revision(), nested_reference,),
        Err(WorkspaceError::StaleIdentity(_))
    ));
    assert_eq!(
        pruned
            .snapshot
            .unresolved_value_reference(pruned.snapshot.revision(), outside_reference)
            .expect("sibling unresolved reference")
            .id,
        outside_reference
    );

    let introduced = workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![
                Edit::IntroduceHole {
                    target: load,
                    goal: "load the visible local".to_owned(),
                },
                Edit::IntroduceHole {
                    target: outside,
                    goal: "outside the local scope".to_owned(),
                },
            ],
        })
        .expect("introduce nested and outside holes");
    let nested = introduced
        .snapshot
        .holes()
        .find(|hole| hole.context == load)
        .expect("nested hole");
    let outside = introduced
        .snapshot
        .holes()
        .find(|hole| hole.context == outside)
        .expect("outside hole");
    assert!(nested.visible_entities.contains(&local_entity));
    assert!(!outside.visible_entities.contains(&local_entity));
    assert_eq!(nested.expected_type, SemanticType::I64);
    assert!(introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            nested.id,
            PageRequest::new(16).expect("page"),
            None,
        )
        .expect("local constructors")
        .items
        .contains(&LegalConstructor::Load(local_entity)));
    assert!(!introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            outside.id,
            PageRequest::new(16).expect("page"),
            None,
        )
        .expect("outside constructors")
        .items
        .contains(&LegalConstructor::Load(local_entity)));

    let nested_id = nested.id;
    let published = introduced.snapshot;
    for refine_first in [false, true] {
        let refine = Edit::RefineHole {
            hole: nested_id,
            expected_type: None,
            goal: "refine a soon-removed hole".to_owned(),
        };
        let replace = Edit::ReplaceExpression {
            target: let_node,
            draft: ExpressionDraft::scalar_i64(0),
        };
        let edits = if refine_first {
            vec![refine, replace]
        } else {
            vec![replace, refine]
        };
        let failure = workspace.apply(Transaction {
            base_revision: published.revision(),
            edits,
        });
        assert!(matches!(
            failure,
            Err(WorkspaceError::InvalidTransaction(_))
        ));
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }
}

#[test]
fn local_defining_subtrees_are_removed_compacted_and_tombstoned() {
    let mut workspace = Workspace::empty_deterministic(65).expect("local workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create local main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let local = DraftBindingId::new(0);
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(7),
                        DraftNode::Load(DraftBindingRef::Local(local)),
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: local,
                                name: "removed".to_owned(),
                                value: DraftNodeId::new(0),
                            }],
                            body: DraftNodeId::new(1),
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("fill local main");
    let published = completed.snapshot;
    let main = entity_named(&published, EntityKind::Main, "main");
    let local_entity = entity_named(&published, EntityKind::ImmutableLocal, "removed");
    let let_node = published
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Let)
        .expect("let node")
        .id;
    let load = published
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Load)
        .expect("load node")
        .id;
    let mut hole_workspace = Workspace::new((*published).clone()).expect("hole workspace");

    let replaced = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: let_node,
                draft: ExpressionDraft::scalar_i64(0),
            }],
        })
        .expect("replace local-defining subtree");
    assert_eq!(
        replaced.snapshot.node(let_node).expect("stable root").id,
        let_node
    );
    assert_eq!(
        replaced
            .snapshot
            .definition(replaced.snapshot.revision(), main)
            .expect("main")
            .id,
        main
    );
    assert!(replaced.snapshot.entity(local_entity).is_err());
    assert!(replaced.snapshot.node(load).is_err());
    assert!(replaced.snapshot.program.bindings.is_empty());
    assert_eq!(
        replaced
            .snapshot
            .program
            .main
            .as_ref()
            .expect("main")
            .local_count,
        0
    );
    assert!(replaced.diff.entries.iter().any(|entry| matches!(
        entry,
        SemanticDiffEntry::EntityDeleted { entity, .. } if *entity == local_entity
    )));
    assert_eq!(run_i64(&replaced.snapshot), 0);

    let introduced = hole_workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::IntroduceHole {
                target: let_node,
                goal: "replace the local declaration".to_owned(),
            }],
        })
        .expect("hole local-defining subtree");
    assert!(introduced.snapshot.entity(local_entity).is_err());
    assert_eq!(
        introduced
            .snapshot
            .holes()
            .next()
            .expect("typed hole")
            .id
            .node(),
        let_node
    );
    assert!(matches!(
        introduced
            .snapshot
            .program
            .main
            .as_ref()
            .expect("main")
            .body
            .kind,
        crate::hir::ExprKind::Hole
    ));
    let filled = hole_workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole: HoleId(let_node),
                draft: ExpressionDraft::scalar_i64(11),
            }],
        })
        .expect("fill compacted hole");
    assert_eq!(run_i64(&filled.snapshot), 11);
}

#[test]
fn surviving_lexical_local_keeps_identity_when_an_earlier_local_compacts() {
    let mut workspace = Workspace::empty_deterministic(138).expect("local relocation workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let a = DraftBindingId::new(0);
    let b = DraftBindingId::new(1);
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(1),
                        DraftNode::Load(DraftBindingRef::Local(a)),
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: a,
                                name: "a".to_owned(),
                                value: DraftNodeId::new(0),
                            }],
                            body: DraftNodeId::new(1),
                        },
                        DraftNode::I64(2),
                        DraftNode::Load(DraftBindingRef::Local(b)),
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: b,
                                name: "b".to_owned(),
                                value: DraftNodeId::new(3),
                            }],
                            body: DraftNodeId::new(4),
                        },
                        DraftNode::Operation {
                            operation: crate::Operation::Add,
                            arguments: vec![DraftNodeId::new(2), DraftNodeId::new(5)],
                        },
                    ],
                    DraftNodeId::new(6),
                ),
            }],
        })
        .expect("create sibling locals");
    let a_entity = entity_named(&completed.snapshot, EntityKind::ImmutableLocal, "a");
    let b_entity = entity_named(&completed.snapshot, EntityKind::ImmutableLocal, "b");
    let a_load = completed
        .snapshot
        .references()
        .iter()
        .find(|edge| edge.target == a_entity)
        .expect("a load")
        .site;
    let a_let = match completed.snapshot.node(a_load).expect("a load").owner {
        SemanticOwner::Node(owner) => owner,
        SemanticOwner::Entity(_) => panic!("load must be owned by let"),
    };
    let b_load = completed
        .snapshot
        .references()
        .iter()
        .find(|edge| edge.target == b_entity)
        .expect("b load")
        .site;
    let b_let = match completed.snapshot.node(b_load).expect("b load").owner {
        SemanticOwner::Node(owner) => owner,
        SemanticOwner::Entity(_) => panic!("load must be owned by let"),
    };
    assert_eq!(completed.snapshot.program.bindings[1].name, "b");

    let compacted = workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: a_let,
                draft: ExpressionDraft::scalar_i64(1),
            }],
        })
        .expect("remove earlier local");
    assert!(compacted.snapshot.entity(a_entity).is_err());
    assert_eq!(
        compacted.snapshot.entity(b_entity).expect("surviving b").id,
        b_entity
    );
    assert_eq!(
        compacted.snapshot.node(b_let).expect("surviving let").id,
        b_let
    );
    assert_eq!(
        compacted.snapshot.node(b_load).expect("surviving load").id,
        b_load
    );
    assert_eq!(compacted.snapshot.program.bindings.len(), 1);
    assert_eq!(compacted.snapshot.program.bindings[0].name, "b");
    assert_eq!(compacted.snapshot.program.bindings[0].id.raw(), 0);
    assert_eq!(
        compacted
            .snapshot
            .program
            .main
            .as_ref()
            .expect("main")
            .local_count,
        1
    );
    assert_eq!(run_i64(&compacted.snapshot), 3);
}

#[test]
fn inserting_a_local_before_a_survivor_rebuilds_places_in_evaluation_order() {
    let mut workspace = Workspace::empty_deterministic(140).expect("place-order workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let outer = DraftBindingId::new(0);
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(2),
                        DraftNode::Load(DraftBindingRef::Local(outer)),
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: outer,
                                name: "outer".to_owned(),
                                value: DraftNodeId::new(0),
                            }],
                            body: DraftNodeId::new(1),
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("create outer local");
    let outer_entity = entity_named(&completed.snapshot, EntityKind::ImmutableLocal, "outer");
    let outer_let = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Let)
        .expect("outer let")
        .id;
    let outer_load = completed
        .snapshot
        .references()
        .iter()
        .find(|edge| edge.target == outer_entity)
        .expect("outer load")
        .site;
    let initializer = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.kind == NodeKind::Literal && node.owner == SemanticOwner::Node(outer_let))
        .expect("outer initializer")
        .id;
    let inner = DraftBindingId::new(0);
    let edited = workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: initializer,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(5),
                        DraftNode::Load(DraftBindingRef::Local(inner)),
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: inner,
                                name: "inner".to_owned(),
                                value: DraftNodeId::new(0),
                            }],
                            body: DraftNodeId::new(1),
                        },
                    ],
                    DraftNodeId::new(2),
                ),
            }],
        })
        .expect("insert inner local before existing definition");
    assert_eq!(
        edited.snapshot.entity(outer_entity).expect("outer").id,
        outer_entity
    );
    assert_eq!(
        edited.snapshot.node(outer_let).expect("outer let").id,
        outer_let
    );
    assert_eq!(
        edited.snapshot.node(outer_load).expect("outer load").id,
        outer_load
    );
    let main = edited.snapshot.program.main.as_ref().expect("main");
    let crate::hir::ExprKind::Let {
        bindings: outer_bindings,
        ..
    } = &main.body.kind
    else {
        panic!("main must retain the outer let");
    };
    let crate::hir::ExprKind::Let {
        bindings: inner_bindings,
        ..
    } = &outer_bindings[0].value.kind
    else {
        panic!("outer initializer must contain the inserted let");
    };
    assert_eq!(inner_bindings[0].place.raw(), 0);
    assert_eq!(outer_bindings[0].place.raw(), 1);
    assert_eq!(outer_bindings[0].slot, 0);
    assert_eq!(inner_bindings[0].slot, 1);
    assert_eq!(main.local_count, 2);
    assert_eq!(run_i64(&edited.snapshot), 5);
}

#[test]
fn removing_an_earlier_match_compacts_and_preserves_a_later_plan() {
    let mut workspace = Workspace::empty_deterministic(141).expect("plan relocation workspace");
    let (_choice, some, none, value_field) = create_choice(&mut workspace);
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: deep_match_draft(2, some, none, value_field),
            }],
        })
        .expect("create two semantic matches");
    let mut sites = completed
        .snapshot
        .nodes()
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            completed.snapshot.indexes.node_match_plans[index].map(|plan| (plan.raw(), node.id))
        })
        .collect::<Vec<_>>();
    sites.sort_by_key(|(plan, _)| *plan);
    let [(0, earlier), (1, later)] = sites.as_slice() else {
        panic!("expected two densely ordered match sites: {sites:?}");
    };
    let edited = workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: *earlier,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::I64(9),
                        DraftNode::EnumValue {
                            variant: some,
                            fields: vec![DraftFieldValue {
                                field: value_field,
                                value: DraftNodeId::new(0),
                            }],
                        },
                    ],
                    DraftNodeId::new(1),
                ),
            }],
        })
        .expect("remove earlier match and compact retained plan");
    assert_eq!(edited.snapshot.program.match_plans.len(), 1);
    assert_eq!(edited.snapshot.program.match_plans[0].id.raw(), 0);
    assert_eq!(
        edited.snapshot.node(*later).expect("later match").id,
        *later
    );
    assert_eq!(
        edited
            .snapshot
            .match_view(edited.snapshot.revision(), *later)
            .expect("retained match view")
            .site,
        *later
    );
    let later_index = edited.snapshot.indexes.node_lookup[later];
    assert_eq!(
        edited.snapshot.indexes.node_match_plans[later_index]
            .expect("retained plan")
            .raw(),
        0
    );
    assert_eq!(run_i64(&edited.snapshot), 1);
}

#[test]
fn malformed_forward_and_out_of_scope_draft_bindings_are_atomic() {
    let mut workspace = Workspace::empty_deterministic(65).expect("scope workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create scope main");
    let published = created.snapshot;
    let hole = published.holes().next().expect("main hole").id;
    let local = DraftBindingId::new(0);

    let forward = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::Load(DraftBindingRef::Local(local)),
                    DraftNode::I64(1),
                    DraftNode::Let {
                        bindings: vec![LocalDraft {
                            binding: local,
                            name: "value".to_owned(),
                            value: DraftNodeId::new(0),
                        }],
                        body: DraftNodeId::new(1),
                    },
                ],
                DraftNodeId::new(2),
            ),
        }],
    });
    assert!(matches!(forward, Err(WorkspaceError::InvalidDraft(_))));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let out_of_scope = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::I64(1),
                    DraftNode::Load(DraftBindingRef::Local(local)),
                    DraftNode::Let {
                        bindings: vec![LocalDraft {
                            binding: local,
                            name: "value".to_owned(),
                            value: DraftNodeId::new(0),
                        }],
                        body: DraftNodeId::new(1),
                    },
                    DraftNode::Load(DraftBindingRef::Local(local)),
                    DraftNode::Operation {
                        operation: crate::Operation::Add,
                        arguments: vec![DraftNodeId::new(2), DraftNodeId::new(3)],
                    },
                ],
                DraftNodeId::new(4),
            ),
        }],
    });
    assert!(matches!(out_of_scope, Err(WorkspaceError::InvalidDraft(_))));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let interleaved = workspace
        .apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: ExpressionDraft::new(
                    vec![
                        DraftNode::Let {
                            bindings: vec![LocalDraft {
                                binding: local,
                                name: "value".to_owned(),
                                value: DraftNodeId::new(4),
                            }],
                            body: DraftNodeId::new(1),
                        },
                        DraftNode::Operation {
                            operation: crate::Operation::Add,
                            arguments: vec![DraftNodeId::new(3), DraftNodeId::new(2)],
                        },
                        DraftNode::I64(2),
                        DraftNode::Load(DraftBindingRef::Local(local)),
                        DraftNode::I64(40),
                    ],
                    DraftNodeId::new(0),
                ),
            }],
        })
        .expect("physical node order must not replace lexical scope");
    assert_eq!(run_i64(&interleaved.snapshot), 42);
}

#[test]
fn malformed_operation_and_node_drafts_are_atomic_and_retry_stable() {
    let mut workspace = Workspace::empty_deterministic(69).expect("draft workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create draft main");
    let published = created.snapshot;
    let hole = published.holes().next().expect("main hole").id;
    let drafts = [
        ExpressionDraft::new(
            vec![DraftNode::Operation {
                operation: crate::Operation::Add,
                arguments: Vec::new(),
            }],
            DraftNodeId::new(0),
        ),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(1),
                DraftNode::Bool(true),
                DraftNode::Operation {
                    operation: crate::Operation::Add,
                    arguments: vec![DraftNodeId::new(0), DraftNodeId::new(1)],
                },
            ],
            DraftNodeId::new(2),
        ),
        ExpressionDraft::new(vec![DraftNode::I64(1)], DraftNodeId::new(99)),
        ExpressionDraft::new(
            vec![
                DraftNode::I64(1),
                DraftNode::Operation {
                    operation: crate::Operation::Add,
                    arguments: vec![DraftNodeId::new(0), DraftNodeId::new(0)],
                },
            ],
            DraftNodeId::new(1),
        ),
    ];
    for draft in drafts {
        let failure = workspace.apply(Transaction {
            base_revision: published.revision(),
            edits: vec![Edit::FillHole { hole, draft }],
        });
        assert!(matches!(failure, Err(WorkspaceError::InvalidDraft(_))));
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }

    let mut control = Workspace::new((*published).clone()).expect("control workspace");
    let fill = |workspace: &mut Workspace| {
        workspace
            .apply(Transaction {
                base_revision: published.revision(),
                edits: vec![Edit::FillHole {
                    hole,
                    draft: ExpressionDraft::scalar_i64(42),
                }],
            })
            .expect("valid retry")
    };
    let retried = fill(&mut workspace);
    let clean = fill(&mut control);
    assert_eq!(retried.diff, clean.diff);
    assert_eq!(retried.snapshot.nodes(), clean.snapshot.nodes());
}

fn deep_local_draft(depth: usize) -> ExpressionDraft {
    let mut nodes = Vec::new();
    for index in 0..depth {
        nodes.push(DraftNode::I64(i64::try_from(index).expect("local value")));
    }
    let last = DraftBindingId::new(u64::try_from(depth - 1).expect("last binding"));
    nodes.push(DraftNode::Load(DraftBindingRef::Local(last)));
    let mut body = DraftNodeId::new(u64::try_from(depth).expect("body id"));
    for index in (0..depth).rev() {
        let node = DraftNodeId::new(u64::try_from(nodes.len()).expect("let id"));
        let binding = DraftBindingId::new(u64::try_from(index).expect("binding id"));
        let name = format!("local-{index}");
        let initial = DraftNodeId::new(u64::try_from(index).expect("initializer id"));
        nodes.push(if index % 2 == 0 {
            DraftNode::Let {
                bindings: vec![LocalDraft {
                    binding,
                    name,
                    value: initial,
                }],
                body,
            }
        } else {
            DraftNode::MutableLocal {
                binding,
                name,
                ty: SemanticType::I64,
                initial,
                body,
            }
        });
        body = node;
    }
    ExpressionDraft::new(nodes, body)
}

fn run_source_free_deep_locals(depth: usize, seed: u64) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("deep local workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticType::I64,
            }],
        })
        .expect("create deep local main");
    let hole = created.snapshot.holes().next().expect("main hole").id;
    let completed = workspace
        .apply(Transaction {
            base_revision: created.snapshot.revision(),
            edits: vec![Edit::FillHole {
                hole,
                draft: deep_local_draft(depth),
            }],
        })
        .expect("fill deep locals");
    assert_eq!(
        run_i64(&completed.snapshot),
        i64::try_from(depth - 1).expect("result")
    );
    let main = entity_named(&completed.snapshot, EntityKind::Main, "main");
    let root = completed
        .snapshot
        .nodes()
        .iter()
        .find(|node| node.owner == SemanticOwner::Entity(main))
        .expect("deep local root")
        .id;
    let removed = workspace
        .apply(Transaction {
            base_revision: completed.snapshot.revision(),
            edits: vec![Edit::ReplaceExpression {
                target: root,
                draft: ExpressionDraft::scalar_i64(42),
            }],
        })
        .expect("remove deep locals");
    assert!(removed.snapshot.program.bindings.is_empty());
    assert_eq!(
        removed
            .snapshot
            .program
            .main
            .as_ref()
            .expect("main")
            .local_count,
        0
    );
    assert_eq!(run_i64(&removed.snapshot), 42);
    drop(completed);
    drop(removed);
    drop(workspace);
}

#[test]
fn deep_source_free_locals_compile_execute_and_drop_on_small_stack() {
    std::thread::Builder::new()
        .name("workspace-deep-locals".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_source_free_deep_locals(512, 65))
        .expect("spawn deep local thread")
        .join()
        .expect("deep local thread completes");
}

#[test]
#[ignore = "20k-local locked-release source-free small-stack stress geometry"]
fn twenty_thousand_source_free_locals_compile_execute_and_drop_on_small_stack() {
    std::thread::Builder::new()
        .name("workspace-20k-locals".to_owned())
        .stack_size(128 * 1024)
        .spawn(|| run_source_free_deep_locals(20_000, 66))
        .expect("spawn deep local thread")
        .join()
        .expect("deep local thread completes");
}
