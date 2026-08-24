//! Task-scoped byte streams with bounded buffering, backpressure, cancellation, and exact close.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{
    CallPolicy, CapabilityAdapter, ExecutionControl, ExecutionError, ExecutionFailureClass,
};
use super::semantic::OwnerId;
use super::value::{ResourceId, ResourceKind, Value};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::Duration;
use tokio::sync::Notify;

pub const STREAM_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_STREAM_CHUNK_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_STREAM_BUFFERED_CHUNKS: usize = 1_024;
pub const MAXIMUM_LIVE_STREAMS: usize = 65_536;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StreamLimits {
    pub maximum_chunk_bytes: usize,
    pub maximum_buffered_chunks: usize,
    pub maximum_total_bytes: u64,
    pub maximum_live_streams: usize,
}

impl Default for StreamLimits {
    fn default() -> Self {
        Self {
            maximum_chunk_bytes: 64 * 1024,
            maximum_buffered_chunks: 8,
            maximum_total_bytes: 64 * 1024 * 1024,
            maximum_live_streams: 1024,
        }
    }
}

impl StreamLimits {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.maximum_chunk_bytes == 0 || self.maximum_chunk_bytes > MAXIMUM_STREAM_CHUNK_BYTES {
            return Err(stream_diagnostic(
                "stream_chunk_limit",
                format!("maximum_chunk_bytes must be 1 through {MAXIMUM_STREAM_CHUNK_BYTES}"),
            ));
        }
        if self.maximum_buffered_chunks == 0
            || self.maximum_buffered_chunks > MAXIMUM_STREAM_BUFFERED_CHUNKS
        {
            return Err(stream_diagnostic(
                "stream_buffer_limit",
                format!(
                    "maximum_buffered_chunks must be 1 through {MAXIMUM_STREAM_BUFFERED_CHUNKS}"
                ),
            ));
        }
        if self.maximum_total_bytes == 0 {
            return Err(stream_diagnostic(
                "stream_total_limit",
                "maximum_total_bytes must be positive",
            ));
        }
        if self.maximum_live_streams == 0 || self.maximum_live_streams > MAXIMUM_LIVE_STREAMS {
            return Err(stream_diagnostic(
                "stream_live_limit",
                format!("maximum_live_streams must be 1 through {MAXIMUM_LIVE_STREAMS}"),
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct StreamRegistry {
    inner: Arc<RegistryInner>,
}

struct RegistryInner {
    limits: StreamLimits,
    next: AtomicU64,
    sources: Mutex<SourceMap>,
}

type SharedSource = Arc<Mutex<Box<dyn ByteSource>>>;
type SourceMap = BTreeMap<ResourceId, SharedSource>;

trait ByteSource: Send {
    fn read(&mut self, control: &ExecutionControl) -> Result<Option<Vec<u8>>, ExecutionError>;
    fn close(&mut self);
}

impl StreamRegistry {
    pub fn new(limits: StreamLimits) -> Result<Self, Diagnostic> {
        limits.validate()?;
        Ok(Self {
            inner: Arc::new(RegistryInner {
                limits,
                next: AtomicU64::new(1),
                sources: Mutex::new(BTreeMap::new()),
            }),
        })
    }

    pub fn limits(&self) -> &StreamLimits {
        &self.inner.limits
    }

    pub fn live_streams(&self) -> usize {
        lock_unpoisoned(&self.inner.sources).len()
    }

    pub fn register_memory(&self, bytes: Vec<u8>) -> Result<StreamLease, ExecutionError> {
        if u64::try_from(bytes.len()).map_or(true, |length| {
            length > self.inner.limits.maximum_total_bytes
        }) {
            return Err(ExecutionError::resource(
                "stream_total_limit",
                "memory stream exceeds its total byte limit",
            ));
        }
        let chunks = bytes
            .chunks(self.inner.limits.maximum_chunk_bytes)
            .map(<[u8]>::to_vec)
            .collect();
        self.insert(Box::new(MemorySource { chunks }))
    }

    pub fn register_pipe(&self) -> Result<(StreamLease, ByteStreamProducer), ExecutionError> {
        self.register_pipe_with_limit(self.inner.limits.maximum_total_bytes)
    }

    pub fn register_pipe_with_limit(
        &self,
        maximum_total_bytes: u64,
    ) -> Result<(StreamLease, ByteStreamProducer), ExecutionError> {
        if maximum_total_bytes == 0 || maximum_total_bytes > self.inner.limits.maximum_total_bytes {
            return Err(ExecutionError::resource(
                "stream_total_limit",
                "per-stream byte limit must be positive and no larger than its registry limit",
            ));
        }
        let mut limits = self.inner.limits.clone();
        limits.maximum_total_bytes = maximum_total_bytes;
        let pipe = Arc::new(Pipe {
            limits,
            state: Mutex::new(PipeState::default()),
            data: Condvar::new(),
            capacity: Notify::new(),
        });
        let lease = self.insert(Box::new(PipeSource { pipe: pipe.clone() }))?;
        Ok((lease, ByteStreamProducer { pipe }))
    }

    pub fn read(
        &self,
        value: &Value,
        control: &ExecutionControl,
    ) -> Result<Option<Vec<u8>>, ExecutionError> {
        self.read_id(stream_id(value)?, control)
    }

    fn read_id(
        &self,
        id: ResourceId,
        control: &ExecutionControl,
    ) -> Result<Option<Vec<u8>>, ExecutionError> {
        let source = lock_unpoisoned(&self.inner.sources)
            .get(&id)
            .cloned()
            .ok_or_else(|| {
                ExecutionError::new(
                    ExecutionFailureClass::Infrastructure,
                    "stream_closed",
                    "byte stream is closed or foreign to this task scope",
                )
            })?;
        lock_unpoisoned(&source).read(control)
    }

    pub fn close(&self, value: &Value) -> Result<(), ExecutionError> {
        self.close_id(stream_id(value)?);
        Ok(())
    }

    pub fn read_all(
        &self,
        value: &Value,
        maximum_bytes: usize,
        control: &ExecutionControl,
    ) -> Result<Vec<u8>, ExecutionError> {
        self.read_all_id(stream_id(value)?, maximum_bytes, control)
    }

    fn read_all_id(
        &self,
        id: ResourceId,
        maximum_bytes: usize,
        control: &ExecutionControl,
    ) -> Result<Vec<u8>, ExecutionError> {
        let maximum = u64::try_from(maximum_bytes).map_err(|_| {
            ExecutionError::resource(
                "stream_read_all_limit",
                "whole-stream byte limit is not representable",
            )
        })?;
        if maximum == 0 || maximum > self.inner.limits.maximum_total_bytes {
            return Err(ExecutionError::resource(
                "stream_read_all_limit",
                "whole-stream byte limit must be positive and no larger than its registry limit",
            ));
        }
        let outcome = (|| {
            let mut output = Vec::new();
            while let Some(chunk) = self.read_id(id, control)? {
                let next = output.len().checked_add(chunk.len()).ok_or_else(|| {
                    ExecutionError::resource(
                        "stream_read_all_limit",
                        "whole-stream byte accounting overflowed",
                    )
                })?;
                if next > maximum_bytes {
                    return Err(ExecutionError::resource(
                        "stream_read_all_limit",
                        "stream exceeds the requested whole-value byte limit",
                    ));
                }
                output.extend_from_slice(&chunk);
            }
            Ok(output)
        })();
        self.close_id(id);
        outcome
    }

    fn insert(&self, source: Box<dyn ByteSource>) -> Result<StreamLease, ExecutionError> {
        let mut sources = lock_unpoisoned(&self.inner.sources);
        if sources.len() >= self.inner.limits.maximum_live_streams {
            return Err(ExecutionError::resource(
                "stream_live_limit",
                "live stream registry is full",
            ));
        }
        let raw = self.inner.next.fetch_add(1, Ordering::AcqRel);
        if raw == 0 || raw == u64::MAX {
            return Err(ExecutionError::resource(
                "stream_identity_exhausted",
                "stream identity domain is exhausted",
            ));
        }
        let id = ResourceId(raw);
        if sources.insert(id, Arc::new(Mutex::new(source))).is_some() {
            return Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "stream_identity_reused",
                "stream identity was unexpectedly reused",
            ));
        }
        Ok(StreamLease {
            registry: self.clone(),
            id: Some(id),
        })
    }

    fn close_id(&self, id: ResourceId) {
        if let Some(source) = lock_unpoisoned(&self.inner.sources).remove(&id) {
            lock_unpoisoned(&source).close();
        }
    }
}

pub struct StreamLease {
    registry: StreamRegistry,
    id: Option<ResourceId>,
}

impl StreamLease {
    pub fn value(&self) -> Value {
        Value::Resource {
            id: self.id.unwrap_or(ResourceId(0)),
            kind: ResourceKind::ByteStream,
        }
    }

    pub fn close(mut self) {
        if let Some(id) = self.id.take() {
            self.registry.close_id(id);
        }
    }

    pub(crate) fn read(
        &self,
        control: &ExecutionControl,
    ) -> Result<Option<Vec<u8>>, ExecutionError> {
        self.registry.read_id(self.live_id()?, control)
    }

    pub(crate) fn read_all(
        &self,
        maximum_bytes: usize,
        control: &ExecutionControl,
    ) -> Result<Vec<u8>, ExecutionError> {
        self.registry
            .read_all_id(self.live_id()?, maximum_bytes, control)
    }

    pub(crate) fn close_registered(&self) {
        if let Some(id) = self.id {
            self.registry.close_id(id);
        }
    }

    fn live_id(&self) -> Result<ResourceId, ExecutionError> {
        self.id.ok_or_else(|| {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "stream_closed",
                "byte stream lease is already closed",
            )
        })
    }
}

impl Drop for StreamLease {
    fn drop(&mut self) {
        if let Some(id) = self.id.take() {
            self.registry.close_id(id);
        }
    }
}

#[derive(Clone)]
pub struct ByteStreamProducer {
    pipe: Arc<Pipe>,
}

struct Pipe {
    limits: StreamLimits,
    state: Mutex<PipeState>,
    data: Condvar,
    capacity: Notify,
}

#[derive(Default)]
struct PipeState {
    queue: VecDeque<Vec<u8>>,
    total: u64,
    closed: bool,
    failure: Option<ExecutionError>,
}

impl ByteStreamProducer {
    pub async fn push(&self, bytes: Vec<u8>) -> Result<(), ExecutionError> {
        if bytes.is_empty() || bytes.len() > self.pipe.limits.maximum_chunk_bytes {
            return Err(ExecutionError::resource(
                "stream_chunk_limit",
                "stream chunk is empty or exceeds its byte limit",
            ));
        }
        loop {
            let notified = self.pipe.capacity.notified();
            {
                let mut state = lock_unpoisoned(&self.pipe.state);
                if state.closed {
                    return Err(ExecutionError::new(
                        ExecutionFailureClass::Cancelled,
                        "stream_consumer_closed",
                        "stream consumer closed before producer completion",
                    ));
                }
                if state.queue.len() < self.pipe.limits.maximum_buffered_chunks {
                    let length = u64::try_from(bytes.len()).map_err(|_| {
                        ExecutionError::resource(
                            "stream_total_limit",
                            "stream chunk length cannot be represented",
                        )
                    })?;
                    let total = state.total.checked_add(length).ok_or_else(|| {
                        ExecutionError::resource(
                            "stream_total_limit",
                            "stream byte accounting overflowed",
                        )
                    })?;
                    if total > self.pipe.limits.maximum_total_bytes {
                        let error = ExecutionError::resource(
                            "stream_total_limit",
                            "stream exceeds its total byte limit",
                        );
                        state.failure = Some(error.clone());
                        state.closed = true;
                        self.pipe.data.notify_all();
                        return Err(error);
                    }
                    state.total = total;
                    state.queue.push_back(bytes);
                    self.pipe.data.notify_one();
                    return Ok(());
                }
            }
            notified.await;
        }
    }

