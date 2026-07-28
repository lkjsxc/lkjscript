use super::*;

mod noop;
pub(super) use noop::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeInvocationConfig {
    pub(super) poll_fuel: u64,
    pub(super) wall_time: Option<Duration>,
    pub(super) max_active_frames: usize,
    pub(super) max_active_values: usize,
    pub(super) max_native_stack_bytes: usize,
    pub(super) max_native_frame_bytes: usize,
}

impl NativeInvocationConfig {
    #[must_use]
    pub const fn new(poll_fuel: u64, wall_time: Option<Duration>) -> Self {
        Self {
            poll_fuel,
            wall_time,
            max_active_frames: MAX_ACTIVE_FRAMES,
            max_active_values: usize::MAX,
            max_native_stack_bytes: DEFAULT_MAX_NATIVE_STACK_BYTES,
            max_native_frame_bytes: DEFAULT_MAX_NATIVE_FRAME_BYTES,
        }
    }

    #[must_use]
    pub const fn with_max_active_frames(mut self, maximum: usize) -> Self {
        self.max_active_frames = maximum;
        self
    }

    #[must_use]
    pub const fn with_max_active_values(mut self, maximum: usize) -> Self {
        self.max_active_values = maximum;
        self
    }

    #[must_use]
    pub const fn with_native_stack_limits(
        mut self,
        maximum_aggregate_bytes: usize,
        maximum_frame_bytes: usize,
    ) -> Self {
        self.max_native_stack_bytes = maximum_aggregate_bytes;
        self.max_native_frame_bytes = maximum_frame_bytes;
        self
    }
}

impl Default for NativeInvocationConfig {
    fn default() -> Self {
        Self::new(u64::MAX, None)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeEntryCount {
    pub(super) source_function: u32,
    pub(super) entries: u64,
}

impl NativeEntryCount {
    #[must_use]
    pub const fn source_function(self) -> u32 {
        self.source_function
    }

    #[must_use]
    pub const fn entries(self) -> u64 {
        self.entries
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeRoot {
    pub(super) reference_type: ReferenceType,
    pub(super) opaque_word: u64,
}

impl NativeRoot {
    #[must_use]
    pub const fn reference_type(self) -> ReferenceType {
        self.reference_type
    }

    #[must_use]
    pub const fn opaque_word(self) -> u64 {
        self.opaque_word
    }

    pub fn set_opaque_word(&mut self, opaque_word: u64) {
        self.opaque_word = opaque_word;
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeServiceError {
    Trap,
    ResourceLimitExceeded,
    HostFailure,
}

/// Safe runtime boundary. Implementations receive only copied typed values and
/// roots; frame addresses and stack traversal remain private to this crate.
pub trait NativeIslandRuntimeServices {
    fn borrow_standard_input(&mut self) -> Result<NativeResource, NativeServiceError>;
    fn new_byte_vector(&mut self, size: i64) -> Result<NativeUnique, NativeServiceError>;
    fn move_byte_vector(&mut self, owner: NativeUnique)
        -> Result<NativeUnique, NativeServiceError>;
    fn borrow_byte_vector(
        &mut self,
        owner: NativeUnique,
        kind: LoanType,
    ) -> Result<NativeLoan, NativeServiceError>;
    fn byte_slice_length(&mut self, loan: NativeLoan) -> Result<i64, NativeServiceError>;
    fn byte_slice_byte_at(
        &mut self,
        loan: NativeLoan,
        index: i64,
    ) -> Result<i64, NativeServiceError>;
    fn byte_slice_read_u32_little_endian(
        &mut self,
        loan: NativeLoan,
        index: i64,
    ) -> Result<i64, NativeServiceError>;
    fn byte_slice_mut_set_byte(
        &mut self,
        loan: NativeLoan,
        index: i64,
        byte: i64,
    ) -> Result<(), NativeServiceError>;
    fn byte_slice_mut_write_u32_little_endian(
        &mut self,
        loan: NativeLoan,
        index: i64,
        word: i64,
    ) -> Result<(), NativeServiceError>;
    fn end_byte_vector_borrow(&mut self, loan: NativeLoan) -> Result<(), NativeServiceError>;
    fn drop_byte_vector(&mut self, owner: NativeUnique) -> Result<(), NativeServiceError>;
    fn clone_static_bytes(&mut self, bytes: &[u8]) -> Result<NativeUnique, NativeServiceError>;
    fn copy_static_bytes_slice(
        &mut self,
        bytes: &[u8],
        start: i64,
        len: i64,
    ) -> Result<NativeUnique, NativeServiceError>;
    fn thaw_static_bytes(&mut self, bytes: &[u8]) -> Result<NativeUnique, NativeServiceError>;
    fn move_bytes(&mut self, owner: NativeUnique) -> Result<NativeUnique, NativeServiceError>;
    fn borrow_bytes(&mut self, owner: NativeUnique) -> Result<NativeLoan, NativeServiceError>;
    fn bytes_length(&mut self, loan: NativeLoan) -> Result<i64, NativeServiceError>;
    fn bytes_byte_at(&mut self, loan: NativeLoan, index: i64) -> Result<i64, NativeServiceError>;
    fn clone_bytes(&mut self, loan: NativeLoan) -> Result<NativeUnique, NativeServiceError>;
    fn copy_bytes_slice(
        &mut self,
        loan: NativeLoan,
        start: i64,
        len: i64,
    ) -> Result<NativeUnique, NativeServiceError>;
    fn end_bytes_borrow(&mut self, loan: NativeLoan) -> Result<(), NativeServiceError>;
    fn drop_bytes(&mut self, owner: NativeUnique) -> Result<(), NativeServiceError>;
    fn freeze_byte_vector(
        &mut self,
        owner: NativeUnique,
    ) -> Result<NativeUnique, NativeServiceError>;
    fn thaw_bytes(&mut self, owner: NativeUnique) -> Result<NativeUnique, NativeServiceError>;
}

pub trait NativeRuntimeServices {
    fn collect_references(&mut self, roots: &mut [NativeRoot]) -> Result<(), NativeServiceError>;

    /// Optionally collect for a verified site. Sys writes any updated roots
    /// back to generated homes before calling `heap_operation`.
    fn prepare_heap_operation(
        &mut self,
        _site: &HeapRuntimeSite,
        _arguments: &[NativeValue],
        _roots: &mut [NativeRoot],
    ) -> Result<bool, NativeServiceError> {
        Ok(false)
    }

    fn heap_operation(
        &mut self,
        _site: &HeapRuntimeSite,
        _arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        Err(NativeServiceError::HostFailure)
    }
}
