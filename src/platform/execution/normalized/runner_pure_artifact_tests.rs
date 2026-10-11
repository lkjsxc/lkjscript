#![allow(clippy::unwrap_used, clippy::panic)]
//! Exercise the actual invocation/encoding owner with retained physical workers.
//! Pure language targets cannot themselves start structured child tasks today.
use super::super::runner::execute_pure_artifact_invocation;
use super::*;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::runtime::structured::{StructuredExecutor, StructuredExecutorHandle};

fn fixture() -> (Arc<NormalizedProgram>, Name) {
    (
        Arc::new(prepare_snapshot(&pure_command_snapshot())),
        Name::new("pure").unwrap(),
    )
}

fn warmed_executor() -> StructuredExecutor {
    let executor = StructuredExecutor::for_test(1);
    let pair = executor
        .handle()
        .run(
            &ExecutionControl::uncancelled(),
            || 17,
            || (29, std::thread::current().id()),
        )
        .unwrap();
    assert!(pair.dispatched);
    assert_eq!((pair.left, pair.right.0), (17, 29));
    assert_ne!(pair.right.1, std::thread::current().id());
    assert_eq!(executor.observe().remaining_workers, 1);
    executor
}

fn assert_joined(handle: &StructuredExecutorHandle) {
    let observed = handle.observe();
    assert!(!observed.dispatch_open);
    assert_eq!(observed.workers_started, 1);
    assert_eq!(observed.joined_workers, 1);
    assert_eq!(observed.remaining_workers, 0);
    assert_eq!(observed.active_dispatches, 0);
}

#[test]
fn pure_artifact_success_joins_its_executor_before_returning_bytes() {
    let (program, target) = fixture();
    let policy = NormalizedCommandPolicy::default();
    let control = ExecutionControl::uncancelled();
    let invocation =
        prepare_command_invocation(&program, &target, b"[]", policy.json, &control).unwrap();
    let mut executor = warmed_executor();
    let observer = executor.handle();
    assert_eq!(
        execute_pure_artifact_invocation(&program, invocation, policy, &control, &mut executor)
            .unwrap(),
        b"null"
    );
    // The owner is still alive: Drop cannot be the source of this joined state.
    assert_joined(&observer);
    assert!(!control.is_cancelled());
}

#[test]
fn pure_artifact_invocation_encoding_and_cancellation_failures_join_then_recover() {
    let (program, target) = fixture();
    for (stage, class, code) in [
        (
            "invocation",
            DiagnosticClass::Resource,
            "normalized_instruction_steps",
        ),
        (
            "encoding",
            DiagnosticClass::Resource,
            "normalized_json_output_bytes",
        ),
        (
            "cancellation",
            DiagnosticClass::Cancelled,
            "execution_cancelled",
        ),
    ] {
        let mut policy = NormalizedCommandPolicy::default();
        if stage == "invocation" {
            policy.execution.instruction_steps = Some(1);
        } else if stage == "encoding" {
            // The literal input is two bytes; the complete Unit output is four.
            policy.json.maximum_bytes = 2;
        }
        let control = ExecutionControl::uncancelled();
        let invocation =
            prepare_command_invocation(&program, &target, b"[]", policy.json, &control).unwrap();
        let mut executor = warmed_executor();
        let observer = executor.handle();
        if stage == "cancellation" {
            control.cancel();
        }
        let error =
            execute_pure_artifact_invocation(&program, invocation, policy, &control, &mut executor)
                .unwrap_err();
        assert_eq!(error.class, class, "{stage}");
        assert_eq!(error.code, code, "{stage}");
        assert!(
            error.notes.is_empty(),
            "successful cleanup must not invent another failure"
        );
        assert_joined(&observer);
        assert_eq!(
            run_pure_artifact_command(
                &program,
                &target,
                b"[]",
                NormalizedCommandPolicy::default(),
                &ExecutionControl::uncancelled()
            )
            .unwrap(),
            b"null"
        );
    }
}