    pub fn finish(&self) {
        let mut state = lock_unpoisoned(&self.pipe.state);
        state.closed = true;
        self.pipe.data.notify_all();
    }

    pub fn fail(&self, error: ExecutionError) {
        let mut state = lock_unpoisoned(&self.pipe.state);
        if state.failure.is_none() {
            state.failure = Some(error);
        }
        state.closed = true;
        self.pipe.data.notify_all();
    }
}

struct MemorySource {
    chunks: VecDeque<Vec<u8>>,
}

impl ByteSource for MemorySource {
    fn read(&mut self, control: &ExecutionControl) -> Result<Option<Vec<u8>>, ExecutionError> {
        control.check()?;
        Ok(self.chunks.pop_front())
    }

    fn close(&mut self) {
        self.chunks.clear();
    }
}

struct PipeSource {
    pipe: Arc<Pipe>,
}

impl ByteSource for PipeSource {
    fn read(&mut self, control: &ExecutionControl) -> Result<Option<Vec<u8>>, ExecutionError> {
        loop {
            control.check()?;
            let mut state = lock_unpoisoned(&self.pipe.state);
            if let Some(bytes) = state.queue.pop_front() {
                self.pipe.capacity.notify_waiters();
                return Ok(Some(bytes));
            }
            if let Some(error) = state.failure.take() {
                return Err(error);
            }
            if state.closed {
                return Ok(None);
            }
            let (next, _) = wait_unpoisoned(&self.pipe.data, state, Duration::from_millis(10));
            state = next;
            drop(state);
        }
    }

