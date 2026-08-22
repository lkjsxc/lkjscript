//! Durable at-least-once jobs with exact attempts, leases, and stale-completion rejection.

use super::database::{PostgresPool, map_postgres_error};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{CallPolicy, CapabilityAdapter, ExecutionError, ExecutionFailureClass};
use super::semantic::OwnerId;
use super::value::Value;
use postgres::Row;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

pub const DURABLE_QUEUE_CONTRACT_VERSION: u16 = 1;
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
    fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Leased => "leased",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    fn parse(value: &str) -> Result<Self, ExecutionError> {
        match value {
            "ready" => Ok(Self::Ready),
            "leased" => Ok(Self::Leased),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(queue_internal("database returned an unknown queue state")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JobLease {
    pub job_id: String,
    pub attempt_id: String,
    pub payload: Vec<u8>,
    pub attempt_number: u32,
    pub lease_until_milliseconds: i64,
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

#[derive(Clone)]
pub struct DurableQueueAdapter {
    interface: OwnerId,
    store: QueueStore,
    limits: QueueLimits,
}

impl fmt::Debug for DurableQueueAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DurableQueueAdapter")
            .field("interface", &self.interface)
            .field("adapter_kind", &self.store.kind())
            .field("limits", &self.limits)
            .finish()
    }
}

#[derive(Clone)]
enum QueueStore {
    Memory(MemoryQueue),
    Postgres(PostgresQueue),
}

impl QueueStore {
    fn kind(&self) -> &'static str {
        match self {
            Self::Memory(_) => "deterministic_memory",
            Self::Postgres(_) => "postgres",
        }
    }
}

impl DurableQueueAdapter {
    pub fn in_memory(interface: OwnerId, limits: QueueLimits) -> Result<Self, Diagnostic> {
        limits.validate()?;
        Ok(Self {
            interface,
            store: QueueStore::Memory(MemoryQueue::default()),
            limits,
        })
    }

    pub fn postgres(
        interface: OwnerId,
        pool: PostgresPool,
        namespace: String,
        limits: QueueLimits,
    ) -> Result<Self, Diagnostic> {
        limits.validate()?;
        validate_token(&namespace, "queue namespace")
            .map_err(|error| queue_diagnostic("queue_namespace", error.message))?;
        Ok(Self {
            interface,
            store: QueueStore::Postgres(PostgresQueue { pool, namespace }),
            limits,
        })
    }

    fn initialize(
        &self,
        policy: &CallPolicy,
        arguments: &[Value],
    ) -> Result<Value, ExecutionError> {
        if !arguments.is_empty() {
            return Err(queue_argument("initialize expects no arguments"));
        }
        match &self.store {
            QueueStore::Memory(_) => Ok(Value::Unit),
            QueueStore::Postgres(store) => {
                store.initialize(policy)?;
                Ok(Value::Unit)
            }
        }
    }

    fn enqueue(&self, policy: &CallPolicy, arguments: &[Value]) -> Result<Value, ExecutionError> {
        let [job_id, idempotency_key, payload, available_at, created_at] = arguments else {
            return Err(queue_argument(
                "enqueue expects job id, idempotency key, payload, available time, and creation time",
            ));
        };
        let job_id = token(job_id, "job id")?;
        let idempotency_key = bounded_text(idempotency_key, "idempotency key", 512)?;
        let payload = bytes(payload, "job payload", self.limits.maximum_payload_bytes)?;
        let available_at = nonnegative_time(available_at, "available time")?;
        let created_at = nonnegative_time(created_at, "creation time")?;
        let inserted = match &self.store {
            QueueStore::Memory(store) => {
                store.enqueue(job_id, idempotency_key, payload, available_at, created_at)?
            }
            QueueStore::Postgres(store) => store.enqueue(
                policy,
                job_id,
                idempotency_key,
                payload,
                available_at,
                created_at,
            )?,
        };
        Ok(Value::Bool(inserted))
    }

    fn claim(&self, policy: &CallPolicy, arguments: &[Value]) -> Result<Value, ExecutionError> {
        let [worker_id, now, lease] = arguments else {
            return Err(queue_argument(
                "claim expects worker id, current time, and lease duration",
            ));
        };
        let worker_id = token(worker_id, "worker id")?;
        let now = nonnegative_time(now, "claim time")?;
        let lease = lease_duration(lease, &self.limits)?;
        let lease = match &self.store {
            QueueStore::Memory(store) => {
                store.claim(worker_id, now, lease, self.limits.maximum_attempts)?
            }
            QueueStore::Postgres(store) => {
                store.claim(policy, worker_id, now, lease, self.limits.maximum_attempts)?
            }
        };
        Ok(Value::List(Arc::new(
            lease.into_iter().map(lease_value).collect(),
        )))
    }

    fn heartbeat(&self, policy: &CallPolicy, arguments: &[Value]) -> Result<Value, ExecutionError> {
        let [job_id, attempt_id, worker_id, now, lease] = arguments else {
            return Err(queue_argument(
                "heartbeat expects job id, attempt id, worker id, current time, and lease duration",
            ));
        };
        let job_id = token(job_id, "job id")?;
        let attempt_id = token(attempt_id, "attempt id")?;
        let worker_id = token(worker_id, "worker id")?;
        let now = nonnegative_time(now, "heartbeat time")?;
        let lease = lease_duration(lease, &self.limits)?;
        let accepted = match &self.store {
            QueueStore::Memory(store) => {
                store.heartbeat(job_id, attempt_id, worker_id, now, lease)?
            }
            QueueStore::Postgres(store) => {
                store.heartbeat(policy, job_id, attempt_id, worker_id, now, lease)?
            }
        };
        Ok(Value::Bool(accepted))
    }

    fn complete(&self, policy: &CallPolicy, arguments: &[Value]) -> Result<Value, ExecutionError> {
        let [job_id, attempt_id, worker_id, now, result] = arguments else {
            return Err(queue_argument(
                "complete expects job id, attempt id, worker id, current time, and result",
            ));
        };
        let job_id = token(job_id, "job id")?;
        let attempt_id = token(attempt_id, "attempt id")?;
        let worker_id = token(worker_id, "worker id")?;
        let now = nonnegative_time(now, "completion time")?;
        let result = bytes(result, "job result", self.limits.maximum_result_bytes)?;
        let accepted = match &self.store {
            QueueStore::Memory(store) => {
                store.complete(job_id, attempt_id, worker_id, now, result)?
            }
            QueueStore::Postgres(store) => {
                store.complete(policy, job_id, attempt_id, worker_id, now, result)?
            }
        };
        Ok(Value::Bool(accepted))
    }

    fn fail(&self, policy: &CallPolicy, arguments: &[Value]) -> Result<Value, ExecutionError> {
        let [
            job_id,
            attempt_id,
            worker_id,
            now,
            retry,
            retry_at,
            error_class,
        ] = arguments
        else {
            return Err(queue_argument(
                "fail expects job id, attempt id, worker id, current time, retry decision, retry time, and error class",
            ));
        };
        let job_id = token(job_id, "job id")?;
        let attempt_id = token(attempt_id, "attempt id")?;
        let worker_id = token(worker_id, "worker id")?;
        let now = nonnegative_time(now, "failure time")?;
        let Value::Bool(retry) = retry else {
            return Err(queue_argument("retry decision must be Bool"));
        };
        let retry_at = nonnegative_time(retry_at, "retry time")?;
        if *retry && retry_at < now {
            return Err(queue_argument("retry time precedes failure time"));
        }
        let error_class = bounded_text(error_class, "error class", 128)?;
        validate_token(error_class, "error class")?;
        let accepted = match &self.store {
            QueueStore::Memory(store) => store.fail(
                job_id,
                attempt_id,
                worker_id,
                now,
                *retry,
                retry_at,
                error_class,
                self.limits.maximum_attempts,
            )?,
            QueueStore::Postgres(store) => store.fail(
                policy,
                job_id,
                attempt_id,
                worker_id,
                now,
                *retry,
                retry_at,
                error_class,
                self.limits.maximum_attempts,
            )?,
        };
        Ok(Value::Bool(accepted))
    }

    fn cancel(&self, policy: &CallPolicy, arguments: &[Value]) -> Result<Value, ExecutionError> {
        let [job_id, now] = arguments else {
            return Err(queue_argument("cancel expects job id and current time"));
        };
        let job_id = token(job_id, "job id")?;
        let now = nonnegative_time(now, "cancellation time")?;
        let accepted = match &self.store {
            QueueStore::Memory(store) => store.cancel(job_id, now)?,
            QueueStore::Postgres(store) => store.cancel(policy, job_id, now)?,
        };
        Ok(Value::Bool(accepted))
    }

    fn inspect(&self, policy: &CallPolicy, arguments: &[Value]) -> Result<Value, ExecutionError> {
        let [job_id] = arguments else {
            return Err(queue_argument("inspect expects one job id"));
        };
        let job_id = token(job_id, "job id")?;
        let snapshot = match &self.store {
            QueueStore::Memory(store) => store.inspect(job_id)?,
            QueueStore::Postgres(store) => store.inspect(policy, job_id)?,
        };
        Ok(Value::List(Arc::new(
            snapshot.into_iter().map(snapshot_value).collect(),
        )))
    }
}

impl CapabilityAdapter for DurableQueueAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        policy.control.check()?;
        match policy.operation.as_str() {
            "initialize" => self.initialize(policy, &arguments),
            "enqueue" => self.enqueue(policy, &arguments),
            "claim" => self.claim(policy, &arguments),
            "heartbeat" => self.heartbeat(policy, &arguments),
            "complete" => self.complete(policy, &arguments),
            "fail" => self.fail(policy, &arguments),
            "cancel" => self.cancel(policy, &arguments),
            "inspect" => self.inspect(policy, &arguments),
            operation => Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "queue_operation_unknown",
                format!("durable queue adapter does not implement '{operation}'"),
            )),
        }
    }

    fn shutdown(&self) -> Result<(), ExecutionError> {
        match &self.store {
            QueueStore::Memory(_) => Ok(()),
            QueueStore::Postgres(store) => store.pool.close(),
        }
    }
}

