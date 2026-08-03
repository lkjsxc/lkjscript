use std::num::NonZeroUsize;
use std::path::Path;

use lkjscript_core::ExecutionOutcome;
use lkjscript_runtime::{Lifecycle, RuntimeError, RuntimeSystem};

use super::support::{package, root, structural_manifest};

#[test]
fn real_worker_rehydrates_nested_semantic_dag_in_fresh_parent_runtime() {
    let system = RuntimeSystem::new(
        lkjscript_runtime::CoordinatorIdentity::new(704).expect("coordinator"),
        NonZeroUsize::new(2).expect("cache"),
    );
    let app = system
        .install_isolated(
            structural_manifest("nested-process-return"),
            package(),
            &root(),
            Path::new(env!("CARGO_BIN_EXE_lkjscript-cell")),
            lkjscript_host::HostEnvironment::default(),
        )
        .expect("parent independently prepares nested return");
    let incarnation = system.start(app).expect("child independently prepares");
    let invocation = system
        .invoke(incarnation, Vec::new())
        .expect("invoke nested return");
    assert!(matches!(invocation.outcome, ExecutionOutcome::Returned(_)));
    let ExecutionOutcome::Returned(value) = &invocation.outcome else {
        return;
    };
    let snapshot = value.as_semantic_dag().expect("semantic DAG return");
    assert!(snapshot.metrics().nodes >= 3);
    let report = invocation
        .rehydration
        .expect("fresh parent rehydration report");
    assert_eq!(
        report.input_canonical_dag_hash,
        report.output_canonical_dag_hash
    );
    assert_eq!(report.nodes, snapshot.metrics().nodes);
    assert!(report.allocations > 0);
    assert!(report.releases > 0);
    assert!(report.cells_reclaimed > 0);
    assert_eq!(report.final_domains, 0);
    assert_eq!(report.final_owners, 0);
    assert_eq!(report.final_loans, 0);
    assert_eq!(report.final_dependencies, 0);
    assert_eq!(report.release_backlog, 0);
    assert!(report.bounded_release_work);
    system.stop(incarnation).expect("stop structural worker");
}

#[test]
fn provenance_rejection_preserves_worker_for_later_valid_rehydration() {
    let system = RuntimeSystem::new(
        lkjscript_runtime::CoordinatorIdentity::new(705).expect("coordinator"),
        NonZeroUsize::new(2).expect("cache"),
    );
    let app = system
        .install_isolated(
            structural_manifest("recoverable-process-return"),
            package(),
            &root(),
            Path::new(env!("CARGO_BIN_EXE_lkjscript-cell-test-worker")),
            lkjscript_host::HostEnvironment::default(),
        )
        .expect("install test worker");
    let incarnation = system.start(app).expect("start test worker");
    assert!(matches!(
        system.invoke(incarnation, vec!["malformed-provenance".into()]),
        Err(RuntimeError::ProcessCell(_))
    ));
    assert_eq!(
        system.status(app).expect("status").lifecycle,
        Lifecycle::Running
    );
    let valid = system
        .invoke(incarnation, Vec::new())
        .expect("later valid invocation");
    assert!(valid.rehydration.is_some());
    assert!(matches!(valid.outcome, ExecutionOutcome::Returned(_)));
    system.stop(incarnation).expect("stop test worker");
}
