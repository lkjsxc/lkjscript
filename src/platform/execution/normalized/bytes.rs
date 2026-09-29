//! Immutable byte values with reusable, privately owned concatenation storage.
//! Storage is derived: equality, ordering and encoders only observe the byte slice.
use crate::platform::execution::{ExecutionControl, ExecutionError};
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::Arc;

#[cfg(test)]
#[path = "bytes_storage_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct BytePayload(Storage);

#[derive(Clone)]
enum Storage {
    Shared(Arc<[u8]>),
    Buffer(Arc<Vec<u8>>),
}

/// Production checked-concatenation work, not allocator internals, RSS, or a quota.
/// The independent reference does not instrument this optimization; its zeros are not
/// evidence that it performs no copies.
#[derive(Clone, Debug, Default, Eq, PartialEq, serde::Serialize)]
pub(crate) struct Work {
    pub concatenations: u64,
    pub empty_operand_reuses: u64,
    pub in_place_appends: u64,
    pub buffer_growths: u64,
    pub fresh_buffers: u64,
    pub payload_bytes_copied: u64,
    pub requested_capacity_bytes: u64,
}

impl BytePayload {
    /// The owned vector descriptor is a separate allocation from its payload.
    /// Like existing scalar charges, this excludes Arc counters/allocator overhead.
    pub(crate) const DESCRIPTOR_BYTES: u64 = std::mem::size_of::<Vec<u8>>() as u64;

    pub(crate) fn concat(
        mut self,
        right: Self,
        maximum: u64,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
        work: &mut Work,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        let length = self.len().checked_add(right.len()).ok_or_else(|| {
            ExecutionError::resource(
                "normalized_bytes_length",
                "byte concatenation length overflowed",
            )
        })?;
        if length as u64 > maximum {
            return Err(ExecutionError::resource(
                "normalized_allocation",
                "byte result exceeds finite value storage",
            ));
        }
        work.concatenations = work.concatenations.saturating_add(1);
        if self.is_empty() || right.is_empty() {
            work.empty_operand_reuses = work.empty_operand_reuses.saturating_add(1);
            return Ok(if self.is_empty() { right } else { self });
        }
        if let Storage::Buffer(shared) = &mut self.0
            && let Some(buffer) = Arc::get_mut(shared)
        {
            // get_mut admits neither a second strong owner nor an outstanding Weak.
            // No mutable view escapes this synchronous operation.
            if length <= buffer.capacity() {
                append(buffer, &right, control, work)?;
                work.in_place_appends = work.in_place_appends.saturating_add(1);
            } else {
                let maximum = usize::try_from(maximum).unwrap_or(usize::MAX);
                let capacity = buffer.capacity().saturating_mul(2).max(length).min(maximum);
                reserve(capacity as u64)?;
                let mut grown = allocate(capacity, work)?;
                work.buffer_growths = work.buffer_growths.saturating_add(1);
                append(&mut grown, buffer, control, work)?;
                append(&mut grown, &right, control, work)?;
                *buffer = grown;
            }
            return Ok(self);
        }
        // A retained prefix (including a map key) remains immutable. Initial storage
        // is exact-sized; only a subsequently unique growing buffer gets spare space.
        reserve(Self::DESCRIPTOR_BYTES)?;
        reserve(length as u64)?;
        let mut buffer = allocate(length, work)?;
        work.fresh_buffers = work.fresh_buffers.saturating_add(1);
        append(&mut buffer, &self, control, work)?;
        append(&mut buffer, &right, control, work)?;
        control.check()?;
        Ok(Self(Storage::Buffer(Arc::new(buffer))))
    }
}

fn allocate(capacity: usize, work: &mut Work) -> Result<Vec<u8>, ExecutionError> {
    work.requested_capacity_bytes = work
        .requested_capacity_bytes
        .saturating_add(capacity as u64);
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(capacity).map_err(|_| {
        ExecutionError::resource(
            "normalized_bytes_storage",
            "cannot allocate admitted byte buffer",
        )
    })?;
    // The counter records the requested capacity, not allocator size-class rounding.
    Ok(buffer)
}

fn append(
    output: &mut Vec<u8>,
    input: &[u8],
    control: &ExecutionControl,
    work: &mut Work,
) -> Result<(), ExecutionError> {
    for chunk in input.chunks(65_536) {
        control.check()?;
        output.extend_from_slice(chunk);
        work.payload_bytes_copied = work.payload_bytes_copied.saturating_add(chunk.len() as u64);
    }
    control.check()
}

impl AsRef<[u8]> for BytePayload {
    fn as_ref(&self) -> &[u8] {
        match &self.0 {
            Storage::Shared(bytes) => bytes,
            Storage::Buffer(bytes) => bytes,
        }
    }
}

impl Deref for BytePayload {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        self.as_ref()
    }
}

impl std::fmt::Debug for BytePayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.as_ref().fmt(formatter)
    }
}

impl PartialEq for BytePayload {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}
impl Eq for BytePayload {}
impl PartialOrd for BytePayload {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for BytePayload {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_ref().cmp(other.as_ref())
    }
}
impl From<Arc<[u8]>> for BytePayload {
    fn from(value: Arc<[u8]>) -> Self {
        Self(Storage::Shared(value))
    }
}
impl From<Vec<u8>> for BytePayload {
    fn from(value: Vec<u8>) -> Self {
        Self::from(Arc::<[u8]>::from(value))
    }
}
impl<const N: usize> From<[u8; N]> for BytePayload {
    fn from(value: [u8; N]) -> Self {
        Self::from(Arc::<[u8]>::from(value))
    }
}
impl From<&[u8]> for BytePayload {
    fn from(value: &[u8]) -> Self {
        Self::from(Arc::<[u8]>::from(value))
    }
}