#[derive(Clone, Default)]
struct MemoryQueue {
    state: Arc<Mutex<MemoryQueueState>>,
}

#[derive(Default)]
struct MemoryQueueState {
    jobs: BTreeMap<String, MemoryJob>,
    idempotency: HashMap<String, String>,
}

#[derive(Clone)]
struct MemoryJob {
    job_id: String,
    idempotency_key: String,
    payload: Vec<u8>,
    state: JobState,
    available_at: i64,
    created_at: i64,
    attempt_count: u32,
    attempt_id: Option<String>,
    worker_id: Option<String>,
    lease_until: Option<i64>,
    result: Option<Vec<u8>>,
    last_error_class: Option<String>,
}

impl MemoryQueue {
    fn enqueue(
        &self,
        job_id: &str,
        idempotency_key: &str,
        payload: &[u8],
        available_at: i64,
        created_at: i64,
    ) -> Result<bool, ExecutionError> {
        let mut state = lock_unpoisoned(&self.state);
        if let Some(existing_id) = state.idempotency.get(idempotency_key) {
            let existing = state
                .jobs
                .get(existing_id)
                .ok_or_else(|| queue_internal("memory queue idempotency index is inconsistent"))?;
            if existing.job_id == job_id && existing.payload == payload {
                return Ok(false);
            }
            return Err(queue_conflict(
                "queue_idempotency_conflict",
                "idempotency key already owns different job input",
            ));
        }
        if state.jobs.contains_key(job_id) {
            return Err(queue_conflict(
                "queue_job_conflict",
                "job id already exists under a different idempotency key",
            ));
        }
        state
            .idempotency
            .insert(idempotency_key.to_owned(), job_id.to_owned());
        state.jobs.insert(
            job_id.to_owned(),
            MemoryJob {
                job_id: job_id.to_owned(),
                idempotency_key: idempotency_key.to_owned(),
                payload: payload.to_vec(),
                state: JobState::Ready,
                available_at,
                created_at,
                attempt_count: 0,
                attempt_id: None,
                worker_id: None,
                lease_until: None,
                result: None,
                last_error_class: None,
            },
        );
        Ok(true)
    }

