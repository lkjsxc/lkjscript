use std::collections::HashSet;
use std::fmt;

use crate::plan::{
    FunctionId, RuntimeCallSlot, Signature, SourceFunctionId, SourceOrigin, TrapCode, ValueType,
};

pub const CURRENT_SEMANTIC_ABI_VERSION: u16 = 1;
pub const CURRENT_NATIVE_ABI_VERSION: u16 = 1;
pub const CURRENT_RUNTIME_ABI_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbiVersions {
    semantic: u16,
    native: u16,
    runtime: u16,
}

impl AbiVersions {
    #[must_use]
    pub const fn new(semantic: u16, native: u16, runtime: u16) -> Self {
        Self {
            semantic,
            native,
            runtime,
        }
    }

    #[must_use]
    pub const fn current() -> Self {
        Self::new(
            CURRENT_SEMANTIC_ABI_VERSION,
            CURRENT_NATIVE_ABI_VERSION,
            CURRENT_RUNTIME_ABI_VERSION,
        )
    }

    #[must_use]
    pub const fn semantic(self) -> u16 {
        self.semantic
    }

    #[must_use]
    pub const fn native(self) -> u16 {
        self.native
    }

    #[must_use]
    pub const fn runtime(self) -> u16 {
        self.runtime
    }
}

