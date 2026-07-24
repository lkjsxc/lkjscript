use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;
use std::time::Duration;

use lkjscript_native::{
    AbiVersions, FunctionId, HeapRuntimeSite, ImageIntegrityError, InstallableImage,
    NativeReference, NativeValue, ReferenceType, RelocationTarget, RuntimeCallSlot, Signature,
    StoreClass, TrapCode, ValueType,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExecutableLimits {
    max_object_code_bytes: u64,
    max_object_metadata_bytes: u64,
    max_object_work_units: u64,
    max_total_code_bytes: u64,
    max_total_metadata_bytes: u64,
    max_total_work_units: u64,
    max_objects: u64,
}

impl ExecutableLimits {
    #[must_use]
    pub const fn new(
        max_object_code_bytes: u64,
        max_object_metadata_bytes: u64,
        max_object_work_units: u64,
        max_total_code_bytes: u64,
        max_total_metadata_bytes: u64,
        max_total_work_units: u64,
        max_objects: u64,
    ) -> Self {
        Self {
            max_object_code_bytes,
            max_object_metadata_bytes,
            max_object_work_units,
            max_total_code_bytes,
            max_total_metadata_bytes,
            max_total_work_units,
            max_objects,
        }
    }
}

impl Default for ExecutableLimits {
    fn default() -> Self {
        Self::new(
            4 * 1024 * 1024,
            4 * 1024 * 1024,
            100_000,
            64 * 1024 * 1024,
            64 * 1024 * 1024,
            1_000_000,
            1_024,
        )
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExecutableUsage {
    code_bytes: u64,
    metadata_bytes: u64,
    work_units: u64,
    objects: u64,
}

impl ExecutableUsage {
    #[must_use]
    pub const fn code_bytes(self) -> u64 {
        self.code_bytes
    }

    #[must_use]
    pub const fn metadata_bytes(self) -> u64 {
        self.metadata_bytes
    }

    #[must_use]
    pub const fn work_units(self) -> u64 {
        self.work_units
    }

    #[must_use]
    pub const fn objects(self) -> u64 {
        self.objects
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutableLimitKind {
    ObjectCodeBytes,
    ObjectMetadataBytes,
    ObjectWorkUnits,
    TotalCodeBytes,
    TotalMetadataBytes,
    TotalWorkUnits,
    ObjectCount,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InstallError {
    UnsupportedPlatform,
    InvalidImage(ImageIntegrityError),
    VersionMismatch {
        expected: AbiVersions,
        actual: AbiVersions,
    },
    LimitExceeded(ExecutableLimitKind),
    AllocationFailed,
    RelocationRange,
    RelocationAddress,
    ProtectionFailed,
}

impl fmt::Display for InstallError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("native executable images are supported only on Linux x86-64")
            }
            Self::InvalidImage(error) => write!(formatter, "invalid installable image: {error}"),
            Self::VersionMismatch { expected, actual } => write!(
                formatter,
                "native ABI version mismatch: expected {expected:?}, received {actual:?}"
            ),
            Self::LimitExceeded(kind) => {
                write!(formatter, "executable install limit exceeded: {kind:?}")
            }
            Self::AllocationFailed => formatter.write_str("RW executable-image allocation failed"),
            Self::RelocationRange => {
                formatter.write_str("executable-image relocation is out of range")
            }
            Self::RelocationAddress => {
                formatter.write_str("executable-image relocation address is invalid")
            }
            Self::ProtectionFailed => {
                formatter.write_str("RW to RX executable-image transition failed")
            }
        }
    }
}

impl std::error::Error for InstallError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvocationError {
    UnknownEntry,
    ArgumentCount {
        expected: usize,
        actual: usize,
    },
    ArgumentType {
        index: usize,
        expected: ValueType,
        actual: ValueType,
    },
    UnsupportedSignature,
    InvalidNativeStatus(u32),
    InvalidNativeTrap(u32),
    InvalidBoolReturn(u64),
    RootCapacityExceeded,
    InvalidActiveFrame,
    LeakedActiveFrames(usize),
}

impl fmt::Display for InvocationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownEntry => formatter.write_str("unknown native entry"),
            Self::ArgumentCount { expected, actual } => {
                write!(
                    formatter,
                    "native entry expected {expected} arguments, received {actual}"
                )
            }
            Self::ArgumentType {
                index,
                expected,
                actual,
            } => write!(
                formatter,
                "native argument {index} has type {actual:?}; expected {expected:?}"
            ),
            Self::UnsupportedSignature => {
                formatter.write_str("native entry signature is unsupported")
            }
            Self::InvalidNativeStatus(status) => {
                write!(formatter, "generated code returned invalid status {status}")
            }
            Self::InvalidNativeTrap(trap) => {
                write!(formatter, "generated code returned invalid trap {trap}")
            }
            Self::InvalidBoolReturn(value) => write!(
                formatter,
                "generated code returned non-canonical Bool {value}"
            ),
            Self::RootCapacityExceeded => {
                formatter.write_str("native root materialization capacity is exceeded")
            }
            Self::InvalidActiveFrame => {
                formatter.write_str("generated active-frame metadata is invalid")
            }
            Self::LeakedActiveFrames(depth) => {
                write!(
                    formatter,
                    "generated invocation leaked {depth} active frames"
                )
            }
        }
    }
}

