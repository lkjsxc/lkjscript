use std::collections::HashSet;
use std::fmt;
use std::marker::PhantomData;
use std::rc::Rc;

use crate::plan::{
    FunctionId, ReferenceType, RuntimeCallSlot, Signature, SourceFunctionId, SourceOrigin,
    TrapCode, ValueType,
};

pub const CURRENT_SEMANTIC_ABI_VERSION: u16 = 1;
pub const CURRENT_NATIVE_ABI_VERSION: u16 = 2;
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

/// Copyable, worker-local runtime-adapter token. The opaque word is never
/// interpreted as an object address by the native ABI. The ownership marker
/// intentionally makes this token non-Send and non-Sync; it is not a source
/// reference or an independently owned heap value.
///
/// ```compile_fail
/// use lkjscript_native::NativeReference;
/// let reference = NativeReference::buf(7);
/// std::thread::spawn(move || reference.opaque_word());
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NativeReference {
    reference_type: ReferenceType,
    opaque_word: u64,
    worker_owner: PhantomData<Rc<()>>,
}

impl NativeReference {
    #[must_use]
    pub const fn new(reference_type: ReferenceType, opaque_word: u64) -> Self {
        Self {
            reference_type,
            opaque_word,
            worker_owner: PhantomData,
        }
    }

    #[must_use]
    pub const fn buf(opaque_word: u64) -> Self {
        Self::new(ReferenceType::Buf, opaque_word)
    }

    #[must_use]
    pub const fn reference_type(self) -> ReferenceType {
        self.reference_type
    }