    fn close(&mut self) {
        let mut state = lock_unpoisoned(&self.pipe.state);
        state.closed = true;
        state.queue.clear();
        self.pipe.capacity.notify_waiters();
        self.pipe.data.notify_all();
    }
}

#[derive(Clone, Debug)]
pub struct ByteStreamAdapter {
    interface: OwnerId,
    registry: StreamRegistry,
}

impl ByteStreamAdapter {
    pub fn new(interface: OwnerId, registry: StreamRegistry) -> Self {
        Self {
            interface,
            registry,
        }
    }
}

impl std::fmt::Debug for StreamRegistry {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StreamRegistry")
            .field("limits", &self.inner.limits)
            .field("live_streams", &self.live_streams())
            .finish()
    }
}

impl CapabilityAdapter for ByteStreamAdapter {
    fn interface(&self) -> &OwnerId {
        &self.interface
    }

    fn call(&self, policy: &CallPolicy, arguments: Vec<Value>) -> Result<Value, ExecutionError> {
        match policy.operation.as_str() {
            "read" => {
                let [stream] = arguments.as_slice() else {
                    return Err(stream_argument("stream read expects one stream"));
                };
                match self.registry.read(stream, &policy.control)? {
                    Some(chunk) => Ok(Value::record(
                        None,
                        [
                            ("done".to_owned(), Value::Bool(false)),
                            ("chunk".to_owned(), Value::bytes(chunk)),
                        ],
                    )),
                    None => Ok(Value::record(
                        None,
                        [
                            ("done".to_owned(), Value::Bool(true)),
                            ("chunk".to_owned(), Value::bytes(Vec::<u8>::new())),
                        ],
                    )),
                }
            }
            "close" => {
                let [stream] = arguments.as_slice() else {
                    return Err(stream_argument("stream close expects one stream"));
                };
                self.registry.close(stream)?;
                Ok(Value::Unit)
            }
            "read-all" => {
                let [stream, Value::I64(maximum_bytes)] = arguments.as_slice() else {
                    return Err(stream_argument(
                        "stream read-all expects a stream and positive I64 byte limit",
                    ));
                };
                let maximum_bytes = usize::try_from(*maximum_bytes).map_err(|_| {
                    ExecutionError::resource(
                        "stream_read_all_limit",
                        "whole-stream byte limit must be a positive platform-sized integer",
                    )
                })?;
                self.registry
                    .read_all(stream, maximum_bytes, &policy.control)
                    .map(Value::bytes)
            }
            operation => Err(ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "stream_operation_unknown",
                format!("byte-stream adapter does not implement '{operation}'"),
            )),
        }
    }
}

