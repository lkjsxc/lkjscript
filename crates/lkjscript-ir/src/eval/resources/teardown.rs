use lkjscript_core::{
    CleanupFailures, CleanupPhase, CleanupSubject, ResourceKind, ResourceTableError,
};

use super::*;

impl EvalResources {
    pub(super) fn teardown(&mut self) -> EvalResourceTeardown {
        let ordinary_obligations = match self.table.assert_zero_ordinary_obligations() {
            Ok(()) => 0,
            Err(ResourceTableError::OutstandingOrdinaryObligations { count }) => count,
            Err(_) => self.table.stats().ordinary_obligations(),
        };
        let emergency_obligations = self.table.emergency_obligations().resources().to_vec();
        let cleanup = self
            .table
            .cleanup_owned_reverse(|observation, payload| payload.validate(&observation));
        let mut cleanup_failures = CleanupFailures::new(self.cleanup_failure_limits);
        let cleanup_attempts = match cleanup {
            Ok(report) => cleanup_records(report, &mut cleanup_failures),
            Err(error) => {
                cleanup_failures.push(
                    CleanupPhase::Emergency,
                    CleanupSubject::EvaluatorProvider,
                    error.to_string(),
                );
                Vec::new()
            }
        };
        self.metrics.resources_closed = self
            .metrics
            .resources_closed
            .saturating_add(u64::try_from(cleanup_attempts.len()).unwrap_or(u64::MAX));
        remove_standard_stream(
            &mut self.table,
            &mut self.standard_output,
            ResourceKind::OutputStream,
            &mut self.metrics,
            &mut cleanup_failures,
        );
        remove_standard_stream(
            &mut self.table,
            &mut self.standard_input,
            ResourceKind::InputStream,
            &mut self.metrics,
            &mut cleanup_failures,
        );
        self.metrics.ordinary_obligations = ordinary_obligations;
        self.metrics.emergency_obligations = emergency_obligations.len();
        self.metrics.cleanup_attempts = cleanup_attempts.len();
        EvalResourceTeardown {
            ordinary_obligations,
            emergency_obligations,
            cleanup_attempts,
            remaining: self.table.stats(),
            cleanup_failures,
        }
    }
}

fn cleanup_records(
    report: lkjscript_core::ResourceCleanupReport<Result<u64, String>>,
    failures: &mut CleanupFailures,
) -> Vec<EvalCleanupAttempt> {
    let mut attempts = Vec::with_capacity(report.count());
    for attempt in report.into_attempts() {
        let resource = attempt.resource().clone();
        let (owner, error) = match attempt.into_outcome() {
            Ok(owner) => (Some(owner), None),
            Err(error) => {
                let subject = resource
                    .kind()
                    .map_or(CleanupSubject::EvaluatorProvider, CleanupSubject::Resource);
                failures.push(CleanupPhase::Emergency, subject, &error);
                (None, Some(error))
            }
        };
        attempts.push(EvalCleanupAttempt {
            resource,
            owner,
            error,
        });
    }
    attempts
}

fn remove_standard_stream(
    table: &mut lkjscript_core::ResourceTable<FakeOwner>,
    resource: &mut Option<EvalResource>,
    kind: ResourceKind,
    metrics: &mut EvalResourceMetrics,
    failures: &mut CleanupFailures,
) {
    let Some(resource) = resource.take() else {
        return;
    };
    match table.remove_borrowed(resource.key, kind, provider_for_kind(kind), table.scope()) {
        Ok(payload) => match payload.validate_binding(
            kind,
            provider_for_kind(kind),
            table.scope(),
            lkjscript_core::ResourceOwnership::Borrowed,
        ) {
            Ok(()) => metrics.borrowed_removed += 1,
            Err(error) => failures.push(
                CleanupPhase::RuntimeTeardown,
                CleanupSubject::BorrowedResource(kind),
                error,
            ),
        },
        Err(error) => failures.push(
            CleanupPhase::RuntimeTeardown,
            CleanupSubject::BorrowedResource(kind),
            format!("borrowed {} removal failed: {error}", kind.as_str()),
        ),
    }
}