    fn claim(
        &self,
        worker_id: &str,
        now: i64,
        lease_duration: i64,
        maximum_attempts: u32,
    ) -> Result<Option<JobLease>, ExecutionError> {
        let mut state = lock_unpoisoned(&self.state);
        for job in state.jobs.values_mut() {
            if job.state == JobState::Leased
                && job.lease_until.is_some_and(|lease| lease <= now)
                && job.attempt_count >= maximum_attempts
            {
                job.state = JobState::Failed;
                job.attempt_id = None;
                job.worker_id = None;
                job.lease_until = None;
                job.last_error_class = Some("attempt_limit".to_owned());
            }
        }
        let selected = state
            .jobs
            .iter()
            .filter(|(_, job)| {
                ((job.state == JobState::Ready && job.available_at <= now)
                    || (job.state == JobState::Leased
                        && job.lease_until.is_some_and(|lease| lease <= now)))
                    && job.attempt_count < maximum_attempts
            })
            .min_by_key(|(job_id, job)| (job.available_at, job.created_at, job_id.as_str()))
            .map(|(job_id, _)| job_id.clone());
        let Some(job_id) = selected else {
            return Ok(None);
        };
        let job = state
            .jobs
            .get_mut(&job_id)
            .ok_or_else(|| queue_internal("selected memory queue job disappeared"))?;
        job.attempt_count = job
            .attempt_count
            .checked_add(1)
            .ok_or_else(|| queue_internal("queue attempt count overflowed"))?;
        let attempt_id = format!("{}:{}", job.job_id, job.attempt_count);
        let lease_until = now
            .checked_add(lease_duration)
            .ok_or_else(|| queue_argument("queue lease time overflowed"))?;
        job.state = JobState::Leased;
        job.attempt_id = Some(attempt_id.clone());
        job.worker_id = Some(worker_id.to_owned());
        job.lease_until = Some(lease_until);
        Ok(Some(JobLease {
            job_id: job.job_id.clone(),
            attempt_id,
            payload: job.payload.clone(),
            attempt_number: job.attempt_count,
            lease_until_milliseconds: lease_until,
        }))
    }

    fn heartbeat(
        &self,
        job_id: &str,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
        lease_duration: i64,
    ) -> Result<bool, ExecutionError> {
        let mut state = lock_unpoisoned(&self.state);
        let Some(job) = state.jobs.get_mut(job_id) else {
            return Ok(false);
        };
        if !owns_live_lease(job, attempt_id, worker_id, now) {
            return Ok(false);
        }
        job.lease_until = Some(
            now.checked_add(lease_duration)
                .ok_or_else(|| queue_argument("queue lease time overflowed"))?,
        );
        Ok(true)
    }

