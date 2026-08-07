#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_vm::{run_chunk, ExecutionInputs};

use super::*;

const SCALAR: &str = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n42\n/main\n";
const FUNCTION_PROGRAM: &str = "def/\nname/\nidentity\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nvalue\ni64\n/params\nvalue\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nidentity/\n41\n/identity\n/main\n";
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
fn compile_snapshot_never_invokes_the_parser_and_attachment_free_snapshot_executes() {
    crate::source::reset_parser_invocation_count();
    let imported = importer::import_source_with_namespace(
        SCALAR,
        "workspace-direct.lkjscript",
        WorkspaceNamespace::deterministic(11),
    )
    .expect("import typed snapshot");
    assert_eq!(crate::source::parser_invocation_count(), 1);

    let direct = WorkspaceSnapshot::from_hir_for_test(
        WorkspaceNamespace::deterministic(12),
        imported.hir().clone(),
        Arc::clone(&imported.provenance),
    )
    .expect("construct attachment-free programmatic snapshot from typed HIR");
    assert!(direct.attachments().is_none());

    crate::source::reset_parser_invocation_count();
    let executable = crate::compile_snapshot(&direct).expect("compile snapshot directly");
    assert_eq!(crate::source::parser_invocation_count(), 0);
    let outcome = run_chunk(
        executable.bytecode(),
        &ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(matches!(
        outcome,
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(42)
    ));
}

#[test]
fn compile_snapshot_rejects_malformed_hir_before_lowering() {
    let valid =
        import_source(SCALAR, "workspace-malformed.lkjscript").expect("import valid snapshot");
    let mut malformed_hir = valid.hir().clone();
    malformed_hir.main.arity = 1;
    let malformed = valid.malformed_hir_for_test(malformed_hir);

    let failure = crate::compile_snapshot(&malformed).expect_err("reject malformed snapshot HIR");
    assert!(failure
        .to_string()
        .contains("main signature lengths are inconsistent"));
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
            .actual
            .as_ref(),
        "i64"
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
    assert_eq!(context.expected_type, crate::Type::I64);
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
        crate::CompileSnapshotError::Incomplete(error) => assert_eq!(error.holes, vec![hole]),
        other => panic!("unexpected compile failure: {other}"),
    }

    let refined = workspace
        .apply(Transaction {
            base_revision: introduced.snapshot.revision(),
            edits: vec![Edit::RefineHole {
                hole,
                expected_type: Some(crate::Type::I64),
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
    let unsupported = workspace.apply(Transaction {
        base_revision: old_revision,
        edits: vec![Edit::ReplaceExpression {
            target: root,
            draft: ExpressionDraft::new(
                vec![
                    DraftNode::I64(1),
                    DraftNode::Match {
                        scrutinee: DraftNodeId::new(0),
                    },
                ],
                DraftNodeId::new(1),
            ),
        }],
    });
    assert!(matches!(
        unsupported,
        Err(WorkspaceError::UnsupportedEdit { .. })
    ));
    assert!(Arc::ptr_eq(&before, &workspace.current()));

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
fn unchanged_descendant_ids_follow_branch_reordering_and_removed_ids_are_stale() {
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
    assert_eq!(edited.snapshot.nodes()[2].id, old_two);
    assert_eq!(edited.snapshot.nodes()[3].id, old_one);
    assert_eq!(edited.snapshot.node(root).expect("root").id, root);

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

#[test]
#[ignore = "20k-node small-stack stress geometry"]
fn twenty_thousand_level_edit_and_drop_complete_on_a_small_native_stack() {
    const DEPTH: usize = 20_000;
    let snapshot = importer::import_source_with_namespace(
        SCALAR,
        "workspace-deep.lkjscript",
        WorkspaceNamespace::deterministic(26),
    )
    .expect("scalar import");
    let root = snapshot.nodes()[0].id;
    let mut nodes = Vec::new();
    nodes
        .try_reserve(
            DEPTH
                .checked_mul(3)
                .and_then(|count| count.checked_add(1))
                .expect("draft geometry"),
        )
        .expect("draft allocation");
    nodes.push(DraftNode::I64(1));
    let mut expression = DraftNodeId::new(0);
    for _ in 0..DEPTH {
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
    let draft = ExpressionDraft::new(nodes, expression);
    std::thread::Builder::new()
        .name("workspace-deep-edit".to_owned())
        .stack_size(128 * 1024)
        .spawn(move || {
            let revision = snapshot.revision();
            let mut workspace = Workspace::new(snapshot).expect("workspace");
            let edited = workspace
                .apply(Transaction {
                    base_revision: revision,
                    edits: vec![Edit::ReplaceExpression {
                        target: root,
                        draft,
                    }],
                })
                .expect("deep semantic edit");
            assert_eq!(edited.snapshot.nodes().len(), DEPTH * 3 + 1);
            drop(edited);
            drop(workspace);
        })
        .expect("spawn small-stack edit")
        .join()
        .expect("small-stack edit completes");
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
