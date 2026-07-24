use std::cell::Cell;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

use lkjscript_native::{
    AbiVersions, FunctionId, ImageIntegrityError, InstallableImage, NativeValue, RelocationTarget,
    RuntimeCallSlot, Signature, TrapCode, ValueType,
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
        }
    }
}

impl std::error::Error for InvocationError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InvocationOutcome {
    Returned(NativeValue),
    Trapped(TrapCode),
    Exited(i64),
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
pub struct ExecutableInstaller {
    limits: ExecutableLimits,
    usage: Cell<ExecutableUsage>,
    not_send_or_sync: PhantomData<Rc<()>>,
}

impl ExecutableInstaller {
    #[must_use]
    pub fn new(limits: ExecutableLimits) -> Self {
        Self {
            limits,
            usage: Cell::new(ExecutableUsage::default()),
            not_send_or_sync: PhantomData,
        }
    }

    #[must_use]
    pub fn usage(&self) -> ExecutableUsage {
        self.usage.get()
    }

    pub fn install<'installer>(
        &'installer self,
        image: InstallableImage,
    ) -> Result<InstalledImage<'installer>, InstallError> {
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
            self.limits.max_object_code_bytes,
            ExecutableLimitKind::ObjectCodeBytes,
        )?;
        check_object_limit(
            accounting.metadata_bytes(),
            self.limits.max_object_metadata_bytes,
            ExecutableLimitKind::ObjectMetadataBytes,
        )?;
        check_object_limit(
            accounting.work_units(),
            self.limits.max_object_work_units,
            ExecutableLimitKind::ObjectWorkUnits,
        )?;
        let next_usage = checked_usage(self.usage.get(), accounting, self.limits)?;
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
        self.usage.set(next_usage);
        Ok(InstalledImage {
            installer: self,
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
pub struct InstalledImage<'installer> {
    installer: &'installer ExecutableInstaller,
    image: InstallableImage,
    mapping: platform::Mapping,
    usage: ExecutableUsage,
    not_send_or_sync: PhantomData<Rc<()>>,
}

impl InstalledImage<'_> {
    #[must_use]
    pub fn entries(&self) -> &[lkjscript_native::EntryMetadata] {
        self.image.entries()
    }

    pub fn invoke(
        &self,
        entry: FunctionId,
        arguments: &[NativeValue],
    ) -> Result<InvocationOutcome, InvocationError> {
        let entry = self
            .image
            .entries()
            .iter()
            .find(|candidate| candidate.function() == entry)
            .ok_or(InvocationError::UnknownEntry)?;
        validate_arguments(entry.signature(), arguments)?;
        let mut state = NativeCallState::default();
        let raw = self.mapping.invoke(
            entry.offset() as usize,
            entry.signature(),
            arguments,
            &mut state,
        )?;
        match state.status {
            0 => Ok(InvocationOutcome::Returned(
                raw.into_value(entry.signature().result())?,
            )),
            1 => Ok(InvocationOutcome::Trapped(match state.trap {
                1 => TrapCode::I64Overflow,
                2 => TrapCode::DivisionByZero,
                3 => TrapCode::Explicit,
                other => return Err(InvocationError::InvalidNativeTrap(other)),
            })),
            2 => Ok(InvocationOutcome::Exited(state.payload)),
            other => Err(InvocationError::InvalidNativeStatus(other)),
        }
    }

    pub fn permissions(&self) -> Result<MappingPermissions, PermissionProbeError> {
        self.mapping.permissions()
    }

    #[must_use]
    pub fn wx_transition_verified(&self) -> bool {
        self.mapping.wx_transition_verified()
    }
}

impl Drop for InstalledImage<'_> {
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

#[repr(C)]
#[derive(Default)]
struct NativeCallState {
    status: u32,
    trap: u32,
    payload: i64,
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
            _ => Err(InvocationError::UnsupportedSignature),
        }
    }
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
        })
        .collect()
}

fn runtime_symbol(slot: RuntimeCallSlot) -> usize {
    match slot {
        RuntimeCallSlot::IdentityI64V1 => runtime_identity_i64_v1 as *const () as usize,
    }
}

extern "C" fn runtime_identity_i64_v1(_state: *mut NativeCallState, value: u64) -> u64 {
    value
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
    }

    #[derive(Debug)]
    pub(super) struct Mapping {
        base: NonNull<u8>,
        length: usize,
        sealed: bool,
        wx_transition_verified: bool,
    }

    impl Mapping {
        pub(super) fn allocate_rw(length: usize) -> Result<Self, InstallError> {
            if length == 0 {
                return Err(InstallError::AllocationFailed);
            }
            // SAFETY: The null hint, anonymous descriptor, zero offset, and nonzero
            // length satisfy mmap. The returned mapping is checked before ownership.
            let pointer = unsafe {
                mmap(
                    std::ptr::null_mut(),
                    length,
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
                    self.length,
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
            let _ = unsafe { munmap(self.base.as_ptr().cast::<c_void>(), self.length) };
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
            ([], ValueType::I64 | ValueType::Bool) => Ok(RawReturn::Integer(call!(
                extern "C" fn(*mut NativeCallState) -> u64
            ))),
            ([], ValueType::F64) => Ok(RawReturn::Float(call!(
                extern "C" fn(*mut NativeCallState) -> f64
            ))),
            ([], ValueType::Unit) => {
                call!(extern "C" fn(*mut NativeCallState));
                Ok(RawReturn::Unit)
            }
            ([MachineArgument::Integer(first)], ValueType::I64 | ValueType::Bool) => {
                Ok(RawReturn::Integer(call!(
                    extern "C" fn(*mut NativeCallState, u64) -> u64,
                    *first
                )))
            }
            ([MachineArgument::Integer(first)], ValueType::F64) => Ok(RawReturn::Float(call!(
                extern "C" fn(*mut NativeCallState, u64) -> f64,
                *first
            ))),
            ([MachineArgument::Integer(first)], ValueType::Unit) => {
                call!(extern "C" fn(*mut NativeCallState, u64), *first);
                Ok(RawReturn::Unit)
            }
            ([MachineArgument::Float(first)], ValueType::I64 | ValueType::Bool) => {
                Ok(RawReturn::Integer(call!(
                    extern "C" fn(*mut NativeCallState, f64) -> u64,
                    *first
                )))
            }
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
                ValueType::I64 | ValueType::Bool,
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
                ValueType::I64 | ValueType::Bool,
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
                ValueType::I64 | ValueType::Bool,
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
                ValueType::I64 | ValueType::Bool,
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

        pub(super) fn wx_transition_verified(&self) -> bool {
            false
        }
    }
}