    fn complete(
        &self,
        job_id: &str,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
        result: &[u8],
    ) -> Result<bool, ExecutionError> {
        let mut state = lock_unpoisoned(&self.state);
        let Some(job) = state.jobs.get_mut(job_id) else {
            return Ok(false);
        };
        if !owns_live_lease(job, attempt_id, worker_id, now) {
            return Ok(false);
        }
        job.state = JobState::Completed;
        job.result = Some(result.to_vec());
        job.attempt_id = None;
        job.worker_id = None;
        job.lease_until = None;
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn fail(
        &self,
        job_id: &str,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
        retry: bool,
        retry_at: i64,
        error_class: &str,
        maximum_attempts: u32,
    ) -> Result<bool, ExecutionError> {
        let mut state = lock_unpoisoned(&self.state);
        let Some(job) = state.jobs.get_mut(job_id) else {
            return Ok(false);
        };
        if !owns_live_lease(job, attempt_id, worker_id, now) {
            return Ok(false);
        }
        job.last_error_class = Some(error_class.to_owned());
        if retry && job.attempt_count < maximum_attempts {
            job.state = JobState::Ready;
            job.available_at = retry_at;
        } else {
            job.state = JobState::Failed;
        }
        job.attempt_id = None;
        job.worker_id = None;
        job.lease_until = None;
        Ok(true)
    }

    fn cancel(&self, job_id: &str, _now: i64) -> Result<bool, ExecutionError> {
        let mut state = lock_unpoisoned(&self.state);
        let Some(job) = state.jobs.get_mut(job_id) else {
            return Ok(false);
        };
        if matches!(job.state, JobState::Completed | JobState::Cancelled) {
            return Ok(false);
        }
        job.state = JobState::Cancelled;
        job.attempt_id = None;
        job.worker_id = None;
        job.lease_until = None;
        Ok(true)
    }

    fn inspect(&self, job_id: &str) -> Result<Option<JobSnapshot>, ExecutionError> {
        let state = lock_unpoisoned(&self.state);
        Ok(state.jobs.get(job_id).map(memory_snapshot))
    }
}

fn owns_live_lease(job: &MemoryJob, attempt_id: &str, worker_id: &str, now: i64) -> bool {
    job.state == JobState::Leased
        && job.attempt_id.as_deref() == Some(attempt_id)
        && job.worker_id.as_deref() == Some(worker_id)
        && job.lease_until.is_some_and(|lease| lease > now)
}

fn memory_snapshot(job: &MemoryJob) -> JobSnapshot {
    let _ = &job.idempotency_key;
    JobSnapshot {
        job_id: job.job_id.clone(),
        state: job.state,
        attempt_count: job.attempt_count,
        available_at_milliseconds: job.available_at,
        lease_until_milliseconds: job.lease_until,
        result: job.result.clone(),
        last_error_class: job.last_error_class.clone(),
    }
}

#[derive(Clone)]
struct PostgresQueue {
    pool: PostgresPool,
    namespace: String,
}

impl PostgresQueue {
    fn initialize(&self, policy: &CallPolicy) -> Result<(), ExecutionError> {
        let mut connection = self.pool.acquire(&policy.control)?;
        let result = connection
            .client()?
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS lkjscript_durable_jobs (\
                 queue_namespace TEXT NOT NULL, \
                 job_id TEXT NOT NULL, \
                 idempotency_key TEXT NOT NULL, \
                 payload BYTEA NOT NULL, \
                 state TEXT NOT NULL CHECK (state IN ('ready','leased','completed','failed','cancelled')), \
                 available_at_ms BIGINT NOT NULL CHECK (available_at_ms >= 0), \
                 created_at_ms BIGINT NOT NULL CHECK (created_at_ms >= 0), \
                 attempt_count BIGINT NOT NULL DEFAULT 0 CHECK (attempt_count >= 0), \
                 attempt_id TEXT, worker_id TEXT, lease_until_ms BIGINT, result BYTEA, \
                 last_error_class TEXT, completed_at_ms BIGINT, \
                 PRIMARY KEY (queue_namespace, job_id), \
                 UNIQUE (queue_namespace, idempotency_key)); \
                 CREATE INDEX IF NOT EXISTS lkjscript_durable_jobs_claim \
                 ON lkjscript_durable_jobs \
                 (queue_namespace, state, available_at_ms, lease_until_ms, created_at_ms, job_id);",
            )
            .map_err(|error| map_postgres_error(error, policy, true));
        if result.is_err() {
            connection.discard();
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn enqueue(
        &self,
        policy: &CallPolicy,
        job_id: &str,
        idempotency_key: &str,
        payload: &[u8],
        available_at: i64,
        created_at: i64,
    ) -> Result<bool, ExecutionError> {
        let mut connection = self.pool.acquire(&policy.control)?;
        let result = (|| {
            let mut transaction = connection
                .client()?
                .transaction()
                .map_err(|error| map_postgres_error(error, policy, false))?;
            let inserted = transaction
                .execute(
                    "INSERT INTO lkjscript_durable_jobs \
                     (queue_namespace, job_id, idempotency_key, payload, state, available_at_ms, created_at_ms) \
                     VALUES ($1, $2, $3, $4, 'ready', $5, $6) ON CONFLICT DO NOTHING",
                    &[&self.namespace, &job_id, &idempotency_key, &payload, &available_at, &created_at],
                )
                .map_err(|error| map_postgres_error(error, policy, false))?;
            if inserted == 0 {
                let existing = transaction
                    .query_opt(
                        "SELECT job_id, payload FROM lkjscript_durable_jobs \
                         WHERE queue_namespace = $1 AND idempotency_key = $2",
                        &[&self.namespace, &idempotency_key],
                    )
                    .map_err(|error| map_postgres_error(error, policy, false))?;
                match existing {
                    Some(row)
                        if row
                            .try_get::<_, String>(0)
                            .map_err(|error| map_postgres_error(error, policy, false))?
                            == job_id
                            && row
                                .try_get::<_, Vec<u8>>(1)
                                .map_err(|error| map_postgres_error(error, policy, false))?
                                == payload => {}
                    Some(_) => {
                        return Err(queue_conflict(
                            "queue_idempotency_conflict",
                            "idempotency key already owns different job input",
                        ));
                    }
                    None => {
                        return Err(queue_conflict(
                            "queue_job_conflict",
                            "job id already exists under a different idempotency key",
                        ));
                    }
                }
            }
            transaction
                .commit()
                .map_err(|error| map_postgres_error(error, policy, true))?;
            Ok(inserted == 1)
        })();
        if result.is_err() {
            connection.discard();
        }
        result
    }

    fn claim(
        &self,
        policy: &CallPolicy,
        worker_id: &str,
        now: i64,
        lease_duration: i64,
        maximum_attempts: u32,
    ) -> Result<Option<JobLease>, ExecutionError> {
        let lease_until = now
            .checked_add(lease_duration)
            .ok_or_else(|| queue_argument("queue lease time overflowed"))?;
        let maximum_attempts = i64::from(maximum_attempts);
        let mut connection = self.pool.acquire(&policy.control)?;
        let result = (|| {
            let mut transaction = connection
                .client()?
                .transaction()
                .map_err(|error| map_postgres_error(error, policy, false))?;
            transaction
                .execute(
                    "UPDATE lkjscript_durable_jobs SET state = 'failed', attempt_id = NULL, \
                     worker_id = NULL, lease_until_ms = NULL, last_error_class = 'attempt_limit' \
                     WHERE queue_namespace = $1 AND state = 'leased' AND lease_until_ms <= $2 \
                     AND attempt_count >= $3",
                    &[&self.namespace, &now, &maximum_attempts],
                )
                .map_err(|error| map_postgres_error(error, policy, false))?;
            let row = transaction
                .query_opt(
                    "WITH candidate AS ( \
                       SELECT job_id FROM lkjscript_durable_jobs \
                       WHERE queue_namespace = $1 AND attempt_count < $2 AND \
                         ((state = 'ready' AND available_at_ms <= $3) OR \
                          (state = 'leased' AND lease_until_ms <= $3)) \
                       ORDER BY available_at_ms, created_at_ms, job_id \
                       FOR UPDATE SKIP LOCKED LIMIT 1 \
                     ) \
                     UPDATE lkjscript_durable_jobs AS job SET \
                       state = 'leased', attempt_count = job.attempt_count + 1, \
                       attempt_id = job.job_id || ':' || (job.attempt_count + 1)::text, \
                       worker_id = $4, lease_until_ms = $5 \
                     FROM candidate WHERE job.queue_namespace = $1 AND job.job_id = candidate.job_id \
                     RETURNING job.job_id, job.attempt_id, job.payload, job.attempt_count, job.lease_until_ms",
                    &[&self.namespace, &maximum_attempts, &now, &worker_id, &lease_until],
                )
                .map_err(|error| map_postgres_error(error, policy, false))?;
            let lease = row.as_ref().map(decode_lease).transpose()?;
            transaction
                .commit()
                .map_err(|error| map_postgres_error(error, policy, true))?;
            Ok(lease)
        })();
        if result.is_err() {
            connection.discard();
        }
        result
    }

    fn heartbeat(
        &self,
        policy: &CallPolicy,
        job_id: &str,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
        lease_duration: i64,
    ) -> Result<bool, ExecutionError> {
        let lease_until = now
            .checked_add(lease_duration)
            .ok_or_else(|| queue_argument("queue lease time overflowed"))?;
        self.mutate(
            policy,
            "UPDATE lkjscript_durable_jobs SET lease_until_ms = $6 \
             WHERE queue_namespace = $1 AND job_id = $2 AND state = 'leased' \
             AND attempt_id = $3 AND worker_id = $4 AND lease_until_ms > $5",
            &[
                &self.namespace,
                &job_id,
                &attempt_id,
                &worker_id,
                &now,
                &lease_until,
            ],
        )
    }

    fn complete(
        &self,
        policy: &CallPolicy,
        job_id: &str,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
        result: &[u8],
    ) -> Result<bool, ExecutionError> {
        self.mutate(
            policy,
            "UPDATE lkjscript_durable_jobs SET state = 'completed', result = $6, \
             completed_at_ms = $5, attempt_id = NULL, worker_id = NULL, lease_until_ms = NULL \
             WHERE queue_namespace = $1 AND job_id = $2 AND state = 'leased' \
             AND attempt_id = $3 AND worker_id = $4 AND lease_until_ms > $5",
            &[
                &self.namespace,
                &job_id,
                &attempt_id,
                &worker_id,
                &now,
                &result,
            ],
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn fail(
        &self,
        policy: &CallPolicy,
        job_id: &str,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
        retry: bool,
        retry_at: i64,
        error_class: &str,
        maximum_attempts: u32,
    ) -> Result<bool, ExecutionError> {
        let maximum_attempts = i64::from(maximum_attempts);
        self.mutate(
            policy,
            "UPDATE lkjscript_durable_jobs SET \
               state = CASE WHEN $6 AND attempt_count < $9 THEN 'ready' ELSE 'failed' END, \
               available_at_ms = CASE WHEN $6 AND attempt_count < $9 THEN $7 ELSE available_at_ms END, \
               last_error_class = $8, attempt_id = NULL, worker_id = NULL, lease_until_ms = NULL \
             WHERE queue_namespace = $1 AND job_id = $2 AND state = 'leased' \
             AND attempt_id = $3 AND worker_id = $4 AND lease_until_ms > $5",
            &[
                &self.namespace,
                &job_id,
                &attempt_id,
                &worker_id,
                &now,
                &retry,
                &retry_at,
                &error_class,
                &maximum_attempts,
            ],
        )
    }

    fn cancel(&self, policy: &CallPolicy, job_id: &str, now: i64) -> Result<bool, ExecutionError> {
        self.mutate(
            policy,
            "UPDATE lkjscript_durable_jobs SET state = 'cancelled', completed_at_ms = $3, \
             attempt_id = NULL, worker_id = NULL, lease_until_ms = NULL \
             WHERE queue_namespace = $1 AND job_id = $2 AND state NOT IN ('completed','cancelled')",
            &[&self.namespace, &job_id, &now],
        )
    }

    fn inspect(
        &self,
        policy: &CallPolicy,
        job_id: &str,
    ) -> Result<Option<JobSnapshot>, ExecutionError> {
        let mut connection = self.pool.acquire(&policy.control)?;
        let result = connection
            .client()?
            .query_opt(
                "SELECT job_id, state, attempt_count, available_at_ms, lease_until_ms, result, last_error_class \
                 FROM lkjscript_durable_jobs WHERE queue_namespace = $1 AND job_id = $2",
                &[&self.namespace, &job_id],
            )
            .map_err(|error| map_postgres_error(error, policy, false))
            .and_then(|row| row.as_ref().map(decode_snapshot).transpose());
        if result.is_err() {
            connection.discard();
        }
        result
    }

    fn mutate(
        &self,
        policy: &CallPolicy,
        statement: &str,
        parameters: &[&(dyn postgres::types::ToSql + Sync)],
    ) -> Result<bool, ExecutionError> {
        let mut connection = self.pool.acquire(&policy.control)?;
        let result = connection
            .client()?
            .execute(statement, parameters)
            .map(|count| count == 1)
            .map_err(|error| map_postgres_error(error, policy, true));
        if result.is_err() {
            connection.discard();
        }
        result
    }
}

fn decode_lease(row: &Row) -> Result<JobLease, ExecutionError> {
    let attempt_number: i64 = row
        .try_get(3)
        .map_err(|_| queue_internal("database returned an invalid attempt number"))?;
    Ok(JobLease {
        job_id: row
            .try_get(0)
            .map_err(|_| queue_internal("database returned an invalid job id"))?,
        attempt_id: row
            .try_get(1)
            .map_err(|_| queue_internal("database returned an invalid attempt id"))?,
        payload: row
            .try_get(2)
            .map_err(|_| queue_internal("database returned an invalid job payload"))?,
        attempt_number: u32::try_from(attempt_number)
            .map_err(|_| queue_internal("database attempt number is out of range"))?,
        lease_until_milliseconds: row
            .try_get(4)
            .map_err(|_| queue_internal("database returned an invalid lease time"))?,
    })
}

fn decode_snapshot(row: &Row) -> Result<JobSnapshot, ExecutionError> {
    let state: String = row
        .try_get(1)
        .map_err(|_| queue_internal("database returned an invalid queue state"))?;
    let attempt_count: i64 = row
        .try_get(2)
        .map_err(|_| queue_internal("database returned an invalid attempt count"))?;
    Ok(JobSnapshot {
        job_id: row
            .try_get(0)
            .map_err(|_| queue_internal("database returned an invalid job id"))?,
        state: JobState::parse(&state)?,
        attempt_count: u32::try_from(attempt_count)
            .map_err(|_| queue_internal("database attempt count is out of range"))?,
        available_at_milliseconds: row
            .try_get(3)
            .map_err(|_| queue_internal("database returned an invalid available time"))?,
        lease_until_milliseconds: row
            .try_get(4)
            .map_err(|_| queue_internal("database returned an invalid lease time"))?,
        result: row
            .try_get(5)
            .map_err(|_| queue_internal("database returned an invalid result"))?,
        last_error_class: row
            .try_get(6)
            .map_err(|_| queue_internal("database returned an invalid error class"))?,
    })
}

fn lease_value(lease: JobLease) -> Value {
    Value::record(
        None,
        [
            ("job_id".to_owned(), Value::text(lease.job_id)),
            ("attempt_id".to_owned(), Value::text(lease.attempt_id)),
            ("payload".to_owned(), Value::bytes(lease.payload)),
            (
                "attempt_number".to_owned(),
                Value::I64(i64::from(lease.attempt_number)),
            ),
            (
                "lease_until_milliseconds".to_owned(),
                Value::I64(lease.lease_until_milliseconds),
            ),
        ],
    )
}

fn snapshot_value(snapshot: JobSnapshot) -> Value {
    Value::record(
        None,
        [
            ("job_id".to_owned(), Value::text(snapshot.job_id)),
            ("state".to_owned(), Value::text(snapshot.state.as_str())),
            (
                "attempt_count".to_owned(),
                Value::I64(i64::from(snapshot.attempt_count)),
            ),
            (
                "available_at_milliseconds".to_owned(),
                Value::I64(snapshot.available_at_milliseconds),
            ),
            (
                "lease_until_milliseconds".to_owned(),
                Value::I64(snapshot.lease_until_milliseconds.unwrap_or(-1)),
            ),
            (
                "result".to_owned(),
                Value::bytes(snapshot.result.unwrap_or_default()),
            ),
            (
                "last_error_class".to_owned(),
                Value::text(snapshot.last_error_class.unwrap_or_default()),
            ),
        ],
    )
}

fn token<'a>(value: &'a Value, label: &str) -> Result<&'a str, ExecutionError> {
    let value = bounded_text(value, label, 256)?;
    validate_token(value, label)?;
    Ok(value)
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

fn bounded_text<'a>(
    value: &'a Value,
    label: &str,
    maximum: usize,
) -> Result<&'a str, ExecutionError> {
    let Value::Text(value) = value else {
        return Err(queue_argument(format!("{label} must be Text")));
    };
    if value.is_empty() || value.len() > maximum || value.contains('\0') {
        return Err(queue_argument(format!(
            "{label} is empty, excessive, or contains NUL"
        )));
    }
    Ok(value)
}

fn bytes<'a>(value: &'a Value, label: &str, maximum: usize) -> Result<&'a [u8], ExecutionError> {
    let Value::Bytes(value) = value else {
        return Err(queue_argument(format!("{label} must be Bytes")));
    };
    if value.len() > maximum {
        return Err(ExecutionError::resource(
            "queue_byte_limit",
            format!("{label} exceeds its exact byte limit"),
        ));
    }
    Ok(value)
}

