//! Representation-neutral execution failure, policy, cancellation, and deadline controls.

use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionFailureClass {
    Trap,
    Capability,
    PossibleVisibility,
    Resource,
    Cancelled,
    Infrastructure,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionError {
    pub class: ExecutionFailureClass,
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub possibly_visible: bool,
}

impl ExecutionError {
    pub fn new(
        class: ExecutionFailureClass,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            class,
            code: code.into(),
            message: message.into(),
            retryable: false,
            possibly_visible: class == ExecutionFailureClass::PossibleVisibility,
        }
    }

    pub fn resource(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(ExecutionFailureClass::Resource, code, message)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunPolicy {
    // Missing is an error; only explicit null disables an existing instruction quota.
    #[serde(deserialize_with = "required_quota")]
    pub instruction_fuel: Option<u64>,
    pub maximum_call_depth: usize,
    pub maximum_value_stack: usize,
    // Omitted cumulative fields impose no quota; callers may select each dimension.
    #[serde(default)]
    pub maximum_allocated_bytes: Option<u64>,
    #[serde(default)]
    pub maximum_collection_items: Option<u64>,
    #[serde(default)]
    pub maximum_capability_calls: Option<u64>,
}

fn required_quota<'de, D: serde::Deserializer<'de>>(decoder: D) -> Result<Option<u64>, D::Error> {
    serde::Deserialize::deserialize(decoder)
}

impl Default for RunPolicy {
    fn default() -> Self {
        Self {
            instruction_fuel: None,
            maximum_call_depth: 4_096,
            maximum_value_stack: 1_000_000,
            maximum_allocated_bytes: None,
            maximum_collection_items: None,
            maximum_capability_calls: None,
        }
    }
}

impl RunPolicy {
    /// No cumulative quotas. Work is still observed; live limits and controls remain separate.
    pub fn unmetered() -> Self {
        Self::default()
    }
}

/// Precharge a cumulative quota, or observe work without imposing a lifetime limit.
/// Exact `u64::MAX` remains distinct from an overflowing lower-bound observation.
/// Storage-size arithmetic must be checked separately before growth.
pub(crate) fn cumulative_charge(
    current: u64,
    additional: u64,
    maximum: Option<u64>,
    code: &'static str,
    message: &'static str,
    saturated: &mut bool,
) -> Result<u64, ExecutionError> {
    match maximum {
        Some(_) if *saturated => Err(ExecutionError::resource(code, message)),
        Some(maximum) => finite_charge(current, additional, maximum, code, message),
        None => match current.checked_add(additional) {
            Some(next) => Ok(next),
            None => {
                *saturated = true;
                Ok(u64::MAX)
            }
        },
    }
}

/// Reserve exact finite capacity without using passive observations as permission.
pub(crate) fn finite_charge(
    current: u64,
    additional: u64,
    maximum: u64,
    code: &'static str,
    message: &'static str,
) -> Result<u64, ExecutionError> {
    current
        .checked_add(additional)
        .filter(|next| *next <= maximum)
        .ok_or_else(|| ExecutionError::resource(code, message))
}

/// Combine passive observations without turning an overflow into a runtime quota.
/// Inherited saturation is retained even when the represented sum itself fits.
pub(crate) fn cumulative_observe(
    current: &mut u64,
    saturated: &mut bool,
    additional: u64,
    additional_saturated: bool,
) {
    *saturated |= additional_saturated;
    match current.checked_add(additional) {
        Some(next) => *current = next,
        None => {
            *current = u64::MAX;
            *saturated = true;
        }
    }
}

/// Runtime-owned cancellation and deadline state. It is never representable as a language value
/// and therefore cannot cross a durable boundary.
#[derive(Clone, Debug)]
pub struct ExecutionControl {
    cancelled: Arc<AtomicBool>,
    deadline: Option<Instant>,
    remaining_checks: Option<Arc<std::sync::atomic::AtomicU64>>,
    cancel_at_capture_nodes: Option<u64>,
    cancel_at_input_nodes: Option<u64>,
    staged_tail_probe: Option<Arc<StagedTailProbe>>,
}

#[derive(Debug)]
struct StagedTailProbe {
    required_capability_calls: u64,
    remaining_transfers: std::sync::atomic::AtomicU64,
}

impl ExecutionControl {
    pub fn uncancelled() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: None,
            remaining_checks: None,
            cancel_at_capture_nodes: None,
            cancel_at_input_nodes: None,
            staged_tail_probe: None,
        }
    }

    pub fn with_deadline(deadline: Instant) -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            deadline: Some(deadline),
            remaining_checks: None,
            cancel_at_capture_nodes: None,
            cancel_at_input_nodes: None,
            staged_tail_probe: None,
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    pub fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    /// Contributor-only deterministic cancellation. Public controls never install this probe.
    pub(crate) fn cancel_after_checks(count: u64) -> Self {
        Self {
            remaining_checks: Some(Arc::new(std::sync::atomic::AtomicU64::new(count))),
            ..Self::uncancelled()
        }
    }

    /// Contributor-only phase probe. Public controls never install this threshold;
    /// unrelated type or value admission checks cannot consume its capture target.
    pub(crate) fn cancel_after_capture_nodes(count: u64) -> Self {
        Self {
            cancel_at_capture_nodes: Some(count),
            ..Self::uncancelled()
        }
    }

    pub(crate) fn capture_admission_node(&self, nodes: u64) -> Result<(), ExecutionError> {
        if self
            .cancel_at_capture_nodes
            .is_some_and(|target| nodes >= target)
        {
            self.cancel();
            self.check()?;
        }
        Ok(())
    }

    /// Contributor-only partial input admission, independent of type lookup work.
    pub(crate) fn cancel_after_input_nodes(count: u64) -> Self {
        Self {
            cancel_at_input_nodes: Some(count),
            ..Self::uncancelled()
        }
    }

    pub(crate) fn input_admission_node(&self, nodes: u64) -> Result<(), ExecutionError> {
        if self
            .cancel_at_input_nodes
            .is_some_and(|target| nodes >= target)
        {
            self.cancel();
            self.check()?;
        }
        Ok(())
    }

    /// Count actual tail transfers only after the contributor's staged-effect
    /// boundary and while its transaction is live. Public controls never install it.
    pub(crate) fn cancel_after_transaction_tail_transfers(
        count: u64,
        required_capability_calls: u64,
    ) -> Self {
        Self {
            staged_tail_probe: Some(Arc::new(StagedTailProbe {
                required_capability_calls,
                remaining_transfers: std::sync::atomic::AtomicU64::new(count),
            })),
            ..Self::uncancelled()
        }
    }

    pub(crate) fn transaction_tail_transfer(
        &self,
        capability_calls: u64,
        active_transactions: usize,
    ) -> Result<(), ExecutionError> {
        if let Some(probe) = &self.staged_tail_probe
            && capability_calls >= probe.required_capability_calls
            && active_transactions == 1
        {
            let prior = probe.remaining_transfers.fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |remaining| remaining.checked_sub(1),
            );
            if prior.is_err() || prior == Ok(1) {
                self.cancel();
                self.check()?;
            }
        }
        Ok(())
    }

    pub fn check(&self) -> Result<(), ExecutionError> {
        if let Some(remaining) = &self.remaining_checks
            && remaining
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                    count.checked_sub(1)
                })
                .is_err()
        {
            self.cancel();
        }
        if self.is_cancelled() {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Cancelled,
                "execution_cancelled",
                "execution was cancelled by its owning task scope",
            ));
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Cancelled,
                "execution_deadline",
                "execution exceeded its operational deadline",
            ));
        }
        Ok(())
    }
}

