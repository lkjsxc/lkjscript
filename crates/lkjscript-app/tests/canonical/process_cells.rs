use std::num::NonZeroUsize;
use std::path::Path;

use lkjscript_core::ExecutionOutcome;
use lkjscript_runtime::{
    Lifecycle, PackageContentId, ProcessCellState, RuntimeError, RuntimeSystem,
};

#[path = "process_cells/support.rs"]
mod support;
use support::{host, manifest, package, root};
#[path = "process_cells/structural.rs"]
mod structural;

#[test]
fn isolated_cell_executes_restarts_and_relays_private_stdio() {
    let system = RuntimeSystem::new(
        lkjscript_runtime::CoordinatorIdentity::new(701).expect("coordinator"),
        NonZeroUsize::new(4).expect("cache"),
    );
    let stdio = lkjscript_host::BufferedStdio::default();
    let app = system
        .install_isolated(
            manifest("isolated-hello"),
            package(),
            &root(),
            Path::new(env!("CARGO_BIN_EXE_lkjscript-cell")),
            host(&stdio),
        )
        .expect("install isolated app");
    let first = system.start(app).expect("start isolated app");
    let first_state = system.status(app).expect("running status").process_cell;
    assert!(matches!(first_state, ProcessCellState::Running { .. }));
    let ProcessCellState::Running {
        process: first_process,
    } = first_state
    else {
        return;
    };
    for _ in 0..2 {
        let outcome = system
            .invoke(first, vec!["ignored".into()])
            .expect("invoke cell");
        assert!(
            matches!(outcome.outcome, ExecutionOutcome::Returned(ref value) if value.is_unit()),
            "{:?}",
            outcome.outcome,
        );
    }
    assert_eq!(stdio.output().expect("cell output"), b"36288003628800");
    assert_eq!(stdio.flushes().expect("cell flushes"), 2);
    system.stop(first).expect("stop first cell");
    assert_eq!(
        system.status(app).expect("stopped status").process_cell,
        ProcessCellState::Stopped
    );
    let second = system.start(app).expect("restart isolated app");
    let second_state = system.status(app).expect("restarted status").process_cell;
    assert!(matches!(second_state, ProcessCellState::Running { .. }));
    let ProcessCellState::Running {
        process: second_process,
    } = second_state
    else {
        return;
    };
    assert_ne!(first, second);
    assert_ne!(first_process, second_process);
    assert!(matches!(
        system.invoke(first, Vec::new()),
        Err(RuntimeError::StaleIncarnation { .. })
    ));
    system.stop(second).expect("stop restarted cell");
}

#[test]
fn isolated_cell_rejects_mismatched_package_content_before_ready() {
    let system = RuntimeSystem::new(
        lkjscript_runtime::CoordinatorIdentity::new(703).expect("coordinator"),
        NonZeroUsize::new(2).expect("cache"),
    );
    let stdio = lkjscript_host::BufferedStdio::default();
    assert!(matches!(
        system.install_isolated(
            manifest("wrong-package"),
            PackageContentId::new([99; 32]).expect("wrong package identity"),
            &root(),
            Path::new(env!("CARGO_BIN_EXE_lkjscript-cell")),
            host(&stdio),
        ),
        Err(RuntimeError::ProcessCell(_))
    ));
    assert!(system.list().expect("application list").is_empty());
    assert!(stdio.output().expect("no output").is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn one_process_cell_crash_does_not_contaminate_another_application() {
    let system = RuntimeSystem::new(
        lkjscript_runtime::CoordinatorIdentity::new(702).expect("coordinator"),
        NonZeroUsize::new(4).expect("cache"),
    );
    let first_stdio = lkjscript_host::BufferedStdio::default();
    let second_stdio = lkjscript_host::BufferedStdio::default();
    let first = system
        .install_isolated(
            manifest("crash-cell"),
            package(),
            &root(),
            Path::new(env!("CARGO_BIN_EXE_lkjscript-cell")),
            host(&first_stdio),
        )
        .expect("install crash cell");
    let second = system
        .install_isolated(
            manifest("survivor-cell"),
            package(),
            &root(),
            Path::new(env!("CARGO_BIN_EXE_lkjscript-cell")),
            host(&second_stdio),
        )
        .expect("install survivor cell");
    let first_incarnation = system.start(first).expect("start crash cell");
    let second_incarnation = system.start(second).expect("start survivor cell");
    let first_state = system
        .status(first)
        .expect("crash cell status")
        .process_cell;
    assert!(matches!(first_state, ProcessCellState::Running { .. }));
    let ProcessCellState::Running { process } = first_state else {
        return;
    };
    let killed = std::process::Command::new("kill")
        .args(["-KILL", &process.to_string()])
        .status()
        .expect("kill worker");
    assert!(killed.success());
    assert!(matches!(
        system.invoke(first_incarnation, Vec::new()),
        Err(RuntimeError::ProcessCell(_))
    ));
    assert_eq!(
        system.status(first).expect("failed status").lifecycle,
        Lifecycle::Failed
    );
    let survivor = system
        .invoke(second_incarnation, Vec::new())
        .expect("survivor invokes");
    assert!(
        matches!(survivor.outcome, ExecutionOutcome::Returned(_)),
        "{:?}",
        survivor.outcome,
    );
    assert_eq!(second_stdio.output().expect("survivor output"), b"3628800");
    system.stop(second_incarnation).expect("stop survivor");
}