impl std::error::Error for InvocationError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeResourceLimitKind {
    PollFuel,
    ActiveFrames,
    MaterializedRoots,
    RuntimeService,
    NativeStackBytes,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InvocationOutcome {
    Returned(NativeValue),
    Trapped(TrapCode),
    Exited(i64),
    DeadlineExceeded,
    ResourceLimitExceeded(NativeResourceLimitKind),
    HostFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeInvocationConfig {
    poll_fuel: u64,
    wall_time: Option<Duration>,
    max_active_frames: usize,
    max_native_stack_bytes: usize,
    max_native_frame_bytes: usize,
}

impl NativeInvocationConfig {
    #[must_use]
    pub const fn new(poll_fuel: u64, wall_time: Option<Duration>) -> Self {
        Self {
            poll_fuel,
            wall_time,
            max_active_frames: MAX_ACTIVE_FRAMES,
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
    source_function: u32,
    entries: u64,
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
    reference_type: ReferenceType,
    opaque_word: u64,
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

#[derive(Default)]
struct NoopNativeRuntimeServices;

impl NativeRuntimeServices for NoopNativeRuntimeServices {
    fn collect_references(&mut self, _roots: &mut [NativeRoot]) -> Result<(), NativeServiceError> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct InvocationReport {
    outcome: InvocationOutcome,
    poll_count: u64,
    native_entries: Vec<NativeEntryCount>,
    peak_active_frame_depth: usize,
    active_frame_depth: usize,
    collection_calls: u64,
    maximum_roots: usize,
    exact_root_counts: Vec<usize>,
    peak_native_stack_bytes: usize,
    reserved_native_stack_bytes: usize,
    heap_operation_calls: u64,
    barrier_count: u64,
}

impl InvocationReport {
    #[must_use]
    pub const fn outcome(&self) -> InvocationOutcome {
        self.outcome
    }

    #[must_use]
    pub const fn poll_count(&self) -> u64 {
        self.poll_count
    }

    #[must_use]
    pub fn native_entries(&self) -> &[NativeEntryCount] {
        &self.native_entries
    }

    #[must_use]
    pub const fn peak_active_frame_depth(&self) -> usize {
        self.peak_active_frame_depth
    }

    #[must_use]
    pub const fn active_frame_depth(&self) -> usize {
        self.active_frame_depth
    }

    #[must_use]
    pub const fn collection_calls(&self) -> u64 {
        self.collection_calls
    }

    #[must_use]
    pub const fn maximum_roots(&self) -> usize {
        self.maximum_roots
    }

    #[must_use]
    pub fn exact_root_counts(&self) -> &[usize] {
        &self.exact_root_counts
    }

    #[must_use]
    pub const fn peak_native_stack_bytes(&self) -> usize {
        self.peak_native_stack_bytes
    }

    #[must_use]
    pub const fn reserved_native_stack_bytes(&self) -> usize {
        self.reserved_native_stack_bytes
    }

    #[must_use]
    pub const fn heap_operation_calls(&self) -> u64 {
        self.heap_operation_calls
    }

    #[must_use]
    pub const fn barrier_count(&self) -> u64 {
        self.barrier_count
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MappingPermissions {
    readable: bool,
    writable: bool,
    executable: bool,
}

impl MappingPermissions {
    #[must_use]
    pub const fn readable(self) -> bool {
        self.readable
    }

    #[must_use]
    pub const fn writable(self) -> bool {
        self.writable
    }

    #[must_use]
    pub const fn executable(self) -> bool {
        self.executable
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PermissionProbeError {
    UnsupportedPlatform,
    ProcMapsUnavailable,
    MappingNotFound,
    MalformedPermissions,
}

impl fmt::Display for PermissionProbeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform => {
                formatter.write_str("mapping permission probe is unsupported")
            }
            Self::ProcMapsUnavailable => formatter.write_str("/proc/self/maps is unavailable"),
            Self::MappingNotFound => {
                formatter.write_str("installed mapping is absent from /proc/self/maps")
            }
            Self::MalformedPermissions => {
                formatter.write_str("installed mapping has malformed permissions")
            }
        }
    }
}

impl std::error::Error for PermissionProbeError {}

#[derive(Debug)]
struct InstallerState {
    limits: ExecutableLimits,
    usage: Cell<ExecutableUsage>,
}

/// Bounded non-Send executable allocation session. Installed images retain an
/// owned lease on this state, so mappings cannot outlive their accounting.
#[derive(Clone, Debug)]
pub struct ExecutableInstaller {
    state: Rc<InstallerState>,
    not_send_or_sync: PhantomData<Rc<()>>,
}

impl ExecutableInstaller {
    #[must_use]
    pub fn new(limits: ExecutableLimits) -> Self {
        Self {
            state: Rc::new(InstallerState {
                limits,
                usage: Cell::new(ExecutableUsage::default()),
            }),
            not_send_or_sync: PhantomData,
        }
    }

    #[must_use]
    pub fn usage(&self) -> ExecutableUsage {
        self.state.usage.get()
    }

    pub fn install(&self, image: InstallableImage) -> Result<InstalledImage, InstallError> {
        if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            return Err(InstallError::UnsupportedPlatform);
        }
        image
            .validate_integrity()
            .map_err(InstallError::InvalidImage)?;
        let expected = AbiVersions::current();
        if image.versions() != expected {
            return Err(InstallError::VersionMismatch {
                expected,
                actual: image.versions(),
            });
        }
        let accounting = image.accounting();
        check_object_limit(
            accounting.code_bytes(),
            self.state.limits.max_object_code_bytes,
            ExecutableLimitKind::ObjectCodeBytes,
        )?;
        check_object_limit(
            accounting.metadata_bytes(),
            self.state.limits.max_object_metadata_bytes,
            ExecutableLimitKind::ObjectMetadataBytes,
        )?;
        check_object_limit(
            accounting.work_units(),
            self.state.limits.max_object_work_units,
            ExecutableLimitKind::ObjectWorkUnits,
        )?;
        let next_usage = checked_usage(self.state.usage.get(), accounting, self.state.limits)?;
        let mut mapping = platform::Mapping::allocate_rw(image.bytes().len())?;
        mapping.copy_from(image.bytes())?;
        for item in image.relocations() {
            let address = match item.target() {
                RelocationTarget::Function(function) => {
                    let entry = image
                        .entries()
                        .iter()
                        .find(|entry| entry.function() == function)
                        .ok_or(InstallError::RelocationAddress)?;
                    mapping.address_at(entry.offset() as usize)?
                }
                RelocationTarget::Runtime(slot) => runtime_symbol(slot),
            };
            mapping.write_absolute64(item.offset() as usize, address)?;
        }
        mapping.seal_rx()?;
        self.state.usage.set(next_usage);
        Ok(InstalledImage {
            installer: Rc::clone(&self.state),
            image,
            mapping,
            usage: ExecutableUsage {
                code_bytes: accounting.code_bytes(),
                metadata_bytes: accounting.metadata_bytes(),
                work_units: accounting.work_units(),
                objects: 1,
            },
            not_send_or_sync: PhantomData,
        })
    }
}

impl Default for ExecutableInstaller {
    fn default() -> Self {
        Self::new(ExecutableLimits::default())
    }
}

#[derive(Debug)]
pub struct InstalledImage {
    installer: Rc<InstallerState>,
    image: InstallableImage,
    mapping: platform::Mapping,
    usage: ExecutableUsage,
    not_send_or_sync: PhantomData<Rc<()>>,
}

impl InstalledImage {
    #[must_use]
    pub fn entries(&self) -> &[lkjscript_native::EntryMetadata] {
        self.image.entries()
    }

    pub fn invoke(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
    ) -> Result<InvocationOutcome, InvocationError> {
        self.invoke_with_config(entry, arguments, &NativeInvocationConfig::default())
            .map(|report| report.outcome)
    }

    pub fn invoke_with_config(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
    ) -> Result<InvocationReport, InvocationError> {
        let mut services = NoopNativeRuntimeServices;
        self.invoke_with_services(entry, arguments, config, &mut services)
    }

    pub fn invoke_with_services(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
        config: &NativeInvocationConfig,
        services: &mut dyn NativeRuntimeServices,
    ) -> Result<InvocationReport, InvocationError> {
        let entry = self
            .image
            .entries()
            .iter()
            .find(|candidate| candidate.function() == entry)
            .ok_or(InvocationError::UnknownEntry)?;
        validate_arguments(entry.signature(), arguments)?;
        let mut state = NativeCallState::new(&self.image, config, services)?;
        let raw = self.mapping.invoke(
            entry.offset() as usize,
            entry.signature(),
            arguments,
            &mut state,
        )?;
        if state.metadata_invalid {
            return Err(InvocationError::InvalidActiveFrame);
        }
        if state.active_depth != 0 {
            let leaked = state.active_depth;
            state.active_depth = 0;
            state.pending_reservation = None;
            state.reserved_native_stack_bytes = 0;
            return Err(InvocationError::LeakedActiveFrames(leaked));
        }
        if state.pending_reservation.is_some() || state.reserved_native_stack_bytes != 0 {
            return Err(InvocationError::InvalidActiveFrame);
        }
        let outcome = match state.status {
            0 => InvocationOutcome::Returned(raw.into_value(entry.signature().result())?),
            1 => InvocationOutcome::Trapped(match state.trap {
                1 => TrapCode::I64Overflow,
                2 => TrapCode::DivisionByZero,
                3 => TrapCode::Explicit,
                other => return Err(InvocationError::InvalidNativeTrap(other)),
            }),
            2 => InvocationOutcome::Exited(state.payload),
            3 => InvocationOutcome::DeadlineExceeded,
            4 => InvocationOutcome::ResourceLimitExceeded(match state.payload {
                1 => NativeResourceLimitKind::PollFuel,
                2 => NativeResourceLimitKind::ActiveFrames,
                3 => NativeResourceLimitKind::MaterializedRoots,
                4 => NativeResourceLimitKind::RuntimeService,
                5 => NativeResourceLimitKind::NativeStackBytes,
                _ => return Err(InvocationError::InvalidNativeStatus(state.status)),
            }),
            5 => InvocationOutcome::HostFailure,
            other => return Err(InvocationError::InvalidNativeStatus(other)),
        };
        let native_entries = state
            .native_entries
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, entries)| *entries != 0)
            .filter_map(|(source_function, entries)| {
                u32::try_from(source_function)
                    .ok()
                    .map(|source_function| NativeEntryCount {
                        source_function,
                        entries,
                    })
            })
            .collect();
        Ok(InvocationReport {
            outcome,
            poll_count: state.poll_count,
            native_entries,
            peak_active_frame_depth: state.peak_active_depth,
            active_frame_depth: state.active_depth,
            collection_calls: state.collection_calls,
            maximum_roots: state.maximum_roots,
            exact_root_counts: state.exact_root_counts.clone(),
            peak_native_stack_bytes: state.peak_native_stack_bytes,
            reserved_native_stack_bytes: state.reserved_native_stack_bytes,
            heap_operation_calls: state.heap_operation_calls,
            barrier_count: state.barrier_count,
        })
    }

    pub fn permissions(&self) -> Result<MappingPermissions, PermissionProbeError> {
        self.mapping.permissions()
    }

    #[must_use]
    pub fn accounted_allocation_bytes(&self) -> u64 {
        u64::try_from(self.mapping.allocation_length()).unwrap_or(u64::MAX)
    }

    #[must_use]
    pub fn wx_transition_verified(&self) -> bool {
        self.mapping.wx_transition_verified()
    }
}

impl Drop for InstalledImage {
    fn drop(&mut self) {
        let current = self.installer.usage.get();
        self.installer.usage.set(ExecutableUsage {
            code_bytes: current.code_bytes.saturating_sub(self.usage.code_bytes),
            metadata_bytes: current
                .metadata_bytes
                .saturating_sub(self.usage.metadata_bytes),
            work_units: current.work_units.saturating_sub(self.usage.work_units),
            objects: current.objects.saturating_sub(1),
        });
    }
}

const MAX_NATIVE_ENTRY_COUNTS: usize = 64;
const MAX_ACTIVE_FRAMES: usize = 64;
const MAX_MATERIALIZED_ROOTS: usize = 65_536;
const MAX_COLLECTION_REPORTS: usize = 65_536;
const DEFAULT_MAX_NATIVE_STACK_BYTES: usize = 4 * 1024 * 1024;
const DEFAULT_MAX_NATIVE_FRAME_BYTES: usize = 1024 * 1024;
const NATIVE_STACK_GUARD_BYTES: usize = 16 * 1024;
const INVALID_SAFEPOINT: u32 = u32::MAX;

#[derive(Clone, Copy)]
struct ActiveFrame {
    function_ordinal: u32,
    rbp: *mut u8,
    safepoint: u32,
    reserved_bytes: usize,
}

const EMPTY_ACTIVE_FRAME: ActiveFrame = ActiveFrame {
    function_ordinal: u32::MAX,
    rbp: std::ptr::null_mut(),
    safepoint: INVALID_SAFEPOINT,
    reserved_bytes: 0,
};

#[derive(Clone, Copy)]
struct PendingFrameReservation {
    function_ordinal: u32,
    rbp: *mut u8,
    frame_bytes: usize,
}

#[derive(Clone, Copy)]
struct RootAddress {
    address: *mut u64,
    original_word: u64,
    reference_type: ReferenceType,
    frame_index: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MaterializeRootError {
    InvalidFrame,
    Capacity,
}

#[repr(C)]
struct NativeCallState<'a> {
    // These first three fields are the stable runtime ABI consumed directly by
    // generated code. Native ABI 2 adds frame operations without changing the
    // semantic or runtime ABI versions.
    status: u32,
    trap: u32,
    payload: i64,
    _scratch_integer_arguments: [u64; 2],
    _scratch_float_arguments: [u64; 2],
    poll_fuel_remaining: u64,
    deadline_ms: i64,
    poll_count: u64,
    native_entries: [u64; MAX_NATIVE_ENTRY_COUNTS],
    image: &'a InstallableImage,
    services: &'a mut dyn NativeRuntimeServices,
    active_frames: [ActiveFrame; MAX_ACTIVE_FRAMES],
    active_depth: usize,
    maximum_active_frames: usize,
    maximum_native_stack_bytes: usize,
    maximum_native_frame_bytes: usize,
    pending_reservation: Option<PendingFrameReservation>,
    reserved_native_stack_bytes: usize,
    peak_native_stack_bytes: usize,
    peak_active_depth: usize,
    collection_calls: u64,
    maximum_roots: usize,
    exact_root_counts: Vec<usize>,
    roots: Vec<NativeRoot>,
    root_addresses: Vec<RootAddress>,
    heap_arguments: Vec<NativeValue>,
    heap_operation_calls: u64,
    barrier_count: u64,
    metadata_invalid: bool,
}

impl<'a> NativeCallState<'a> {
    fn new(
        image: &'a InstallableImage,
        config: &NativeInvocationConfig,
        services: &'a mut dyn NativeRuntimeServices,
    ) -> Result<Self, InvocationError> {
        let maximum_active_frames = config.max_active_frames.min(MAX_ACTIVE_FRAMES);
        let maximum_map_roots = image
            .safepoints()
            .iter()
            .map(|safepoint| safepoint.stack_map().roots().len())
            .max()
            .unwrap_or(0);
        let initial_root_capacity = maximum_map_roots.min(MAX_MATERIALIZED_ROOTS);
        let mut roots = Vec::new();
        let mut root_addresses = Vec::new();
        let mut exact_root_counts = Vec::new();
        let mut heap_arguments = Vec::new();
        roots
            .try_reserve_exact(initial_root_capacity)
            .map_err(|_| InvocationError::RootCapacityExceeded)?;
        root_addresses
            .try_reserve_exact(initial_root_capacity)
            .map_err(|_| InvocationError::RootCapacityExceeded)?;
        if image
            .runtime_calls()
            .contains(&RuntimeCallSlot::CollectReferenceV1)
            || !image.heap_runtime_sites().is_empty()
        {
            exact_root_counts
                .try_reserve(1)
                .map_err(|_| InvocationError::RootCapacityExceeded)?;
        }
        heap_arguments
            .try_reserve_exact(16)
            .map_err(|_| InvocationError::RootCapacityExceeded)?;
        let (deadline_ms, status) = match config.wall_time {
            Some(duration) => match crate::now_ms_monotonic() {
                Ok(now) => {
                    let delta = i64::try_from(duration.as_millis()).unwrap_or(i64::MAX);
                    (now.saturating_add(delta), 0)
                }
                Err(_) => (-1, 5),
            },
            None => (-1, 0),
        };
        Ok(Self {
            status,
            trap: 0,
            payload: 0,
            _scratch_integer_arguments: [0; 2],
            _scratch_float_arguments: [0; 2],
            poll_fuel_remaining: config.poll_fuel,
            deadline_ms,
            poll_count: 0,
            native_entries: [0; MAX_NATIVE_ENTRY_COUNTS],
            image,
            services,
            active_frames: [EMPTY_ACTIVE_FRAME; MAX_ACTIVE_FRAMES],
            active_depth: 0,
            maximum_active_frames,
            maximum_native_stack_bytes: config.max_native_stack_bytes,
            maximum_native_frame_bytes: config.max_native_frame_bytes,
            pending_reservation: None,
            reserved_native_stack_bytes: 0,
            peak_native_stack_bytes: 0,
            peak_active_depth: 0,
            collection_calls: 0,
            maximum_roots: 0,
            exact_root_counts,
            roots,
            root_addresses,
            heap_arguments,
            heap_operation_calls: 0,
            barrier_count: 0,
            metadata_invalid: false,
        })
    }

    fn reserve_frame(&mut self, function_ordinal: u32, frame_bytes: u64, rbp: *mut u8) {
        if self.status != 0 {
            return;
        }
        if self.active_depth >= self.maximum_active_frames {
            self.status = 4;
            self.payload = 2;
            return;
        }
        if self.pending_reservation.is_some() {
            self.invalidate_active_frame();
            return;
        }
        let Some(entry) = self.image.entries().get(function_ordinal as usize) else {
            self.invalidate_active_frame();
            return;
        };
        let Some(descriptor_bytes) = self
            .image
            .frames()
            .iter()
            .find(|frame| frame.function() == entry.function())
            .map(|frame| frame.frame_bytes())
        else {
            self.invalidate_active_frame();
            return;
        };
        let Ok(frame_bytes) = usize::try_from(frame_bytes) else {
            self.invalidate_active_frame();
            return;
        };
        if usize::try_from(descriptor_bytes).ok() != Some(frame_bytes)
            || rbp.is_null()
            || !(rbp as usize).is_multiple_of(16)
        {
            self.invalidate_active_frame();
            return;
        }
        let Some(next_reserved_bytes) = self.reserved_native_stack_bytes.checked_add(frame_bytes)
        else {
            self.status = 4;
            self.payload = 5;
            return;
        };
        if frame_bytes > self.maximum_native_frame_bytes
            || next_reserved_bytes > self.maximum_native_stack_bytes
            || !platform::native_stack_reservation_fits(rbp, frame_bytes, NATIVE_STACK_GUARD_BYTES)
        {
            self.status = 4;
            self.payload = 5;
            return;
        }
        self.pending_reservation = Some(PendingFrameReservation {
            function_ordinal,
            rbp,
            frame_bytes,
        });
        self.reserved_native_stack_bytes = next_reserved_bytes;
        self.peak_native_stack_bytes = self
            .peak_native_stack_bytes
            .max(self.reserved_native_stack_bytes);
    }

    fn register_frame(&mut self, function_ordinal: u32, rbp: *mut u8) {
        if self.status != 0 {
            return;
        }
        let Some(reservation) = self.pending_reservation else {
            self.invalidate_active_frame();
            return;
        };
        if reservation.function_ordinal != function_ordinal
            || reservation.rbp != rbp
            || self.active_depth >= self.maximum_active_frames
        {
            self.invalidate_active_frame();
            return;
        }
        self.pending_reservation = None;
        self.active_frames[self.active_depth] = ActiveFrame {
            function_ordinal,
            rbp,
            safepoint: INVALID_SAFEPOINT,
            reserved_bytes: reservation.frame_bytes,
        };
        self.active_depth += 1;
        self.peak_active_depth = self.peak_active_depth.max(self.active_depth);
    }

    fn publish_safepoint(&mut self, safepoint: u32) {
        if self.status != 0 {
            return;
        }
        let Some(frame) = self
            .active_frames
            .get_mut(self.active_depth.saturating_sub(1))
        else {
            self.invalidate_active_frame();
            return;
        };
        let Some(entry) = self.image.entries().get(frame.function_ordinal as usize) else {
            self.invalidate_active_frame();
            return;
        };
        let valid = self
            .image
            .safepoints()
            .get(safepoint as usize)
            .is_some_and(|map| map.id() == safepoint && map.function() == entry.function());
        if !valid {
            self.invalidate_active_frame();
            return;
        }
        frame.safepoint = safepoint;
    }

    fn unregister_frame(&mut self, function_ordinal: u32, rbp: *mut u8) {
        let Some(index) = self.active_depth.checked_sub(1) else {
            self.invalidate_active_frame();
            return;
        };
        let frame = self.active_frames[index];
        if frame.function_ordinal != function_ordinal || frame.rbp != rbp {
            self.invalidate_active_frame();
            return;
        }
        let Some(next_reserved_bytes) = self
            .reserved_native_stack_bytes
            .checked_sub(frame.reserved_bytes)
        else {
            self.invalidate_active_frame();
            return;
        };
        self.active_frames[index] = EMPTY_ACTIVE_FRAME;
        self.active_depth = index;
        self.reserved_native_stack_bytes = next_reserved_bytes;
    }

    fn collect_references(&mut self, argument: u64) -> u64 {
        if self.status != 0 {
            return argument;
        }
        self.roots.clear();
        self.root_addresses.clear();
        for frame_index in 0..self.active_depth {
            match self.materialize_frame_roots(frame_index) {
                Ok(()) => {}
                Err(MaterializeRootError::InvalidFrame) => {
                    self.invalidate_active_frame();
                    return argument;
                }
                Err(MaterializeRootError::Capacity) => {
                    self.status = 4;
                    self.payload = 3;
                    return argument;
                }
            }
        }
        let root_count = self.roots.len();
        if self.exact_root_counts.len() == MAX_COLLECTION_REPORTS
            || self.exact_root_counts.try_reserve(1).is_err()
        {
            self.status = 4;
            self.payload = 3;
            return argument;
        }
        self.collection_calls = self.collection_calls.saturating_add(1);
        self.maximum_roots = self.maximum_roots.max(root_count);
        self.exact_root_counts.push(root_count);
        match self.services.collect_references(&mut self.roots) {
            Ok(()) => {}
            Err(NativeServiceError::Trap) => {
                self.status = 1;
                self.trap = TrapCode::Explicit.as_u32();
                return argument;
            }
            Err(NativeServiceError::ResourceLimitExceeded) => {
                self.status = 4;
                self.payload = 4;
                return argument;
            }
            Err(NativeServiceError::HostFailure) => {
                self.status = 5;
                return argument;
            }
        }
        for (root, address) in self.roots.iter().zip(&self.root_addresses) {
            if root.reference_type != address.reference_type {
                self.invalidate_active_frame();
                return argument;
            }
            // SAFETY: materialize_frame_roots validated this exact aligned home
            // against retained image metadata and the live generated frame.
            unsafe { address.address.write(root.opaque_word) };
        }
        self.root_addresses
            .iter()
            .zip(&self.roots)
            .rev()
            .find(|(address, _)| {
                address.frame_index + 1 == self.active_depth
                    && address.original_word == argument
                    && address.reference_type == ReferenceType::Buf
            })
            .map_or(argument, |(_, root)| root.opaque_word)
    }

    fn dispatch_heap_operation(&mut self, site_id: u32) {
        if self.status != 0 {
            return;
        }
        let Some(site) = self
            .image
            .heap_runtime_sites()
            .get(site_id as usize)
            .cloned()
        else {
            self.invalidate_active_frame();
            return;
        };
        if site.id() != site_id || site.safepoint() as usize >= self.image.safepoints().len() {
            self.invalidate_active_frame();
            return;
        }
        let Some(frame_index) = self.active_depth.checked_sub(1) else {
            self.invalidate_active_frame();
            return;
        };
        let frame = self.active_frames[frame_index];
        let Some(entry) = self.image.entries().get(frame.function_ordinal as usize) else {
            self.invalidate_active_frame();
            return;
        };
        if entry.function() != site.function() || frame.safepoint != site.safepoint() {
            self.invalidate_active_frame();
            return;
        }
        let Some(facts) = self
            .image
            .frames()
            .iter()
            .find(|facts| facts.function() == site.function())
            .cloned()
        else {
            self.invalidate_active_frame();
            return;
        };
        self.heap_arguments.clear();
        for home in site.arguments() {
            if !facts.homes().contains(home) {
                self.invalidate_active_frame();
                return;
            }
            // SAFETY: image integrity and the active descriptor validate this
            // aligned home inside the currently registered generated frame.
            let address = unsafe {
                frame
                    .rbp
                    .offset(home.rbp_displacement() as isize)
                    .cast::<u64>()
            };
            // SAFETY: the exact home above is initialized before dispatch.
            let word = unsafe { address.read() };
            let value = match home.value_type() {
                ValueType::I64 => NativeValue::I64(word as i64),
                ValueType::F64 => NativeValue::F64Bits(word),
                ValueType::Bool if word <= 1 => NativeValue::Bool(word == 1),
                ValueType::Bool => {
                    self.invalidate_active_frame();
                    return;
                }
                ValueType::Unit if word == 0 => NativeValue::Unit,
                ValueType::Unit => {
                    self.invalidate_active_frame();
                    return;
                }
                ValueType::Reference(reference_type) => {
                    NativeValue::Reference(NativeReference::new(reference_type, word))
                }
            };
            self.heap_arguments.push(value);
        }
        self.roots.clear();
        self.root_addresses.clear();
        for active in 0..self.active_depth {
            match self.materialize_frame_roots(active) {
                Ok(()) => {}
                Err(MaterializeRootError::InvalidFrame) => {
                    self.invalidate_active_frame();
                    return;
                }
                Err(MaterializeRootError::Capacity) => {
                    self.status = 4;
                    self.payload = 3;
                    return;
                }
            }
        }
        let collected =
            self.services
                .prepare_heap_operation(&site, &self.heap_arguments, &mut self.roots);
        let collected = match collected {
            Ok(collected) => collected,
            Err(NativeServiceError::Trap) => {
                self.status = 1;
                self.trap = TrapCode::Explicit.as_u32();
                return;
            }
            Err(NativeServiceError::ResourceLimitExceeded) => {
                self.status = 4;
                self.payload = 4;
                return;
            }
            Err(NativeServiceError::HostFailure) => {
                self.status = 5;
                return;
            }
        };
        for (root, address) in self.roots.iter().zip(&self.root_addresses) {
            if root.reference_type != address.reference_type {
                self.invalidate_active_frame();
                return;
            }
            // SAFETY: root addresses came only from validated active homes.
            unsafe { address.address.write(root.opaque_word) };
        }
        if collected {
            let count = self.roots.len();
            if self.exact_root_counts.len() == MAX_COLLECTION_REPORTS
                || self.exact_root_counts.try_reserve(1).is_err()
            {
                self.status = 4;
                self.payload = 3;
                return;
            }
            self.collection_calls = self.collection_calls.saturating_add(1);
            self.maximum_roots = self.maximum_roots.max(count);
            self.exact_root_counts.push(count);
        }
        let result = self.services.heap_operation(&site, &self.heap_arguments);
        let result = match result {
            Ok(result) => result,
            Err(NativeServiceError::Trap) => {
                self.status = 1;
                self.trap = TrapCode::Explicit.as_u32();
                return;
            }
            Err(NativeServiceError::ResourceLimitExceeded) => {
                self.status = 4;
                self.payload = 4;
                return;
            }
            Err(NativeServiceError::HostFailure) => {
                self.status = 5;
                return;
            }
        };
        if result.value_type() != site.descriptor().result_type() {
            self.invalidate_active_frame();
            return;
        }
        let Some(word) = native_value_word(result, site.descriptor().result_type()) else {
            self.invalidate_active_frame();
            return;
        };
        let result_home = site.result();
        if !facts.homes().contains(&result_home) {
            self.invalidate_active_frame();
            return;
        }
        // SAFETY: retained site integrity binds this exact initialized result
        // home to the current active generated frame.
        unsafe {
            frame
                .rbp
                .offset(result_home.rbp_displacement() as isize)
                .cast::<u64>()
                .write(word)
        };
        self.heap_operation_calls = self.heap_operation_calls.saturating_add(1);
        if site.descriptor().store() != StoreClass::None {
            self.barrier_count = self.barrier_count.saturating_add(1);
        }
    }

    fn materialize_frame_roots(&mut self, frame_index: usize) -> Result<(), MaterializeRootError> {
        let Some(frame) = self.active_frames.get(frame_index).copied() else {
            return Err(MaterializeRootError::InvalidFrame);
        };
        if frame.rbp.is_null() || frame.safepoint == INVALID_SAFEPOINT {
            return Err(MaterializeRootError::InvalidFrame);
        }
        let Some(entry) = self.image.entries().get(frame.function_ordinal as usize) else {
            return Err(MaterializeRootError::InvalidFrame);
        };
        let Some(frame_facts) = self
            .image
            .frames()
            .iter()
            .find(|facts| facts.function() == entry.function())
        else {
            return Err(MaterializeRootError::InvalidFrame);
        };
        let Some(safepoint) = self.image.safepoints().get(frame.safepoint as usize) else {
            return Err(MaterializeRootError::InvalidFrame);
        };
        if safepoint.id() != frame.safepoint || safepoint.function() != entry.function() {
            return Err(MaterializeRootError::InvalidFrame);
        }
        let additional = safepoint.stack_map().roots().len();
        let next_root_count = self
            .roots
            .len()
            .checked_add(additional)
            .filter(|count| *count <= MAX_MATERIALIZED_ROOTS)
            .ok_or(MaterializeRootError::Capacity)?;
        let root_additional = next_root_count
            .checked_sub(self.roots.len())
            .ok_or(MaterializeRootError::Capacity)?;
        let address_additional = next_root_count
            .checked_sub(self.root_addresses.len())
            .ok_or(MaterializeRootError::Capacity)?;
        if (root_additional != 0 && self.roots.try_reserve_exact(root_additional).is_err())
            || (address_additional != 0
                && self
                    .root_addresses
                    .try_reserve_exact(address_additional)
                    .is_err())
        {
            return Err(MaterializeRootError::Capacity);
        }
        for root in safepoint.stack_map().roots() {
            let displacement = root.rbp_displacement();
            let in_frame = displacement <= -16
                && displacement % 8 == 0
                && displacement
                    .checked_neg()
                    .and_then(|value| u32::try_from(value).ok())
                    .is_some_and(|value| value <= frame_facts.frame_bytes())
                && frame_facts.homes().iter().any(|home| {
                    home.kind() == root.kind()
                        && home.rbp_displacement() == displacement
                        && home.value_type() == ValueType::Reference(root.reference_type())
                });
            if !in_frame {
                return Err(MaterializeRootError::InvalidFrame);
            }
            // SAFETY: the retained descriptor bounds the negative displacement
            // within this registered generated frame. Homes are aligned words.
            let address = unsafe { frame.rbp.offset(displacement as isize).cast::<u64>() };
            // SAFETY: address is the validated live word home above.
            let opaque_word = unsafe { address.read() };
            self.roots.push(NativeRoot {
                reference_type: root.reference_type(),
                opaque_word,
            });
            self.root_addresses.push(RootAddress {
                address,
                original_word: opaque_word,
                reference_type: root.reference_type(),
                frame_index,
            });
        }
        Ok(())
    }

    fn invalidate_active_frame(&mut self) {
        if let Some(reservation) = self.pending_reservation.take() {
            self.reserved_native_stack_bytes = self
                .reserved_native_stack_bytes
                .saturating_sub(reservation.frame_bytes);
        }
        self.metadata_invalid = true;
        self.status = 5;
    }
}

#[derive(Clone, Copy)]
enum MachineArgument {
    Integer(u64),
    Float(f64),
}

#[derive(Clone, Copy)]
enum RawReturn {
    Integer(u64),
    Float(f64),
    Unit,
}

impl RawReturn {
    fn into_value(self, value_type: ValueType) -> Result<NativeValue, InvocationError> {
        match (self, value_type) {
            (Self::Integer(value), ValueType::I64) => Ok(NativeValue::I64(value as i64)),
            (Self::Integer(value), ValueType::Bool) if value <= 1 => {
                Ok(NativeValue::Bool(value == 1))
            }
            (Self::Integer(value), ValueType::Bool) => {
                Err(InvocationError::InvalidBoolReturn(value))
            }
            (Self::Float(value), ValueType::F64) => Ok(NativeValue::F64Bits(value.to_bits())),
            (Self::Unit, ValueType::Unit) => Ok(NativeValue::Unit),
            (Self::Integer(value), ValueType::Reference(reference_type)) => Ok(
                NativeValue::Reference(NativeReference::new(reference_type, value)),
            ),
            _ => Err(InvocationError::UnsupportedSignature),
        }
    }
}

fn native_value_word(value: NativeValue, expected: ValueType) -> Option<u64> {
    if value.value_type() != expected {
        return None;
    }
    Some(match value {
        NativeValue::I64(value) => value as u64,
        NativeValue::F64Bits(bits) => bits,
        NativeValue::Bool(value) => u64::from(value),
        NativeValue::Unit => 0,
        NativeValue::Reference(reference) => reference.opaque_word(),
    })
}

fn validate_arguments(
    signature: &Signature,
    arguments: &[NativeValue],
) -> Result<(), InvocationError> {
    if signature.parameters().len() != arguments.len() {
        return Err(InvocationError::ArgumentCount {
            expected: signature.parameters().len(),
            actual: arguments.len(),
        });
    }
    for (index, (expected, actual)) in signature
        .parameters()
        .iter()
        .copied()
        .zip(arguments.iter().copied())
        .enumerate()
    {
        if expected != actual.value_type() {
            return Err(InvocationError::ArgumentType {
                index,
                expected,
                actual: actual.value_type(),
            });
        }
    }
    Ok(())
}

fn machine_arguments(arguments: &[NativeValue]) -> Vec<MachineArgument> {
    arguments
        .iter()
        .filter_map(|argument| match argument {
            NativeValue::I64(value) => Some(MachineArgument::Integer(*value as u64)),
            NativeValue::F64Bits(bits) => Some(MachineArgument::Float(f64::from_bits(*bits))),
            NativeValue::Bool(value) => Some(MachineArgument::Integer(u64::from(*value))),
            NativeValue::Unit => None,
            NativeValue::Reference(reference) => {
                Some(MachineArgument::Integer(reference.opaque_word()))
            }
        })
        .collect()
}

fn runtime_symbol(slot: RuntimeCallSlot) -> usize {
    match slot {
        RuntimeCallSlot::IdentityI64V1 => runtime_identity_i64_v1 as *const () as usize,
        RuntimeCallSlot::PollV1 => runtime_poll_v1 as *const () as usize,
        RuntimeCallSlot::EnterFunctionV1 => runtime_enter_function_v1 as *const () as usize,
        RuntimeCallSlot::CollectReferenceV1 => runtime_collect_reference_v1 as *const () as usize,
        RuntimeCallSlot::HeapDispatchV1 => runtime_heap_dispatch_v1 as *const () as usize,
        RuntimeCallSlot::ReserveFrameV1 => runtime_reserve_frame_v1 as *const () as usize,
        RuntimeCallSlot::RegisterFrameV1 => runtime_register_frame_v1 as *const () as usize,
        RuntimeCallSlot::PublishSafepointV1 => runtime_publish_safepoint_v1 as *const () as usize,
        RuntimeCallSlot::UnregisterFrameV1 => runtime_unregister_frame_v1 as *const () as usize,
    }
}

extern "C" fn runtime_identity_i64_v1(_state: *mut NativeCallState<'_>, value: u64) -> u64 {
    value
}

extern "C" fn runtime_poll_v1(state: *mut NativeCallState<'_>) {
    // SAFETY: generated runtime calls receive the live invocation state as the
    // implicit first argument. The mapping and call cannot outlive this stack
    // value, and generated code cannot replace the context pointer.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    if state.status != 0 {
        return;
    }
    state.poll_count = state.poll_count.saturating_add(1);
    if state.poll_fuel_remaining == 0 {
        state.status = 4;
        state.payload = 1;
        return;
    }
    state.poll_fuel_remaining -= 1;
    if state.deadline_ms >= 0 {
        match crate::now_ms_monotonic() {
            Ok(now) if now >= state.deadline_ms => state.status = 3,
            Ok(_) => {}
            Err(_) => state.status = 5,
        }
    }
}

extern "C" fn runtime_enter_function_v1(state: *mut NativeCallState<'_>, function: u64) {
    // SAFETY: the context provenance is identical to runtime_poll_v1.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    if state.status != 0 {
        return;
    }
    let Ok(index) = usize::try_from(function) else {
        state.status = 5;
        return;
    };
    let Some(entries) = state.native_entries.get_mut(index) else {
        state.status = 5;
        return;
    };
    *entries = entries.saturating_add(1);
}

extern "C" fn runtime_reserve_frame_v1(
    state: *mut NativeCallState<'_>,
    function_ordinal: u64,
    frame_bytes: u64,
    rbp: *mut u8,
) -> *mut NativeCallState<'_> {
    // SAFETY: generated ABI-2 minimal prologues pass their invocation context
    // and current frame base before touching generated-frame storage.
    let Some(invocation) = (unsafe { state.as_mut() }) else {
        return state;
    };
    let Ok(function_ordinal) = u32::try_from(function_ordinal) else {
        invocation.invalidate_active_frame();
        return state;
    };
    invocation.reserve_frame(function_ordinal, frame_bytes, rbp);
    state
}

extern "C" fn runtime_register_frame_v1(
    state: *mut NativeCallState<'_>,
    function_ordinal: u64,
    rbp: *mut u8,
) {
    // SAFETY: generated ABI-2 prologues pass their invocation context and
    // current frame base. Both remain live until the matching epilogue.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    let Ok(function_ordinal) = u32::try_from(function_ordinal) else {
        state.invalidate_active_frame();
        return;
    };
    state.register_frame(function_ordinal, rbp);
}

