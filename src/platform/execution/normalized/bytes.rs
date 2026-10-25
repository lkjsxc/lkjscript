//! Immutable byte values with reusable, privately owned concatenation storage.
//! Storage is derived: equality, ordering and encoders only observe the byte slice.
use crate::platform::execution::{ExecutionControl, ExecutionError};
use std::cmp::Ordering;
use std::ops::Deref;
use std::sync::Arc;

#[path = "bytes_slice.rs"]
mod slices;

#[cfg(test)]
#[path = "bytes_storage_tests.rs"]
mod tests;

#[derive(Clone)]
pub struct BytePayload(Storage);

#[derive(Clone)]
enum Storage {
    Shared(Arc<[u8]>),
    Buffer(Arc<Vec<u8>>),
    Window(Arc<slices::Window>),
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
    /// At least one observation is a saturated lower bound.
    pub saturated: bool,
}

impl Work {
    pub(super) fn include(&mut self, child: &Self) {
        self.saturated |= child.saturated;
        macro_rules! add {
            ($($field:ident),* $(,)?) => {
                $(observe(&mut self.$field, child.$field, &mut self.saturated);)*
            };
        }
        add!(
            concatenations,
            empty_operand_reuses,
            in_place_appends,
            buffer_growths,
            fresh_buffers,
            payload_bytes_copied,
            requested_capacity_bytes
        );
    }
}

impl BytePayload {
    /// Explicit freeze boundary: retain the vector allocation, never copy its octets.
    pub(super) fn adopt_vec(bytes: Vec<u8>) -> Self {
        Self(Storage::Buffer(Arc::new(bytes)))
    }
    /// The owned vector descriptor is a separate allocation from its payload.
    /// Like existing scalar charges, this excludes Arc counters/allocator overhead.
    pub(crate) const DESCRIPTOR_BYTES: u64 = std::mem::size_of::<Vec<u8>>() as u64;

    pub(crate) fn concat(
        mut self,
        right: Self,
        maximum: Option<u64>,
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
        admit_length(length, maximum)?;
        observe(&mut work.concatenations, 1, &mut work.saturated);
        if self.is_empty() || right.is_empty() {
            observe(&mut work.empty_operand_reuses, 1, &mut work.saturated);
            return Ok(if self.is_empty() { right } else { self });
        }
        if let Storage::Buffer(shared) = &mut self.0
            && let Some(buffer) = Arc::get_mut(shared)
        {
            // get_mut admits neither a second strong owner nor an outstanding Weak.
            // No mutable view escapes this synchronous operation.
            if length <= buffer.capacity() {
                append(buffer, &right, control, work)?;
                observe(&mut work.in_place_appends, 1, &mut work.saturated);
            } else {
                let capacity = growth_capacity(buffer.capacity(), length, maximum)?;
                reserve(capacity as u64)?;
                control.check()?;
                let mut grown = allocate(capacity, work)?;
                observe(&mut work.buffer_growths, 1, &mut work.saturated);
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
        control.check()?;
        let mut buffer = allocate(length, work)?;
        observe(&mut work.fresh_buffers, 1, &mut work.saturated);
        append(&mut buffer, &self, control, work)?;
        append(&mut buffer, &right, control, work)?;
        control.check()?;
        Ok(Self(Storage::Buffer(Arc::new(buffer))))
    }
}

fn length_error() -> ExecutionError {
    ExecutionError::resource(
        "normalized_bytes_length",
        "byte length or allocation extent is not representable",
    )
}

fn admit_length(length: usize, maximum: Option<u64>) -> Result<(), ExecutionError> {
    if isize::try_from(length).is_err() || i64::try_from(length).is_err() {
        return Err(length_error());
    }
    if maximum.is_some_and(|maximum| length as u64 > maximum) {
        return Err(ExecutionError::resource(
            "normalized_allocation",
            "byte result exceeds its explicit value limit",
        ));
    }
    Ok(())
}

fn growth_capacity(
    current: usize,
    length: usize,
    maximum: Option<u64>,
) -> Result<usize, ExecutionError> {
    admit_length(length, maximum)?;
    let mut capacity = current
        .checked_mul(2)
        .filter(|capacity| isize::try_from(*capacity).is_ok())
        .filter(|capacity| i64::try_from(*capacity).is_ok())
        .unwrap_or(length)
        .max(length);
    if let Some(maximum) = maximum
        && capacity as u64 > maximum
    {
        // This conversion is exact: maximum is below an existing usize value.
        capacity = usize::try_from(maximum).map_err(|_| length_error())?;
    }
    Ok(capacity)
}

fn observe(counter: &mut u64, amount: u64, saturated: &mut bool) {
    let (value, overflow) = counter.overflowing_add(amount);
    if overflow {
        *counter = u64::MAX;
        *saturated = true;
    } else {
        *counter = value;
    }
}

fn allocate(capacity: usize, work: &mut Work) -> Result<Vec<u8>, ExecutionError> {
    observe(
        &mut work.requested_capacity_bytes,
        capacity as u64,
        &mut work.saturated,
    );
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
        observe(
            &mut work.payload_bytes_copied,
            chunk.len() as u64,
            &mut work.saturated,
        );
    }
    control.check()
}

impl AsRef<[u8]> for BytePayload {
    fn as_ref(&self) -> &[u8] {
        match &self.0 {
            Storage::Shared(bytes) => bytes,
            Storage::Buffer(bytes) => bytes,
            Storage::Window(window) => window.bytes(),
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

#[cfg(test)]
mod adoption_tests {
    use super::*;
    #[test]
    fn byte_buffer_vec_adoption_retains_nonempty_payload_allocation_and_capacity() {
        let mut vector = Vec::with_capacity(128);
        vector.extend([0, 255, 128]);
        let pointer = vector.as_ptr();
        let capacity = vector.capacity();
        let payload = BytePayload::adopt_vec(vector);
        assert_eq!(payload.as_ptr(), pointer);
        assert_eq!(&*payload, &[0, 255, 128]);
        let Storage::Buffer(backing) = &payload.0 else {
            panic!("freeze must adopt the Vec")
        };
        assert_eq!(backing.capacity(), capacity);
        let alias = payload.clone();
        assert_eq!(alias.as_ptr(), pointer);
    }
}


#[cfg(test)]
mod aggregation_tests {
    use super::*;

    #[test]
    fn byte_work_merge_preserves_child_saturation_and_records_sum_overflow() {
        let mut parent = Work { concatenations: u64::MAX - 1, ..Work::default() };
        let child = Work { concatenations: 1, ..Work::default() };
        parent.include(&child);
        assert_eq!(parent.concatenations, u64::MAX);
        assert!(!parent.saturated);
        parent.include(&child);
        assert_eq!(parent.concatenations, u64::MAX);
        assert!(parent.saturated);
        let mut parent = Work::default();
        parent.include(&Work { saturated: true, ..Work::default() });
        assert!(parent.saturated);
        parent.include(&Work::default());
        assert!(parent.saturated);
    }
}