fn stream_id(value: &Value) -> Result<ResourceId, ExecutionError> {
    match value {
        Value::Resource {
            id,
            kind: ResourceKind::ByteStream,
        } => Ok(*id),
        _ => Err(stream_argument("value is not a task-scoped byte stream")),
    }
}

fn stream_argument(message: impl Into<String>) -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "stream_adapter_argument",
        message,
    )
}

fn stream_diagnostic(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, code, message)
}

fn lock_unpoisoned<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn wait_unpoisoned<'a, T>(
    condition: &Condvar,
    guard: MutexGuard<'a, T>,
    duration: Duration,
) -> (MutexGuard<'a, T>, std::sync::WaitTimeoutResult) {
    match condition.wait_timeout(guard, duration) {
        Ok(result) => result,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pipe_backpressure_and_close_are_bounded() {
        let registry = StreamRegistry::new(StreamLimits {
            maximum_chunk_bytes: 4,
            maximum_buffered_chunks: 1,
            maximum_total_bytes: 8,
            maximum_live_streams: 1,
        })
        .expect("registry");
        let (lease, producer) = registry.register_pipe().expect("pipe");
        producer.push(vec![1, 2, 3, 4]).await.expect("first");
        let value = lease.value();
        assert_eq!(
            registry
                .read(&value, &ExecutionControl::uncancelled())
                .expect("read"),
            Some(vec![1, 2, 3, 4])
        );
        producer.finish();
        assert_eq!(
            registry
                .read(&value, &ExecutionControl::uncancelled())
                .expect("end"),
            None
        );
        drop(lease);
        assert_eq!(registry.live_streams(), 0);
    }

    #[test]
    fn memory_stream_chunks_without_serializable_handles() {
        let registry = StreamRegistry::new(StreamLimits {
            maximum_chunk_bytes: 2,
            maximum_buffered_chunks: 1,
            maximum_total_bytes: 8,
            maximum_live_streams: 1,
        })
        .expect("registry");
        let lease = registry
            .register_memory(vec![1, 2, 3, 4, 5])
            .expect("memory stream");
        let value = lease.value();
        assert!(!value.is_durable());
        let control = ExecutionControl::uncancelled();
        assert_eq!(
            registry.read(&value, &control).expect("one"),
            Some(vec![1, 2])
        );
        assert_eq!(
            registry.read(&value, &control).expect("two"),
            Some(vec![3, 4])
        );
        assert_eq!(
            registry.read(&value, &control).expect("three"),
            Some(vec![5])
        );
        assert_eq!(registry.read(&value, &control).expect("end"), None);
    }

    #[test]
    fn whole_value_convenience_closes_on_success_and_limit_failure() {
        let registry = StreamRegistry::new(StreamLimits {
            maximum_chunk_bytes: 2,
            maximum_buffered_chunks: 1,
            maximum_total_bytes: 8,
            maximum_live_streams: 1,
        })
        .expect("registry");
        let lease = registry
            .register_memory(vec![1, 2, 3, 4, 5])
            .expect("memory stream");
        assert_eq!(
            registry
                .read_all(&lease.value(), 5, &ExecutionControl::uncancelled())
                .expect("whole value"),
            vec![1, 2, 3, 4, 5]
        );
        assert_eq!(registry.live_streams(), 0);

        let lease = registry
            .register_memory(vec![1, 2, 3, 4, 5])
            .expect("memory stream");
        let error = registry
            .read_all(&lease.value(), 4, &ExecutionControl::uncancelled())
            .expect_err("limit must reject");
        assert_eq!(error.code, "stream_read_all_limit");
        assert_eq!(registry.live_streams(), 0);
    }
}