extern "C" fn runtime_publish_safepoint_v1(state: *mut NativeCallState<'_>, safepoint: u64) {
    // SAFETY: context provenance is identical to runtime_register_frame_v1.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    let Ok(safepoint) = u32::try_from(safepoint) else {
        state.invalidate_active_frame();
        return;
    };
    state.publish_safepoint(safepoint);
}

extern "C" fn runtime_unregister_frame_v1(
    state: *mut NativeCallState<'_>,
    function_ordinal: u64,
    rbp: *mut u8,
) {
    // SAFETY: context/frame provenance is identical to registration.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    let Ok(function_ordinal) = u32::try_from(function_ordinal) else {
        state.invalidate_active_frame();
        return;
    };
    state.unregister_frame(function_ordinal, rbp);
}

extern "C" fn runtime_heap_dispatch_v1(state: *mut NativeCallState<'_>, site: u64) {
    // SAFETY: generated heap sites publish and pass their retained dense site
    // identity. Raw frame homes are validated and accessed only in this module.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return;
    };
    let Ok(site) = u32::try_from(site) else {
        state.invalidate_active_frame();
        return;
    };
    state.dispatch_heap_operation(site);
}

extern "C" fn runtime_collect_reference_v1(state: *mut NativeCallState<'_>, reference: u64) -> u64 {
    // SAFETY: generated collecting calls publish an exact safepoint before
    // entering this trampoline; all raw frame access remains in this module.
    let Some(state) = (unsafe { state.as_mut() }) else {
        return reference;
    };
    state.collect_references(reference)
}

