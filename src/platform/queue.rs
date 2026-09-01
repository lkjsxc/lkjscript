//! Durable at-least-once jobs with exact attempts, leases, and stale-completion rejection.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use serde::{Deserialize, Serialize};
use std::fmt;

mod data;
use data::DataQueue;

pub const DURABLE_QUEUE_CONTRACT_VERSION: u16 = 2;
pub const MAXIMUM_QUEUE_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;
pub const MAXIMUM_QUEUE_LEASE_MILLISECONDS: i64 = 24 * 60 * 60 * 1_000;
pub const MAXIMUM_QUEUE_ATTEMPTS: u32 = 1_000_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueueLimits {
    pub maximum_payload_bytes: usize,
    pub maximum_result_bytes: usize,
    pub maximum_lease_milliseconds: i64,
    pub maximum_attempts: u32,
}

impl Default for QueueLimits {
    fn default() -> Self {
        Self {
            maximum_payload_bytes: 1024 * 1024,
            maximum_result_bytes: 1024 * 1024,
            maximum_lease_milliseconds: 5 * 60 * 1_000,
            maximum_attempts: 100,
        }
    }
}

impl QueueLimits {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.maximum_payload_bytes == 0
            || self.maximum_payload_bytes > MAXIMUM_QUEUE_PAYLOAD_BYTES
            || self.maximum_result_bytes == 0
            || self.maximum_result_bytes > MAXIMUM_QUEUE_PAYLOAD_BYTES
        {
            return Err(queue_diagnostic(
                "queue_byte_limit",
                format!(
                    "queue payload and result limits must be 1 through {MAXIMUM_QUEUE_PAYLOAD_BYTES} bytes"
                ),
            ));
        }
        if self.maximum_lease_milliseconds <= 0
            || self.maximum_lease_milliseconds > MAXIMUM_QUEUE_LEASE_MILLISECONDS
        {
            return Err(queue_diagnostic(
                "queue_lease_limit",
                format!(
                    "maximum queue lease must be 1 through {MAXIMUM_QUEUE_LEASE_MILLISECONDS} milliseconds"
                ),
            ));
        }
        if self.maximum_attempts == 0 || self.maximum_attempts > MAXIMUM_QUEUE_ATTEMPTS {
            return Err(queue_diagnostic(
                "queue_attempt_limit",
                format!("maximum attempts must be 1 through {MAXIMUM_QUEUE_ATTEMPTS}"),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Ready,
    Leased,
    Completed,
    Failed,
    Cancelled,
}

impl JobState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Leased => "leased",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct JobLease {
    pub(crate) job_id: String,
    pub(crate) attempt_id: String,
    pub(crate) worker_id: String,
    pub(crate) payload: Vec<u8>,
    pub(crate) attempt_number: u32,
    pub(crate) lease_until_milliseconds: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobSnapshot {
    pub job_id: String,
    pub state: JobState,
    pub attempt_count: u32,
    pub available_at_milliseconds: i64,
    pub lease_until_milliseconds: Option<i64>,
    pub result: Option<Vec<u8>>,
    pub last_error_class: Option<String>,
}

/// Representation-neutral durable-queue host engine. Artifact codecs own value layouts while this
/// type owns queue state transitions, first-party persistence, bounds, and cleanup.
#[derive(Clone)]
pub(crate) struct DurableQueueEngine {
    store: DataQueue,
    limits: QueueLimits,
}

impl fmt::Debug for DurableQueueEngine {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableQueueEngine")
            .field("adapter_kind", &"first_party_data")
            .field("limits", &self.limits)
            .finish()
    }
}

impl DurableQueueEngine {
    pub(crate) fn data(
        store: super::data::DataStore,
        limits: QueueLimits,
    ) -> Result<Self, Diagnostic> {
        limits.validate()?;
        Ok(Self {
            store: DataQueue::new(store, limits.clone()),
            limits,
        })
    }

    pub(crate) fn preflight(&self) -> Result<(), ExecutionError> {
        self.store.preflight()
    }

    pub(crate) fn shutdown(&self) -> Result<(), ExecutionError> {
        Ok(())
    }

    pub(crate) fn initialize(
        &self,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        self.store.initialize(control, possible_visibility)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn enqueue(
        &self,
        job_id: &str,
        idempotency_key: &str,
        payload: &[u8],
        available_at: i64,
        created_at: i64,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<bool, ExecutionError> {
        control.check()?;
        validate_token(job_id, "job id")?;
        validate_bounded_text(idempotency_key, "idempotency key", 512)?;
        validate_bytes(payload, "job payload", self.limits.maximum_payload_bytes)?;
        validate_nonnegative_time(available_at, "available time")?;
        validate_nonnegative_time(created_at, "creation time")?;
        self.store.enqueue(
            job_id,
            idempotency_key,
            payload,
            available_at,
            created_at,
            control,
            possible_visibility,
        )
    }

    pub(crate) fn claim(
        &self,
        worker_id: &str,
        now: i64,
        lease: i64,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<Option<JobLease>, ExecutionError> {
        control.check()?;
        validate_token(worker_id, "worker id")?;
        validate_nonnegative_time(now, "claim time")?;
        validate_lease_duration(lease, &self.limits)?;
        self.store.claim(
            worker_id,
            now,
            lease,
            self.limits.maximum_attempts,
            control,
            possible_visibility,
        )
    }

    pub(crate) fn heartbeat_lease(
        &self,
        mut lease_authority: JobLease,
        now: i64,
        lease_duration: i64,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<Option<JobLease>, ExecutionError> {
        self.validate_lease_call(
            &lease_authority.job_id,
            &lease_authority.attempt_id,
            &lease_authority.worker_id,
            now,
            control,
        )?;
        validate_lease_duration(lease_duration, &self.limits)?;
        let lease_until = now
            .checked_add(lease_duration)
            .ok_or_else(|| queue_argument("queue lease time overflowed"))?;
        let renewed = self.store.heartbeat(
            &lease_authority.job_id,
            &lease_authority.attempt_id,
            &lease_authority.worker_id,
            now,
            lease_duration,
            control,
            possible_visibility,
        )?;
        if renewed {
            lease_authority.lease_until_milliseconds = lease_until;
            Ok(Some(lease_authority))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn complete_lease(
        &self,
        lease_authority: JobLease,
        now: i64,
        result: &[u8],
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<bool, ExecutionError> {
        self.validate_lease_call(
            &lease_authority.job_id,
            &lease_authority.attempt_id,
            &lease_authority.worker_id,
            now,
            control,
        )?;
        validate_bytes(result, "job result", self.limits.maximum_result_bytes)?;
        self.store.complete(
            &lease_authority.job_id,
            &lease_authority.attempt_id,
            &lease_authority.worker_id,
            now,
            result,
            control,
            possible_visibility,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn fail_lease(
        &self,
        lease_authority: JobLease,
        now: i64,
        retry: bool,
        retry_at: i64,
        error_class: &str,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<bool, ExecutionError> {
        self.validate_lease_call(
            &lease_authority.job_id,
            &lease_authority.attempt_id,
            &lease_authority.worker_id,
            now,
            control,
        )?;
        validate_nonnegative_time(retry_at, "retry time")?;
        if retry && retry_at < now {
            return Err(queue_argument("retry time precedes failure time"));
        }
        validate_bounded_text(error_class, "error class", 128)?;
        validate_token(error_class, "error class")?;
        self.store.fail(
            &lease_authority.job_id,
            &lease_authority.attempt_id,
            &lease_authority.worker_id,
            now,
            retry,
            retry_at,
            error_class,
            self.limits.maximum_attempts,
            control,
            possible_visibility,
        )
    }

    pub(crate) fn cancel(
        &self,
        job_id: &str,
        now: i64,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<bool, ExecutionError> {
        control.check()?;
        validate_token(job_id, "job id")?;
        validate_nonnegative_time(now, "cancellation time")?;
        self.store.cancel(job_id, now, control, possible_visibility)
    }

    pub(crate) fn inspect(
        &self,
        job_id: &str,
        control: &ExecutionControl,
    ) -> Result<Option<JobSnapshot>, ExecutionError> {
        control.check()?;
        validate_token(job_id, "job id")?;
        self.store.inspect(job_id, control)
    }

    fn validate_lease_call(
        &self,
        job_id: &str,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        validate_token(job_id, "job id")?;
        validate_token(attempt_id, "attempt id")?;
        validate_token(worker_id, "worker id")?;
        validate_nonnegative_time(now, "queue operation time")
    }
}

fn validate_token(value: &str, label: &str) -> Result<(), ExecutionError> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':'))
    {
        return Err(queue_argument(format!("{label} is not a canonical token")));
    }
    Ok(())
}

fn validate_bounded_text(value: &str, label: &str, maximum: usize) -> Result<(), ExecutionError> {
    if value.is_empty() || value.len() > maximum || value.contains('\0') {
        return Err(queue_argument(format!(
            "{label} is empty, excessive, or contains NUL",
        )));
    }
    Ok(())
}

fn validate_bytes(value: &[u8], label: &str, maximum: usize) -> Result<(), ExecutionError> {
    if value.len() > maximum {
        return Err(ExecutionError::resource(
            "queue_byte_limit",
            format!("{label} exceeds its exact byte limit"),
        ));
    }
    Ok(())
}

fn validate_nonnegative_time(value: i64, label: &str) -> Result<(), ExecutionError> {
    if value < 0 {
        return Err(queue_argument(format!("{label} must be non-negative")));
    }
    Ok(())
}

fn validate_lease_duration(value: i64, limits: &QueueLimits) -> Result<(), ExecutionError> {
    if value <= 0 || value > limits.maximum_lease_milliseconds {
        return Err(ExecutionError::resource(
            "queue_lease_limit",
            "queue lease duration is zero or exceeds its exact limit",
        ));
    }
    Ok(())
}

fn queue_argument(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Capability, "queue_argument", message)
}

fn queue_conflict(code: &'static str, message: &'static str) -> ExecutionError {
    ExecutionError::new(ExecutionFailureClass::Capability, code, message)
}

fn queue_internal(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "queue_adapter_internal",
        message,
    )
}

fn queue_diagnostic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::data::{DataLimits, DataStore};
    use std::ops::Deref;

    struct TestEngine {
        _directory: tempfile::TempDir,
        engine: DurableQueueEngine,
    }

    impl Deref for TestEngine {
        type Target = DurableQueueEngine;

        fn deref(&self) -> &Self::Target {
            &self.engine
        }
    }

    fn engine(limits: QueueLimits) -> TestEngine {
        let directory = tempfile::tempdir().expect("queue data directory");
        let root = directory.path().join("store");
        DataStore::initialize(&root).expect("initialize queue data store");
        let store = DataStore::open(&root, "queue-tests", DataLimits::default())
            .expect("open queue data store");
        let engine = DurableQueueEngine::data(store, limits).expect("queue engine");
        TestEngine {
            _directory: directory,
            engine,
        }
    }

    fn enqueue(
        engine: &DurableQueueEngine,
        job: &str,
        key: &str,
        payload: &[u8],
    ) -> Result<bool, ExecutionError> {
        engine.enqueue(
            job,
            key,
            payload,
            0,
            0,
            &ExecutionControl::uncancelled(),
            false,
        )
    }

    fn claim(engine: &DurableQueueEngine, worker: &str, now: i64) -> JobLease {
        engine
            .claim(worker, now, 10, &ExecutionControl::uncancelled(), false)
            .expect("claim")
            .expect("lease")
    }

    #[test]
    fn duplicate_enqueue_is_idempotent_and_conflicting_input_rejects() {
        let engine = engine(QueueLimits::default());
        assert!(enqueue(&engine, "job-1", "key-1", b"a").expect("insert"));
        assert!(!enqueue(&engine, "job-1", "key-1", b"a").expect("replay"));
        assert_eq!(
            enqueue(&engine, "job-1", "key-1", b"changed")
                .expect_err("conflicting replay must reject")
                .code,
            "queue_idempotency_conflict"
        );
    }

    #[test]
    fn lease_loss_and_stale_completion_cannot_publish_twice() {
        let engine = engine(QueueLimits::default());
        enqueue(&engine, "job-1", "key-1", b"payload").expect("enqueue");
        let first = claim(&engine, "worker-1", 100);
        assert_eq!(first.attempt_number, 1);
        let second = claim(&engine, "worker-2", 110);
        assert_eq!(second.attempt_number, 2);
        assert!(
            !engine
                .complete_lease(
                    first,
                    110,
                    b"stale",
                    &ExecutionControl::uncancelled(),
                    false,
                )
                .expect("stale completion")
        );
        assert!(
            engine
                .complete_lease(
                    second,
                    111,
                    b"result",
                    &ExecutionControl::uncancelled(),
                    false,
                )
                .expect("current completion")
        );
        let snapshot = engine
            .inspect("job-1", &ExecutionControl::uncancelled())
            .expect("inspect")
            .expect("snapshot");
        assert_eq!(snapshot.state, JobState::Completed);
        assert_eq!(snapshot.result.as_deref(), Some(b"result".as_slice()));
    }

    #[test]
    fn retry_policy_is_explicit_and_attempt_bound_is_terminal() {
        let engine = engine(QueueLimits {
            maximum_attempts: 2,
            ..QueueLimits::default()
        });
        enqueue(&engine, "job-1", "key-1", b"payload").expect("enqueue");
        let first = claim(&engine, "worker", 0);
        assert!(
            engine
                .fail_lease(
                    first,
                    1,
                    true,
                    5,
                    "provider_unavailable",
                    &ExecutionControl::uncancelled(),
                    false,
                )
                .expect("first failure")
        );
        let second = claim(&engine, "worker", 5);
        assert!(
            engine
                .fail_lease(
                    second,
                    6,
                    true,
                    7,
                    "provider_unavailable",
                    &ExecutionControl::uncancelled(),
                    false,
                )
                .expect("second failure")
        );
        assert!(
            engine
                .claim("worker", 7, 10, &ExecutionControl::uncancelled(), false,)
                .expect("terminal claim")
                .is_none()
        );
    }

    #[test]
    fn heartbeat_cancel_and_limits_are_owned_by_the_engine() {
        let engine = engine(QueueLimits::default());
        engine
            .initialize(&ExecutionControl::uncancelled(), false)
            .expect("initialize");
        enqueue(&engine, "job-1", "key-1", b"payload").expect("enqueue");
        let lease = claim(&engine, "worker", 0);
        let renewed = engine
            .heartbeat_lease(lease, 1, 20, &ExecutionControl::uncancelled(), false)
            .expect("heartbeat")
            .expect("renewed lease");
        assert_eq!(renewed.lease_until_milliseconds, 21);
        assert!(
            engine
                .cancel("job-1", 2, &ExecutionControl::uncancelled(), false,)
                .expect("cancel")
        );
        assert_eq!(
            engine
                .claim("worker", 0, 0, &ExecutionControl::uncancelled(), false,)
                .expect_err("zero lease")
                .code,
            "queue_lease_limit"
        );
    }
}
