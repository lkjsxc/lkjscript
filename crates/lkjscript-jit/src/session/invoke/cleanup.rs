use super::*;

pub(super) fn native_cleanup_failures(
    report: &InvocationReport,
    execution: &ExecutionPolicy,
) -> CleanupFailures {
    let mut failures = CleanupFailures::with_retention(execution.cleanup_retention());
    for failure in report.cleanup_failures() {
        failures.push(
            CleanupPhase::Ordinary,
            CleanupSubject::UniqueStorage,
            format!(
                "native instruction cleanup {:?} failed with {:?}",
                failure.slot(),
                failure.error()
            ),
        );
    }
    for _ in 0..report.omitted_cleanup_failures() {
        failures.push(
            CleanupPhase::Ordinary,
            CleanupSubject::UniqueStorage,
            "omitted native instruction cleanup failure",
        );
    }
    failures
}