fn check_object_limit(
    actual: u64,
    maximum: u64,
    kind: ExecutableLimitKind,
) -> Result<(), InstallError> {
    if actual > maximum {
        return Err(InstallError::LimitExceeded(kind));
    }
    Ok(())
}

fn checked_usage(
    current: ExecutableUsage,
    accounting: lkjscript_native::CodeAccounting,
    limits: ExecutableLimits,
) -> Result<ExecutableUsage, InstallError> {
    let next = ExecutableUsage {
        code_bytes: current
            .code_bytes
            .checked_add(accounting.code_bytes())
            .ok_or(InstallError::LimitExceeded(
                ExecutableLimitKind::TotalCodeBytes,
            ))?,
        metadata_bytes: current
            .metadata_bytes
            .checked_add(accounting.metadata_bytes())
            .ok_or(InstallError::LimitExceeded(
                ExecutableLimitKind::TotalMetadataBytes,
            ))?,
        work_units: current
            .work_units
            .checked_add(accounting.work_units())
            .ok_or(InstallError::LimitExceeded(
                ExecutableLimitKind::TotalWorkUnits,
            ))?,
        objects: current
            .objects
            .checked_add(1)
            .ok_or(InstallError::LimitExceeded(
                ExecutableLimitKind::ObjectCount,
            ))?,
    };
    check_object_limit(
        next.code_bytes,
        limits.max_total_code_bytes,
        ExecutableLimitKind::TotalCodeBytes,
    )?;
    check_object_limit(
        next.metadata_bytes,
        limits.max_total_metadata_bytes,
        ExecutableLimitKind::TotalMetadataBytes,
    )?;
    check_object_limit(
        next.work_units,
        limits.max_total_work_units,
        ExecutableLimitKind::TotalWorkUnits,
    )?;
    check_object_limit(
        next.objects,
        limits.max_objects,
        ExecutableLimitKind::ObjectCount,
    )?;
    Ok(next)
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod platform {
    use std::ffi::c_void;
    use std::ptr::NonNull;

    use super::{
        machine_arguments, InstallError, InvocationError, MachineArgument, MappingPermissions,
        NativeCallState, NativeValue, PermissionProbeError, RawReturn, Signature, ValueType,
    };

    const PROT_READ: i32 = 0x1;
    const PROT_WRITE: i32 = 0x2;
    const PROT_EXEC: i32 = 0x4;
    const MAP_PRIVATE: i32 = 0x2;
    const MAP_ANONYMOUS: i32 = 0x20;

    unsafe extern "C" {
        fn mmap(
            address: *mut c_void,
            length: usize,
            protection: i32,
            flags: i32,
            descriptor: i32,
            offset: isize,
        ) -> *mut c_void;
        fn mprotect(address: *mut c_void, length: usize, protection: i32) -> i32;
        fn munmap(address: *mut c_void, length: usize) -> i32;
        fn sysconf(name: i32) -> isize;
        fn pthread_self() -> usize;
        fn pthread_getattr_np(thread: usize, attributes: *mut c_void) -> i32;
        fn pthread_attr_getstack(
            attributes: *const c_void,
            stack_address: *mut *mut c_void,
            stack_size: *mut usize,
        ) -> i32;
        fn pthread_attr_destroy(attributes: *mut c_void) -> i32;
    }

    pub(super) fn native_stack_reservation_fits(
        rbp: *mut u8,
        frame_bytes: usize,
        guard_bytes: usize,
    ) -> bool {
        // pthread_attr_t is opaque here. This word-aligned buffer is larger
        // than the Linux x86-64 glibc and musl objects passed to these APIs.
        let mut attributes = [0_usize; 16];
        // SAFETY: pthread_self has no arguments; pthread_getattr_np initializes
        // the oversized aligned opaque storage for the current live thread.
        let initialized =
            unsafe { pthread_getattr_np(pthread_self(), attributes.as_mut_ptr().cast::<c_void>()) }
                == 0;
        if !initialized {
            return false;
        }
        let mut stack_address = std::ptr::null_mut();
        let mut stack_size = 0_usize;
        // SAFETY: successful pthread_getattr_np initialized the attributes;
        // both output pointers are valid for writes during this call.
        let stack_result = unsafe {
            pthread_attr_getstack(
                attributes.as_ptr().cast::<c_void>(),
                &mut stack_address,
                &mut stack_size,
            )
        };
        // SAFETY: initialized pthread attributes are destroyed exactly once.
        let destroy_result =
            unsafe { pthread_attr_destroy(attributes.as_mut_ptr().cast::<c_void>()) };
        if stack_result != 0 || destroy_result != 0 {
            return false;
        }
        let stack_low = stack_address as usize;
        let Some(stack_high) = stack_low.checked_add(stack_size) else {
            return false;
        };
        let frame_base = rbp as usize;
        let Some(requested_low) = frame_base.checked_sub(frame_bytes) else {
            return false;
        };
        let Some(guarded_low) = stack_low.checked_add(guard_bytes) else {
            return false;
        };
        stack_low < stack_high
            && stack_low <= frame_base
            && frame_base < stack_high
            && guarded_low <= requested_low
    }

    const SC_PAGESIZE: i32 = 30;

    #[derive(Debug)]
    pub(super) struct Mapping {
        base: NonNull<u8>,
        length: usize,
        allocation_length: usize,
        sealed: bool,
        wx_transition_verified: bool,
    }

    impl Mapping {
        pub(super) fn allocate_rw(length: usize) -> Result<Self, InstallError> {
            if length == 0 {
                return Err(InstallError::AllocationFailed);
            }
            // SAFETY: SC_PAGESIZE has no pointer contract and returns one
            // process page-size value.
            let page_size = unsafe { sysconf(SC_PAGESIZE) };
            let page_size = usize::try_from(page_size)
                .ok()
                .filter(|value| *value > 0)
                .ok_or(InstallError::AllocationFailed)?;
            let allocation_length = length
                .checked_add(page_size - 1)
                .map(|value| value / page_size * page_size)
                .ok_or(InstallError::AllocationFailed)?;
            // SAFETY: The null hint, anonymous descriptor, zero offset, and
            // nonzero page-rounded length satisfy mmap. The returned mapping is
            // checked before ownership.
            let pointer = unsafe {
                mmap(
                    std::ptr::null_mut(),
                    allocation_length,
                    PROT_READ | PROT_WRITE,
                    MAP_PRIVATE | MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            if pointer as isize == -1 {
                return Err(InstallError::AllocationFailed);
            }
            let base = NonNull::new(pointer.cast::<u8>()).ok_or(InstallError::AllocationFailed)?;
            Ok(Self {
                base,
                length,
                allocation_length,
                sealed: false,
                wx_transition_verified: false,
            })
        }

        pub(super) fn copy_from(&mut self, bytes: &[u8]) -> Result<(), InstallError> {
            if self.sealed || bytes.len() != self.length {
                return Err(InstallError::RelocationRange);
            }
            // SAFETY: This Mapping uniquely owns a writable mapping of exactly
            // `length` bytes and source/destination cannot overlap.
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), self.base.as_ptr(), self.length);
            }
            Ok(())
        }

        pub(super) fn write_absolute64(
            &mut self,
            offset: usize,
            address: usize,
        ) -> Result<(), InstallError> {
            if self.sealed {
                return Err(InstallError::ProtectionFailed);
            }
            let end = offset.checked_add(8).ok_or(InstallError::RelocationRange)?;
            if end > self.length {
                return Err(InstallError::RelocationRange);
            }
            let address = u64::try_from(address).map_err(|_| InstallError::RelocationAddress)?;
            // SAFETY: The checked range is within the uniquely owned RW mapping.
            unsafe {
                std::ptr::copy_nonoverlapping(
                    address.to_le_bytes().as_ptr(),
                    self.base.as_ptr().add(offset),
                    8,
                );
            }
            Ok(())
        }

        pub(super) fn address_at(&self, offset: usize) -> Result<usize, InstallError> {
            if offset >= self.length {
                return Err(InstallError::RelocationAddress);
            }
            (self.base.as_ptr() as usize)
                .checked_add(offset)
                .ok_or(InstallError::RelocationAddress)
        }

        pub(super) fn seal_rx(&mut self) -> Result<(), InstallError> {
            if self.sealed {
                return Err(InstallError::ProtectionFailed);
            }
            let writable_phase_verified = self.permissions().ok().is_some_and(|permissions| {
                permissions.readable && permissions.writable && !permissions.executable
            });
            // SAFETY: The mapping base came from mmap and remains live. Linux
            // accepts the original length and rounds it to mapped pages.
            let result = unsafe {
                mprotect(
                    self.base.as_ptr().cast::<c_void>(),
                    self.allocation_length,
                    PROT_READ | PROT_EXEC,
                )
            };
            if result != 0 {
                return Err(InstallError::ProtectionFailed);
            }
            self.sealed = true;
            let executable_phase_verified = self.permissions().ok().is_some_and(|permissions| {
                permissions.readable && !permissions.writable && permissions.executable
            });
            self.wx_transition_verified = writable_phase_verified && executable_phase_verified;
            Ok(())
        }

        pub(super) fn allocation_length(&self) -> usize {
            self.allocation_length
        }

        pub(super) fn wx_transition_verified(&self) -> bool {
            self.wx_transition_verified
        }

        pub(super) fn invoke(
            &self,
            offset: usize,
            signature: &Signature,
            arguments: &[NativeValue],
            state: &mut NativeCallState,
        ) -> Result<RawReturn, InvocationError> {
            if !self.sealed || offset >= self.length {
                return Err(InvocationError::UnknownEntry);
            }
            let address = self.base.as_ptr().wrapping_add(offset).cast::<c_void>();
            let arguments = machine_arguments(arguments);
            // SAFETY: InstallableImage can only arise from the verified closed
            // encoder. Installation validates the entry offset/signature and
            // seals the complete mapping RX before this conversion and call.
            unsafe { invoke_typed(address, signature.result(), &arguments, state) }
        }

        pub(super) fn permissions(&self) -> Result<MappingPermissions, PermissionProbeError> {
            let maps = std::fs::read_to_string("/proc/self/maps")
                .map_err(|_| PermissionProbeError::ProcMapsUnavailable)?;
            let address = self.base.as_ptr() as usize;
            for line in maps.lines() {
                let mut fields = line.split_whitespace();
                let range = match fields.next() {
                    Some(value) => value,
                    None => continue,
                };
                let permissions = match fields.next() {
                    Some(value) => value,
                    None => continue,
                };
                let mut bounds = range.split('-');
                let start = match bounds
                    .next()
                    .and_then(|value| usize::from_str_radix(value, 16).ok())
                {
                    Some(value) => value,
                    None => continue,
                };
                let end = match bounds
                    .next()
                    .and_then(|value| usize::from_str_radix(value, 16).ok())
                {
                    Some(value) => value,
                    None => continue,
                };
                if start <= address && address < end {
                    let bytes = permissions.as_bytes();
                    if bytes.len() < 3 {
                        return Err(PermissionProbeError::MalformedPermissions);
                    }
                    return Ok(MappingPermissions {
                        readable: bytes[0] == b'r',
                        writable: bytes[1] == b'w',
                        executable: bytes[2] == b'x',
                    });
                }
            }
            Err(PermissionProbeError::MappingNotFound)
        }
    }

    impl Drop for Mapping {
        fn drop(&mut self) {
            // SAFETY: This Mapping owns this still-live mmap range exactly once.
            let _ = unsafe { munmap(self.base.as_ptr().cast::<c_void>(), self.allocation_length) };
        }
    }

    unsafe fn invoke_typed(
        address: *mut c_void,
        result: ValueType,
        arguments: &[MachineArgument],
        state: &mut NativeCallState,
    ) -> Result<RawReturn, InvocationError> {
        macro_rules! call {
            ($type:ty $(, $argument:expr)*) => {{
                // SAFETY: The caller validates that `address` is a sealed entry
                // with the exact closed SysV signature selected by this match.
                let function: $type = unsafe { std::mem::transmute(address) };
                function(state as *mut NativeCallState $(, $argument)*)
            }};
        }

        match (arguments, result) {
            ([], ValueType::I64 | ValueType::Bool | ValueType::Reference(_)) => Ok(
                RawReturn::Integer(call!(extern "C" fn(*mut NativeCallState) -> u64)),
            ),
            ([], ValueType::F64) => Ok(RawReturn::Float(call!(
                extern "C" fn(*mut NativeCallState) -> f64
            ))),
            ([], ValueType::Unit) => {
                call!(extern "C" fn(*mut NativeCallState));
                Ok(RawReturn::Unit)
            }
            (
                [MachineArgument::Integer(first)],
                ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
            ) => Ok(RawReturn::Integer(call!(
                extern "C" fn(*mut NativeCallState, u64) -> u64,
                *first
            ))),
            ([MachineArgument::Integer(first)], ValueType::F64) => Ok(RawReturn::Float(call!(
                extern "C" fn(*mut NativeCallState, u64) -> f64,
                *first
            ))),
            ([MachineArgument::Integer(first)], ValueType::Unit) => {
                call!(extern "C" fn(*mut NativeCallState, u64), *first);
                Ok(RawReturn::Unit)
            }
            (
                [MachineArgument::Float(first)],
                ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
            ) => Ok(RawReturn::Integer(call!(
                extern "C" fn(*mut NativeCallState, f64) -> u64,
                *first
            ))),
            ([MachineArgument::Float(first)], ValueType::F64) => Ok(RawReturn::Float(call!(
                extern "C" fn(*mut NativeCallState, f64) -> f64,
                *first
            ))),
            ([MachineArgument::Float(first)], ValueType::Unit) => {
                call!(extern "C" fn(*mut NativeCallState, f64), *first);
                Ok(RawReturn::Unit)
            }
            (
                [MachineArgument::Integer(first), MachineArgument::Integer(second)],
                ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
            ) => Ok(RawReturn::Integer(call!(
                extern "C" fn(*mut NativeCallState, u64, u64) -> u64,
                *first,
                *second
            ))),
            (
                [MachineArgument::Integer(first), MachineArgument::Integer(second)],
                ValueType::F64,
            ) => Ok(RawReturn::Float(call!(
                extern "C" fn(*mut NativeCallState, u64, u64) -> f64,
                *first,
                *second
            ))),
            (
                [MachineArgument::Integer(first), MachineArgument::Integer(second)],
                ValueType::Unit,
            ) => {
                call!(
                    extern "C" fn(*mut NativeCallState, u64, u64),
                    *first,
                    *second
                );
                Ok(RawReturn::Unit)
            }
            (
                [MachineArgument::Integer(first), MachineArgument::Float(second)],
                ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
            ) => Ok(RawReturn::Integer(call!(
                extern "C" fn(*mut NativeCallState, u64, f64) -> u64,
                *first,
                *second
            ))),
            ([MachineArgument::Integer(first), MachineArgument::Float(second)], ValueType::F64) => {
                Ok(RawReturn::Float(call!(
                    extern "C" fn(*mut NativeCallState, u64, f64) -> f64,
                    *first,
                    *second
                )))
            }
            (
                [MachineArgument::Integer(first), MachineArgument::Float(second)],
                ValueType::Unit,
            ) => {
                call!(
                    extern "C" fn(*mut NativeCallState, u64, f64),
                    *first,
                    *second
                );
                Ok(RawReturn::Unit)
            }
            (
                [MachineArgument::Float(first), MachineArgument::Integer(second)],
                ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
            ) => Ok(RawReturn::Integer(call!(
                extern "C" fn(*mut NativeCallState, f64, u64) -> u64,
                *first,
                *second
            ))),
            ([MachineArgument::Float(first), MachineArgument::Integer(second)], ValueType::F64) => {
                Ok(RawReturn::Float(call!(
                    extern "C" fn(*mut NativeCallState, f64, u64) -> f64,
                    *first,
                    *second
                )))
            }
            (
                [MachineArgument::Float(first), MachineArgument::Integer(second)],
                ValueType::Unit,
            ) => {
                call!(
                    extern "C" fn(*mut NativeCallState, f64, u64),
                    *first,
                    *second
                );
                Ok(RawReturn::Unit)
            }
            (
                [MachineArgument::Float(first), MachineArgument::Float(second)],
                ValueType::I64 | ValueType::Bool | ValueType::Reference(_),
            ) => Ok(RawReturn::Integer(call!(
                extern "C" fn(*mut NativeCallState, f64, f64) -> u64,
                *first,
                *second
            ))),
            ([MachineArgument::Float(first), MachineArgument::Float(second)], ValueType::F64) => {
                Ok(RawReturn::Float(call!(
                    extern "C" fn(*mut NativeCallState, f64, f64) -> f64,
                    *first,
                    *second
                )))
            }
            ([MachineArgument::Float(first), MachineArgument::Float(second)], ValueType::Unit) => {
                call!(
                    extern "C" fn(*mut NativeCallState, f64, f64),
                    *first,
                    *second
                );
                Ok(RawReturn::Unit)
            }
            _ => Err(InvocationError::UnsupportedSignature),
        }
    }
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
mod platform {
    use super::{
        InstallError, InvocationError, MappingPermissions, NativeCallState, NativeValue,
        PermissionProbeError, RawReturn, Signature,
    };

    pub(super) fn native_stack_reservation_fits(
        _rbp: *mut u8,
        _frame_bytes: usize,
        _guard_bytes: usize,
    ) -> bool {
        false
    }

    #[derive(Debug)]
    pub(super) struct Mapping;

    impl Mapping {
        pub(super) fn allocate_rw(_length: usize) -> Result<Self, InstallError> {
            Err(InstallError::UnsupportedPlatform)
        }

        pub(super) fn copy_from(&mut self, _bytes: &[u8]) -> Result<(), InstallError> {
            Err(InstallError::UnsupportedPlatform)
        }

        pub(super) fn write_absolute64(
            &mut self,
            _offset: usize,
            _address: usize,
        ) -> Result<(), InstallError> {
            Err(InstallError::UnsupportedPlatform)
        }

        pub(super) fn address_at(&self, _offset: usize) -> Result<usize, InstallError> {
            Err(InstallError::UnsupportedPlatform)
        }

        pub(super) fn seal_rx(&mut self) -> Result<(), InstallError> {
            Err(InstallError::UnsupportedPlatform)
        }

        pub(super) fn invoke(
            &self,
            _offset: usize,
            _signature: &Signature,
            _arguments: &[NativeValue],
            _state: &mut NativeCallState,
        ) -> Result<RawReturn, InvocationError> {
            Err(InvocationError::UnsupportedSignature)
        }

        pub(super) fn permissions(&self) -> Result<MappingPermissions, PermissionProbeError> {
            Err(PermissionProbeError::UnsupportedPlatform)
        }

        pub(super) fn allocation_length(&self) -> usize {
            0
        }

        pub(super) fn wx_transition_verified(&self) -> bool {
            false
        }
    }
}
