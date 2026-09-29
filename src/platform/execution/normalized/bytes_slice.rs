//! Flat, immutable byte windows and explicit detachment of retained storage.
use super::*;

#[cfg(test)]
#[path = "bytes_slice_storage_tests.rs"]
mod tests;

// A window owns one original backing, never another window. Nested slicing cannot
// create a descriptor chain or a recursively dropped ownership graph.
#[derive(Clone)]
enum Backing {
    Shared(Arc<[u8]>),
    Buffer(Arc<Vec<u8>>),
}

pub(super) struct Window {
    backing: Backing,
    start: usize,
    end: usize,
}

impl Window {
    pub(super) fn bytes(&self) -> &[u8] {
        let bytes: &[u8] = match &self.backing {
            Backing::Shared(bytes) => bytes,
            Backing::Buffer(bytes) => bytes,
        };
        &bytes[self.start..self.end]
    }
}

impl BytePayload {
    /// Arc counters/allocator metadata are excluded, as for existing byte buffers.
    pub(crate) const SLICE_DESCRIPTOR_BYTES: u64 = std::mem::size_of::<Window>() as u64;

    pub(crate) fn slice(
        mut self,
        start: i64,
        end: i64,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        let start = usize::try_from(start).map_err(|_| range_error())?;
        let end = usize::try_from(end).map_err(|_| range_error())?;
        if start > end || end > self.len() {
            return Err(range_error());
        }
        if start == end {
            // In particular, an empty view must not keep a large backing alive.
            return Ok(Self::from([]));
        }
        if start == 0 && end == self.len() {
            return Ok(self);
        }
        if let Storage::Window(shared) = &mut self.0
            && let Some(window) = Arc::get_mut(shared)
        {
            // Neither strong nor Weak aliases may observe metadata mutation.
            // Bounds above prove both sums stay within the original backing.
            window.end = window.start + end;
            window.start += start;
            control.check()?;
            return Ok(self);
        }
        reserve(Self::SLICE_DESCRIPTOR_BYTES)?;
        let (backing, offset) = match self.0 {
            Storage::Shared(bytes) => (Backing::Shared(bytes), 0),
            Storage::Buffer(bytes) => (Backing::Buffer(bytes), 0),
            Storage::Window(window) => (window.backing.clone(), window.start),
        };
        control.check()?;
        Ok(Self(Storage::Window(Arc::new(Window {
            backing,
            start: offset + start,
            end: offset + end,
        }))))
    }

    pub(crate) fn detached_copy(
        &self,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        control.check()?;
        if self.is_empty() {
            return Ok(Self::from([]));
        }
        // Reserve both allocations before making either one. The vector is moved
        // into its descriptor, not converted into a second copied payload.
        reserve(Self::DESCRIPTOR_BYTES)?;
        control.check()?;
        reserve(self.len() as u64)?;
        control.check()?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(self.len()).map_err(|_| {
            ExecutionError::resource(
                "normalized_bytes_storage",
                "cannot allocate admitted byte copy",
            )
        })?;
        for chunk in self.chunks(65_536) {
            control.check()?;
            bytes.extend_from_slice(chunk);
        }
        control.check()?;
        Ok(Self(Storage::Buffer(Arc::new(bytes))))
    }
}

fn range_error() -> ExecutionError {
    ExecutionError::new(
        crate::platform::execution::ExecutionFailureClass::Trap,
        "normalized_bytes_range",
        "byte range requires 0 <= start <= end <= byte length",
    )
}
