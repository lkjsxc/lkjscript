//! Verified typed-SSA to callable Linux x86-64 baseline-JIT runtime.
//!
//! This crate is deliberately scalar and allocation-free on generated paths.
//! It never consumes syntax, HIR, or bytecode semantics. Bytecode link metadata
//! is used only to identify a VM function entry for automatic tiering.

#![forbid(unsafe_code)]

mod lower;

use std::fmt;
use std::time::{Duration, Instant};

use lkjscript_core::{
    ExecutionConfig, ExecutionOutcome, HeapObj, HostError, OwnedValue, ResourceLimitKind, Trap,
    Value,
};
use lkjscript_ir::{BytecodeLinkMetadata, Signature as IrSignature, SsaType, VerifiedProgram};
use lkjscript_native::{
    AbiVersions, BackendLimits, CodeAccounting, EntryMetadata, FrameFacts, OutcomeMapEntry,
    Relocation, RuntimeCallSlot, Safepoint, SourceMapEntry, TrapMapEntry,
};
use lkjscript_sys::executable::{
    ExecutableInstaller, ExecutableLimits, InstallError, InstalledImage, InvocationError,
    InvocationOutcome, NativeInvocationConfig,
};

pub use lkjscript_ir::FunctionId;
pub use lkjscript_native::{
    NativeValue, TrapCode, ValueType, CURRENT_NATIVE_ABI_VERSION, CURRENT_RUNTIME_ABI_VERSION,
};
pub use lower::{LoweringError, LoweringFailureCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tier {
    Baseline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TierState {
    VmOnly,
    Observed,
    BaselineCompiling,
    BaselineNative,
    Disabled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailureCode {
    UnsupportedType,
    UnsupportedOperation,
    UnsupportedSignature,
    IndirectCall,
    RecursionUnsupported,
    InvalidVerifiedProgram,
    BackendVerification,
    CompileWallTime,
    InstallLimit,
    InstallFailure,
    InvocationFailure,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineError {
    code: FailureCode,
    function: Option<FunctionId>,
    detail: String,
}

impl EngineError {
    fn new(code: FailureCode, function: Option<FunctionId>, detail: impl Into<String>) -> Self {
        Self {
            code,
            function,
            detail: detail.into(),
        }
    }

    #[doc(hidden)]
    pub fn new_unavailable(function: FunctionId) -> Self {
        Self::new(
            FailureCode::InvocationFailure,
            Some(function),
            "native tier is unavailable",
        )
    }

    pub const fn code(&self) -> FailureCode {
        self.code
    }

    pub const fn function(&self) -> Option<FunctionId> {
        self.function
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(function) = self.function {
            write!(
                formatter,
                "baseline JIT {:?} in function {}: {}",
                self.code,
                function.raw(),
                self.detail
            )
        } else {
            write!(formatter, "baseline JIT {:?}: {}", self.code, self.detail)
        }
    }
}

impl std::error::Error for EngineError {}

impl From<LoweringError> for EngineError {
    fn from(error: LoweringError) -> Self {
        let code = match error.code() {
            LoweringFailureCode::UnsupportedType => FailureCode::UnsupportedType,
            LoweringFailureCode::UnsupportedOperation => FailureCode::UnsupportedOperation,
            LoweringFailureCode::UnsupportedSignature => FailureCode::UnsupportedSignature,
            LoweringFailureCode::IndirectCall => FailureCode::IndirectCall,
            LoweringFailureCode::RecursiveCallGraph => FailureCode::RecursionUnsupported,
            LoweringFailureCode::InvalidFunction => FailureCode::InvalidVerifiedProgram,
            LoweringFailureCode::Backend => FailureCode::BackendVerification,
        };
        Self::new(code, error.function(), error.detail())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JitConfig {
    pub backend_limits: BackendLimits,
    pub executable_limits: ExecutableLimits,
    pub max_object_compile_time: Duration,
    pub max_total_compile_time: Duration,
    pub auto_threshold: u64,
    pub auto_enabled: bool,
    pub max_attempts_per_function: u8,
    pub retain_machine_code_diagnostics: bool,
    pub max_diagnostic_bytes: u64,
    pub epoch: u64,
}

impl Default for JitConfig {
    fn default() -> Self {
        Self {
            backend_limits: BackendLimits::default(),
            executable_limits: ExecutableLimits::default(),
            max_object_compile_time: Duration::from_millis(250),
            max_total_compile_time: Duration::from_secs(2),
            auto_threshold: 64,
            auto_enabled: true,
            max_attempts_per_function: 2,
            retain_machine_code_diagnostics: false,
            max_diagnostic_bytes: 16 * 1024 * 1024,
            epoch: 1,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionTierRecord {
    function: FunctionId,
    name: String,
    state: TierState,
    call_count: u64,
    attempts: u8,
    last_failure: Option<FailureCode>,
    code_object: Option<u64>,
    epoch: u64,
    native_entries: u64,
}

impl FunctionTierRecord {
    pub const fn function(&self) -> FunctionId {
        self.function
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn state(&self) -> TierState {
        self.state
    }

    pub const fn call_count(&self) -> u64 {
        self.call_count
    }

    pub const fn attempts(&self) -> u8 {
        self.attempts
    }

    pub const fn last_failure(&self) -> Option<FailureCode> {
        self.last_failure
    }

    pub const fn code_object(&self) -> Option<u64> {
        self.code_object
    }

    pub const fn epoch(&self) -> u64 {
        self.epoch
    }

    pub const fn native_entries(&self) -> u64 {
        self.native_entries
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileStats {
    lowering_and_encoding: Duration,
    installation: Duration,
    work_units: u64,
}

impl CompileStats {
    pub const fn lowering_and_encoding(&self) -> Duration {
        self.lowering_and_encoding
    }

    pub const fn installation(&self) -> Duration {
        self.installation
    }

    pub const fn work_units(&self) -> u64 {
        self.work_units
    }
}

#[derive(Debug)]
pub struct CodeObject {
    identity: u64,
    functions: Vec<FunctionId>,
    tier: Tier,
    versions: AbiVersions,
    entries: Vec<EntryMetadata>,
    accounting: CodeAccounting,
    accounted_allocation_bytes: u64,
    relocations: Vec<Relocation>,
    runtime_calls: Vec<RuntimeCallSlot>,
    frames: Vec<FrameFacts>,
    safepoints: Vec<Safepoint>,
    source_map: Vec<SourceMapEntry>,
    trap_map: Vec<TrapMapEntry>,
    outcome_map: Vec<OutcomeMapEntry>,
    compile_stats: CompileStats,
    invalidated: bool,
    explicit_traps: Vec<String>,
    diagnostic_machine_code: Option<Vec<u8>>,
    native_entry_count: u64,
    installed: InstalledImage,
}

impl CodeObject {
    pub const fn identity(&self) -> u64 {
        self.identity
    }

    pub fn functions(&self) -> &[FunctionId] {
        &self.functions
    }

    pub const fn tier(&self) -> Tier {
        self.tier
    }

    pub const fn versions(&self) -> AbiVersions {
        self.versions
    }

    pub fn entries(&self) -> &[EntryMetadata] {
        &self.entries
    }

    pub const fn accounting(&self) -> CodeAccounting {
        self.accounting
    }

    pub const fn accounted_allocation_bytes(&self) -> u64 {
        self.accounted_allocation_bytes
    }

    pub fn relocations(&self) -> &[Relocation] {
        &self.relocations
    }

    pub fn runtime_calls(&self) -> &[RuntimeCallSlot] {
        &self.runtime_calls
    }

    pub fn frames(&self) -> &[FrameFacts] {
        &self.frames
    }

    pub fn safepoints(&self) -> &[Safepoint] {
        &self.safepoints
    }

    pub fn source_map(&self) -> &[SourceMapEntry] {
        &self.source_map
    }

    pub fn trap_map(&self) -> &[TrapMapEntry] {
        &self.trap_map
    }

    pub fn outcome_map(&self) -> &[OutcomeMapEntry] {
        &self.outcome_map
    }

    pub const fn compile_stats(&self) -> &CompileStats {
        &self.compile_stats
    }

    pub const fn invalidated(&self) -> bool {
        self.invalidated
    }

    pub fn diagnostic_machine_code(&self) -> Option<&[u8]> {
        self.diagnostic_machine_code.as_deref()
    }

    pub const fn native_entry_count(&self) -> u64 {
        self.native_entry_count
    }

    pub fn wx_transition_verified(&self) -> bool {
        self.installed.wx_transition_verified()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeObjectRecord {
    pub identity: u64,
    pub functions: Vec<FunctionId>,
    pub tier: Tier,
    pub versions: AbiVersions,
    pub code_bytes: u64,
    pub metadata_bytes: u64,
    pub accounted_allocation_bytes: u64,
    pub relocation_count: usize,
    pub runtime_calls: Vec<RuntimeCallSlot>,
    pub safepoint_count: usize,
    pub exact_scalar_stack_maps: bool,
    pub diagnostic_machine_code: Option<Vec<u8>>,
    pub compile_stats: CompileStats,
    pub invalidated: bool,
    pub native_entry_count: u64,
    pub wx_transition_verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JitStats {
    pub functions: Vec<FunctionTierRecord>,
    pub code_objects: Vec<CodeObjectRecord>,
    pub native_entries: u64,
    pub direct_native_calls: u64,
    pub poll_v1_calls: u64,
    pub vm_fallbacks: u64,
    pub compile_failures: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct JitExecution {
    pub outcome: ExecutionOutcome,
    pub stats: JitStats,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarSignature {
    parameters: Vec<ValueType>,
    result: ValueType,
}

impl ScalarSignature {
    pub fn parameters(&self) -> &[ValueType] {
        &self.parameters
    }

    pub const fn result(&self) -> ValueType {
        self.result
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryDecision {
    Interpret,
    Native(FunctionId),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalarInvocationOutcome {
    Returned(NativeValue),
    Trapped(TrapCode),
    Exited(i64),
    DeadlineExceeded,
    ResourceLimitExceeded,
    HostFailure,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarInvocation {
    pub outcome: ScalarInvocationOutcome,
    pub poll_count: u64,
}

pub struct JitSession {
    program: VerifiedProgram,
    links: Option<BytecodeLinkMetadata>,
    installer: ExecutableInstaller,
    config: JitConfig,
    functions: Vec<FunctionTierRecord>,
    objects: Vec<CodeObject>,
    next_object: u64,
    total_compile_time: Duration,
    native_entries: u64,
    direct_native_calls: u64,
    poll_v1_calls: u64,
    vm_fallbacks: u64,
    compile_failures: u64,
    diagnostic_bytes: u64,
}

impl fmt::Debug for JitSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("JitSession")
            .field("config", &self.config)
            .field("functions", &self.functions)
            .field("objects", &self.objects.len())
            .finish()
    }
}

impl JitSession {
    pub fn new_auto(
        program: &VerifiedProgram,
        links: &BytecodeLinkMetadata,
        config: JitConfig,
    ) -> Self {
        Self::new(program, Some(links.clone()), config)
    }

    fn new(
        program: &VerifiedProgram,
        links: Option<BytecodeLinkMetadata>,
        config: JitConfig,
    ) -> Self {
        let functions = program
            .program()
            .functions
            .iter()
            .map(|function| FunctionTierRecord {
                function: function.id,
                name: function.name.clone(),
                state: TierState::VmOnly,
                call_count: 0,
                attempts: 0,
                last_failure: None,
                code_object: None,
                epoch: config.epoch,
                native_entries: 0,
            })
            .collect();
        Self {
            program: program.clone(),
            links,
            installer: ExecutableInstaller::new(config.executable_limits),
            config,
            functions,
            objects: Vec::new(),
            next_object: 1,
            total_compile_time: Duration::ZERO,
            native_entries: 0,
            direct_native_calls: 0,
            poll_v1_calls: 0,
            vm_fallbacks: 0,
            compile_failures: 0,
            diagnostic_bytes: 0,
        }
    }

    pub fn observe_function_entry(&mut self, prototype: u32) -> EntryDecision {
        let Some(function) = self.function_for_prototype(prototype) else {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        };
        let Some(index) = function.index() else {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        };
        let Some(record) = self.functions.get_mut(index) else {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        };
        record.call_count = record.call_count.saturating_add(1);
        if record.state == TierState::BaselineNative {
            return EntryDecision::Native(function);
        }
        if !self.config.auto_enabled {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        }
        if record.epoch != self.config.epoch {
            record.epoch = self.config.epoch;
            if record.attempts < self.config.max_attempts_per_function {
                record.state = TierState::Observed;
                record.last_failure = None;
            }
        }
        if matches!(
            record.state,
            TierState::Disabled | TierState::BaselineCompiling
        ) || record.last_failure.is_some() && record.epoch == self.config.epoch
        {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        }
        if record.state == TierState::VmOnly {
            record.state = TierState::Observed;
        }
        if record.call_count < self.config.auto_threshold.max(1) {
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        }

        record.state = TierState::BaselineCompiling;
        record.attempts = record.attempts.saturating_add(1);
        match self.compile_group(function) {
            Ok(_) => {}
            Err(error) => {
                self.compile_failures = self.compile_failures.saturating_add(1);
                if let Some(record) = self.functions.get_mut(index) {
                    record.last_failure = Some(error.code());
                    record.state = if matches!(
                        error.code(),
                        FailureCode::UnsupportedType
                            | FailureCode::UnsupportedOperation
                            | FailureCode::UnsupportedSignature
                            | FailureCode::IndirectCall
                            | FailureCode::RecursionUnsupported
                    ) || record.attempts >= self.config.max_attempts_per_function
                    {
                        TierState::Disabled
                    } else {
                        TierState::Observed
                    };
                }
            }
        }
        // Compilation at this entry is for later calls; this invocation remains
        // in the VM and is never mislabeled as OSR.
        self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
        EntryDecision::Interpret
    }

    pub fn scalar_signature(&self, function: FunctionId) -> Option<ScalarSignature> {
        let signature: &IrSignature = function
            .index()
            .and_then(|index| self.program.program().functions.get(index))
            .filter(|item| item.id == function)
            .map(|item| &item.signature)?;
        let parameters = signature
            .parameters
            .iter()
            .map(native_type)
            .collect::<Option<Vec<_>>>()?;
        let result = native_type(&signature.result)?;
        Some(ScalarSignature { parameters, result })
    }

    pub fn record_invocation_failure(&mut self, function: FunctionId) {
        self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
        let Some(index) = function.index() else {
            return;
        };
        if let Some(record) = self.functions.get_mut(index) {
            record.last_failure = Some(FailureCode::InvocationFailure);
            record.state = TierState::Disabled;
            record.code_object = None;
        }
    }

    pub fn trap_message_for(&self, function: FunctionId, trap: TrapCode) -> String {
        self.trap_message(function, trap)
    }

    pub fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionConfig,
    ) -> Result<ScalarInvocation, EngineError> {
        let index = function.index().ok_or_else(|| {
            EngineError::new(
                FailureCode::InvocationFailure,
                Some(function),
                "function ID cannot index tier state",
            )
        })?;
        let object_id = self
            .functions
            .get(index)
            .filter(|record| record.state == TierState::BaselineNative)
            .and_then(|record| record.code_object)
            .ok_or_else(|| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "function has no installed baseline code object",
                )
            })?;
        let object_index = self
            .objects
            .iter()
            .position(|object| object.identity == object_id && !object.invalidated)
            .ok_or_else(|| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "installed baseline code object is unavailable",
                )
            })?;
        let native = self.objects[object_index]
            .entries
            .iter()
            .find(|entry| entry.source_function().get() == function.raw())
            .map(EntryMetadata::function)
            .ok_or_else(|| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "code object has no source-function entry",
                )
            })?;
        let config = NativeInvocationConfig::new(execution.instruction_fuel, execution.wall_time);
        let report = self.objects[object_index]
            .installed
            .invoke_with_config(native, arguments, &config)
            .map_err(|error| invocation_error(function, error))?;
        self.poll_v1_calls = self.poll_v1_calls.saturating_add(report.poll_count());
        let mut invocation_entries = 0_u64;
        for count in report.native_entries() {
            invocation_entries = invocation_entries.saturating_add(count.entries());
            self.native_entries = self.native_entries.saturating_add(count.entries());
            if count.source_function() != function.raw() {
                self.direct_native_calls = self.direct_native_calls.saturating_add(count.entries());
            }
            if let Ok(source_index) = usize::try_from(count.source_function()) {
                if let Some(record) = self.functions.get_mut(source_index) {
                    record.native_entries = record.native_entries.saturating_add(count.entries());
                }
            }
        }
        self.objects[object_index].native_entry_count = self.objects[object_index]
            .native_entry_count
            .saturating_add(invocation_entries);
        let outcome = match report.outcome() {
            InvocationOutcome::Returned(value) => ScalarInvocationOutcome::Returned(value),
            InvocationOutcome::Trapped(trap) => ScalarInvocationOutcome::Trapped(trap),
            InvocationOutcome::Exited(code) => ScalarInvocationOutcome::Exited(code),
            InvocationOutcome::DeadlineExceeded => ScalarInvocationOutcome::DeadlineExceeded,
            InvocationOutcome::ResourceLimitExceeded(_) => {
                ScalarInvocationOutcome::ResourceLimitExceeded
            }
            InvocationOutcome::HostFailure => ScalarInvocationOutcome::HostFailure,
        };
        Ok(ScalarInvocation {
            outcome,
            poll_count: report.poll_count(),
        })
    }

    pub fn advance_epoch(&mut self, epoch: u64) {
        if epoch == self.config.epoch {
            return;
        }
        self.config.epoch = epoch;
        for record in &mut self.functions {
            record.epoch = epoch;
            if record.state == TierState::Disabled
                && record.attempts < self.config.max_attempts_per_function
            {
                record.state = TierState::Observed;
                record.last_failure = None;
            }
        }
    }

    pub fn invalidate_object(&mut self, identity: u64, next_epoch: u64) -> bool {
        let Some(object) = self
            .objects
            .iter_mut()
            .find(|object| object.identity == identity)
        else {
            return false;
        };
        object.invalidated = true;
        for record in &mut self.functions {
            if record.code_object == Some(identity) {
                record.code_object = None;
                record.state = TierState::Observed;
            }
        }
        self.advance_epoch(next_epoch);
        true
    }

    pub fn stats(&self) -> JitStats {
        JitStats {
            functions: self.functions.clone(),
            code_objects: self
                .objects
                .iter()
                .map(|object| CodeObjectRecord {
                    identity: object.identity,
                    functions: object.functions.clone(),
                    tier: object.tier,
                    versions: object.versions,
                    code_bytes: object.accounting.code_bytes(),
                    metadata_bytes: object.accounting.metadata_bytes(),
                    accounted_allocation_bytes: object.accounted_allocation_bytes,
                    relocation_count: object.relocations.len(),
                    runtime_calls: object.runtime_calls.clone(),
                    safepoint_count: object.safepoints.len(),
                    exact_scalar_stack_maps: object
                        .safepoints
                        .iter()
                        .all(|point| point.stack_map().reference_slots().is_empty()),
                    diagnostic_machine_code: object.diagnostic_machine_code.clone(),
                    compile_stats: object.compile_stats.clone(),
                    invalidated: object.invalidated,
                    native_entry_count: object.native_entry_count,
                    wx_transition_verified: object.wx_transition_verified(),
                })
                .collect(),
            native_entries: self.native_entries,
            direct_native_calls: self.direct_native_calls,
            poll_v1_calls: self.poll_v1_calls,
            vm_fallbacks: self.vm_fallbacks,
            compile_failures: self.compile_failures,
        }
    }

    pub fn code_objects(&self) -> &[CodeObject] {
        &self.objects
    }

    fn function_for_prototype(&self, prototype: u32) -> Option<FunctionId> {
        self.links
            .as_ref()?
            .functions
            .iter()
            .find_map(|link| (link.prototype == Some(prototype)).then_some(link.function))
    }

    fn compile_group(&mut self, root: FunctionId) -> Result<u64, EngineError> {
        let started = Instant::now();
        let lowered = lower::lower_group(&self.program, root, self.config.backend_limits)?;
        let lowering_and_encoding = started.elapsed();
        if lowering_and_encoding > self.config.max_object_compile_time
            || self
                .total_compile_time
                .saturating_add(lowering_and_encoding)
                > self.config.max_total_compile_time
        {
            return Err(EngineError::new(
                FailureCode::CompileWallTime,
                Some(root),
                "native compilation wall-time budget exceeded",
            ));
        }

        let accounting = lowered.image.accounting();
        let versions = lowered.image.versions();
        let diagnostic_machine_code = if self.config.retain_machine_code_diagnostics {
            let bytes = u64::try_from(lowered.image.bytes().len()).map_err(|_| {
                EngineError::new(
                    FailureCode::InstallLimit,
                    Some(root),
                    "diagnostic machine-code byte count overflow",
                )
            })?;
            let next = self.diagnostic_bytes.checked_add(bytes).ok_or_else(|| {
                EngineError::new(
                    FailureCode::InstallLimit,
                    Some(root),
                    "diagnostic machine-code byte count overflow",
                )
            })?;
            if next > self.config.max_diagnostic_bytes {
                return Err(EngineError::new(
                    FailureCode::InstallLimit,
                    Some(root),
                    "diagnostic machine-code byte budget exceeded",
                ));
            }
            self.diagnostic_bytes = next;
            Some(lowered.image.bytes().to_vec())
        } else {
            None
        };
        let entries = lowered.image.entries().to_vec();
        let relocations = lowered.image.relocations().to_vec();
        let runtime_calls = lowered.image.runtime_calls().to_vec();
        let frames = lowered.image.frames().to_vec();
        let safepoints = lowered.image.safepoints().to_vec();
        let source_map = lowered.image.source_map().to_vec();
        let trap_map = lowered.image.trap_map().to_vec();
        let outcome_map = lowered.image.outcome_map().to_vec();
        let install_started = Instant::now();
        let installed = self
            .installer
            .install(lowered.image)
            .map_err(|error| install_error(root, error))?;
        let installation = install_started.elapsed();
        let accounted_allocation_bytes = installed.accounted_allocation_bytes();
        let total = lowering_and_encoding.saturating_add(installation);
        if total > self.config.max_object_compile_time
            || self.total_compile_time.saturating_add(total) > self.config.max_total_compile_time
        {
            return Err(EngineError::new(
                FailureCode::CompileWallTime,
                Some(root),
                "native compile/install wall-time budget exceeded",
            ));
        }
        self.total_compile_time = self.total_compile_time.saturating_add(total);
        let identity = self.next_object;
        self.next_object = self.next_object.saturating_add(1);
        let object = CodeObject {
            identity,
            functions: lowered.functions.clone(),
            tier: Tier::Baseline,
            versions,
            entries,
            accounting,
            accounted_allocation_bytes,
            relocations,
            runtime_calls,
            frames,
            safepoints,
            source_map,
            trap_map,
            outcome_map,
            compile_stats: CompileStats {
                lowering_and_encoding,
                installation,
                work_units: accounting.work_units(),
            },
            invalidated: false,
            explicit_traps: lowered.explicit_traps,
            diagnostic_machine_code,
            native_entry_count: 0,
            installed,
        };
        self.objects.push(object);
        for function in lowered.functions {
            if let Some(index) = function.index() {
                if let Some(record) = self.functions.get_mut(index) {
                    record.attempts = record.attempts.max(1);
                    record.state = TierState::BaselineNative;
                    record.last_failure = None;
                    record.code_object = Some(identity);
                    record.epoch = self.config.epoch;
                }
            }
        }
        // This deterministic mapping assertion catches a backend metadata
        // mismatch before any generated entry can be selected.
        for (source, native) in lowered.native_functions {
            if !self.objects.last().is_some_and(|object| {
                object.entries.iter().any(|entry| {
                    entry.source_function().get() == source.raw() && entry.function() == native
                })
            }) {
                return Err(EngineError::new(
                    FailureCode::BackendVerification,
                    Some(source),
                    "encoded entry metadata does not match deterministic function mapping",
                ));
            }
        }
        Ok(identity)
    }

    fn trap_message(&self, function: FunctionId, trap: TrapCode) -> String {
        match trap {
            TrapCode::I64Overflow => "checked I64 overflow".to_string(),
            TrapCode::DivisionByZero => "div: I64 division by zero".to_string(),
            TrapCode::Explicit => self
                .functions
                .get(function.index().unwrap_or(usize::MAX))
                .and_then(|record| record.code_object)
                .and_then(|identity| {
                    self.objects
                        .iter()
                        .find(|object| object.identity == identity)
                })
                .and_then(|object| object.explicit_traps.first())
                .cloned()
                .unwrap_or_else(|| "explicit SSA trap".to_string()),
        }
    }
}

pub fn execute_forced(
    program: &VerifiedProgram,
    execution: &ExecutionConfig,
    config: JitConfig,
) -> Result<JitExecution, EngineError> {
    let main = program.program().main;
    let mut session = JitSession::new(program, None, config);
    session.compile_group(main)?;
    let invocation = session.invoke_scalar(main, &[], execution)?;
    let outcome = scalar_to_execution(&session, main, invocation.outcome)?;
    let stats = session.stats();
    if stats.native_entries == 0
        || stats
            .functions
            .iter()
            .filter(|record| record.code_object.is_some())
            .any(|record| record.state != TierState::BaselineNative)
    {
        return Err(EngineError::new(
            FailureCode::InvocationFailure,
            Some(main),
            "forced engine did not enter every installed required native function",
        ));
    }
    Ok(JitExecution { outcome, stats })
}

fn scalar_to_execution(
    session: &JitSession,
    function: FunctionId,
    outcome: ScalarInvocationOutcome,
) -> Result<ExecutionOutcome, EngineError> {
    Ok(match outcome {
        ScalarInvocationOutcome::Returned(value) => {
            ExecutionOutcome::Returned(owned_scalar(value).map_err(|error| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    error.to_string(),
                )
            })?)
        }
        ScalarInvocationOutcome::Trapped(trap) => {
            ExecutionOutcome::Trapped(Trap::new(session.trap_message(function, trap)))
        }
        ScalarInvocationOutcome::Exited(code) => match i32::try_from(code) {
            Ok(code) => ExecutionOutcome::Exited(code),
            Err(_) => ExecutionOutcome::Trapped(Trap::new("exit code out of range")),
        },
        ScalarInvocationOutcome::DeadlineExceeded => ExecutionOutcome::DeadlineExceeded,
        ScalarInvocationOutcome::ResourceLimitExceeded => {
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::InstructionFuel)
        }
        ScalarInvocationOutcome::HostFailure => {
            ExecutionOutcome::HostFailure(HostError::new("native PollV1 host clock failure"))
        }
    })
}

fn owned_scalar(value: NativeValue) -> lkjscript_core::Result<OwnedValue> {
    match value {
        NativeValue::Unit => OwnedValue::from_vm_snapshot(Value::UNIT, Vec::new()),
        NativeValue::Bool(value) => {
            OwnedValue::from_vm_snapshot(Value::from_bool(value), Vec::new())
        }
        NativeValue::I64(value) => match Value::from_small_i64(value) {
            Some(value) => OwnedValue::from_vm_snapshot(value, Vec::new()),
            None => {
                OwnedValue::from_vm_snapshot(Value::from_heap(0), vec![Some(HeapObj::Int(value))])
            }
        },
        NativeValue::F64Bits(bits) => OwnedValue::from_vm_snapshot(
            Value::from_heap(0),
            vec![Some(HeapObj::Float(f64::from_bits(bits)))],
        ),
    }
}

fn install_error(function: FunctionId, error: InstallError) -> EngineError {
    let code = match error {
        InstallError::LimitExceeded(_) => FailureCode::InstallLimit,
        _ => FailureCode::InstallFailure,
    };
    EngineError::new(code, Some(function), error.to_string())
}

fn invocation_error(function: FunctionId, error: InvocationError) -> EngineError {
    EngineError::new(
        FailureCode::InvocationFailure,
        Some(function),
        error.to_string(),
    )
}

pub fn native_type(ty: &SsaType) -> Option<ValueType> {
    match ty {
        SsaType::Unit => Some(ValueType::Unit),
        SsaType::Bool => Some(ValueType::Bool),
        SsaType::I64 => Some(ValueType::I64),
        SsaType::F64 => Some(ValueType::F64),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
    use lkjscript_ir::{
        verify, Block, BlockId, BlockMetadata, EffectSet, Function, FunctionId, Origin, Program,
        Signature, SourceMetadata, SsaType, StructuredOutcome, Terminator,
    };

    use super::{execute_forced, JitConfig};

    fn terminal_program(
        terminator: Terminator,
        effects: EffectSet,
    ) -> lkjscript_ir::VerifiedProgram {
        verify(Program {
            sources: vec![SourceMetadata {
                id: 0,
                path: "terminal.lkjscript".into(),
            }],
            products: Vec::new(),
            functions: vec![Function {
                id: FunctionId::new(0),
                name: "main".into(),
                signature: Signature::monomorphic(Vec::new(), SsaType::I64),
                effects,
                entry: BlockId::new(0),
                blocks: vec![Block {
                    id: BlockId::new(0),
                    parameters: Vec::new(),
                    instructions: Vec::new(),
                    terminator,
                    metadata: BlockMetadata {
                        loop_header: false,
                        origin: Origin::SYNTHETIC,
                        frame_state: None,
                    },
                }],
                origin: Origin::SYNTHETIC,
            }],
            main: FunctionId::new(0),
        })
        .expect("verify terminal SSA")
    }

    #[test]
    fn verified_ssa_trap_and_structured_outcome_reach_native_status() {
        let trap = terminal_program(
            Terminator::Trap {
                message: "exact native trap".into(),
            },
            EffectSet::MAY_TRAP,
        );
        let executed = execute_forced(&trap, &ExecutionConfig::default(), JitConfig::default())
            .expect("execute explicit native trap");
        assert!(matches!(
            executed.outcome,
            ExecutionOutcome::Trapped(trap) if trap.as_str() == "exact native trap"
        ));

        let deadline = terminal_program(
            Terminator::Outcome {
                outcome: StructuredOutcome::DeadlineExceeded,
                detail: None,
            },
            EffectSet::PURE,
        );
        let executed = execute_forced(&deadline, &ExecutionConfig::default(), JitConfig::default())
            .expect("execute native structured outcome");
        assert_eq!(executed.outcome, ExecutionOutcome::DeadlineExceeded);
    }
}
