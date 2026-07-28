mod bootstrap;
mod close;
mod metrics;
mod model;
mod provider;
mod session;
mod teardown;

use metrics::{EvalCleanupAttempt, EvalResourceMetrics, EvalResourceTeardown};
use model::ownership_for_kind;
pub use model::EvalResource;
use model::{provider_for_kind, FakeOwner};
use provider::FakeProviders;
pub(super) use session::EvalResources;

use super::EvalOutcome;

pub(super) fn finish_evaluation(
    resources: &mut EvalResources,
    primary: EvalOutcome,
    cleanup_failures: lkjscript_core::CleanupFailures,
) -> EvalOutcome {
    finish_evaluation_with_failures(resources, primary, cleanup_failures).0
}

#[cfg(test)]
fn finish_evaluation_with_report(
    resources: &mut EvalResources,
    primary: EvalOutcome,
) -> (EvalOutcome, EvalResourceTeardown) {
    let failures = lkjscript_core::CleanupFailures::new(resources.cleanup_failure_limits);
    finish_evaluation_with_failures(resources, primary, failures)
}

fn finish_evaluation_with_failures(
    resources: &mut EvalResources,
    primary: EvalOutcome,
    mut cleanup_failures: lkjscript_core::CleanupFailures,
) -> (EvalOutcome, EvalResourceTeardown) {
    let teardown = resources.teardown();
    cleanup_failures.append(teardown.cleanup_failures.clone());
    let outcome = primary.with_cleanup_failures(cleanup_failures);
    (outcome, teardown)
}

#[cfg(test)]
mod tests;