impl Default for AbiVersions {
    fn default() -> Self {
        Self::current()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NativeValue {
    I64(i64),
    F64Bits(u64),
    Bool(bool),
    Unit,
}

impl NativeValue {
    #[must_use]
    pub fn f64(value: f64) -> Self {
        Self::F64Bits(value.to_bits())
    }

    #[must_use]
    pub const fn value_type(self) -> ValueType {
        match self {
            Self::I64(_) => ValueType::I64,
            Self::F64Bits(_) => ValueType::F64,
            Self::Bool(_) => ValueType::Bool,
            Self::Unit => ValueType::Unit,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntryMetadata {
    function: FunctionId,
    source_function: SourceFunctionId,
    signature: Signature,
    offset: u32,
    end: u32,
}

impl EntryMetadata {
    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn source_function(&self) -> SourceFunctionId {
        self.source_function
    }

    #[must_use]
    pub fn signature(&self) -> &Signature {
        &self.signature
    }

    #[must_use]
    pub const fn offset(&self) -> u32 {
        self.offset
    }

    #[must_use]
    pub const fn end(&self) -> u32 {
        self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelocationKind {
    Absolute64,
}

impl RelocationKind {
    #[must_use]
    pub const fn width(self) -> usize {
        match self {
            Self::Absolute64 => 8,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelocationTarget {
    Function(FunctionId),
    Runtime(RuntimeCallSlot),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Relocation {
    offset: u32,
    kind: RelocationKind,
    target: RelocationTarget,
}

impl Relocation {
    #[must_use]
    pub const fn offset(self) -> u32 {
        self.offset
    }

    #[must_use]
    pub const fn kind(self) -> RelocationKind {
        self.kind
    }

    #[must_use]
    pub const fn target(self) -> RelocationTarget {
        self.target
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameFacts {
    function: FunctionId,
    frame_bytes: u32,
    value_slots: u32,
    local_slots: u32,
    outgoing_machine_arguments: u8,
    uses_red_zone: bool,
    call_site_aligned_16: bool,
}

impl FrameFacts {
    #[must_use]
    pub const fn function(self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn frame_bytes(self) -> u32 {
        self.frame_bytes
    }

    #[must_use]
    pub const fn value_slots(self) -> u32 {
        self.value_slots
    }

    #[must_use]
    pub const fn local_slots(self) -> u32 {
        self.local_slots
    }

    #[must_use]
    pub const fn outgoing_machine_arguments(self) -> u8 {
        self.outgoing_machine_arguments
    }

    #[must_use]
    pub const fn uses_red_zone(self) -> bool {
        self.uses_red_zone
    }

    #[must_use]
    pub const fn call_site_aligned_16(self) -> bool {
        self.call_site_aligned_16
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarStackMap {
    reference_slots: Vec<u32>,
}

impl ScalarStackMap {
    #[must_use]
    pub fn reference_slots(&self) -> &[u32] {
        &self.reference_slots
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Safepoint {
    function: FunctionId,
    code_offset: u32,
    stack_map: ScalarStackMap,
}

impl Safepoint {
    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn code_offset(&self) -> u32 {
        self.code_offset
    }

    #[must_use]
    pub const fn stack_map(&self) -> &ScalarStackMap {
        &self.stack_map
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceMapEntry {
    function: FunctionId,
    code_start: u32,
    code_end: u32,
    source: Option<SourceOrigin>,
}

impl SourceMapEntry {
    #[must_use]
    pub const fn function(self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn code_start(self) -> u32 {
        self.code_start
    }

    #[must_use]
    pub const fn code_end(self) -> u32 {
        self.code_end
    }

    #[must_use]
    pub const fn source(self) -> Option<SourceOrigin> {
        self.source
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TrapMapEntry {
    function: FunctionId,
    code_offset: u32,
    trap: TrapCode,
}

impl TrapMapEntry {
    #[must_use]
    pub const fn function(self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn code_offset(self) -> u32 {
        self.code_offset
    }

    #[must_use]
    pub const fn trap(self) -> TrapCode {
        self.trap
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutcomeKind {
    Return,
    Trap(TrapCode),
    Exit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutcomeMapEntry {
    function: FunctionId,
    code_offset: u32,
    outcome: OutcomeKind,
}

impl OutcomeMapEntry {
    #[must_use]
    pub const fn function(self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn code_offset(self) -> u32 {
        self.code_offset
    }

    #[must_use]
    pub const fn outcome(self) -> OutcomeKind {
        self.outcome
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodeAccounting {
    code_bytes: u64,
    metadata_bytes: u64,
    work_units: u64,
}

impl CodeAccounting {
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
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ImageIntegrityError {
    EmptyCode,
    CodeAccountingMismatch,
    MetadataAccountingMismatch,
    EntryRange,
    DuplicateEntry,
    RelocationRange,
    RelocationTarget,
    FrameFacts,
    Safepoint,
    SourceMap,
    TrapMap,
    OutcomeMap,
    RuntimeCallSet,
}

impl fmt::Display for ImageIntegrityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyCode => "installable image has no code",
            Self::CodeAccountingMismatch => "installable image code accounting is inconsistent",
            Self::MetadataAccountingMismatch => {
                "installable image metadata accounting is inconsistent"
            }
            Self::EntryRange => "installable image entry range is invalid",
            Self::DuplicateEntry => "installable image has duplicate entries",
            Self::RelocationRange => "installable image relocation range is invalid",
            Self::RelocationTarget => "installable image relocation target is invalid",
            Self::FrameFacts => "installable image frame facts are invalid",
            Self::Safepoint => "installable image safepoint is invalid",
            Self::SourceMap => "installable image source map is invalid",
            Self::TrapMap => "installable image trap map is invalid",
            Self::OutcomeMap => "installable image outcome map is invalid",
            Self::RuntimeCallSet => "installable image runtime-call set is invalid",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for ImageIntegrityError {}

#[derive(Debug)]
pub struct InstallableImage {
    bytes: Box<[u8]>,
    entries: Box<[EntryMetadata]>,
    relocations: Box<[Relocation]>,
    runtime_calls: Box<[RuntimeCallSlot]>,
    frames: Box<[FrameFacts]>,
    safepoints: Box<[Safepoint]>,
    source_map: Box<[SourceMapEntry]>,
    trap_map: Box<[TrapMapEntry]>,
    outcome_map: Box<[OutcomeMapEntry]>,
    accounting: CodeAccounting,
    versions: AbiVersions,
}

impl InstallableImage {
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    #[must_use]
    pub fn entries(&self) -> &[EntryMetadata] {
        &self.entries
    }

    #[must_use]
    pub fn relocations(&self) -> &[Relocation] {
        &self.relocations
    }

    #[must_use]
    pub fn runtime_calls(&self) -> &[RuntimeCallSlot] {
        &self.runtime_calls
    }

    #[must_use]
    pub fn frames(&self) -> &[FrameFacts] {
        &self.frames
    }

    #[must_use]
    pub fn safepoints(&self) -> &[Safepoint] {
        &self.safepoints
    }

    #[must_use]
    pub fn source_map(&self) -> &[SourceMapEntry] {
        &self.source_map
    }

    #[must_use]
    pub fn trap_map(&self) -> &[TrapMapEntry] {
        &self.trap_map
    }

    #[must_use]
    pub fn outcome_map(&self) -> &[OutcomeMapEntry] {
        &self.outcome_map
    }

    #[must_use]
    pub const fn accounting(&self) -> CodeAccounting {
        self.accounting
    }

    #[must_use]
    pub const fn versions(&self) -> AbiVersions {
        self.versions
    }

    pub fn validate_integrity(&self) -> Result<(), ImageIntegrityError> {
        if self.bytes.is_empty() {
            return Err(ImageIntegrityError::EmptyCode);
        }
        if usize::try_from(self.accounting.code_bytes).ok() != Some(self.bytes.len()) {
            return Err(ImageIntegrityError::CodeAccountingMismatch);
        }
        let expected_metadata = metadata_bytes(MetadataSlices {
            entries: &self.entries,
            relocations: &self.relocations,
            runtime_calls: &self.runtime_calls,
            frames: &self.frames,
            safepoints: &self.safepoints,
            source_map: &self.source_map,
            trap_map: &self.trap_map,
            outcome_map: &self.outcome_map,
        })
        .ok_or(ImageIntegrityError::MetadataAccountingMismatch)?;
        if expected_metadata != self.accounting.metadata_bytes {
            return Err(ImageIntegrityError::MetadataAccountingMismatch);
        }

        let mut functions = HashSet::new();
        for entry in &self.entries {
            if entry.offset >= entry.end || entry.end as usize > self.bytes.len() {
                return Err(ImageIntegrityError::EntryRange);
            }
            if !functions.insert(entry.function) {
                return Err(ImageIntegrityError::DuplicateEntry);
            }
        }
        if self.entries.is_empty() {
            return Err(ImageIntegrityError::EntryRange);
        }

        let runtime_calls: HashSet<_> = self.runtime_calls.iter().copied().collect();
        if runtime_calls.len() != self.runtime_calls.len() {
            return Err(ImageIntegrityError::RuntimeCallSet);
        }
        for relocation in &self.relocations {
            let start = relocation.offset as usize;
            let end = start
                .checked_add(relocation.kind.width())
                .ok_or(ImageIntegrityError::RelocationRange)?;
            if end > self.bytes.len() {
                return Err(ImageIntegrityError::RelocationRange);
            }
            match relocation.target {
                RelocationTarget::Function(function) => {
                    if !functions.contains(&function) {
                        return Err(ImageIntegrityError::RelocationTarget);
                    }
                }
                RelocationTarget::Runtime(slot) => {
                    if !runtime_calls.contains(&slot) {
                        return Err(ImageIntegrityError::RelocationTarget);
                    }
                }
            }
        }
        for frame in &self.frames {
            if !functions.contains(&frame.function)
                || frame.frame_bytes % 16 != 0
                || frame.uses_red_zone
                || !frame.call_site_aligned_16
            {
                return Err(ImageIntegrityError::FrameFacts);
            }
        }
        if self.frames.len() != self.entries.len() {
            return Err(ImageIntegrityError::FrameFacts);
        }
        for safepoint in &self.safepoints {
            if !offset_in_function(&self.entries, safepoint.function, safepoint.code_offset)
                || !safepoint.stack_map.reference_slots.is_empty()
            {
                return Err(ImageIntegrityError::Safepoint);
            }
        }
        for source in &self.source_map {
            if source.code_start >= source.code_end
                || !range_in_function(
                    &self.entries,
                    source.function,
                    source.code_start,
                    source.code_end,
                )
            {
                return Err(ImageIntegrityError::SourceMap);
            }
        }
        for trap in &self.trap_map {
            if !offset_in_function(&self.entries, trap.function, trap.code_offset) {
                return Err(ImageIntegrityError::TrapMap);
            }
        }
        for outcome in &self.outcome_map {
            if !offset_in_function(&self.entries, outcome.function, outcome.code_offset) {
                return Err(ImageIntegrityError::OutcomeMap);
            }
        }
        Ok(())
    }

    pub(crate) fn new(parts: ImageParts) -> Result<Self, ImageIntegrityError> {
        let metadata_bytes = metadata_bytes(MetadataSlices {
            entries: &parts.entries,
            relocations: &parts.relocations,
            runtime_calls: &parts.runtime_calls,
            frames: &parts.frames,
            safepoints: &parts.safepoints,
            source_map: &parts.source_map,
            trap_map: &parts.trap_map,
            outcome_map: &parts.outcome_map,
        })
        .ok_or(ImageIntegrityError::MetadataAccountingMismatch)?;
        let code_bytes = u64::try_from(parts.bytes.len())
            .map_err(|_| ImageIntegrityError::CodeAccountingMismatch)?;
        let image = Self {
            bytes: parts.bytes.into_boxed_slice(),
            entries: parts.entries.into_boxed_slice(),
            relocations: parts.relocations.into_boxed_slice(),
            runtime_calls: parts.runtime_calls.into_boxed_slice(),
            frames: parts.frames.into_boxed_slice(),
            safepoints: parts.safepoints.into_boxed_slice(),
            source_map: parts.source_map.into_boxed_slice(),
            trap_map: parts.trap_map.into_boxed_slice(),
            outcome_map: parts.outcome_map.into_boxed_slice(),
            accounting: CodeAccounting {
                code_bytes,
                metadata_bytes,
                work_units: parts.work_units,
            },
            versions: parts.versions,
        };
        image.validate_integrity()?;
        Ok(image)
    }
}

pub(crate) struct ImageParts {
    pub(crate) bytes: Vec<u8>,
    pub(crate) entries: Vec<EntryMetadata>,
    pub(crate) relocations: Vec<Relocation>,
    pub(crate) runtime_calls: Vec<RuntimeCallSlot>,
    pub(crate) frames: Vec<FrameFacts>,
    pub(crate) safepoints: Vec<Safepoint>,
    pub(crate) source_map: Vec<SourceMapEntry>,
    pub(crate) trap_map: Vec<TrapMapEntry>,
    pub(crate) outcome_map: Vec<OutcomeMapEntry>,
    pub(crate) work_units: u64,
    pub(crate) versions: AbiVersions,
}

pub(crate) fn entry_metadata(
    function: FunctionId,
    source_function: SourceFunctionId,
    signature: Signature,
    offset: u32,
    end: u32,
) -> EntryMetadata {
    EntryMetadata {
        function,
        source_function,
        signature,
        offset,
        end,
    }
}

pub(crate) fn relocation(
    offset: u32,
    kind: RelocationKind,
    target: RelocationTarget,
) -> Relocation {
    Relocation {
        offset,
        kind,
        target,
    }
}

pub(crate) fn frame_facts(
    function: FunctionId,
    frame_bytes: u32,
    value_slots: u32,
    local_slots: u32,
    outgoing_machine_arguments: u8,
) -> FrameFacts {
    FrameFacts {
        function,
        frame_bytes,
        value_slots,
        local_slots,
        outgoing_machine_arguments,
        uses_red_zone: false,
        call_site_aligned_16: true,
    }
}

pub(crate) fn scalar_safepoint(function: FunctionId, code_offset: u32) -> Safepoint {
    Safepoint {
        function,
        code_offset,
        stack_map: ScalarStackMap {
            reference_slots: Vec::new(),
        },
    }
}

pub(crate) fn source_map_entry(
    function: FunctionId,
    code_start: u32,
    code_end: u32,
    source: Option<SourceOrigin>,
) -> SourceMapEntry {
    SourceMapEntry {
        function,
        code_start,
        code_end,
        source,
    }
}

pub(crate) fn trap_map_entry(
    function: FunctionId,
    code_offset: u32,
    trap: TrapCode,
) -> TrapMapEntry {
    TrapMapEntry {
        function,
        code_offset,
        trap,
    }
}

pub(crate) fn outcome_map_entry(
    function: FunctionId,
    code_offset: u32,
    outcome: OutcomeKind,
) -> OutcomeMapEntry {
    OutcomeMapEntry {
        function,
        code_offset,
        outcome,
    }
}

fn offset_in_function(entries: &[EntryMetadata], function: FunctionId, offset: u32) -> bool {
    entries
        .iter()
        .any(|entry| entry.function == function && entry.offset <= offset && offset < entry.end)
}

fn range_in_function(
    entries: &[EntryMetadata],
    function: FunctionId,
    start: u32,
    end: u32,
) -> bool {
    entries
        .iter()
        .any(|entry| entry.function == function && entry.offset <= start && end <= entry.end)
}

struct MetadataSlices<'a> {
    entries: &'a [EntryMetadata],
    relocations: &'a [Relocation],
    runtime_calls: &'a [RuntimeCallSlot],
    frames: &'a [FrameFacts],
    safepoints: &'a [Safepoint],
    source_map: &'a [SourceMapEntry],
    trap_map: &'a [TrapMapEntry],
    outcome_map: &'a [OutcomeMapEntry],
}

fn metadata_bytes(parts: MetadataSlices<'_>) -> Option<u64> {
    let mut bytes = 64_u64;
    bytes = add_records(bytes, parts.entries.len(), 32)?;
    for entry in parts.entries {
        bytes = add_records(bytes, entry.signature.parameters().len(), 1)?;
    }
    bytes = add_records(bytes, parts.relocations.len(), 24)?;
    bytes = add_records(bytes, parts.runtime_calls.len(), 8)?;
    bytes = add_records(bytes, parts.frames.len(), 32)?;
    bytes = add_records(bytes, parts.safepoints.len(), 24)?;
    bytes = add_records(bytes, parts.source_map.len(), 24)?;
    bytes = add_records(bytes, parts.trap_map.len(), 16)?;
    add_records(bytes, parts.outcome_map.len(), 16)
}

fn add_records(bytes: u64, count: usize, record_bytes: u64) -> Option<u64> {
    let count = u64::try_from(count).ok()?;
    bytes.checked_add(count.checked_mul(record_bytes)?)
}
