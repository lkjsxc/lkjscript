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

mod code;
mod config;
mod error;
mod execute;
mod heap;
mod scalar;
mod session;
mod stats;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

use execute::optimization_metadata_bytes_estimate;
use heap::*;
use scalar::scalar_to_execution;

pub use code::{CodeObject, CodeObjectRecord};
pub use config::{JitConfig, Tier, TierState};
pub use error::{EngineError, FailureCode};
pub use execute::{execute_forced, execute_optimizing};
pub use scalar::{
    native_type, EntryDecision, JitExecution, ScalarInvocation, ScalarInvocationOutcome,
    ScalarSignature,
};
pub use stats::{CompileStats, FunctionTierRecord, JitStats};

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

struct JitHeapServices<'a> {
    heap: &'a mut GcHeap,
    enums: &'a [lkjscript_ir::EnumMetadata],
    force_collection: bool,
    logical_aggregate_constructions: u64,
    max_logical_aggregate_constructions: u64,
    last_trap: Option<String>,
    last_resource: Option<ResourceLimitKind>,
}
