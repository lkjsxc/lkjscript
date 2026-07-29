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
    CleanupFailures, CleanupPhase, CleanupSubject, Error as CoreError, ErrorClass, ExecutionConfig,
    ExecutionOutcome, GcConfig, GcHeap, GcLimit, HeapObj, HostError, OwnedValue, ProductId,
    ResourceLimitKind, Trap, UniqueKeyWord, UniqueStore, UniqueStoreError, UniqueStoreId,
    UniqueStoreLimits, Value, MAX_BUFFER_BYTES, MAX_LIST_EQUAL_STEPS,
};
use lkjscript_executable::{
    ExecutableInstaller, ExecutableLimits, InstallError, InstalledImage, InvocationError,
    InvocationOutcome, InvocationReport, NativeInvocationConfig, NativeResourceLimitKind,
    NativeRoot, NativeRuntimeServices, NativeServiceError,
};
use lkjscript_ir::{
    optimize, optimize_scheduled, BytecodeLinkMetadata, OptimizationCertificate,
    OptimizationFailureCode, OptimizationLimits, OptimizationStats, Signature as IrSignature,
    SsaType, VerifiedOptimizedProgram, VerifiedProgram,
};
use lkjscript_native::{
    BackendLimits, CodeAccounting, EntryMetadata, FrameFacts, HeapOperation, HeapRuntimeSite,
    ImageContracts, LoanType, NativeLoan, NativeUnique, OutcomeMapEntry, ReferenceType, Relocation,
    RuntimeCallSlot, Safepoint, SourceMapEntry, TrapMapEntry,
};

pub use lkjscript_ir::FunctionId;
pub use lkjscript_native::{NativeValue, TrapCode, ValueType};
pub use lower::{LoweringError, LoweringFailureCode};

mod code;
mod config;
mod error;
mod execute;
mod heap;
mod island;
mod resource_plan;
mod scalar;
mod session;
mod stats;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

use execute::optimization_metadata_bytes_estimate;
use heap::*;
use island::*;
use scalar::scalar_to_execution;

pub use code::{CodeObject, CodeObjectRecord, NumericConversionSiteCounts};
pub use config::{JitConfig, Tier, TierState};
pub use error::{EngineError, FailureCode};
pub use execute::{
    execute_forced, execute_forced_with_capabilities, execute_optimizing,
    execute_optimizing_with_capabilities,
};
pub use scalar::{
    native_type, EntryDecision, JitExecution, ScalarInvocation, ScalarInvocationOutcome,
    ScalarSignature,
};
pub use stats::{
    CompileStats, FunctionTierRecord, JitStats, NativeResourceStats, NativeUniqueStats,
};

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
    poll_calls: u64,
    vm_fallbacks: u64,
    compile_failures: u64,
    native_invocations: u64,
    metrics_started: Option<Instant>,
    time_to_first_native_entry: Option<Duration>,
    first_native_call: Option<Duration>,
    native_execution: Duration,
    diagnostic_bytes: u64,
    heap: Option<GcHeap>,
    maximum_roots: usize,
    runtime_heap_attempts: u64,
    runtime_heap_successes: u64,
    barrier_count: u64,
    collector_runtime_invocations: u64,
    resource_runtime_calls: u64,
    unique_runtime_calls: u64,
    native_resources: NativeResourceStats,
    native_unique: NativeUniqueStats,
    returned_unique: Option<Vec<u8>>,
    next_resource_scope: u64,
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

struct JitHeapServices<'a> {
    heap: &'a mut GcHeap,
    enums: &'a [lkjscript_ir::EnumMetadata],
    force_collection: bool,
    logical_aggregate_constructions: u64,
    max_logical_aggregate_constructions: u64,
    last_trap: Option<String>,
    last_resource: Option<ResourceLimitKind>,
}