#[test]
fn pure_artifact_post_preflight_cancellation_checkpoints_always_join_owned_workers() {
    let (program, target) = fixture();
    let policy = NormalizedCommandPolicy::default();
    let mut failures = 0;
    let mut successes = 0;
    for checks in [0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 1024, 4096] {
        let invocation = prepare_command_invocation(
            &program,
            &target,
            b"[]",
            policy.json,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
        let mut executor = warmed_executor();
        let observer = executor.handle();
        let control = ExecutionControl::cancel_after_checks(checks);
        match execute_pure_artifact_invocation(
            &program,
            invocation,
            policy,
            &control,
            &mut executor,
        ) {
            Ok(bytes) => {
                successes += 1;
                assert_eq!(bytes, b"null");
            }
            Err(error) => {
                failures += 1;
                assert_eq!(error.class, DiagnosticClass::Cancelled);
                assert_eq!(error.code, "execution_cancelled");
            }
        }
        assert_joined(&observer);
    }
    assert!(failures > 0 && successes > 0);
    assert_eq!(
        run_pure_artifact_command(
            &program,
            &target,
            b"[]",
            policy,
            &ExecutionControl::uncancelled()
        )
        .unwrap(),
        b"null"
    );
}

/// Two separately caught panic-payload drops exhaust the worker's existing
/// boundaries. The final inert payload terminates that owned thread, making its
/// actual JoinHandle report failure. No scheduler state or diagnostic is forged.
fn failed_worker_executor() -> StructuredExecutor {
    struct FailingPayload(u8);
    impl Drop for FailingPayload {
        fn drop(&mut self) {
            if self.0 != 0 {
                std::panic::panic_any(FailingPayload(self.0 - 1));
            }
        }
    }
    let executor = StructuredExecutor::for_test(1);
    let control = ExecutionControl::uncancelled();
    let error = executor
        .handle()
        .run(
            &control,
            || 17,
            || -> () {
                std::panic::panic_any(FailingPayload(2));
            },
        )
        .unwrap_err();
    assert_eq!(error.code, "normalized_parallel_worker");
    assert_eq!(executor.observe().workers_started, 1);
    assert_eq!(executor.observe().active_dispatches, 0);
    assert!(control.is_cancelled());
    executor
}

#[test]
fn pure_artifact_cleanup_failure_rejects_success_and_preserves_primary_failure_diagnostics() {
    let (program, target) = fixture();
    for (stage, class, code) in [
        (
            "success",
            DiagnosticClass::Infrastructure,
            "normalized_parallel_worker",
        ),
        (
            "invocation",
            DiagnosticClass::Resource,
            "normalized_instruction_steps",
        ),
        (
            "encoding",
            DiagnosticClass::Resource,
            "normalized_json_output_bytes",
        ),
        (
            "cancellation",
            DiagnosticClass::Cancelled,
            "execution_cancelled",
        ),
    ] {
        let mut policy = NormalizedCommandPolicy::default();
        if stage == "invocation" {
            policy.execution.instruction_steps = Some(1);
        } else if stage == "encoding" {
            policy.json.maximum_bytes = 2;
        }
        let control = ExecutionControl::uncancelled();
        let invocation =
            prepare_command_invocation(&program, &target, b"[]", policy.json, &control).unwrap();
        let mut executor = failed_worker_executor();
        let observer = executor.handle();
        if stage == "cancellation" {
            control.cancel();
        }
        let error =
            execute_pure_artifact_invocation(&program, invocation, policy, &control, &mut executor)
                .unwrap_err();
        assert_eq!(error.class, class, "{stage}");
        assert_eq!(error.code, code, "{stage}");
        if stage != "success" {
            assert_eq!(error.notes, ["pure artifact structured cleanup failed with safe code 'normalized_parallel_worker'".to_owned()]);
        }
        assert_joined(&observer);
        // Cleanup is idempotently terminal, including its original failure.
        assert_eq!(
            executor.shutdown().unwrap_err().code,
            "normalized_parallel_worker"
        );
        assert_eq!(
            run_pure_artifact_command(
                &program,
                &target,
                b"[]",
                NormalizedCommandPolicy::default(),
                &ExecutionControl::uncancelled()
            )
            .unwrap(),
            b"null"
        );
    }
}
