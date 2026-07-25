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
    Error as CoreError, ErrorClass, ExecutionConfig, ExecutionOutcome, GcConfig, GcHeap, GcLimit,
    HeapObj, HostError, OwnedValue, ProductId, ResourceLimitKind, Trap, Value, MAX_BUFFER_BYTES,
    MAX_LIST_EQUAL_STEPS,
};
use lkjscript_ir::{
    optimize, BytecodeLinkMetadata, OptimizationCertificate, OptimizationFailureCode,
    OptimizationLimits, OptimizationStats, Signature as IrSignature, SsaType,
    VerifiedOptimizedProgram, VerifiedProgram,
};
use lkjscript_native::{
    AbiVersions, BackendLimits, CodeAccounting, EntryMetadata, FrameFacts, HeapOperation,
    HeapRuntimeSite, OutcomeMapEntry, ReferenceType, Relocation, RuntimeCallSlot, Safepoint,
    SourceMapEntry, TrapMapEntry,
};
use lkjscript_sys::executable::{
    ExecutableInstaller, ExecutableLimits, InstallError, InstalledImage, InvocationError,
    InvocationOutcome, NativeInvocationConfig, NativeResourceLimitKind, NativeRoot,
    NativeRuntimeServices, NativeServiceError,
};

pub use lkjscript_ir::FunctionId;
pub use lkjscript_native::{
    NativeValue, TrapCode, ValueType, CURRENT_NATIVE_ABI_VERSION, CURRENT_RUNTIME_ABI_VERSION,
};
pub use lower::{LoweringError, LoweringFailureCode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Tier {
    Baseline,
    Optimizing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TierState {
    VmOnly,
    Observed,
    BaselineCompiling,
    BaselineNative,
    OptimizingCandidate,
    OptimizingCompiling,
    OptimizedNative,
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
    OptimizationBudget,
    CertificateVerification,
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
                "JIT {:?} in function {}: {}",
                self.code,
                function.raw(),
                self.detail
            )
        } else {
            write!(formatter, "JIT {:?}: {}", self.code, self.detail)
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
    pub collect_metrics: bool,
    pub max_diagnostic_bytes: u64,
    pub epoch: u64,
    /// Force exact-root collection before every allocation-capable heap site.
    pub force_gc_before_allocation: bool,
    pub optimization_limits: OptimizationLimits,
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
            collect_metrics: false,
            max_diagnostic_bytes: 16 * 1024 * 1024,
            epoch: 1,
            force_gc_before_allocation: false,
            optimization_limits: OptimizationLimits::default(),
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
    auto_entry_eligible: bool,
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

    pub const fn auto_entry_eligible(&self) -> bool {
        self.auto_entry_eligible
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompileStats {
    optimization: Duration,
    lowering_and_encoding: Duration,
    installation: Duration,
    work_units: u64,
}

impl CompileStats {
    pub const fn optimization(&self) -> Duration {
        self.optimization
    }

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
    optimization_certificate: Option<OptimizationCertificate>,
    optimization_stats: Option<OptimizationStats>,
    invalidated: bool,
    explicit_traps: Vec<(u32, String)>,
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

    pub fn optimization_certificate(&self) -> Option<&OptimizationCertificate> {
        self.optimization_certificate.as_ref()
    }

    pub const fn optimization_stats(&self) -> Option<&OptimizationStats> {
        self.optimization_stats.as_ref()
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
    pub optimization_certificate: Option<OptimizationCertificate>,
    pub optimization_stats: Option<OptimizationStats>,
    pub optimization_metadata_bytes_estimate: u64,
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
    pub native_invocations: u64,
    pub time_to_first_native_entry: Option<Duration>,
    pub first_native_call: Option<Duration>,
    pub native_execution: Duration,
    pub auto_threshold: u64,
    pub auto_enabled: bool,
    pub code_cache_peak_objects: u64,
    pub code_cache_peak_bytes: u64,
    pub metadata_cache_peak_bytes: u64,
    pub accounted_allocation_peak_bytes: u64,
    pub allocations: u64,
    pub allocation_bytes_estimate: u64,
    pub collections: u64,
    pub peak_live_heap_bytes_estimate: usize,
    pub maximum_roots: usize,
    pub runtime_heap_attempts: u64,
    pub runtime_heap_successes: u64,
    pub barrier_count: u64,
    pub peak_native_frame_depth: usize,
    pub vm_to_native_transitions: u64,
    pub native_to_vm_transitions: u64,
    pub baseline_native_entries: u64,
    pub optimizing_native_entries: u64,
    pub baseline_code_objects: u64,
    pub optimizing_code_objects: u64,
    pub optimizing_passes: u64,
    pub optimization_discovery_passes: u64,
    pub optimization_checker_passes: u64,
    pub optimization_reconstruction_passes: u64,
    pub optimization_cleanup_passes: u64,
    pub optimization_validation_passes: u64,
    pub optimization_certificate_records: u64,
    pub optimization_certificate_bytes_estimate: u64,
    pub algebraic_rewrites: u64,
    pub gvn_rewrites: u64,
    pub checked_i64_rewrites: u64,
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
    Trapped(TrapCode, Option<u32>),
    Exited(i64),
    DeadlineExceeded,
    ResourceLimitExceeded(ResourceLimitKind),
    HostFailure,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarInvocation {
    pub outcome: ScalarInvocationOutcome,
    pub poll_count: u64,
}

#[derive(Clone)]
enum ProgramAuthority {
    Baseline(VerifiedProgram),
    Optimizing(VerifiedOptimizedProgram),
}

impl ProgramAuthority {
    fn program(&self) -> &lkjscript_ir::Program {
        match self {
            Self::Baseline(program) => program.program(),
            Self::Optimizing(program) => program.program(),
        }
    }

    const fn tier(&self) -> Tier {
        match self {
            Self::Baseline(_) => Tier::Baseline,
            Self::Optimizing(_) => Tier::Optimizing,
        }
    }
}

pub struct JitSession {
    program: ProgramAuthority,
    links: Option<BytecodeLinkMetadata>,
    installer: ExecutableInstaller,
    config: JitConfig,
    functions: Vec<FunctionTierRecord>,
    objects: Vec<CodeObject>,
    next_object: u64,
    total_compile_time: Duration,
    optimization_time: Duration,
    native_entries: u64,
    direct_native_calls: u64,
    poll_v1_calls: u64,
    vm_fallbacks: u64,
    compile_failures: u64,
    native_invocations: u64,
    metrics_started: Option<Instant>,
    time_to_first_native_entry: Option<Duration>,
    first_native_call: Option<Duration>,
    native_execution: Duration,
    diagnostic_bytes: u64,
    heap: GcHeap,
    maximum_roots: usize,
    runtime_heap_attempts: u64,
    runtime_heap_successes: u64,
    barrier_count: u64,
    peak_native_frame_depth: usize,
    vm_to_native_transitions: u64,
    native_to_vm_transitions: u64,
    last_runtime_trap: Option<String>,
    last_runtime_resource: Option<ResourceLimitKind>,
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
        Self::new(
            ProgramAuthority::Baseline(program.clone()),
            Some(links.clone()),
            config,
            Duration::ZERO,
        )
    }

    fn new_baseline(program: &VerifiedProgram, config: JitConfig) -> Self {
        Self::new(
            ProgramAuthority::Baseline(program.clone()),
            None,
            config,
            Duration::ZERO,
        )
    }

    fn new_optimizing(
        program: VerifiedOptimizedProgram,
        config: JitConfig,
        optimization_time: Duration,
    ) -> Self {
        Self::new(
            ProgramAuthority::Optimizing(program),
            None,
            config,
            optimization_time,
        )
    }

    fn new(
        program: ProgramAuthority,
        links: Option<BytecodeLinkMetadata>,
        config: JitConfig,
        optimization_time: Duration,
    ) -> Self {
        let initial_state = match program.tier() {
            Tier::Baseline => TierState::VmOnly,
            Tier::Optimizing => TierState::OptimizingCandidate,
        };
        let functions = program
            .program()
            .functions
            .iter()
            .map(|function| FunctionTierRecord {
                function: function.id,
                name: function.name.clone(),
                state: initial_state,
                call_count: 0,
                attempts: 0,
                last_failure: None,
                code_object: None,
                epoch: config.epoch,
                native_entries: 0,
                auto_entry_eligible: function
                    .signature
                    .parameters
                    .iter()
                    .chain(std::iter::once(function.signature.result.as_ref()))
                    .all(|ty| native_type(ty).is_some()),
            })
            .collect();
        Self {
            program,
            links,
            installer: ExecutableInstaller::new(config.executable_limits),
            config,
            functions,
            objects: Vec::new(),
            next_object: 1,
            total_compile_time: optimization_time,
            optimization_time,
            native_entries: 0,
            direct_native_calls: 0,
            poll_v1_calls: 0,
            vm_fallbacks: 0,
            compile_failures: 0,
            native_invocations: 0,
            metrics_started: config.collect_metrics.then(Instant::now),
            time_to_first_native_entry: None,
            first_native_call: None,
            native_execution: Duration::ZERO,
            diagnostic_bytes: 0,
            heap: GcHeap::default(),
            maximum_roots: 0,
            runtime_heap_attempts: 0,
            runtime_heap_successes: 0,
            barrier_count: 0,
            peak_native_frame_depth: 0,
            vm_to_native_transitions: 0,
            native_to_vm_transitions: 0,
            last_runtime_trap: None,
            last_runtime_resource: None,
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
        if !record.auto_entry_eligible {
            // A supported compilation group may contain reference-signature
            // helpers for direct generated calls. Until VM/native reference
            // adapters exist, those helpers are never eligible VM entries.
            self.vm_fallbacks = self.vm_fallbacks.saturating_add(1);
            return EntryDecision::Interpret;
        }
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

    pub fn trap_message_for(
        &self,
        function: FunctionId,
        trap: TrapCode,
        site: Option<u32>,
    ) -> String {
        self.trap_message(function, trap, site)
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
        let expected_state = match self.program.tier() {
            Tier::Baseline => TierState::BaselineNative,
            Tier::Optimizing => TierState::OptimizedNative,
        };
        let object_id = self
            .functions
            .get(index)
            .filter(|record| record.state == expected_state)
            .and_then(|record| record.code_object)
            .ok_or_else(|| {
                EngineError::new(
                    FailureCode::InvocationFailure,
                    Some(function),
                    "function has no installed code object for the selected tier",
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
                    "installed code object is unavailable",
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
        self.heap.set_config(GcConfig {
            max_allocations: execution.max_allocations,
            max_heap_bytes: execution.max_heap_bytes,
            collect_before_every_allocation: self.config.force_gc_before_allocation,
            ..self.heap.config()
        });
        self.last_runtime_trap = None;
        self.last_runtime_resource = None;
        let config = NativeInvocationConfig::new(execution.instruction_fuel, execution.wall_time)
            .with_max_active_frames(execution.max_frames)
            .with_max_active_values(execution.max_stack_values);
        if self.time_to_first_native_entry.is_none() {
            self.time_to_first_native_entry = self.metrics_started.map(|started| started.elapsed());
        }
        let invocation_started = self.config.collect_metrics.then(Instant::now);
        if self.links.is_some() {
            self.vm_to_native_transitions = self.vm_to_native_transitions.saturating_add(1);
        }
        let force_collection = self.config.force_gc_before_allocation;
        let mut services = JitHeapServices::new(&mut self.heap, force_collection);
        let report = self.objects[object_index].installed.invoke_with_services(
            native,
            arguments,
            &config,
            &mut services,
        );
        self.last_runtime_trap = services.last_trap.take();
        self.last_runtime_resource = services.last_resource;
        if self.links.is_some() {
            self.native_to_vm_transitions = self.native_to_vm_transitions.saturating_add(1);
        }
        if let Some(started) = invocation_started {
            let elapsed = started.elapsed();
            self.native_invocations = self.native_invocations.saturating_add(1);
            self.native_execution = self.native_execution.saturating_add(elapsed);
            if self.first_native_call.is_none() {
                self.first_native_call = Some(elapsed);
            }
        }
        let report = report.map_err(|error| invocation_error(function, error))?;
        self.poll_v1_calls = self.poll_v1_calls.saturating_add(report.poll_count());
        self.maximum_roots = self.maximum_roots.max(report.maximum_roots());
        self.runtime_heap_attempts = self
            .runtime_heap_attempts
            .saturating_add(report.heap_operation_attempts());
        self.runtime_heap_successes = self
            .runtime_heap_successes
            .saturating_add(report.heap_operation_successes());
        self.barrier_count = self.barrier_count.saturating_add(report.barrier_count());
        self.peak_native_frame_depth = self
            .peak_native_frame_depth
            .max(report.peak_active_frame_depth());
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
        // The root machine entry was invoked even when its frame/value
        // reservation failed before generated EnterFunctionV1. Count that
        // actual entry once so forced-tier evidence never reports zero native
        // execution for a structured pre-entry resource outcome.
        if invocation_entries == 0 {
            invocation_entries = 1;
            self.native_entries = self.native_entries.saturating_add(1);
            if let Some(record) = self.functions.get_mut(index) {
                record.native_entries = record.native_entries.saturating_add(1);
            }
        }
        self.objects[object_index].native_entry_count = self.objects[object_index]
            .native_entry_count
            .saturating_add(invocation_entries);
        let outcome = match report.outcome() {
            InvocationOutcome::Returned(value) => ScalarInvocationOutcome::Returned(value),
            InvocationOutcome::Trapped(trap) => {
                ScalarInvocationOutcome::Trapped(trap, report.trap_site())
            }
            InvocationOutcome::Exited(code) => ScalarInvocationOutcome::Exited(code),
            InvocationOutcome::DeadlineExceeded => ScalarInvocationOutcome::DeadlineExceeded,
            InvocationOutcome::ResourceLimitExceeded(kind) => {
                let kind = match kind {
                    NativeResourceLimitKind::PollFuel => ResourceLimitKind::InstructionFuel,
                    NativeResourceLimitKind::ActiveFrames
                    | NativeResourceLimitKind::NativeStackBytes => ResourceLimitKind::FrameDepth,
                    NativeResourceLimitKind::MaterializedRoots
                    | NativeResourceLimitKind::ActiveValues => ResourceLimitKind::StackValues,
                    NativeResourceLimitKind::RuntimeService => self
                        .last_runtime_resource
                        .unwrap_or(ResourceLimitKind::Allocations),
                };
                ScalarInvocationOutcome::ResourceLimitExceeded(kind)
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
        let code_cache_peak_objects = u64::try_from(self.objects.len()).unwrap_or(u64::MAX);
        let code_cache_peak_bytes = self.objects.iter().fold(0_u64, |total, object| {
            total.saturating_add(object.accounting.code_bytes())
        });
        let metadata_cache_peak_bytes = self.objects.iter().fold(0_u64, |total, object| {
            total
                .saturating_add(object.accounting.metadata_bytes())
                .saturating_add(optimization_metadata_bytes_estimate(
                    object.optimization_stats.as_ref(),
                ))
        });
        let accounted_allocation_peak_bytes = self.objects.iter().fold(0_u64, |total, object| {
            total.saturating_add(object.accounted_allocation_bytes)
        });
        let baseline_native_entries = self
            .objects
            .iter()
            .filter(|object| object.tier == Tier::Baseline)
            .fold(0_u64, |total, object| {
                total.saturating_add(object.native_entry_count)
            });
        let optimizing_native_entries = self
            .objects
            .iter()
            .filter(|object| object.tier == Tier::Optimizing)
            .fold(0_u64, |total, object| {
                total.saturating_add(object.native_entry_count)
            });
        let baseline_code_objects = self
            .objects
            .iter()
            .filter(|object| object.tier == Tier::Baseline)
            .count() as u64;
        let optimizing_code_objects = self
            .objects
            .iter()
            .filter(|object| object.tier == Tier::Optimizing)
            .count() as u64;
        let optimization_totals = self
            .objects
            .iter()
            .filter_map(|object| object.optimization_stats.as_ref())
            .fold(OptimizationStats::default(), |mut total, stats| {
                total.iterations = total.iterations.saturating_add(stats.iterations);
                total.discovery_passes = total
                    .discovery_passes
                    .saturating_add(stats.discovery_passes);
                total.checker_passes = total.checker_passes.saturating_add(stats.checker_passes);
                total.reconstruction_passes = total
                    .reconstruction_passes
                    .saturating_add(stats.reconstruction_passes);
                total.cleanup_passes = total.cleanup_passes.saturating_add(stats.cleanup_passes);
                total.validation_passes = total
                    .validation_passes
                    .saturating_add(stats.validation_passes);
                total.optimizing_passes = total
                    .optimizing_passes
                    .saturating_add(stats.optimizing_passes);
                total.certificate_records = total
                    .certificate_records
                    .saturating_add(stats.certificate_records);
                total.certificate_bytes_estimate = total
                    .certificate_bytes_estimate
                    .saturating_add(stats.certificate_bytes_estimate);
                total.algebraic_rewrites = total
                    .algebraic_rewrites
                    .saturating_add(stats.algebraic_rewrites);
                total.gvn_rewrites = total.gvn_rewrites.saturating_add(stats.gvn_rewrites);
                total.checked_i64_rewrites = total
                    .checked_i64_rewrites
                    .saturating_add(stats.checked_i64_rewrites);
                total
            });
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
                        .all(|point| point.stack_map().roots().is_empty()),
                    diagnostic_machine_code: object.diagnostic_machine_code.clone(),
                    compile_stats: object.compile_stats.clone(),
                    optimization_certificate: object.optimization_certificate.clone(),
                    optimization_stats: object.optimization_stats,
                    optimization_metadata_bytes_estimate: optimization_metadata_bytes_estimate(
                        object.optimization_stats.as_ref(),
                    ),
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
            native_invocations: self.native_invocations,
            time_to_first_native_entry: self.time_to_first_native_entry,
            first_native_call: self.first_native_call,
            native_execution: self.native_execution,
            auto_threshold: self.config.auto_threshold,
            auto_enabled: self.config.auto_enabled,
            code_cache_peak_objects,
            code_cache_peak_bytes,
            metadata_cache_peak_bytes,
            accounted_allocation_peak_bytes,
            allocations: self.heap.total_allocations(),
            allocation_bytes_estimate: self.heap.total_allocated_bytes(),
            collections: self.heap.collections(),
            peak_live_heap_bytes_estimate: self.heap.peak_live_heap_bytes(),
            maximum_roots: self.maximum_roots,
            runtime_heap_attempts: self.runtime_heap_attempts,
            runtime_heap_successes: self.runtime_heap_successes,
            barrier_count: self.barrier_count,
            peak_native_frame_depth: self.peak_native_frame_depth,
            vm_to_native_transitions: self.vm_to_native_transitions,
            native_to_vm_transitions: self.native_to_vm_transitions,
            baseline_native_entries,
            optimizing_native_entries,
            baseline_code_objects,
            optimizing_code_objects,
            optimizing_passes: optimization_totals.optimizing_passes,
            optimization_discovery_passes: optimization_totals.discovery_passes,
            optimization_checker_passes: optimization_totals.checker_passes,
            optimization_reconstruction_passes: optimization_totals.reconstruction_passes,
            optimization_cleanup_passes: optimization_totals.cleanup_passes,
            optimization_validation_passes: optimization_totals.validation_passes,
            optimization_certificate_records: optimization_totals.certificate_records,
            optimization_certificate_bytes_estimate: optimization_totals.certificate_bytes_estimate,
            algebraic_rewrites: optimization_totals.algebraic_rewrites,
            gvn_rewrites: optimization_totals.gvn_rewrites,
            checked_i64_rewrites: optimization_totals.checked_i64_rewrites,
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
        if self.links.is_some() && self.scalar_signature(root).is_none() {
            return Err(EngineError::new(
                FailureCode::UnsupportedType,
                Some(root),
                "automatic tiering conservatively keeps reference-typed functions in the VM",
            ));
        }
        let tier = self.program.tier();
        if tier == Tier::Optimizing {
            if let Some(record) = root.index().and_then(|index| self.functions.get_mut(index)) {
                record.state = TierState::OptimizingCompiling;
            }
        }
        let started = Instant::now();
        let lowered = match &self.program {
            ProgramAuthority::Baseline(program) => {
                lower::lower_baseline_group(program, root, self.config.backend_limits)?
            }
            ProgramAuthority::Optimizing(program) => {
                lower::lower_optimizing_group(program, root, self.config.backend_limits)?
            }
        };
        let lowering_and_encoding = started.elapsed();
        if self.optimization_time.saturating_add(lowering_and_encoding)
            > self.config.max_object_compile_time
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
        if self.optimization_time.saturating_add(total) > self.config.max_object_compile_time
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
        let (optimization_certificate, optimization_stats) = match &self.program {
            ProgramAuthority::Baseline(_) => (None, None),
            ProgramAuthority::Optimizing(program) => {
                (Some(program.certificate().clone()), Some(*program.stats()))
            }
        };
        let object = CodeObject {
            identity,
            functions: lowered.functions.clone(),
            tier,
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
                optimization: self.optimization_time,
                lowering_and_encoding,
                installation,
                work_units: accounting
                    .work_units()
                    .saturating_add(optimization_stats.map_or(0, |stats| stats.work_units)),
            },
            optimization_certificate,
            optimization_stats,
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
                    record.code_object = Some(identity);
                    record.epoch = self.config.epoch;
                    if self.links.is_none() || record.auto_entry_eligible {
                        record.attempts = record.attempts.max(1);
                        record.state = match tier {
                            Tier::Baseline => TierState::BaselineNative,
                            Tier::Optimizing => TierState::OptimizedNative,
                        };
                        record.last_failure = None;
                    }
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

    fn trap_message(&self, function: FunctionId, trap: TrapCode, site: Option<u32>) -> String {
        match trap {
            TrapCode::I64Overflow => "checked I64 overflow".to_string(),
            TrapCode::DivisionByZero => "div: I64 division by zero".to_string(),
            TrapCode::Explicit => self
                .last_runtime_trap
                .clone()
                .or_else(|| {
                    self.functions
                        .get(function.index().unwrap_or(usize::MAX))
                        .and_then(|record| record.code_object)
                        .and_then(|identity| {
                            self.objects
                                .iter()
                                .find(|object| object.identity == identity)
                        })
                        .and_then(|object| {
                            let site = site?;
                            object
                                .explicit_traps
                                .iter()
                                .find_map(|(candidate, message)| {
                                    (*candidate == site).then(|| message.clone())
                                })
                        })
                })
                .unwrap_or_else(|| "explicit SSA trap".to_string()),
        }
    }
}

fn optimization_metadata_bytes_estimate(stats: Option<&OptimizationStats>) -> u64 {
    stats.map_or(0, |stats| {
        stats
            .certificate_bytes_estimate
            .saturating_add(17_u64.saturating_mul(8))
    })
}

pub fn execute_forced(
    program: &VerifiedProgram,
    execution: &ExecutionConfig,
    config: JitConfig,
) -> Result<JitExecution, EngineError> {
    let main = program.program().main;
    let mut session = JitSession::new_baseline(program, config);
    session.compile_group(main)?;
    let invocation = session.invoke_scalar(main, &[], execution)?;
    let outcome = scalar_to_execution(&session, main, invocation.outcome)?;
    let stats = session.stats();
    verify_forced_entry(&outcome, &stats, main, TierState::BaselineNative)?;
    if stats.optimizing_code_objects != 0 || stats.optimizing_native_entries != 0 {
        return Err(EngineError::new(
            FailureCode::InvocationFailure,
            Some(main),
            "forced baseline engine installed or entered optimizing code",
        ));
    }
    Ok(JitExecution { outcome, stats })
}

/// Proof-optimize the complete verified program, install only optimizing-tier
/// code for the required reachable group, and enter optimized main.
pub fn execute_optimizing(
    program: &VerifiedProgram,
    execution: &ExecutionConfig,
    config: JitConfig,
) -> Result<JitExecution, EngineError> {
    let started = Instant::now();
    let optimized = optimize(program, config.optimization_limits).map_err(optimization_error)?;
    let optimization_time = started.elapsed();
    if optimization_time > config.max_object_compile_time
        || optimization_time > config.max_total_compile_time
    {
        return Err(EngineError::new(
            FailureCode::CompileWallTime,
            Some(program.program().main),
            "optimizing pass wall-time budget exceeded",
        ));
    }
    let main = optimized.program().main;
    let mut session = JitSession::new_optimizing(optimized, config, optimization_time);
    session.compile_group(main)?;
    let invocation = session.invoke_scalar(main, &[], execution)?;
    let outcome = scalar_to_execution(&session, main, invocation.outcome)?;
    let stats = session.stats();
    verify_forced_entry(&outcome, &stats, main, TierState::OptimizedNative)?;
    if stats.baseline_code_objects != 0
        || stats.baseline_native_entries != 0
        || stats.optimizing_code_objects == 0
        || stats.optimizing_native_entries == 0
        || stats.vm_fallbacks != 0
    {
        return Err(EngineError::new(
            FailureCode::InvocationFailure,
            Some(main),
            "forced optimizing engine did not remain optimizing-only",
        ));
    }
    Ok(JitExecution { outcome, stats })
}

fn verify_forced_entry(
    _outcome: &ExecutionOutcome,
    stats: &JitStats,
    main: FunctionId,
    expected_state: TierState,
) -> Result<(), EngineError> {
    if stats.native_entries == 0
        || stats
            .functions
            .iter()
            .filter(|record| record.code_object.is_some())
            .any(|record| record.state != expected_state)
    {
        return Err(EngineError::new(
            FailureCode::InvocationFailure,
            Some(main),
            "forced engine did not enter installed code in the selected tier",
        ));
    }
    Ok(())
}

fn optimization_error(error: lkjscript_ir::OptimizationError) -> EngineError {
    let code = match error.code() {
        OptimizationFailureCode::BudgetExceeded => FailureCode::OptimizationBudget,
        OptimizationFailureCode::InputVerification
        | OptimizationFailureCode::CertificateMismatch
        | OptimizationFailureCode::IllegalEdit
        | OptimizationFailureCode::CandidateMismatch
        | OptimizationFailureCode::OutputVerification => FailureCode::CertificateVerification,
    };
    EngineError::new(code, None, error.to_string())
}

fn scalar_to_execution(
    session: &JitSession,
    function: FunctionId,
    outcome: ScalarInvocationOutcome,
) -> Result<ExecutionOutcome, EngineError> {
    Ok(match outcome {
        ScalarInvocationOutcome::Returned(value) => {
            let owned = match value {
                NativeValue::Reference(reference) => {
                    let value =
                        native_reference_value(&session.heap, reference).map_err(|error| {
                            EngineError::new(FailureCode::InvocationFailure, Some(function), error)
                        })?;
                    session.heap.snapshot(value).map_err(|error| {
                        EngineError::new(
                            FailureCode::InvocationFailure,
                            Some(function),
                            error.to_string(),
                        )
                    })?
                }
                scalar => owned_scalar(scalar).map_err(|error| {
                    EngineError::new(
                        FailureCode::InvocationFailure,
                        Some(function),
                        error.to_string(),
                    )
                })?,
            };
            ExecutionOutcome::Returned(owned)
        }
        ScalarInvocationOutcome::Trapped(trap, site) => {
            ExecutionOutcome::Trapped(Trap::new(session.trap_message(function, trap, site)))
        }
        ScalarInvocationOutcome::Exited(code) => match i32::try_from(code) {
            Ok(code) => ExecutionOutcome::Exited(code),
            Err(_) => ExecutionOutcome::Trapped(Trap::new("exit code out of range")),
        },
        ScalarInvocationOutcome::DeadlineExceeded => ExecutionOutcome::DeadlineExceeded,
        ScalarInvocationOutcome::ResourceLimitExceeded(kind) => {
            ExecutionOutcome::ResourceLimitExceeded(kind)
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
        NativeValue::Reference(_) => Err(lkjscript_core::Error::msg(
            "scalar JIT cannot return a native reference",
        )),
    }
}

struct JitHeapServices<'a> {
    heap: &'a mut GcHeap,
    force_collection: bool,
    last_trap: Option<String>,
    last_resource: Option<ResourceLimitKind>,
}

impl<'a> JitHeapServices<'a> {
    fn new(heap: &'a mut GcHeap, force_collection: bool) -> Self {
        Self {
            heap,
            force_collection,
            last_trap: None,
            last_resource: None,
        }
    }

    fn trap<T>(&mut self, message: impl Into<String>) -> Result<T, NativeServiceError> {
        self.last_trap = Some(message.into());
        Err(NativeServiceError::Trap)
    }

    fn roots(&mut self, roots: &[NativeRoot]) -> Result<Vec<Value>, NativeServiceError> {
        roots
            .iter()
            .map(|root| {
                native_reference_value(
                    self.heap,
                    lkjscript_native::NativeReference::new(
                        root.reference_type(),
                        root.opaque_word(),
                    ),
                )
                .map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })
            })
            .collect()
    }

    fn allocate(
        &mut self,
        object: HeapObj,
        reference_type: ReferenceType,
    ) -> Result<Value, NativeServiceError> {
        self.heap
            .try_alloc_with_layout(object, reference_layout_key(reference_type))
            .map_err(|limit| {
                self.last_resource = Some(match limit {
                    GcLimit::Allocations => ResourceLimitKind::Allocations,
                    GcLimit::HeapBytes => ResourceLimitKind::HeapBytes,
                });
                NativeServiceError::ResourceLimitExceeded
            })
    }

    fn mutate<T>(
        &mut self,
        value: Value,
        mutation: impl FnOnce(&mut HeapObj) -> lkjscript_core::Result<T>,
    ) -> Result<T, NativeServiceError> {
        self.heap.mutate(value, mutation).map_err(|error| {
            if error.class() == ErrorClass::Resource(ResourceLimitKind::HeapBytes) {
                self.last_resource = Some(ResourceLimitKind::HeapBytes);
                NativeServiceError::ResourceLimitExceeded
            } else {
                self.last_trap = Some(error.to_string());
                NativeServiceError::Trap
            }
        })
    }

    fn result_error(
        &mut self,
        message: &str,
        result_type: ValueType,
    ) -> Result<NativeValue, NativeServiceError> {
        let payload = self.allocate(HeapObj::Str(message.into()), ReferenceType::Str)?;
        let reference_type = result_type
            .reference_type()
            .ok_or(NativeServiceError::HostFailure)?;
        let result = self.allocate(HeapObj::ResultErr(payload), reference_type)?;
        self.native_from_value(result, result_type)
    }

    fn value_from_native(&mut self, value: NativeValue) -> Result<Value, NativeServiceError> {
        match value {
            NativeValue::Unit => Ok(Value::UNIT),
            NativeValue::Bool(value) => Ok(Value::from_bool(value)),
            NativeValue::I64(value) => match Value::from_small_i64(value) {
                Some(value) => Ok(value),
                None => self.allocate(HeapObj::Int(value), scalar_box_layout(1)),
            },
            NativeValue::F64Bits(bits) => {
                self.allocate(HeapObj::Float(f64::from_bits(bits)), scalar_box_layout(2))
            }
            NativeValue::Reference(reference) => native_reference_value(self.heap, reference)
                .map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                }),
        }
    }

    fn native_from_value(
        &mut self,
        value: Value,
        expected: ValueType,
    ) -> Result<NativeValue, NativeServiceError> {
        match expected {
            ValueType::Unit if value.is_unit() => Ok(NativeValue::Unit),
            ValueType::Bool => value.as_bool().map(NativeValue::Bool).ok_or_else(|| {
                self.last_trap = Some("heap operation produced non-Bool".into());
                NativeServiceError::Trap
            }),
            ValueType::I64 => {
                let number = if let Some(number) = value.as_small_i64() {
                    Some(number)
                } else {
                    match self.heap.get(value) {
                        Ok(HeapObj::Int(number)) => Some(*number),
                        _ => None,
                    }
                };
                number.map(NativeValue::I64).ok_or_else(|| {
                    self.last_trap = Some("heap operation produced non-I64".into());
                    NativeServiceError::Trap
                })
            }
            ValueType::F64 => match self.heap.get(value) {
                Ok(HeapObj::Float(number)) => Ok(NativeValue::F64Bits(number.to_bits())),
                _ => self.trap("heap operation produced non-F64"),
            },
            ValueType::Reference(reference_type) => {
                reference_native_value(self.heap, value, reference_type).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })
            }
            _ => self.trap("heap operation result category mismatch"),
        }
    }

    fn execute(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        let descriptor = site.descriptor();
        let result_type = descriptor.result_type();
        let argument = |index: usize| {
            arguments
                .get(index)
                .copied()
                .ok_or(NativeServiceError::HostFailure)
        };
        let as_i64 = |value: NativeValue| match value {
            NativeValue::I64(value) => Ok(value),
            _ => Err(NativeServiceError::HostFailure),
        };
        let as_f64 = |value: NativeValue| match value {
            NativeValue::F64Bits(bits) => Ok(f64::from_bits(bits)),
            _ => Err(NativeServiceError::HostFailure),
        };
        let as_reference = |value: NativeValue| match value {
            NativeValue::Reference(reference) => Ok(reference),
            _ => Err(NativeServiceError::HostFailure),
        };
        match descriptor.operation() {
            HeapOperation::ConstantStr(text) => {
                let ValueType::Reference(reference_type) = result_type else {
                    return self.trap("string constant result layout mismatch");
                };
                let value = self.allocate(HeapObj::Str(text.clone()), reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::EmptyStr => {
                let ValueType::Reference(reference_type) = result_type else {
                    return self.trap("empty-str result layout mismatch");
                };
                let value = self.allocate(HeapObj::Str(String::new()), reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::EmptyList | HeapOperation::None => Ok(NativeValue::Reference(
                lkjscript_native::NativeReference::new(
                    result_type
                        .reference_type()
                        .ok_or(NativeServiceError::HostFailure)?,
                    0,
                ),
            )),
            HeapOperation::ProductValue { product, fields } => {
                if usize::from(*fields) != arguments.len() {
                    return self.trap("product field count mismatch");
                }
                let fields = arguments
                    .iter()
                    .copied()
                    .map(|value| self.value_from_native(value))
                    .collect::<Result<Vec<_>, _>>()?;
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(
                    HeapObj::Product {
                        product: ProductId::new(
                            u16::try_from(*product).map_err(|_| NativeServiceError::HostFailure)?,
                        ),
                        fields,
                    },
                    reference_type,
                )?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::ProductField { product, field, .. } => {
                let product_value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let value = match self.heap.get(product_value) {
                    Ok(HeapObj::Product {
                        product: actual,
                        fields,
                    }) if u32::from(actual.raw()) == *product => {
                        fields.get(usize::from(*field)).copied()
                    }
                    _ => return self.trap("product field identity mismatch"),
                }
                .ok_or_else(|| {
                    self.last_trap = Some("product field out of bounds".into());
                    NativeServiceError::Trap
                })?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::WithProductField { product, field, .. } => {
                let source = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let replacement = self.value_from_native(argument(1)?)?;
                let mut fields = match self.heap.get(source) {
                    Ok(HeapObj::Product {
                        product: actual,
                        fields,
                    }) if u32::from(actual.raw()) == *product => fields.clone(),
                    _ => return self.trap("product replacement identity mismatch"),
                };
                let Some(slot) = fields.get_mut(usize::from(*field)) else {
                    return self.trap("product replacement field out of bounds");
                };
                *slot = replacement;
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(
                    HeapObj::Product {
                        product: ProductId::new(
                            u16::try_from(*product).map_err(|_| NativeServiceError::HostFailure)?,
                        ),
                        fields,
                    },
                    reference_type,
                )?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::Cons => {
                let car = self.value_from_native(argument(0)?)?;
                let cdr = native_reference_value(self.heap, as_reference(argument(1)?)?).map_err(
                    |message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    },
                )?;
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let pair = self.allocate(HeapObj::Pair { car, cdr }, reference_type)?;
                self.native_from_value(pair, result_type)
            }
            HeapOperation::Car | HeapOperation::Cdr => {
                let list = native_reference_value(self.heap, as_reference(argument(0)?)?).map_err(
                    |message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    },
                )?;
                if list.is_empty_list() {
                    return self.trap(if matches!(descriptor.operation(), HeapOperation::Car) {
                        "car expects pair"
                    } else {
                        "cdr expects pair"
                    });
                }
                let value = match self.heap.get(list) {
                    Ok(HeapObj::Pair { car, cdr }) => {
                        if matches!(descriptor.operation(), HeapOperation::Car) {
                            *car
                        } else {
                            *cdr
                        }
                    }
                    _ => {
                        return self.trap(if matches!(descriptor.operation(), HeapOperation::Car) {
                            "car expects pair"
                        } else {
                            "cdr expects pair"
                        })
                    }
                };
                self.native_from_value(value, result_type)
            }
            HeapOperation::IsEmptyList => {
                let reference = as_reference(argument(0)?)?;
                let value = native_reference_value(self.heap, reference).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })?;
                Ok(NativeValue::Bool(value.is_empty_list()))
            }
            HeapOperation::Some | HeapOperation::Ok | HeapOperation::Err => {
                let inner = self.value_from_native(argument(0)?)?;
                let object = match descriptor.operation() {
                    HeapOperation::Some => HeapObj::OptionSome(inner),
                    HeapOperation::Ok => HeapObj::ResultOk(inner),
                    HeapOperation::Err => HeapObj::ResultErr(inner),
                    _ => return Err(NativeServiceError::HostFailure),
                };
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(object, reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::IsSome => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                Ok(NativeValue::Bool(!value.is_none()))
            }
            HeapOperation::UnwrapSome => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                if value.is_none() {
                    return self.trap("unwrap-some on none");
                }
                let inner = match self.heap.get(value) {
                    Ok(HeapObj::OptionSome(inner)) => *inner,
                    _ => return self.trap("unwrap-some operand is not Option"),
                };
                self.native_from_value(inner, result_type)
            }
            HeapOperation::IsOk => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                match self.heap.get(value) {
                    Ok(HeapObj::ResultOk(_)) => Ok(NativeValue::Bool(true)),
                    Ok(HeapObj::ResultErr(_)) => Ok(NativeValue::Bool(false)),
                    _ => self.trap("is-ok operand is not Result"),
                }
            }
            HeapOperation::UnwrapOk | HeapOperation::UnwrapErr => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let inner = match (descriptor.operation(), self.heap.get(value)) {
                    (HeapOperation::UnwrapOk, Ok(HeapObj::ResultOk(inner)))
                    | (HeapOperation::UnwrapErr, Ok(HeapObj::ResultErr(inner))) => *inner,
                    (HeapOperation::UnwrapOk, Ok(HeapObj::ResultErr(error))) => {
                        let message = match self.heap.get(*error) {
                            Ok(HeapObj::Str(message)) => format!("unwrap-ok: {message}"),
                            _ => "unwrap-ok on Err".to_string(),
                        };
                        return self.trap(message);
                    }
                    (HeapOperation::UnwrapErr, Ok(HeapObj::ResultOk(_))) => {
                        return self.trap("unwrap-err on Ok")
                    }
                    _ => return self.trap("unwrap Result category mismatch"),
                };
                self.native_from_value(inner, result_type)
            }
            HeapOperation::BufNew => {
                let size = usize::try_from(as_i64(argument(0)?)?).map_err(|_| {
                    self.last_trap = Some("buf-new size out of range".into());
                    NativeServiceError::Trap
                })?;
                if size > MAX_BUFFER_BYTES {
                    return self.trap("buf-new size out of range");
                }
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(HeapObj::Buf(vec![0; size]), reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::BufLen | HeapOperation::BufRef | HeapOperation::BufGetU32 => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let HeapObj::Buf(bytes) =
                    self.heap.get(value).map_err(|_| NativeServiceError::Trap)?
                else {
                    return self.trap("expected buf");
                };
                match descriptor.operation() {
                    HeapOperation::BufLen => Ok(NativeValue::I64(
                        i64::try_from(bytes.len()).map_err(|_| NativeServiceError::Trap)?,
                    )),
                    HeapOperation::BufRef => {
                        let index = index(as_i64(argument(1)?)?, "buf-ref").map_err(|message| {
                            self.last_trap = Some(message);
                            NativeServiceError::Trap
                        })?;
                        let byte = bytes.get(index).copied().ok_or_else(|| {
                            self.last_trap = Some("buf-ref out of bounds".into());
                            NativeServiceError::Trap
                        })?;
                        Ok(NativeValue::I64(i64::from(byte)))
                    }
                    HeapOperation::BufGetU32 => {
                        let index =
                            index(as_i64(argument(1)?)?, "buf-get-u32").map_err(|message| {
                                self.last_trap = Some(message);
                                NativeServiceError::Trap
                            })?;
                        let end = index.checked_add(4).ok_or_else(|| {
                            self.last_trap = Some("buf-get-u32 index overflow".into());
                            NativeServiceError::Trap
                        })?;
                        let slice = bytes.get(index..end).ok_or_else(|| {
                            self.last_trap = Some("buf-get-u32 out of bounds".into());
                            NativeServiceError::Trap
                        })?;
                        let mut word = [0; 4];
                        word.copy_from_slice(slice);
                        Ok(NativeValue::I64(i64::from(u32::from_le_bytes(word))))
                    }
                    _ => Err(NativeServiceError::HostFailure),
                }
            }
            HeapOperation::BufSet | HeapOperation::BufSetU32 => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let number = as_i64(argument(2)?)?;
                let operation = if matches!(descriptor.operation(), HeapOperation::BufSet) {
                    "buf-set"
                } else {
                    "buf-set-u32"
                };
                let index = index(as_i64(argument(1)?)?, operation).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })?;
                if matches!(descriptor.operation(), HeapOperation::BufSet) {
                    let byte = u8::try_from(number).map_err(|_| {
                        self.last_trap = Some("buf-set byte out of range".into());
                        NativeServiceError::Trap
                    })?;
                    match self.heap.get(value) {
                        Ok(HeapObj::Buf(bytes)) if index < bytes.len() => {}
                        Ok(HeapObj::Buf(_)) => return self.trap("buf-set out of bounds"),
                        _ => return self.trap("expected buf"),
                    }
                    self.mutate(value, |object| {
                        let HeapObj::Buf(bytes) = object else {
                            return Err(CoreError::msg("expected buf"));
                        };
                        bytes[index] = byte;
                        Ok(())
                    })?;
                } else {
                    let end = index.checked_add(4).ok_or_else(|| {
                        self.last_trap = Some("buf-set-u32 index overflow".into());
                        NativeServiceError::Trap
                    })?;
                    let number = u32::try_from(number).map_err(|_| {
                        self.last_trap = Some("buf-set-u32 value out of range".into());
                        NativeServiceError::Trap
                    })?;
                    match self.heap.get(value) {
                        Ok(HeapObj::Buf(bytes)) if end <= bytes.len() => {}
                        Ok(HeapObj::Buf(_)) => return self.trap("buf-set-u32 out of bounds"),
                        _ => return self.trap("expected buf"),
                    }
                    self.mutate(value, |object| {
                        let HeapObj::Buf(bytes) = object else {
                            return Err(CoreError::msg("expected buf"));
                        };
                        bytes[index..end].copy_from_slice(&number.to_le_bytes());
                        Ok(())
                    })?;
                }
                Ok(NativeValue::Unit)
            }
            HeapOperation::BufClone | HeapOperation::BufFromStr => {
                let bytes = if matches!(descriptor.operation(), HeapOperation::BufClone) {
                    let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                        .map_err(|message| {
                            self.last_trap = Some(message);
                            NativeServiceError::Trap
                        })?;
                    match self.heap.get(value) {
                        Ok(HeapObj::Buf(bytes)) => bytes.clone(),
                        _ => return self.trap("expected buf"),
                    }
                } else {
                    let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                        .map_err(|message| {
                            self.last_trap = Some(message);
                            NativeServiceError::Trap
                        })?;
                    match self.heap.get(value) {
                        Ok(HeapObj::Str(text)) => text.as_bytes().to_vec(),
                        _ => return self.trap("expected string"),
                    }
                };
                if bytes.len() > MAX_BUFFER_BYTES {
                    return self.trap("buf-from-str string exceeds buffer limit");
                }
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(HeapObj::Buf(bytes), reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::BufToStr | HeapOperation::BufSlice => {
                let buffer = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let bytes = match self.heap.get(buffer) {
                    Ok(HeapObj::Buf(bytes)) => bytes.clone(),
                    _ => return self.trap("expected buf"),
                };
                let (success, payload_type) = match descriptor.operation() {
                    HeapOperation::BufToStr => match String::from_utf8(bytes) {
                        Ok(text) => (Ok(HeapObj::Str(text)), ReferenceType::Str),
                        Err(_) => (Err("buf-to-str: invalid UTF-8"), ReferenceType::Str),
                    },
                    HeapOperation::BufSlice => {
                        let offset = match usize::try_from(as_i64(argument(1)?)?) {
                            Ok(offset) => offset,
                            Err(_) => {
                                return self
                                    .result_error("buf-slice offset out of range", result_type)
                            }
                        };
                        let length = match usize::try_from(as_i64(argument(2)?)?) {
                            Ok(length) => length,
                            Err(_) => {
                                return self
                                    .result_error("buf-slice length out of range", result_type)
                            }
                        };
                        let Some(end) = offset.checked_add(length) else {
                            return self.result_error("buf-slice range overflow", result_type);
                        };
                        match bytes.get(offset..end) {
                            Some(slice) => (Ok(HeapObj::Buf(slice.to_vec())), ReferenceType::Buf),
                            None => (Err("buf-slice range out of bounds"), ReferenceType::Str),
                        }
                    }
                    _ => return Err(NativeServiceError::HostFailure),
                };
                let is_ok = success.is_ok();
                let payload = match success {
                    Ok(object) => self.allocate(object, payload_type)?,
                    Err(message) => {
                        self.allocate(HeapObj::Str(message.into()), ReferenceType::Str)?
                    }
                };
                let result_reference = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let wrapper = if is_ok {
                    HeapObj::ResultOk(payload)
                } else {
                    HeapObj::ResultErr(payload)
                };
                let value = self.allocate(wrapper, result_reference)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::StrLen | HeapOperation::StrRef => {
                let value = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let text = match self.heap.get(value) {
                    Ok(HeapObj::Str(text)) => text,
                    _ => return self.trap("expected string"),
                };
                if matches!(descriptor.operation(), HeapOperation::StrLen) {
                    Ok(NativeValue::I64(
                        i64::try_from(text.len()).map_err(|_| NativeServiceError::Trap)?,
                    ))
                } else {
                    let index = index(as_i64(argument(1)?)?, "str-ref").map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                    let byte = text.as_bytes().get(index).copied().ok_or_else(|| {
                        self.last_trap = Some("str-ref out of bounds".into());
                        NativeServiceError::Trap
                    })?;
                    Ok(NativeValue::I64(i64::from(byte)))
                }
            }
            HeapOperation::StrAppend | HeapOperation::StrSlice => {
                let first = native_reference_value(self.heap, as_reference(argument(0)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let mut text = match self.heap.get(first) {
                    Ok(HeapObj::Str(text)) => text.clone(),
                    _ => return self.trap("expected string"),
                };
                if matches!(descriptor.operation(), HeapOperation::StrAppend) {
                    let second = native_reference_value(self.heap, as_reference(argument(1)?)?)
                        .map_err(|message| {
                            self.last_trap = Some(message);
                            NativeServiceError::Trap
                        })?;
                    let right = match self.heap.get(second) {
                        Ok(HeapObj::Str(text)) => text,
                        _ => return self.trap("expected string"),
                    };
                    text.push_str(right);
                } else {
                    let start = usize::try_from(as_i64(argument(1)?)?).map_err(|_| {
                        self.last_trap = Some("str-slice start out of range".into());
                        NativeServiceError::Trap
                    })?;
                    let end = usize::try_from(as_i64(argument(2)?)?).map_err(|_| {
                        self.last_trap = Some("str-slice end out of range".into());
                        NativeServiceError::Trap
                    })?;
                    let bytes = text.as_bytes().get(start..end).ok_or_else(|| {
                        self.last_trap = Some("str-slice out of bounds".into());
                        NativeServiceError::Trap
                    })?;
                    text = std::str::from_utf8(bytes)
                        .map_err(|_| {
                            self.last_trap = Some("str-slice splits UTF-8".into());
                            NativeServiceError::Trap
                        })?
                        .to_owned();
                }
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(HeapObj::Str(text), reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::StrFromByte | HeapOperation::StrFromI64 | HeapOperation::StrFromF64 => {
                let text = match descriptor.operation() {
                    HeapOperation::StrFromByte => {
                        let byte = u8::try_from(as_i64(argument(0)?)?).map_err(|_| {
                            self.last_trap = Some("str-from-byte out of range".into());
                            NativeServiceError::Trap
                        })?;
                        String::from(char::from(byte))
                    }
                    HeapOperation::StrFromI64 => as_i64(argument(0)?)?.to_string(),
                    HeapOperation::StrFromF64 => as_f64(argument(0)?)?.to_string(),
                    _ => return Err(NativeServiceError::HostFailure),
                };
                let reference_type = result_type
                    .reference_type()
                    .ok_or(NativeServiceError::HostFailure)?;
                let value = self.allocate(HeapObj::Str(text), reference_type)?;
                self.native_from_value(value, result_type)
            }
            HeapOperation::EqualValue => {
                let left = self.value_from_native(argument(0)?)?;
                let right = self.value_from_native(argument(1)?)?;
                let equal = value_equal(self.heap, left, right).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })?;
                Ok(NativeValue::Bool(equal))
            }
            HeapOperation::SameObject => {
                let left = as_reference(argument(0)?)?;
                let right = as_reference(argument(1)?)?;
                Ok(NativeValue::Bool(left.opaque_word() == right.opaque_word()))
            }
            HeapOperation::ListEqual => {
                let left = native_reference_value(self.heap, as_reference(argument(0)?)?).map_err(
                    |message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    },
                )?;
                let right = native_reference_value(self.heap, as_reference(argument(1)?)?)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                let equal = list_values_equal(self.heap, left, right, MAX_LIST_EQUAL_STEPS)
                    .map_err(|message| {
                        self.last_trap = Some(message);
                        NativeServiceError::Trap
                    })?;
                Ok(NativeValue::Bool(equal))
            }
        }
    }
}

impl NativeRuntimeServices for JitHeapServices<'_> {
    fn collect_references(&mut self, roots: &mut [NativeRoot]) -> Result<(), NativeServiceError> {
        let values = self.roots(roots)?;
        self.heap.collect(&values);
        Ok(())
    }

    fn prepare_heap_operation(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
        roots: &mut [NativeRoot],
    ) -> Result<bool, NativeServiceError> {
        if arguments.len() != site.descriptor().input_types().len()
            || arguments
                .iter()
                .zip(site.descriptor().input_types())
                .any(|(value, expected)| value.value_type() != *expected)
        {
            return self.trap("heap runtime argument metadata mismatch");
        }
        for value in arguments {
            if let NativeValue::Reference(reference) = value {
                native_reference_value(self.heap, *reference).map_err(|message| {
                    self.last_trap = Some(message);
                    NativeServiceError::Trap
                })?;
            }
        }
        let root_values = self.roots(roots)?;
        // The baseline slice deliberately uses the bounded slow path for every
        // source allocation. This collects before publication; stress mode
        // additionally collects at non-allocating heap sites.
        let collected = self.force_collection
            || site.descriptor().allocation() == lkjscript_native::AllocationClass::Bounded;
        if collected {
            self.heap.collect(&root_values);
        }
        Ok(collected)
    }

    fn heap_operation(
        &mut self,
        site: &HeapRuntimeSite,
        arguments: &[NativeValue],
    ) -> Result<NativeValue, NativeServiceError> {
        self.execute(site, arguments)
    }
}

fn scalar_box_layout(discriminator: u32) -> ReferenceType {
    ReferenceType::Product(lkjscript_native::LayoutIdentity::new(
        u32::MAX - discriminator,
    ))
}

fn reference_layout_key(reference_type: ReferenceType) -> u64 {
    match reference_type {
        ReferenceType::Buf => 1_u64 << 56,
        ReferenceType::Str => 2_u64 << 56,
        ReferenceType::List(layout, _) => (3_u64 << 56) | u64::from(layout.get()),
        ReferenceType::Option(layout, _) => (4_u64 << 56) | u64::from(layout.get()),
        ReferenceType::Result(layout, _, _) => (5_u64 << 56) | u64::from(layout.get()),
        ReferenceType::Product(layout) => (6_u64 << 56) | u64::from(layout.get()),
    }
}

fn native_reference_value(
    heap: &GcHeap,
    reference: lkjscript_native::NativeReference,
) -> Result<Value, String> {
    let reference_type = reference.reference_type();
    let word = reference.opaque_word();
    if word == 0 {
        return match reference_type {
            ReferenceType::List(_, _) => Ok(Value::EMPTY_LIST),
            ReferenceType::Option(_, _) => Ok(Value::NONE),
            _ => Err("zero native reference is invalid for this category".into()),
        };
    }
    let index = u32::try_from(word - 1).map_err(|_| "native heap handle out of range")?;
    let value = Value::from_heap(index);
    if heap.layout_of(value) != Some(reference_layout_key(reference_type)) {
        return Err("native heap handle layout mismatch".into());
    }
    let category_matches = match (reference_type, heap.get(value)) {
        (ReferenceType::Buf, Ok(HeapObj::Buf(_)))
        | (ReferenceType::Str, Ok(HeapObj::Str(_)))
        | (ReferenceType::List(_, _), Ok(HeapObj::Pair { .. }))
        | (ReferenceType::Option(_, _), Ok(HeapObj::OptionSome(_)))
        | (ReferenceType::Result(_, _, _), Ok(HeapObj::ResultOk(_) | HeapObj::ResultErr(_))) => {
            true
        }
        (ReferenceType::Product(layout), Ok(HeapObj::Product { product, .. })) => {
            layout == lkjscript_native::LayoutIdentity::product(u32::from(product.raw()))
        }
        _ => false,
    };
    if !category_matches {
        return Err("native heap handle category mismatch".into());
    }
    Ok(value)
}

fn reference_native_value(
    heap: &GcHeap,
    value: Value,
    reference_type: ReferenceType,
) -> Result<NativeValue, String> {
    if value.is_empty_list() && matches!(reference_type, ReferenceType::List(_, _))
        || value.is_none() && matches!(reference_type, ReferenceType::Option(_, _))
    {
        return Ok(NativeValue::Reference(
            lkjscript_native::NativeReference::new(reference_type, 0),
        ));
    }
    let index = value.as_heap().ok_or("expected heap reference result")?;
    let reference = lkjscript_native::NativeReference::new(reference_type, u64::from(index) + 1);
    native_reference_value(heap, reference)?;
    Ok(NativeValue::Reference(reference))
}

fn index(value: i64, operation: &str) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| format!("{operation} index out of range"))
}

fn list_values_equal(
    heap: &GcHeap,
    mut left: Value,
    mut right: Value,
    limit: usize,
) -> Result<bool, String> {
    for _ in 0..limit {
        if left.is_empty_list() || right.is_empty_list() {
            return Ok(left.is_empty_list() && right.is_empty_list());
        }
        let (left_car, left_cdr) = match heap.get(left) {
            Ok(HeapObj::Pair { car, cdr }) => (*car, *cdr),
            _ => return Err("list-equal expects proper List values".into()),
        };
        let (right_car, right_cdr) = match heap.get(right) {
            Ok(HeapObj::Pair { car, cdr }) => (*car, *cdr),
            _ => return Err("list-equal expects proper List values".into()),
        };
        if !value_equal(heap, left_car, right_car)? {
            return Ok(false);
        }
        left = left_cdr;
        right = right_cdr;
    }
    if left.is_empty_list() || right.is_empty_list() {
        Ok(left.is_empty_list() && right.is_empty_list())
    } else {
        Err("list-equal step limit exceeded".into())
    }
}

fn value_equal(heap: &GcHeap, left: Value, right: Value) -> Result<bool, String> {
    if left == right {
        return Ok(true);
    }
    if let (Some(left), Some(right)) = (left.as_bool(), right.as_bool()) {
        return Ok(left == right);
    }
    if let (Some(left), Some(right)) = (left.as_small_i64(), right.as_small_i64()) {
        return Ok(left == right);
    }
    match (heap.get(left), heap.get(right)) {
        (Ok(HeapObj::Int(left)), Ok(HeapObj::Int(right))) => Ok(left == right),
        (Ok(HeapObj::Float(left)), Ok(HeapObj::Float(right))) => Ok(left == right),
        (Ok(HeapObj::Str(left)), Ok(HeapObj::Str(right))) => Ok(left == right),
        (Ok(HeapObj::OptionSome(left)), Ok(HeapObj::OptionSome(right)))
        | (Ok(HeapObj::ResultOk(left)), Ok(HeapObj::ResultOk(right)))
        | (Ok(HeapObj::ResultErr(left)), Ok(HeapObj::ResultErr(right))) => {
            value_equal(heap, *left, *right)
        }
        (Ok(HeapObj::ResultOk(_)), Ok(HeapObj::ResultErr(_)))
        | (Ok(HeapObj::ResultErr(_)), Ok(HeapObj::ResultOk(_))) => Ok(false),
        _ if left.is_none() || right.is_none() => Ok(left.is_none() && right.is_none()),
        _ => Err("equal-value category mismatch".into()),
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
    use lkjscript_core::{
        ExecutionConfig, ExecutionOutcome, GcHeap, HeapObj, Value, MAX_LIST_EQUAL_STEPS,
    };
    use lkjscript_ir::{
        verify, Block, BlockId, BlockMetadata, BlockParameter, CallTarget, Constant, EffectSet,
        FailureBehavior, FrameState, Function, FunctionId, Instruction, InstructionKind,
        InstructionMetadata, Origin, Program, Safepoint as IrSafepoint, Signature, SourceMetadata,
        SsaType, StructuredOutcome, Terminator, TraitId, TraitMetadata, TraitRole, ValueId,
    };

    use super::{execute_forced, JitConfig};

    fn core_traits() -> Vec<TraitMetadata> {
        [
            ("Copy", TraitRole::Copy),
            ("Clone", TraitRole::Clone),
            ("Drop", TraitRole::Drop),
            ("Send", TraitRole::Send),
            ("Sync", TraitRole::Sync),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (name, role))| TraitMetadata {
            id: TraitId::new(index as u32),
            name: name.into(),
            role,
            source: None,
        })
        .collect()
    }

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
            traits: core_traits(),
            implementations: Vec::new(),
            functions: vec![Function {
                id: FunctionId::new(0),
                name: "main".into(),
                signature: Signature::monomorphic(Vec::new(), SsaType::I64),
                places: Vec::new(),
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
    fn product11_product0_and_product19_unit_result_layouts_reject_cross_handles() {
        let mix = |state: u32, value: u32| state.wrapping_mul(16_777_619) ^ value;
        let old_product = |product: u32| mix(8, product + 1);
        assert_eq!(
            mix(old_product(11), old_product(0)),
            mix(
                old_product(19),
                lkjscript_native::ValueType::Unit.layout_identity().get(),
            ),
            "the superseded 32-bit Result heap tag collided"
        );

        let first = lkjscript_native::ReferenceType::Result(
            lkjscript_native::LayoutIdentity::new(70_000),
            lkjscript_native::LayoutIdentity::product(11),
            lkjscript_native::LayoutIdentity::product(0),
        );
        let second = lkjscript_native::ReferenceType::Result(
            lkjscript_native::LayoutIdentity::new(70_001),
            lkjscript_native::LayoutIdentity::product(19),
            lkjscript_native::ValueType::Unit.layout_identity(),
        );
        assert_ne!(
            super::reference_layout_key(first),
            super::reference_layout_key(second)
        );

        let mut heap = GcHeap::default();
        let value = heap
            .try_alloc_with_layout(
                HeapObj::ResultOk(Value::UNIT),
                super::reference_layout_key(first),
            )
            .expect("first Result layout allocation");
        let reference = lkjscript_native::NativeReference::new(
            second,
            u64::from(value.as_heap().expect("heap handle")) + 1,
        );
        assert_eq!(
            super::native_reference_value(&heap, reference),
            Err("native heap handle layout mismatch".into())
        );
    }

    #[test]
    fn generated_list_equality_accepts_exact_limit_and_rejects_limit_plus_one() {
        let mut heap = GcHeap::default();
        let mut at_limit = Value::EMPTY_LIST;
        for _ in 0..MAX_LIST_EQUAL_STEPS {
            at_limit = heap
                .alloc(HeapObj::Pair {
                    car: Value::UNIT,
                    cdr: at_limit,
                })
                .expect("list allocation");
        }
        assert_eq!(
            super::list_values_equal(&heap, at_limit, at_limit, MAX_LIST_EQUAL_STEPS),
            Ok(true)
        );
        let over_limit = heap
            .alloc(HeapObj::Pair {
                car: Value::UNIT,
                cdr: at_limit,
            })
            .expect("over-limit list allocation");
        assert_eq!(
            super::list_values_equal(&heap, over_limit, over_limit, MAX_LIST_EQUAL_STEPS),
            Err("list-equal step limit exceeded".into())
        );
    }

    #[test]
    fn selected_conditional_callee_trap_retains_exact_site_message() {
        let metadata = |effects, safepoint, failure| InstructionMetadata {
            origin: Origin::SYNTHETIC,
            effects,
            safepoint,
            failure,
            frame_state: (safepoint == IrSafepoint::Required).then_some(FrameState {
                bytecode_position: 0,
                locals: Vec::new(),
                operand_stack: Vec::new(),
            }),
        };
        let block_metadata = || BlockMetadata {
            loop_header: false,
            origin: Origin::SYNTHETIC,
            frame_state: None,
        };
        let callee_signature = Signature::monomorphic(vec![SsaType::Bool], SsaType::I64);
        let program = verify(Program {
            sources: vec![SourceMetadata {
                id: 0,
                path: "selected-trap.lkjscript".into(),
            }],
            products: Vec::new(),
            traits: core_traits(),
            implementations: Vec::new(),
            functions: vec![
                Function {
                    id: FunctionId::new(0),
                    name: "choose".into(),
                    signature: callee_signature.clone(),
                    places: Vec::new(),
                    effects: EffectSet::MAY_TRAP,
                    entry: BlockId::new(0),
                    blocks: vec![
                        Block {
                            id: BlockId::new(0),
                            parameters: vec![BlockParameter {
                                id: ValueId::new(0),
                                ty: SsaType::Bool,
                                owner_place: None,
                                origin: Origin::SYNTHETIC,
                            }],
                            instructions: Vec::new(),
                            terminator: Terminator::ConditionalBranch {
                                condition: ValueId::new(0),
                                true_target: BlockId::new(1),
                                true_arguments: Vec::new(),
                                false_target: BlockId::new(2),
                                false_arguments: Vec::new(),
                            },
                            metadata: block_metadata(),
                        },
                        Block {
                            id: BlockId::new(1),
                            parameters: Vec::new(),
                            instructions: Vec::new(),
                            terminator: Terminator::Trap {
                                message: "first trap".into(),
                            },
                            metadata: block_metadata(),
                        },
                        Block {
                            id: BlockId::new(2),
                            parameters: Vec::new(),
                            instructions: Vec::new(),
                            terminator: Terminator::Trap {
                                message: "selected callee trap".into(),
                            },
                            metadata: block_metadata(),
                        },
                    ],
                    origin: Origin::SYNTHETIC,
                },
                Function {
                    id: FunctionId::new(1),
                    name: "main".into(),
                    signature: Signature::monomorphic(Vec::new(), SsaType::I64),
                    places: Vec::new(),
                    effects: EffectSet::MAY_TRAP,
                    entry: BlockId::new(0),
                    blocks: vec![Block {
                        id: BlockId::new(0),
                        parameters: Vec::new(),
                        instructions: vec![
                            Instruction {
                                id: ValueId::new(0),
                                ty: SsaType::Bool,
                                kind: InstructionKind::Constant(Constant::Bool(false)),
                                metadata: metadata(
                                    EffectSet::PURE,
                                    IrSafepoint::None,
                                    FailureBehavior::None,
                                ),
                            },
                            Instruction {
                                id: ValueId::new(1),
                                ty: SsaType::I64,
                                kind: InstructionKind::Call {
                                    target: CallTarget::Direct(FunctionId::new(0)),
                                    arguments: vec![ValueId::new(0)],
                                    signature: callee_signature,
                                    instantiation: None,
                                },
                                metadata: metadata(
                                    EffectSet::MAY_TRAP,
                                    IrSafepoint::Required,
                                    FailureBehavior::Trap,
                                ),
                            },
                        ],
                        terminator: Terminator::Return(ValueId::new(1)),
                        metadata: block_metadata(),
                    }],
                    origin: Origin::SYNTHETIC,
                },
            ],
            main: FunctionId::new(1),
        })
        .expect("verify selected trap SSA");
        let executed = execute_forced(&program, &ExecutionConfig::default(), JitConfig::default())
            .expect("execute selected native trap");
        assert!(matches!(
            executed.outcome,
            ExecutionOutcome::Trapped(trap) if trap.as_str() == "selected callee trap"
        ));
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
