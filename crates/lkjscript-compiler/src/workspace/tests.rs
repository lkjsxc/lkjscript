#![allow(clippy::expect_used, clippy::panic)]

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
                    parameters: vec![ParameterDraft {
                        name: "value".to_owned(),
                        ty: SemanticTypeRef::I64,
                    }],
                    return_type: SemanticTypeRef::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticTypeRef::I64,
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
                    parameters: vec![ParameterDraft {
                        name: "value".to_owned(),
                        ty: SemanticTypeRef::I64,
                    }],
                    return_type: SemanticTypeRef::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticTypeRef::I64,
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
    assert_eq!(identity_context.expected_type, crate::Type::I64);
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
            .map(|node| (node.kind, node.actual_type.to_string()))
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
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: SemanticTypeRef::I64,
                }],
                return_type: SemanticTypeRef::I64,
            },
            Edit::CreateMain {
                return_type: SemanticTypeRef::I64,
            },
            Edit::CreateMain {
                return_type: SemanticTypeRef::I64,
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
                    parameters: vec![ParameterDraft {
                        name: "value".to_owned(),
                        ty: SemanticTypeRef::I64,
                    }],
                    return_type: SemanticTypeRef::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticTypeRef::I64,
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
            parameters: Vec::new(),
            return_type: SemanticTypeRef::I64,
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
            parameters: Vec::new(),
            return_type: SemanticTypeRef::I64,
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
            parameters: vec![ParameterDraft {
                name: "value".to_owned(),
                ty: SemanticTypeRef::ByteVector,
            }],
            return_type: SemanticTypeRef::ByteSlice,
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
            parameters: vec![
                ParameterDraft {
                    name: "value".to_owned(),
                    ty: SemanticTypeRef::I64,
                },
                ParameterDraft {
                    name: "value".to_owned(),
                    ty: SemanticTypeRef::I64,
                },
            ],
            return_type: SemanticTypeRef::I64,
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
    assert!(matches!(invisible, Err(WorkspaceError::InvisibleEntity)));
    assert!(Arc::ptr_eq(&published, &workspace.current()));

    let wrong_arity = workspace.apply(Transaction {
        base_revision: published.revision(),
        edits: vec![Edit::FillHole {
            hole: main_hole,
            draft: ExpressionDraft::new(
                vec![DraftNode::Call {
                    callee: function,
                    arguments: Vec::new(),
                }],
                DraftNodeId::new(0),
            ),
        }],
    });
    assert!(matches!(wrong_arity, Err(WorkspaceError::InvalidDraft(_))));
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
fn declarations_created_in_separate_revisions_refresh_hole_scope_and_keep_ids() {
    let mut function_first = Workspace::empty_deterministic(34).expect("function-first workspace");
    let function_created = function_first
        .apply(Transaction {
            base_revision: function_first.current().revision(),
            edits: vec![Edit::CreateFunction {
                name: "identity".to_owned(),
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: SemanticTypeRef::I64,
                }],
                return_type: SemanticTypeRef::I64,
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
                return_type: SemanticTypeRef::I64,
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
                return_type: SemanticTypeRef::I64,
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
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: SemanticTypeRef::I64,
                }],
                return_type: SemanticTypeRef::I64,
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
        "entity e0g1 kind=main name=\"main\" owner=-\n",
        "body e0g1 name=\"main\"\n",
        "  node n0g1 kind=literal type=\"i64\" expected=\"i64\"\n",
        "type n0g1 actual=\"i64\" expected=\"i64\"\n",
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
    assert_eq!(context.expected_type, crate::Type::I64);
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
    assert!(projection.contains("node n0g1 kind=hole type=\"i64\" expected=\"i64\" [HOLE]"));
    assert!(projection.contains("type n0g1 actual=\"i64\" expected=\"i64\" [HOLE]"));
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
                expected_type: Some(SemanticTypeRef::I64),
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
        .find(|node| node.owner == SemanticOwner::Node(root) && node.actual_type.as_ref() == "bool")
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
            .node(alternative)
            .expect("alternative root preserved")
            .actual_type
            .as_ref(),
        "i64"
    );
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

fn deep_if_draft(depth: usize) -> ExpressionDraft {
    let mut nodes = Vec::new();
    nodes
        .try_reserve(
            depth
                .checked_mul(3)
                .and_then(|count| count.checked_add(1))
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
    ExpressionDraft::new(nodes, expression)
}

fn run_source_free_deep(depth: usize, seed: u64) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("empty workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticTypeRef::I64,
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
    assert_eq!(completed.snapshot.nodes().len(), depth * 3 + 1);
    let projection = completed
        .snapshot
        .project(&[ProjectionSlice::Body(main)])
        .expect("deep body projection");
    assert_eq!(projection.matches("node n").count(), depth * 3 + 1);
    assert_eq!(run_i64(&completed.snapshot), 1);
}

#[test]
fn source_free_index_root_resolution_is_one_lookup_per_node() {
    for (seed, depth) in [(40, 32_usize), (41, 64), (42, 128)] {
        let mut workspace = Workspace::empty_deterministic(seed).expect("empty workspace");
        let created = workspace
            .apply(Transaction {
                base_revision: workspace.current().revision(),
                edits: vec![Edit::CreateMain {
                    return_type: SemanticTypeRef::I64,
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
                        ty: SemanticTypeRef::I64,
                    },
                    ProductFieldDraft {
                        name: "right".to_owned(),
                        ty: SemanticTypeRef::I64,
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
                            ty: SemanticTypeRef::I64,
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
fn source_free_nominal_declarations_publish_stable_children_types_and_dependencies() {
    let mut workspace = Workspace::empty_deterministic(50).expect("empty workspace");
    let (pair, left, right) = create_pair(&mut workspace);
    let pair_snapshot = workspace.current();
    assert_eq!(
        pair_snapshot
            .entity_type(pair_snapshot.revision(), pair)
            .expect("product type")
            .declared,
        Some(SemanticTypeView::Known(SemanticTypeRef::Product(pair)))
    );
    assert_eq!(
        pair_snapshot
            .entity_type(pair_snapshot.revision(), left)
            .expect("field type")
            .declared,
        Some(SemanticTypeView::Known(SemanticTypeRef::I64))
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
        Some(SemanticTypeView::Known(SemanticTypeRef::Enum(choice)))
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
                parameters: vec![ParameterDraft {
                    name: "value".to_owned(),
                    ty: SemanticTypeRef::Product(pair),
                }],
                return_type: SemanticTypeRef::Product(pair),
            }],
        })
        .expect("create nominal function");
    let keep = entity_named(&function.snapshot, EntityKind::Function, "keep-pair");
    let signature = function
        .snapshot
        .function_signature(function.snapshot.revision(), keep)
        .expect("structured nominal signature");
    assert_eq!(
        signature.parameters,
        vec![SemanticTypeView::Known(SemanticTypeRef::Product(pair))]
    );
    assert_eq!(
        signature.result,
        SemanticTypeView::Known(SemanticTypeRef::Product(pair))
    );
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
                    ty: SemanticTypeRef::I64,
                },
                ProductFieldDraft {
                    name: "value".to_owned(),
                    ty: SemanticTypeRef::I64,
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
                ty: SemanticTypeRef::ByteVector,
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
                        ty: SemanticTypeRef::I64,
                    },
                    EnumFieldDraft {
                        name: "value".to_owned(),
                        ty: SemanticTypeRef::I64,
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
                    ty: SemanticTypeRef::ByteSlice,
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
                ty: SemanticTypeRef::Product(foreign_pair),
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
                ty: SemanticTypeRef::Enum(local_pair),
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
                ty: SemanticTypeRef::Product(stale),
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
                    ty: SemanticTypeRef::I64,
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
                        ty: SemanticTypeRef::I64,
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
                return_type: SemanticTypeRef::I64,
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
                return_type: SemanticTypeRef::Product(pair),
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
                return_type: SemanticTypeRef::Enum(choice),
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
                return_type: SemanticTypeRef::I64,
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
                return_type: SemanticTypeRef::I64,
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

fn create_owned_workspace(seed: u64) -> (Workspace, EntityId, EntityId, HoleId, HoleId) {
    let mut workspace = Workspace::empty_deterministic(seed).expect("owned workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![
                Edit::CreateFunction {
                    name: "consume".to_owned(),
                    parameters: vec![ParameterDraft {
                        name: "bytes".to_owned(),
                        ty: SemanticTypeRef::ByteVector,
                    }],
                    return_type: SemanticTypeRef::I64,
                },
                Edit::CreateMain {
                    return_type: SemanticTypeRef::I64,
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
                return_type: SemanticTypeRef::I64,
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

#[derive(Debug, Eq, PartialEq)]
struct CanonicalWorkspaceObservation {
    entities: Vec<(String, EntityKind, Option<String>)>,
    nodes: Vec<(NodeKind, String, Option<String>)>,
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
            (
                node.kind,
                canonical_type_text(snapshot, &snapshot.indexes.node_actual_types[index]),
                snapshot.indexes.node_expected_types[index]
                    .as_ref()
                    .map(|ty| canonical_type_text(snapshot, ty)),
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
    let mut containment = snapshot
        .containment()
        .iter()
        .map(|edge| (owner(edge.owner), child(edge.child)))
        .collect::<Vec<_>>();
    containment.sort();
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
                return_type: SemanticTypeRef::I64,
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
                        ty: SemanticTypeRef::I64,
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
                return_type: SemanticTypeRef::Enum(choice),
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
        .find(|entity| entity.kind == EntityKind::Enum)
        .expect("generic enum entity")
        .id;
    assert!(matches!(
        hole.expected_semantic_type,
        SemanticTypeView::Unsupported {
            nominal: Some(identity),
            ..
        } if identity == enumeration
    ));
    assert!(introduced
        .snapshot
        .legal_constructors(
            introduced.snapshot.revision(),
            hole.id,
            PageRequest::new(16).expect("page"),
            None,
        )
        .expect("generic constructors")
        .items
        .iter()
        .all(|constructor| !matches!(constructor, LegalConstructor::EnumVariant(_))));
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
                    arguments: vec![DraftNodeId::new(2)],
                },
                DraftNode::Move(DraftBindingRef::Local(owner)),
                DraftNode::Call {
                    callee: function,
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
                return_type: SemanticTypeRef::I64,
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
    assert_eq!(
        nested.expected_semantic_type,
        SemanticTypeView::Known(SemanticTypeRef::I64)
    );
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
}

#[test]
fn local_defining_subtrees_cannot_be_removed_or_hidden() {
    let mut workspace = Workspace::empty_deterministic(65).expect("local workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticTypeRef::I64,
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
                                name: "retained".to_owned(),
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
    let mut control = Workspace::new((*published).clone()).expect("control workspace");

    for edit in [
        Edit::ReplaceExpression {
            target: let_node,
            draft: ExpressionDraft::scalar_i64(0),
        },
        Edit::IntroduceHole {
            target: let_node,
            goal: "remove the local declaration".to_owned(),
        },
    ] {
        let failure = workspace.apply(Transaction {
            base_revision: published.revision(),
            edits: vec![edit],
        });
        assert!(matches!(
            failure,
            Err(WorkspaceError::UnsupportedEdit { .. })
        ));
        assert!(Arc::ptr_eq(&published, &workspace.current()));
    }

    let apply_allowed = |workspace: &mut Workspace| {
        workspace
            .apply(Transaction {
                base_revision: published.revision(),
                edits: vec![Edit::IntroduceHole {
                    target: load,
                    goal: "edit inside the retained scope".to_owned(),
                }],
            })
            .expect("edit a non-defining descendant")
    };
    let after_failures = apply_allowed(&mut workspace);
    let clean_control = apply_allowed(&mut control);
    assert_eq!(after_failures.diff, clean_control.diff);
    assert_eq!(
        after_failures.snapshot.entities(),
        clean_control.snapshot.entities()
    );
}

#[test]
fn malformed_forward_and_out_of_scope_draft_bindings_are_atomic() {
    let mut workspace = Workspace::empty_deterministic(65).expect("scope workspace");
    let created = workspace
        .apply(Transaction {
            base_revision: workspace.current().revision(),
            edits: vec![Edit::CreateMain {
                return_type: SemanticTypeRef::I64,
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
                return_type: SemanticTypeRef::I64,
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
        nodes.push(DraftNode::Let {
            bindings: vec![LocalDraft {
                binding: DraftBindingId::new(u64::try_from(index).expect("binding id")),
                name: format!("local-{index}"),
                value: DraftNodeId::new(u64::try_from(index).expect("initializer id")),
            }],
            body,
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
                return_type: SemanticTypeRef::I64,
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
    drop(completed);
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