impl Default for ExecutionControl {
    fn default() -> Self {
        Self::uncancelled()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contributor_capture_cancellation_ignores_unrelated_checks_and_normal_controls() {
        let control = ExecutionControl::cancel_after_capture_nodes(3);
        for _ in 0..1024 {
            assert!(control.check().is_ok());
        }
        assert!(control.capture_admission_node(1).is_ok());
        assert!(control.capture_admission_node(2).is_ok());
        assert!(!control.is_cancelled());
        let clone = control.clone();
        assert_eq!(
            clone.capture_admission_node(3).unwrap_err().class,
            ExecutionFailureClass::Cancelled
        );
        assert!(control.is_cancelled());

        let normal = ExecutionControl::uncancelled();
        assert!(normal.capture_admission_node(u64::MAX).is_ok());
        assert!(!normal.is_cancelled());
        let checked = ExecutionControl::cancel_after_checks(1);
        assert!(checked.capture_admission_node(u64::MAX).is_ok());
        assert!(checked.check().is_ok());
        assert_eq!(
            checked.check().unwrap_err().class,
            ExecutionFailureClass::Cancelled
        );
    }

    #[test]
    fn contributor_input_cancellation_targets_partial_admission() {
        let control = ExecutionControl::cancel_after_input_nodes(3);
        for _ in 0..1024 {
            assert!(control.check().is_ok());
            assert!(control.capture_admission_node(u64::MAX).is_ok());
        }
        assert!(control.input_admission_node(1).is_ok());
        assert!(control.input_admission_node(2).is_ok());
        assert!(!control.is_cancelled());
        assert_eq!(
            control.clone().input_admission_node(3).unwrap_err().class,
            ExecutionFailureClass::Cancelled
        );
        assert!(control.is_cancelled());
    }

    #[test]
    fn contributor_staged_tail_cancellation_requires_completed_stage_and_live_transaction() {
        let control = ExecutionControl::cancel_after_transaction_tail_transfers(3, 2);
        for _ in 0..1024 {
            assert!(control.check().is_ok());
            assert!(control.transaction_tail_transfer(1, 1).is_ok());
            assert!(control.transaction_tail_transfer(2, 0).is_ok());
            assert!(control.transaction_tail_transfer(2, 2).is_ok());
            assert!(control.input_admission_node(u64::MAX).is_ok());
        }
        assert!(control.transaction_tail_transfer(2, 1).is_ok());
        let clone = control.clone();
        assert!(clone.transaction_tail_transfer(2, 1).is_ok());
        assert!(!control.is_cancelled());
        assert_eq!(
            clone.transaction_tail_transfer(2, 1).unwrap_err().class,
            ExecutionFailureClass::Cancelled
        );
        assert!(control.is_cancelled());

        let normal = ExecutionControl::uncancelled();
        assert!(normal.transaction_tail_transfer(u64::MAX, 1).is_ok());
        assert!(normal.input_admission_node(u64::MAX).is_ok());
        assert!(!normal.is_cancelled());
        let checked = ExecutionControl::cancel_after_checks(1);
        assert!(checked.transaction_tail_transfer(u64::MAX, 1).is_ok());
        assert!(checked.input_admission_node(u64::MAX).is_ok());
        assert!(checked.check().is_ok());
        assert_eq!(
            checked.check().unwrap_err().class,
            ExecutionFailureClass::Cancelled
        );
    }

    #[test]
    fn default_cumulative_quotas_observe_former_thresholds_without_rejecting() {
        let policy = RunPolicy::default();
        assert_eq!(policy, RunPolicy::unmetered());
        assert_eq!(policy.maximum_call_depth, 4_096);
        assert_eq!(policy.maximum_value_stack, 1_000_000);
        for (selected, former) in [
            (policy.instruction_fuel, 10_000_000),
            (policy.maximum_allocated_bytes, 268_435_456),
            (policy.maximum_collection_items, 1_000_000),
            (policy.maximum_capability_calls, 100_000),
        ] {
            assert_eq!(selected, None);
            let mut saturated = false;
            assert_eq!(
                cumulative_charge(former, 1, selected, "quota", "exhausted", &mut saturated)
                    .unwrap(),
                former + 1
            );
            assert!(!saturated);
            assert_eq!(
                cumulative_charge(u64::MAX, 1, selected, "quota", "exhausted", &mut saturated)
                    .unwrap(),
                u64::MAX
            );
            assert!(saturated);
            let mut finite_saturated = false;
            let explicit = cumulative_charge(
                former,
                1,
                Some(former),
                "quota",
                "exhausted",
                &mut finite_saturated,
            )
            .unwrap_err();
            assert_eq!(explicit.class, ExecutionFailureClass::Resource);
            assert_eq!(explicit.code, "quota");
            assert!(!finite_saturated);
        }
    }

    #[test]
    fn cumulative_quota_overflow_and_unbounded_telemetry_are_distinct() {
        for (current, additional, maximum, before, expected, after) in [
            (0, 0, Some(0), false, Some(0), false),
            (0, 1, Some(0), false, None, false),
            (0, 5, Some(4), false, None, false),
            (0, 5, Some(5), false, Some(5), false),
            (4, 1, Some(5), false, Some(5), false),
            (5, 1, Some(5), false, None, false),
            (u64::MAX - 1, 1, None, false, Some(u64::MAX), false),
            (
                u64::MAX - 1,
                1,
                Some(u64::MAX),
                false,
                Some(u64::MAX),
                false,
            ),
            (u64::MAX, 0, Some(u64::MAX), false, Some(u64::MAX), false),
            (u64::MAX, 1, Some(u64::MAX), false, None, false),
            (u64::MAX, 1, None, false, Some(u64::MAX), true),
            (u64::MAX, 1, None, true, Some(u64::MAX), true),
            (u64::MAX, 0, Some(u64::MAX), true, None, true),
        ] {
            let mut saturated = before;
            assert_eq!(
                cumulative_charge(
                    current,
                    additional,
                    maximum,
                    "quota",
                    "exhausted",
                    &mut saturated,
                )
                .ok(),
                expected,
                "current={current} additional={additional} maximum={maximum:?} saturated={before}"
            );
            assert_eq!(saturated, after);
        }
    }

    #[test]
    fn cumulative_observation_merges_preserve_exact_maximum_and_inherited_saturation() {
        let mut count = u64::MAX - 1;
        let mut saturated = false;
        cumulative_observe(&mut count, &mut saturated, 1, false);
        assert_eq!(count, u64::MAX);
        assert!(!saturated);
        cumulative_observe(&mut count, &mut saturated, 1, false);
        assert_eq!(count, u64::MAX);
        assert!(saturated);

        let mut merged = 0;
        let mut merged_saturated = false;
        cumulative_observe(&mut merged, &mut merged_saturated, count, saturated);
        assert_eq!(merged, u64::MAX);
        assert!(merged_saturated);
        cumulative_observe(&mut merged, &mut merged_saturated, 0, false);
        assert_eq!(merged, u64::MAX);
        assert!(merged_saturated);
    }
}
