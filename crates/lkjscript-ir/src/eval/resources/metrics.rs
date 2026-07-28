use lkjscript_core::{ResourceObservation, ResourceTableStats};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct EvalResourceMetrics {
    pub(super) resources_opened: u64,
    pub(super) resources_closed: u64,
    pub(super) failed_acquisitions: u64,
    pub(super) slots_reused: u64,
    pub(super) stale_key_failures: u64,
    pub(super) borrowed_installed: usize,
    pub(super) borrowed_removed: usize,
    pub(super) ordinary_obligations: usize,
    pub(super) emergency_obligations: usize,
    pub(super) cleanup_attempts: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EvalCleanupAttempt {
    pub(super) resource: ResourceObservation,
    pub(super) owner: Option<u64>,
    pub(super) error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct EvalResourceTeardown {
    pub(super) ordinary_obligations: usize,
    pub(super) emergency_obligations: Vec<ResourceObservation>,
    pub(super) cleanup_attempts: Vec<EvalCleanupAttempt>,
    pub(super) remaining: ResourceTableStats,
    pub(super) cleanup_error: Option<String>,
}
