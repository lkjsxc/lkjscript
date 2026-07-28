use super::*;
use crate::ResourceKind;

#[test]
fn cleanup_failures_bound_records_bytes_and_utf8() {
    let limits = CleanupFailureLimits::new(2, 4).unwrap_or_else(|| std::process::abort());
    let mut failures = CleanupFailures::new(limits);
    failures.push(
        CleanupPhase::Emergency,
        CleanupSubject::Resource(ResourceKind::FileReader),
        "aébc",
    );
    failures.push(
        CleanupPhase::RuntimeTeardown,
        CleanupSubject::Terminal,
        "second",
    );
    failures.push(
        CleanupPhase::RuntimeTeardown,
        CleanupSubject::StandardOutput,
        "third",
    );

    assert_eq!(failures.retained().len(), 2);
    assert_eq!(failures.retained()[0].message(), "aéb");
    assert_eq!(failures.retained()[0].omitted_message_bytes(), 1);
    assert_eq!(failures.retained()[1].message(), "");
    assert_eq!(failures.retained()[1].omitted_message_bytes(), 6);
    assert_eq!(failures.retained_message_bytes(), 4);
    assert_eq!(failures.omitted_message_bytes(), 12);
    assert_eq!(failures.omitted_failures(), 1);
}

#[test]
fn cleanup_attachment_retains_primary_outcome() {
    let mut failures = CleanupFailures::new(CleanupFailureLimits::default());
    failures.push(
        CleanupPhase::Emergency,
        CleanupSubject::UniqueStorage,
        "release failed",
    );
    let outcome = ExecutionOutcome::Exited(7).with_cleanup_failures(failures.clone());

    assert_eq!(outcome.primary(), &ExecutionOutcome::Exited(7));
    assert_eq!(outcome.cleanup_failures(), Some(&failures));
    assert_eq!(outcome.returned(), None);
    assert!(outcome
        .summary()
        .starts_with("CleanupFailed(primary=Exited(7)"));
}

#[test]
fn zero_retention_still_records_omitted_failure() {
    let limits = CleanupFailureLimits::new(0, 0).unwrap_or_else(|| std::process::abort());
    let mut failures = CleanupFailures::new(limits);
    failures.push(
        CleanupPhase::Ordinary,
        CleanupSubject::EvaluatorProvider,
        "failure",
    );
    assert!(!failures.is_empty());
    assert_eq!(failures.retained(), []);
    assert_eq!(failures.omitted_failures(), 1);
    assert_eq!(failures.omitted_message_bytes(), 7);
}