fn nonnegative_time(value: &Value, label: &str) -> Result<i64, ExecutionError> {
    let Value::I64(value) = value else {
        return Err(queue_argument(format!("{label} must be I64")));
    };
    if *value < 0 {
        return Err(queue_argument(format!("{label} must be non-negative")));
    }
    Ok(*value)
}

fn lease_duration(value: &Value, limits: &QueueLimits) -> Result<i64, ExecutionError> {
    let lease = nonnegative_time(value, "lease duration")?;
    if lease == 0 || lease > limits.maximum_lease_milliseconds {
        return Err(ExecutionError::resource(
            "queue_lease_limit",
            "lease duration is zero or exceeds its exact limit",
        ));
    }
    Ok(lease)
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

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PackageId;
    use crate::platform::execution::ExecutionControl;
    use crate::platform::language::{Idempotency, Visibility};

    fn owner() -> OwnerId {
        OwnerId::deterministic_for_test(
            PackageId::parse("1234567890abcdef1234567890abcdef").expect("package id"),
            "queue",
            "DurableQueue",
        )
    }

    fn policy(operation: &str) -> CallPolicy {
        CallPolicy {
            requirement: "jobs".to_owned(),
            interface: owner(),
            operation: operation.to_owned(),
            idempotency: Idempotency::IdempotentWithKey,
            visibility: Visibility::Possible,
            limits: BTreeMap::new(),
            control: ExecutionControl::uncancelled(),
        }
    }

    fn enqueue(adapter: &DurableQueueAdapter, job: &str, key: &str, payload: &[u8]) -> Value {
        adapter
            .call(
                &policy("enqueue"),
                vec![
                    Value::text(job),
                    Value::text(key),
                    Value::bytes(payload.to_vec()),
                    Value::I64(0),
                    Value::I64(0),
                ],
            )
            .expect("enqueue")
    }

    fn claim(adapter: &DurableQueueAdapter, worker: &str, now: i64) -> JobLease {
        let result = adapter
            .call(
                &policy("claim"),
                vec![Value::text(worker), Value::I64(now), Value::I64(10)],
            )
            .expect("claim");
        let Value::List(items) = result else {
            panic!("claim did not return a list")
        };
        let [Value::Record { fields, .. }] = items.as_slice() else {
            panic!("claim did not return one lease")
        };
        JobLease {
            job_id: value_text(&fields["job_id"]),
            attempt_id: value_text(&fields["attempt_id"]),
            payload: value_bytes(&fields["payload"]),
            attempt_number: u32::try_from(value_i64(&fields["attempt_number"]))
                .expect("attempt number"),
            lease_until_milliseconds: value_i64(&fields["lease_until_milliseconds"]),
        }
    }

    #[test]
    fn duplicate_enqueue_is_idempotent_and_conflicting_input_rejects() {
        let adapter =
            DurableQueueAdapter::in_memory(owner(), QueueLimits::default()).expect("adapter");
        assert!(matches!(
            enqueue(&adapter, "job-1", "key-1", b"a"),
            Value::Bool(true)
        ));
        assert!(matches!(
            enqueue(&adapter, "job-1", "key-1", b"a"),
            Value::Bool(false)
        ));
        let error = adapter
            .call(
                &policy("enqueue"),
                vec![
                    Value::text("job-1"),
                    Value::text("key-1"),
                    Value::bytes(b"changed".to_vec()),
                    Value::I64(0),
                    Value::I64(0),
                ],
            )
            .expect_err("conflicting replay must reject");
        assert_eq!(error.code, "queue_idempotency_conflict");
    }

    #[test]
    fn lease_loss_and_stale_completion_cannot_publish_twice() {
        let adapter =
            DurableQueueAdapter::in_memory(owner(), QueueLimits::default()).expect("adapter");
        enqueue(&adapter, "job-1", "key-1", b"payload");
        let first = claim(&adapter, "worker-1", 100);
        assert_eq!(first.attempt_number, 1);
        let second = claim(&adapter, "worker-2", 110);
        assert_eq!(second.attempt_number, 2);
        let stale = adapter
            .call(
                &policy("complete"),
                vec![
                    Value::text(first.job_id.as_str()),
                    Value::text(first.attempt_id.as_str()),
                    Value::text("worker-1"),
                    Value::I64(110),
                    Value::bytes(b"stale".to_vec()),
                ],
            )
            .expect("stale completion is a typed rejection");
        assert!(matches!(stale, Value::Bool(false)));
        let accepted = adapter
            .call(
                &policy("complete"),
                vec![
                    Value::text(second.job_id.as_str()),
                    Value::text(second.attempt_id.as_str()),
                    Value::text("worker-2"),
                    Value::I64(111),
                    Value::bytes(b"result".to_vec()),
                ],
            )
            .expect("current completion");
        assert!(matches!(accepted, Value::Bool(true)));
        let repeated = adapter
            .call(
                &policy("complete"),
                vec![
                    Value::text(second.job_id.as_str()),
                    Value::text(second.attempt_id.as_str()),
                    Value::text("worker-2"),
                    Value::I64(111),
                    Value::bytes(b"result".to_vec()),
                ],
            )
            .expect("repeated completion");
        assert!(matches!(repeated, Value::Bool(false)));
    }

    #[test]
    fn retry_policy_is_explicit_and_attempt_bound_is_terminal() {
        let limits = QueueLimits {
            maximum_attempts: 2,
            ..QueueLimits::default()
        };
        let adapter = DurableQueueAdapter::in_memory(owner(), limits).expect("adapter");
        enqueue(&adapter, "job-1", "key-1", b"payload");
        let first = claim(&adapter, "worker", 0);
        let failed = adapter
            .call(
                &policy("fail"),
                vec![
                    Value::text(first.job_id.as_str()),
                    Value::text(first.attempt_id.as_str()),
                    Value::text("worker"),
                    Value::I64(1),
                    Value::Bool(true),
                    Value::I64(5),
                    Value::text("provider_unavailable"),
                ],
            )
            .expect("retry decision");
        assert!(matches!(failed, Value::Bool(true)));
        let second = claim(&adapter, "worker", 5);
        let failed = adapter
            .call(
                &policy("fail"),
                vec![
                    Value::text(second.job_id.as_str()),
                    Value::text(second.attempt_id.as_str()),
                    Value::text("worker"),
                    Value::I64(6),
                    Value::Bool(true),
                    Value::I64(7),
                    Value::text("provider_unavailable"),
                ],
            )
            .expect("attempt bound");
        assert!(matches!(failed, Value::Bool(true)));
        let empty = adapter
            .call(
                &policy("claim"),
                vec![Value::text("worker"), Value::I64(7), Value::I64(10)],
            )
            .expect("no third attempt");
        assert!(matches!(empty, Value::List(items) if items.is_empty()));
    }

    fn value_text(value: &Value) -> String {
        let Value::Text(value) = value else {
            panic!("not text")
        };
        value.to_string()
    }

    fn value_bytes(value: &Value) -> Vec<u8> {
        let Value::Bytes(value) = value else {
            panic!("not bytes")
        };
        value.to_vec()
    }

    fn value_i64(value: &Value) -> i64 {
        let Value::I64(value) = value else {
            panic!("not i64")
        };
        *value
    }
}