    #[must_use]
    pub const fn opaque_word(self) -> u64 {
        self.opaque_word
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NativeValue {
    I64(i64),
    F64Bits(u64),
    Bool(bool),
    Unit,
    Reference(NativeReference),
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
            Self::Reference(reference) => ValueType::Reference(reference.reference_type()),
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FrameHomeKind {
    Local(u32),
    Value(u32),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FrameHome {
    kind: FrameHomeKind,
    value_type: ValueType,
    rbp_displacement: i32,
}

impl FrameHome {
    #[must_use]
    pub const fn kind(self) -> FrameHomeKind {
        self.kind
    }

    #[must_use]
    pub const fn value_type(self) -> ValueType {
        self.value_type
    }

    #[must_use]
    pub const fn rbp_displacement(self) -> i32 {
        self.rbp_displacement
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameFacts {
    function: FunctionId,
    frame_bytes: u32,
    value_slots: u32,
    local_slots: u32,
    outgoing_machine_arguments: u8,
    uses_red_zone: bool,
    call_site_aligned_16: bool,
    homes: Vec<FrameHome>,
}

impl FrameFacts {
    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn frame_bytes(&self) -> u32 {
        self.frame_bytes
    }

    #[must_use]
    pub const fn value_slots(&self) -> u32 {
        self.value_slots
    }

    #[must_use]
    pub const fn local_slots(&self) -> u32 {
        self.local_slots
    }

    #[must_use]
    pub const fn outgoing_machine_arguments(&self) -> u8 {
        self.outgoing_machine_arguments
    }

    #[must_use]
    pub const fn uses_red_zone(&self) -> bool {
        self.uses_red_zone
    }

    #[must_use]
    pub const fn call_site_aligned_16(&self) -> bool {
        self.call_site_aligned_16
    }

    #[must_use]
    pub fn homes(&self) -> &[FrameHome] {
        &self.homes
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RootLocation {
    rbp_displacement: i32,
    kind: FrameHomeKind,
    reference_type: ReferenceType,
}

impl RootLocation {
    #[must_use]
    pub const fn rbp_displacement(self) -> i32 {
        self.rbp_displacement
    }

    #[must_use]
    pub const fn kind(self) -> FrameHomeKind {
        self.kind
    }

    #[must_use]
    pub const fn reference_type(self) -> ReferenceType {
        self.reference_type
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactStackMap {
    roots: Vec<RootLocation>,
}

impl ExactStackMap {
    #[must_use]
    pub fn roots(&self) -> &[RootLocation] {
        &self.roots
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Safepoint {
    id: u32,
    function: FunctionId,
    code_offset: u32,
    stack_map: ExactStackMap,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RootMapRequirement {
    id: u32,
    function: FunctionId,
    roots: Vec<RootLocation>,
}

impl Safepoint {
    #[must_use]
    pub const fn id(&self) -> u32 {
        self.id
    }

    #[must_use]
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    #[must_use]
    pub const fn code_offset(&self) -> u32 {
        self.code_offset
    }

    #[must_use]
    pub const fn stack_map(&self) -> &ExactStackMap {
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
    DeadlineExceeded,
    ResourceLimitExceeded,
    HostFailure,
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
    RootRequirement,
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
            Self::RootRequirement => {
                "installable image stack map disagrees with its verifier requirement"
            }
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
    root_requirements: Box<[RootMapRequirement]>,
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
            root_requirements: &self.root_requirements,
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
        let mut relocated_runtime_calls = HashSet::new();
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
                    relocated_runtime_calls.insert(slot);
                }
            }
        }
        if relocated_runtime_calls != runtime_calls {
            return Err(ImageIntegrityError::RuntimeCallSet);
        }
        let mut frame_functions = HashSet::new();
        for frame in &self.frames {
            let entry = self
                .entries
                .iter()
                .find(|entry| entry.function == frame.function)
                .ok_or(ImageIntegrityError::FrameFacts)?;
            if !frame_functions.insert(frame.function)
                || frame.frame_bytes == 0
                || frame.frame_bytes % 16 != 0
                || frame.uses_red_zone
                || !frame.call_site_aligned_16
                || !valid_frame_homes(frame, entry)
            {
                return Err(ImageIntegrityError::FrameFacts);
            }
        }
        if self.frames.len() != self.entries.len() {
            return Err(ImageIntegrityError::FrameFacts);
        }
        let mut call_offsets = HashSet::new();
        for (expected_id, safepoint) in self.safepoints.iter().enumerate() {
            if safepoint.id as usize != expected_id
                || !offset_in_function(&self.entries, safepoint.function, safepoint.code_offset)
                || !call_offsets.insert((safepoint.function, safepoint.code_offset))
            {
                return Err(ImageIntegrityError::Safepoint);
            }
            let frame = self
                .frames
                .iter()
                .find(|frame| frame.function == safepoint.function)
                .ok_or(ImageIntegrityError::Safepoint)?;
            if !valid_stack_map(frame, &safepoint.stack_map) {
                return Err(ImageIntegrityError::Safepoint);
            }
        }
        if self.root_requirements.len() != self.safepoints.len()
            || self.root_requirements.iter().zip(&self.safepoints).any(
                |(requirement, safepoint)| {
                    requirement.id != safepoint.id
                        || requirement.function != safepoint.function
                        || requirement.roots != safepoint.stack_map.roots
                },
            )
        {
            return Err(ImageIntegrityError::RootRequirement);
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
            root_requirements: &parts.root_requirements,
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
            root_requirements: parts.root_requirements.into_boxed_slice(),
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
    pub(crate) root_requirements: Vec<RootMapRequirement>,
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
    homes: Vec<FrameHome>,
) -> FrameFacts {
    FrameFacts {
        function,
        frame_bytes,
        value_slots,
        local_slots,
        outgoing_machine_arguments,
        uses_red_zone: false,
        call_site_aligned_16: true,
        homes,
    }
}

pub(crate) const fn frame_home(
    kind: FrameHomeKind,
    value_type: ValueType,
    rbp_displacement: i32,
) -> FrameHome {
    FrameHome {
        kind,
        value_type,
        rbp_displacement,
    }
}

pub(crate) fn exact_safepoint(
    id: u32,
    function: FunctionId,
    code_offset: u32,
    roots: Vec<RootLocation>,
) -> Safepoint {
    Safepoint {
        id,
        function,
        code_offset,
        stack_map: ExactStackMap { roots },
    }
}

pub(crate) fn root_map_requirement(
    id: u32,
    function: FunctionId,
    roots: Vec<RootLocation>,
) -> RootMapRequirement {
    RootMapRequirement {
        id,
        function,
        roots,
    }
}

pub(crate) const fn root_location(
    rbp_displacement: i32,
    kind: FrameHomeKind,
    reference_type: ReferenceType,
) -> RootLocation {
    RootLocation {
        rbp_displacement,
        kind,
        reference_type,
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

fn valid_frame_homes(frame: &FrameFacts, entry: &EntryMetadata) -> bool {
    let expected = match usize::try_from(frame.value_slots).ok().and_then(|values| {
        usize::try_from(frame.local_slots)
            .ok()
            .and_then(|locals| values.checked_add(locals))
    }) {
        Some(expected) => expected,
        None => return false,
    };
    if frame.homes.len() != expected {
        return false;
    }
    let mut kinds = HashSet::new();
    for home in &frame.homes {
        let displacement = match u32::try_from(home.rbp_displacement.checked_neg().unwrap_or(0)) {
            Ok(displacement) => displacement,
            Err(_) => return false,
        };
        if home.rbp_displacement > -16
            || home.rbp_displacement % 8 != 0
            || displacement > frame.frame_bytes
            || canonical_home_displacement(frame, home.kind) != Some(home.rbp_displacement)
            || !kinds.insert(home.kind)
        {
            return false;
        }
        match home.kind {
            FrameHomeKind::Local(index) if index < frame.local_slots => {}
            FrameHomeKind::Value(index) if index < frame.value_slots => {
                if entry
                    .signature
                    .parameters()
                    .get(index as usize)
                    .is_some_and(|parameter| *parameter != home.value_type)
                {
                    return false;
                }
            }
            FrameHomeKind::Local(_) | FrameHomeKind::Value(_) => return false,
        }
    }
    true
}

fn canonical_home_displacement(frame: &FrameFacts, kind: FrameHomeKind) -> Option<i32> {
    let slot = match kind {
        FrameHomeKind::Local(index) => u64::from(index).checked_add(1)?,
        FrameHomeKind::Value(index) => u64::from(frame.local_slots)
            .checked_add(u64::from(index))?
            .checked_add(1)?,
    };
    let bytes = slot.checked_add(1)?.checked_mul(8)?;
    i32::try_from(bytes).ok()?.checked_neg()
}

fn valid_stack_map(frame: &FrameFacts, map: &ExactStackMap) -> bool {
    if map.roots.windows(2).any(|pair| pair[0] >= pair[1]) {
        return false;
    }
    map.roots.iter().all(|root| {
        frame.homes.iter().any(|home| {
            home.kind == root.kind
                && home.rbp_displacement == root.rbp_displacement
                && home.value_type == ValueType::Reference(root.reference_type)
        })
    })
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
    root_requirements: &'a [RootMapRequirement],
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
    for frame in parts.frames {
        bytes = add_records(bytes, frame.homes.len(), 16)?;
    }
    bytes = add_records(bytes, parts.safepoints.len(), 24)?;
    for safepoint in parts.safepoints {
        bytes = add_records(bytes, safepoint.stack_map.roots.len(), 16)?;
    }
    bytes = add_records(bytes, parts.root_requirements.len(), 16)?;
    for requirement in parts.root_requirements {
        bytes = add_records(bytes, requirement.roots.len(), 16)?;
    }
    bytes = add_records(bytes, parts.source_map.len(), 24)?;
    bytes = add_records(bytes, parts.trap_map.len(), 16)?;
    add_records(bytes, parts.outcome_map.len(), 16)
}

fn add_records(bytes: u64, count: usize, record_bytes: u64) -> Option<u64> {
    let count = u64::try_from(count).ok()?;
    bytes.checked_add(count.checked_mul(record_bytes)?)
}

#[cfg(test)]
mod tests {
    use crate::{
        encode, BackendLimits, EncodingConfig, MachinePlanBuilder, ReferenceType, RuntimeCallSlot,
        Signature, SourceFunctionId, ValueType,
    };

    #[test]
    fn integrity_rejects_out_of_frame_root_without_accounting_change(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let buf = ValueType::Reference(ReferenceType::Buf);
        let mut plan = MachinePlanBuilder::new();
        let function =
            plan.declare_function(SourceFunctionId::new(1), Signature::new(vec![buf], buf)?)?;
        let mut builder = plan.function_builder(function)?;
        let entry = builder.create_block()?;
        builder.set_entry(entry)?;
        let input = builder.parameter(0)?;
        let collected =
            builder.runtime_call(entry, RuntimeCallSlot::CollectReferenceV1, vec![input])?;
        builder.return_value(entry, collected)?;
        plan.define_function(builder.finish())?;
        let mut image = encode(
            plan.verify(BackendLimits::default())?,
            EncodingConfig::default(),
        )?;
        image.accounting.metadata_bytes -= 1;
        assert_eq!(
            image.validate_integrity(),
            Err(super::ImageIntegrityError::MetadataAccountingMismatch)
        );
        image.accounting.metadata_bytes += 1;
        image.safepoints[0].id = 7;
        assert_eq!(
            image.validate_integrity(),
            Err(super::ImageIntegrityError::Safepoint)
        );
        image.safepoints[0].id = 0;
        image.safepoints[0].stack_map.roots[0].rbp_displacement = -8;
        assert_eq!(
            image.validate_integrity(),
            Err(super::ImageIntegrityError::Safepoint)
        );
        image.safepoints[0].stack_map.roots[0].rbp_displacement =
            image.root_requirements[0].roots[0].rbp_displacement;
        let _omitted_live_root = image.safepoints[0].stack_map.roots.pop();
        image.accounting.metadata_bytes -= 16;
        assert_eq!(
            image.validate_integrity(),
            Err(super::ImageIntegrityError::RootRequirement)
        );
        Ok(())
    }
}
