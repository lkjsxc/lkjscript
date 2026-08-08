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
                        ty: crate::Type::I64,
                    }],
                    return_type: crate::Type::I64,
                },
                Edit::CreateMain {
                    return_type: crate::Type::I64,
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
                draft: ExpressionDraft::new(vec![DraftNode::Load(parameter)], DraftNodeId::new(0)),
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
                        ty: crate::Type::I64,
                    }],
                    return_type: crate::Type::I64,
                },
                Edit::CreateMain {
                    return_type: crate::Type::I64,
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
                draft: ExpressionDraft::new(vec![DraftNode::Load(parameter)], DraftNodeId::new(0)),
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
                    ty: crate::Type::I64,
                }],
                return_type: crate::Type::I64,
            },
            Edit::CreateMain {
                return_type: crate::Type::I64,
            },
            Edit::CreateMain {
                return_type: crate::Type::I64,
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
                        ty: crate::Type::I64,
                    }],
                    return_type: crate::Type::I64,
                },
                Edit::CreateMain {
                    return_type: crate::Type::I64,
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
            return_type: crate::Type::I64,
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
            return_type: crate::Type::I64,
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
            name: "owned".to_owned(),
            parameters: vec![ParameterDraft {
                name: "value".to_owned(),
                ty: crate::Type::ByteVector,
            }],
            return_type: crate::Type::ByteVector,
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
                    ty: crate::Type::I64,
                },
                ParameterDraft {
                    name: "value".to_owned(),
                    ty: crate::Type::I64,
                },
            ],
            return_type: crate::Type::I64,
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
            draft: ExpressionDraft::new(vec![DraftNode::Load(parameter)], DraftNodeId::new(0)),
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
                    ty: crate::Type::I64,
                }],
                return_type: crate::Type::I64,
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
                return_type: crate::Type::I64,
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
                return_type: crate::Type::I64,
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
                    ty: crate::Type::I64,
                }],
                return_type: crate::Type::I64,
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
                return_type: crate::Type::I64,
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
                    return_type: crate::Type::I64,
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
