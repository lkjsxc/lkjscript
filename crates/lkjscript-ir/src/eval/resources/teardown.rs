use lkjscript_core::{ResourceKind, ResourceTableError};

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
        let (cleanup_attempts, mut errors) = match cleanup {
            Ok(report) => cleanup_records(report),
            Err(error) => (Vec::new(), vec![error.to_string()]),
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
            &mut errors,
        );
        remove_standard_stream(
            &mut self.table,
            &mut self.standard_input,
            ResourceKind::InputStream,
            &mut self.metrics,
            &mut errors,
        );
        self.metrics.ordinary_obligations = ordinary_obligations;
        self.metrics.emergency_obligations = emergency_obligations.len();
        self.metrics.cleanup_attempts = cleanup_attempts.len();
        EvalResourceTeardown {
            ordinary_obligations,
            emergency_obligations,
            cleanup_attempts,
            remaining: self.table.stats(),
            cleanup_error: (!errors.is_empty()).then(|| errors.join("; ")),
        }
    }
}

fn cleanup_records(
    report: lkjscript_core::ResourceCleanupReport<Result<u64, String>>,
) -> (Vec<EvalCleanupAttempt>, Vec<String>) {
    let mut attempts = Vec::with_capacity(report.count());
    let mut errors = Vec::new();
    for attempt in report.into_attempts() {
        let resource = attempt.resource().clone();
        let (owner, error) = match attempt.into_outcome() {
            Ok(owner) => (Some(owner), None),
            Err(error) => {
                errors.push(error.clone());
                (None, Some(error))
            }
        };
        attempts.push(EvalCleanupAttempt {
            resource,
            owner,
            error,
        });
    }
    (attempts, errors)
}

fn remove_standard_stream(
    table: &mut lkjscript_core::ResourceTable<FakeOwner>,
    resource: &mut Option<EvalResource>,
    kind: ResourceKind,
    metrics: &mut EvalResourceMetrics,
    errors: &mut Vec<String>,
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
            Err(error) => errors.push(error),
        },
        Err(error) => errors.push(format!(
            "borrowed {} removal failed: {error}",
            kind.as_str()
        )),
    }
}
