#![allow(clippy::expect_used)]

use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_vm::{run_chunk, ExecutionInputs};

use super::*;

const SCALAR: &str = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n42\n/main\n";

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
