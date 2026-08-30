//! Durable-queue state transitions over the first-party ordered data engine.

use super::{JobLease, JobSnapshot, JobState, QueueLimits, queue_conflict, queue_internal};
use crate::platform::data::{
    DataCommitOutcome, DataEntry, DataExpectation, DataKey, DataKeyPart, DataScanDirection,
    DataSchema, DataSchemaExpectation, DataStore, DataTransaction,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};

const JOB_SPACE: &str = "__queue.jobs";
const IDEMPOTENCY_SPACE: &str = "__queue.idempotency";
const CLAIM_SPACE: &str = "__queue.claim";
const SCHEMA_SPACE: &str = "__queue.schema";
const QUEUE_SCHEMA_IDENTITY: &str = "lkjscript-durable-queue-data-1";
const JOB_MAGIC: &[u8; 8] = b"LKJQJOB1";
const JOB_CHECKSUM_DOMAIN: &str = "lkjscript.queue.data-job.v1";
const MAXIMUM_TRANSACTION_RETRIES: usize = 32;

#[derive(Clone, Debug)]
pub(super) struct DataQueue {
    store: DataStore,
    limits: QueueLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredJob {
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

impl DataQueue {
    pub(super) const fn new(store: DataStore, limits: QueueLimits) -> Self {
        Self { store, limits }
    }

    pub(super) fn preflight(&self) -> Result<(), ExecutionError> {
        self.store
            .verify()
            .map(|_| ())
            .map_err(|error| map_data_error(error, false))
    }

    pub(super) fn initialize(
        &self,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<(), ExecutionError> {
        let schema = queue_schema();
        self.transact(
            control,
            possible_visibility,
            |transaction| match transaction
                .schema_read(SCHEMA_SPACE)
                .map_err(|error| map_data_error(error, false))?
            {
                Some(existing) if existing == schema => Ok(()),
                Some(_) => Err(queue_conflict(
                    "queue_schema_conflict",
                    "durable queue namespace has a divergent schema marker",
                )),
                None => {
                    if !transaction
                        .schema_set(
                            SCHEMA_SPACE,
                            &DataSchemaExpectation::Missing,
                            schema.clone(),
                        )
                        .map_err(|error| map_data_error(error, false))?
                    {
                        return Err(queue_internal(
                            "durable queue schema expectation changed inside one transaction",
                        ));
                    }
                    Ok(())
                }
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn enqueue(
        &self,
        job_id: &str,
        idempotency_key: &str,
        payload: &[u8],
        available_at: i64,
        created_at: i64,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<bool, ExecutionError> {
        self.transact(control, possible_visibility, |transaction| {
            let idempotency_key_value = text_key(&self.store, idempotency_key)?;
            if let Some(index) = transaction
                .get(IDEMPOTENCY_SPACE, &idempotency_key_value)
                .map_err(|error| map_data_error(error, false))?
            {
                let indexed_job = std::str::from_utf8(&index.value).map_err(|_| {
                    queue_internal("durable queue idempotency index contains non-UTF-8 data")
                })?;
                let existing = self.require_job(transaction, indexed_job)?;
                if existing.0.job_id == job_id && existing.0.payload == payload {
                    return Ok(false);
                }
                return Err(queue_conflict(
                    "queue_idempotency_conflict",
                    "idempotency key already owns different job input",
                ));
            }
            let job_key = text_key(&self.store, job_id)?;
            if transaction
                .get(JOB_SPACE, &job_key)
                .map_err(|error| map_data_error(error, false))?
                .is_some()
            {
                return Err(queue_conflict(
                    "queue_job_conflict",
                    "job id already exists under a different idempotency key",
                ));
            }
            let job = StoredJob {
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
            };
            if !transaction
                .put(
                    JOB_SPACE,
                    &job_key,
                    encode_job(&job)?,
                    DataExpectation::Missing,
                )
                .map_err(|error| map_data_error(error, false))?
            {
                return Err(queue_internal(
                    "durable queue primary insert lost its expectation",
                ));
            }
            if !transaction
                .put(
                    IDEMPOTENCY_SPACE,
                    &idempotency_key_value,
                    job_id.as_bytes().to_vec(),
                    DataExpectation::Missing,
                )
                .map_err(|error| map_data_error(error, false))?
                || !transaction
                    .put(
                        CLAIM_SPACE,
                        &claim_key(&self.store, &job)?,
                        Vec::new(),
                        DataExpectation::Missing,
                    )
                    .map_err(|error| map_data_error(error, false))?
            {
                return Err(queue_internal(
                    "durable queue dependent index insert lost its expectation",
                ));
            }
            Ok(true)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn claim(
        &self,
        worker_id: &str,
        now: i64,
        lease_duration: i64,
        maximum_attempts: u32,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<Option<JobLease>, ExecutionError> {
        self.transact(control, possible_visibility, |transaction| {
            let page = transaction
                .scan(
                    CLAIM_SPACE,
                    &[],
                    DataScanDirection::Forward,
                    1,
                    self.store.limits().maximum_scan_bytes,
                    self.store.limits().maximum_scan_work,
                    None,
                )
                .map_err(|error| map_data_error(error, false))?;
            let Some(index) = page.items.first() else {
                return Ok(None);
            };
            let (claim_at, job_id) = decode_claim_key(&index.key)?;
            if claim_at > now {
                return Ok(None);
            }
            let (mut job, primary) = self.require_job(transaction, &job_id)?;
            if claim_key(&self.store, &job)? != index.key {
                return Err(queue_internal(
                    "durable queue claim index disagrees with its primary record",
                ));
            }
            if job.attempt_count >= maximum_attempts {
                job.state = JobState::Failed;
                job.attempt_id = None;
                job.worker_id = None;
                job.lease_until = None;
                job.last_error_class = Some("attempt_limit".to_owned());
                self.replace_job(transaction, &job, &primary)?;
                self.delete_index(transaction, &index.key, index.revision)?;
                return Ok(None);
            }
            job.attempt_count = job
                .attempt_count
                .checked_add(1)
                .ok_or_else(|| queue_internal("durable queue attempt counter overflowed"))?;
            let attempt_id = format!("{}:{}", job.job_id, job.attempt_count);
            let lease_until = now
                .checked_add(lease_duration)
                .ok_or_else(|| super::queue_argument("queue lease time overflowed"))?;
            job.state = JobState::Leased;
            job.attempt_id = Some(attempt_id.clone());
            job.worker_id = Some(worker_id.to_owned());
            job.lease_until = Some(lease_until);
            self.replace_job(transaction, &job, &primary)?;
            self.delete_index(transaction, &index.key, index.revision)?;
            self.insert_claim_index(transaction, &job)?;
            Ok(Some(JobLease {
                job_id: job.job_id.clone(),
                attempt_id,
                payload: job.payload.clone(),
                attempt_number: job.attempt_count,
                lease_until_milliseconds: lease_until,
            }))
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn heartbeat(
        &self,
        job_id: &str,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
        lease_duration: i64,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<bool, ExecutionError> {
        self.transact(control, possible_visibility, |transaction| {
            let Some((mut job, primary)) = self.read_job(transaction, job_id)? else {
                return Ok(false);
            };
            if !owns_live_lease(&job, attempt_id, worker_id, now) {
                return Ok(false);
            }
            let old_index = self.require_claim_index(transaction, &job)?;
            job.lease_until = Some(
                now.checked_add(lease_duration)
                    .ok_or_else(|| super::queue_argument("queue lease time overflowed"))?,
            );
            self.replace_job(transaction, &job, &primary)?;
            self.delete_index(transaction, &old_index.0, old_index.1.revision)?;
            self.insert_claim_index(transaction, &job)?;
            Ok(true)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn complete(
        &self,
        job_id: &str,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
        result: &[u8],
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<bool, ExecutionError> {
        self.transact(control, possible_visibility, |transaction| {
            let Some((mut job, primary)) = self.read_job(transaction, job_id)? else {
                return Ok(false);
            };
            if !owns_live_lease(&job, attempt_id, worker_id, now) {
                return Ok(false);
            }
            let old_index = self.require_claim_index(transaction, &job)?;
            job.state = JobState::Completed;
            job.result = Some(result.to_vec());
            job.attempt_id = None;
            job.worker_id = None;
            job.lease_until = None;
            self.replace_job(transaction, &job, &primary)?;
            self.delete_index(transaction, &old_index.0, old_index.1.revision)?;
            Ok(true)
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn fail(
        &self,
        job_id: &str,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
        retry: bool,
        retry_at: i64,
        error_class: &str,
        maximum_attempts: u32,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<bool, ExecutionError> {
        self.transact(control, possible_visibility, |transaction| {
            let Some((mut job, primary)) = self.read_job(transaction, job_id)? else {
                return Ok(false);
            };
            if !owns_live_lease(&job, attempt_id, worker_id, now) {
                return Ok(false);
            }
            let old_index = self.require_claim_index(transaction, &job)?;
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
            self.replace_job(transaction, &job, &primary)?;
            self.delete_index(transaction, &old_index.0, old_index.1.revision)?;
            if job.state == JobState::Ready {
                self.insert_claim_index(transaction, &job)?;
            }
            Ok(true)
        })
    }

    pub(super) fn cancel(
        &self,
        job_id: &str,
        _now: i64,
        control: &ExecutionControl,
        possible_visibility: bool,
    ) -> Result<bool, ExecutionError> {
        self.transact(control, possible_visibility, |transaction| {
            let Some((mut job, primary)) = self.read_job(transaction, job_id)? else {
                return Ok(false);
            };
            if matches!(job.state, JobState::Completed | JobState::Cancelled) {
                return Ok(false);
            }
            let old_index = self.require_claim_index(transaction, &job)?;
            job.state = JobState::Cancelled;
            job.attempt_id = None;
            job.worker_id = None;
            job.lease_until = None;
            self.replace_job(transaction, &job, &primary)?;
            self.delete_index(transaction, &old_index.0, old_index.1.revision)?;
            Ok(true)
        })
    }

    pub(super) fn inspect(
        &self,
        job_id: &str,
        control: &ExecutionControl,
    ) -> Result<Option<JobSnapshot>, ExecutionError> {
        control.check()?;
        let transaction = self
            .store
            .begin()
            .map_err(|error| map_data_error(error, false))?;
        self.read_job(&transaction, job_id).map(|job| {
            job.map(|(job, _)| JobSnapshot {
                job_id: job.job_id,
                state: job.state,
                attempt_count: job.attempt_count,
                available_at_milliseconds: job.available_at,
                lease_until_milliseconds: job.lease_until,
                result: job.result,
                last_error_class: job.last_error_class,
            })
        })
    }

    fn transact<T, F>(
        &self,
        control: &ExecutionControl,
        possible_visibility: bool,
        mut operation: F,
    ) -> Result<T, ExecutionError>
    where
        F: FnMut(&mut DataTransaction) -> Result<T, ExecutionError>,
    {
        for _ in 0..MAXIMUM_TRANSACTION_RETRIES {
            control.check()?;
            let mut transaction = self
                .store
                .begin()
                .map_err(|error| map_data_error(error, false))?;
            let output = operation(&mut transaction)?;
            control.check()?;
            match transaction
                .commit()
                .map_err(|error| map_data_error(error, possible_visibility))?
            {
                DataCommitOutcome::Committed { .. } | DataCommitOutcome::Unchanged { .. } => {
                    return Ok(output);
                }
                DataCommitOutcome::Conflict { .. } => continue,
            }
        }
        let mut error = ExecutionError::new(
            ExecutionFailureClass::Capability,
            "queue_data_conflict",
            "durable queue exhausted its bounded exact-base conflict retries",
        );
        error.retryable = true;
        Err(error)
    }

    fn read_job(
        &self,
        transaction: &DataTransaction,
        job_id: &str,
    ) -> Result<Option<(StoredJob, DataEntry)>, ExecutionError> {
        let key = text_key(&self.store, job_id)?;
        transaction
            .get(JOB_SPACE, &key)
            .map_err(|error| map_data_error(error, false))?
            .map(|entry| decode_job(&entry.value, &self.limits).map(|job| (job, entry)))
            .transpose()
    }

    fn require_job(
        &self,
        transaction: &DataTransaction,
        job_id: &str,
    ) -> Result<(StoredJob, DataEntry), ExecutionError> {
        self.read_job(transaction, job_id)?.ok_or_else(|| {
            queue_internal("durable queue secondary index references a missing primary record")
        })
    }

    fn replace_job(
        &self,
        transaction: &mut DataTransaction,
        job: &StoredJob,
        primary: &DataEntry,
    ) -> Result<(), ExecutionError> {
        if !transaction
            .put(
                JOB_SPACE,
                &text_key(&self.store, &job.job_id)?,
                encode_job(job)?,
                DataExpectation::Exact(primary.revision),
            )
            .map_err(|error| map_data_error(error, false))?
        {
            return Err(queue_internal(
                "durable queue primary expectation changed inside one transaction",
            ));
        }
        Ok(())
    }

    fn require_claim_index(
        &self,
        transaction: &DataTransaction,
        job: &StoredJob,
    ) -> Result<(DataKey, DataEntry), ExecutionError> {
        let key = claim_key(&self.store, job)?;
        let entry = transaction
            .get(CLAIM_SPACE, &key)
            .map_err(|error| map_data_error(error, false))?
            .ok_or_else(|| {
                queue_internal("durable queue primary references a missing claim index")
            })?;
        Ok((key, entry))
    }

    fn delete_index(
        &self,
        transaction: &mut DataTransaction,
        key: &DataKey,
        revision: crate::platform::data::DataEntryRevision,
    ) -> Result<(), ExecutionError> {
        if !transaction
            .delete(CLAIM_SPACE, key, DataExpectation::Exact(revision))
            .map_err(|error| map_data_error(error, false))?
        {
            return Err(queue_internal(
                "durable queue claim-index expectation changed inside one transaction",
            ));
        }
        Ok(())
    }

    fn insert_claim_index(
        &self,
        transaction: &mut DataTransaction,
        job: &StoredJob,
    ) -> Result<(), ExecutionError> {
        if !transaction
            .put(
                CLAIM_SPACE,
                &claim_key(&self.store, job)?,
                Vec::new(),
                DataExpectation::Missing,
            )
            .map_err(|error| map_data_error(error, false))?
        {
            return Err(queue_internal(
                "durable queue claim-index insertion lost its expectation",
            ));
        }
        Ok(())
    }
}

fn owns_live_lease(job: &StoredJob, attempt_id: &str, worker_id: &str, now: i64) -> bool {
    job.state == JobState::Leased
        && job.attempt_id.as_deref() == Some(attempt_id)
        && job.worker_id.as_deref() == Some(worker_id)
        && job.lease_until.is_some_and(|lease| lease > now)
}

fn text_key(store: &DataStore, value: &str) -> Result<DataKey, ExecutionError> {
    DataKey::new(vec![DataKeyPart::Text(value.to_owned())], store.limits())
        .map_err(|error| map_data_error(error, false))
}

fn claim_key(store: &DataStore, job: &StoredJob) -> Result<DataKey, ExecutionError> {
    let claim_at = match job.state {
        JobState::Ready => job.available_at,
        JobState::Leased => job
            .lease_until
            .ok_or_else(|| queue_internal("leased durable queue job has no lease deadline"))?,
        _ => {
            return Err(queue_internal(
                "terminal durable queue job cannot own a claim index",
            ));
        }
    };
    DataKey::new(
        vec![
            DataKeyPart::I64(claim_at),
            DataKeyPart::I64(job.created_at),
            DataKeyPart::Text(job.job_id.clone()),
        ],
        store.limits(),
    )
    .map_err(|error| map_data_error(error, false))
}

fn decode_claim_key(key: &DataKey) -> Result<(i64, String), ExecutionError> {
    let [
        DataKeyPart::I64(claim_at),
        DataKeyPart::I64(_),
        DataKeyPart::Text(job_id),
    ] = key.parts()
    else {
        return Err(queue_internal(
            "durable queue claim index contains a foreign key shape",
        ));
    };
    Ok((*claim_at, job_id.clone()))
}

fn queue_schema() -> DataSchema {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.queue.data-schema.v1");
    hasher.update(&(QUEUE_SCHEMA_IDENTITY.len() as u64).to_be_bytes());
    hasher.update(QUEUE_SCHEMA_IDENTITY.as_bytes());
    DataSchema {
        identity: QUEUE_SCHEMA_IDENTITY.to_owned(),
        digest: hasher.finalize().as_bytes().to_vec(),
    }
}

fn encode_job(job: &StoredJob) -> Result<Vec<u8>, ExecutionError> {
    let mut output = Vec::new();
    output.extend_from_slice(JOB_MAGIC);
    push_text(&mut output, &job.job_id)?;
    push_text(&mut output, &job.idempotency_key)?;
    push_blob(&mut output, &job.payload)?;
    output.push(match job.state {
        JobState::Ready => 0,
        JobState::Leased => 1,
        JobState::Completed => 2,
        JobState::Failed => 3,
        JobState::Cancelled => 4,
    });
    output.extend_from_slice(&job.available_at.to_be_bytes());
    output.extend_from_slice(&job.created_at.to_be_bytes());
    output.extend_from_slice(&job.attempt_count.to_be_bytes());
    push_optional_text(&mut output, job.attempt_id.as_deref())?;
    push_optional_text(&mut output, job.worker_id.as_deref())?;
    push_optional_i64(&mut output, job.lease_until);
    push_optional_blob(&mut output, job.result.as_deref())?;
    push_optional_text(&mut output, job.last_error_class.as_deref())?;
    let checksum = digest(JOB_CHECKSUM_DOMAIN, &output);
    output.extend_from_slice(&checksum);
    Ok(output)
}

fn decode_job(bytes: &[u8], limits: &QueueLimits) -> Result<StoredJob, ExecutionError> {
    let payload_length = bytes
        .len()
        .checked_sub(32)
        .ok_or_else(|| queue_internal("durable queue primary record is truncated"))?;
    let (payload, checksum) = bytes.split_at(payload_length);
    if digest(JOB_CHECKSUM_DOMAIN, payload).as_slice() != checksum {
        return Err(queue_internal(
            "durable queue primary record checksum is corrupt",
        ));
    }
    let mut cursor = Cursor::new(payload);
    if cursor.take(8)? != JOB_MAGIC {
        return Err(queue_internal(
            "durable queue primary record has a foreign magic value",
        ));
    }
    let job_id = cursor.text(512)?;
    let idempotency_key = cursor.text(512)?;
    let payload = cursor.blob(limits.maximum_payload_bytes)?;
    let state = match cursor.u8()? {
        0 => JobState::Ready,
        1 => JobState::Leased,
        2 => JobState::Completed,
        3 => JobState::Failed,
        4 => JobState::Cancelled,
        _ => return Err(queue_internal("durable queue primary has a foreign state")),
    };
    let available_at = cursor.i64()?;
    let created_at = cursor.i64()?;
    let attempt_count = cursor.u32()?;
    let attempt_id = cursor.optional_text(512)?;
    let worker_id = cursor.optional_text(512)?;
    let lease_until = cursor.optional_i64()?;
    let result = cursor.optional_blob(limits.maximum_result_bytes)?;
    let last_error_class = cursor.optional_text(128)?;
    cursor.finish()?;
    let job = StoredJob {
        job_id,
        idempotency_key,
        payload,
        state,
        available_at,
        created_at,
        attempt_count,
        attempt_id,
        worker_id,
        lease_until,
        result,
        last_error_class,
    };
    validate_job_state(&job)?;
    Ok(job)
}

fn validate_job_state(job: &StoredJob) -> Result<(), ExecutionError> {
    let leased = job.state == JobState::Leased;
    if leased != (job.attempt_id.is_some() && job.worker_id.is_some() && job.lease_until.is_some())
    {
        return Err(queue_internal(
            "durable queue lease fields disagree with the primary state",
        ));
    }
    if !leased && (job.attempt_id.is_some() || job.worker_id.is_some() || job.lease_until.is_some())
    {
        return Err(queue_internal(
            "terminal or ready durable queue primary retains lease fields",
        ));
    }
    Ok(())
}

fn push_text(output: &mut Vec<u8>, value: &str) -> Result<(), ExecutionError> {
    push_blob(output, value.as_bytes())
}

fn push_blob(output: &mut Vec<u8>, value: &[u8]) -> Result<(), ExecutionError> {
    let length = u32::try_from(value.len())
        .map_err(|_| queue_internal("durable queue field length exceeds u32"))?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

fn push_optional_text(output: &mut Vec<u8>, value: Option<&str>) -> Result<(), ExecutionError> {
    match value {
        Some(value) => {
            output.push(1);
            push_text(output, value)
        }
        None => {
            output.push(0);
            Ok(())
        }
    }
}

fn push_optional_blob(output: &mut Vec<u8>, value: Option<&[u8]>) -> Result<(), ExecutionError> {
    match value {
        Some(value) => {
            output.push(1);
            push_blob(output, value)
        }
        None => {
            output.push(0);
            Ok(())
        }
    }
}

fn push_optional_i64(output: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            output.push(1);
            output.extend_from_slice(&value.to_be_bytes());
        }
        None => output.push(0),
    }
}

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ExecutionError> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or_else(|| queue_internal("durable queue decode offset overflowed"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| queue_internal("durable queue primary record is truncated"))?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, ExecutionError> {
        self.take(1)?
            .first()
            .copied()
            .ok_or_else(|| queue_internal("durable queue primary record is truncated"))
    }

    fn u32(&mut self) -> Result<u32, ExecutionError> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i64(&mut self) -> Result<i64, ExecutionError> {
        let bytes = self.take(8)?;
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn blob(&mut self, maximum: usize) -> Result<Vec<u8>, ExecutionError> {
        let length = usize::try_from(self.u32()?)
            .map_err(|_| queue_internal("durable queue field length is unsupported"))?;
        if length > maximum {
            return Err(queue_internal(
                "durable queue primary field exceeds its exact byte limit",
            ));
        }
        Ok(self.take(length)?.to_vec())
    }

    fn text(&mut self, maximum: usize) -> Result<String, ExecutionError> {
        String::from_utf8(self.blob(maximum)?)
            .map_err(|_| queue_internal("durable queue primary text is not UTF-8"))
    }

    fn optional_text(&mut self, maximum: usize) -> Result<Option<String>, ExecutionError> {
        match self.u8()? {
            0 => Ok(None),
            1 => self.text(maximum).map(Some),
            _ => Err(queue_internal(
                "durable queue primary has a noncanonical option tag",
            )),
        }
    }

    fn optional_blob(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ExecutionError> {
        match self.u8()? {
            0 => Ok(None),
            1 => self.blob(maximum).map(Some),
            _ => Err(queue_internal(
                "durable queue primary has a noncanonical option tag",
            )),
        }
    }

    fn optional_i64(&mut self) -> Result<Option<i64>, ExecutionError> {
        match self.u8()? {
            0 => Ok(None),
            1 => self.i64().map(Some),
            _ => Err(queue_internal(
                "durable queue primary has a noncanonical option tag",
            )),
        }
    }

    fn finish(self) -> Result<(), ExecutionError> {
        if self.offset != self.bytes.len() {
            return Err(queue_internal(
                "durable queue primary record contains trailing bytes",
            ));
        }
        Ok(())
    }
}

fn digest(domain: &'static str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn map_data_error(error: Diagnostic, possible_visibility: bool) -> ExecutionError {
    let class = match error.class {
        DiagnosticClass::Resource => ExecutionFailureClass::Resource,
        DiagnosticClass::Cancelled => ExecutionFailureClass::Cancelled,
        DiagnosticClass::Source | DiagnosticClass::Semantic | DiagnosticClass::Capability => {
            ExecutionFailureClass::Capability
        }
        DiagnosticClass::Corrupt | DiagnosticClass::Infrastructure
            if possible_visibility && error.code.contains("unknown") =>
        {
            ExecutionFailureClass::PossibleVisibility
        }
        DiagnosticClass::Corrupt | DiagnosticClass::Infrastructure => {
            ExecutionFailureClass::Infrastructure
        }
    };
    ExecutionError::new(class, error.code, error.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn queue() -> (TempDir, DataQueue) {
        let temporary = TempDir::new().expect("temporary queue root");
        let root = temporary.path().join("data");
        DataStore::initialize(&root).expect("initialize data root");
        let store = DataStore::open(&root, "queue-test", Default::default()).expect("open store");
        (temporary, DataQueue::new(store, QueueLimits::default()))
    }

    #[test]
    fn durable_queue_reopens_and_rejects_stale_attempt() {
        let (temporary, queue) = queue();
        let control = ExecutionControl::uncancelled();
        queue.initialize(&control, true).expect("initialize queue");
        assert!(
            queue
                .enqueue("job-1", "key-1", b"payload", 0, 0, &control, true)
                .expect("enqueue")
        );
        let first = queue
            .claim("worker-1", 0, 10, 100, &control, true)
            .expect("claim")
            .expect("lease");
        let root = temporary.path().join("data");
        let reopened_store =
            DataStore::open(&root, "queue-test", Default::default()).expect("reopen store");
        let reopened = DataQueue::new(reopened_store, QueueLimits::default());
        let second = reopened
            .claim("worker-2", 10, 10, 100, &control, true)
            .expect("reclaim")
            .expect("second lease");
        assert_eq!(second.attempt_number, 2);
        assert!(
            !reopened
                .complete(
                    &first.job_id,
                    &first.attempt_id,
                    "worker-1",
                    10,
                    b"stale",
                    &control,
                    true,
                )
                .expect("stale completion")
        );
        assert!(
            reopened
                .complete(
                    &second.job_id,
                    &second.attempt_id,
                    "worker-2",
                    11,
                    b"done",
                    &control,
                    true,
                )
                .expect("complete")
        );
        assert_eq!(
            reopened
                .inspect("job-1", &control)
                .expect("inspect")
                .expect("job")
                .result,
            Some(b"done".to_vec())
        );
    }

    #[test]
    fn job_codec_is_canonical_and_strict() {
        let job = StoredJob {
            job_id: "job".to_owned(),
            idempotency_key: "key".to_owned(),
            payload: vec![1, 2],
            state: JobState::Ready,
            available_at: 1,
            created_at: 0,
            attempt_count: 0,
            attempt_id: None,
            worker_id: None,
            lease_until: None,
            result: None,
            last_error_class: None,
        };
        let encoded = encode_job(&job).expect("encode");
        assert_eq!(
            decode_job(&encoded, &QueueLimits::default()).expect("decode"),
            job
        );
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(decode_job(&trailing, &QueueLimits::default()).is_err());
        let mut corrupt = encoded;
        corrupt[8] ^= 1;
        assert!(decode_job(&corrupt, &QueueLimits::default()).is_err());
    }
}
