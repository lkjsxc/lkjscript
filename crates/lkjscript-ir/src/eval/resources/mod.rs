mod bootstrap;
mod close;
mod metrics;
mod model;
mod provider;
mod session;
mod teardown;

use metrics::{EvalCleanupAttempt, EvalResourceMetrics, EvalResourceTeardown};
#[cfg(test)]
use model::ownership_for_kind;
pub use model::EvalResource;
use model::{provider_for_kind, FakeOwner};
use provider::FakeProviders;
pub(super) use session::EvalResources;

use super::EvalOutcome;

pub(super) fn finish_evaluation(
    resources: &mut EvalResources,
    primary: EvalOutcome,
) -> EvalOutcome {
    finish_evaluation_with_report(resources, primary).0
}

fn finish_evaluation_with_report(
    resources: &mut EvalResources,
    primary: EvalOutcome,
) -> (EvalOutcome, EvalResourceTeardown) {
    let teardown = resources.teardown();
    let outcome = match &teardown.cleanup_error {
        Some(error) => EvalOutcome::HostFailure(format!(
            "evaluator resource cleanup failed after {primary:?}: {error}"
        )),
        None => primary,
    };
    (outcome, teardown)
}

#[cfg(test)]
mod tests;
