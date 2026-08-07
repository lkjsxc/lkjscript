//! Verified typed-SSA to callable Linux x86-64 baseline-JIT runtime.
//!
//! The product entry point makes one baseline compilation and pre-entry
//! preparation attempt, then either executes the installed native group or
//! declines without entering generated code. It never consumes syntax, HIR,
//! or bytecode semantics.

#![forbid(unsafe_code)]

mod attempt;
mod lower;

use std::time::{Duration, Instant};

use lkjscript_core::{
    CleanupFailures, CleanupPhase, CleanupSubject, ExecutionOutcome, ExecutionPolicy, HostError,
    OwnedValue, ResourceLimitKind, SemanticValue, Trap, UniqueKeyWord, UniqueStore,
    UniqueStoreError, UniqueStoreId, Value,
};
use lkjscript_executable::{
    ExecutableInstaller, ExecutableLimits, InstallError, InstalledImage, InvocationOutcome,
    InvocationReport, NativeInvocationConfig, NativeResourceLimitKind, NativeRuntimeServices,
    NativeServiceError,
};
use lkjscript_ir::{
    optimize, optimize_scheduled, OptimizationCertificate, OptimizationFailureCode,
    OptimizationLimits, OptimizationStats, SsaType, VerifiedOptimizedProgram, VerifiedProgram,
};
use lkjscript_native::{
    BackendLimits, CodeAccounting, EntryMetadata, HeapOperation, HeapRuntimeSite, ImageContracts,
    LoanType, NativeLoan, NativeUnique, ReferenceType, Relocation, RuntimeCallSlot,
};

pub use lkjscript_ir::FunctionId;
pub use lkjscript_native::{NativeValue, TrapCode, ValueType};
pub use lower::{LoweringError, LoweringFailureCode};

mod code;
mod config;
mod error;
mod execute;
mod island;
mod native_run;
mod resource_plan;
mod runtime_values;
mod scalar;
mod stats;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests;

use code::CodeObject;
use execute::{capability_arguments, optimization_metadata_bytes_estimate};
use island::*;
use runtime_values::*;
use scalar::scalar_to_execution;

pub use attempt::{
    attempt_baseline, attempt_baseline_with_capabilities,
    attempt_baseline_with_capabilities_from_start, BaselineAttempt, BaselineAttemptTimings,
    BaselineDecline, BaselineDeclineReason, BaselineEnteredError, BaselineEnteredFailure,
    BaselineExecution,
};
pub use code::{CodeObjectRecord, NumericConversionSiteCounts};
pub use config::JitConfig;
pub use error::{EngineError, FailureCode};
pub use execute::{
    execute_forced, execute_forced_with_capabilities, execute_optimizing,
    execute_optimizing_with_capabilities,
};
pub use lkjscript_executable::{EnteredInvocationError, PreEntryError};
pub use scalar::{native_type, JitExecution, ScalarInvocation, ScalarInvocationOutcome};
pub use stats::{
    CompileStats, JitStats, NativeResourceStats, NativeStructuralStats, NativeUniqueStats,
};

struct ReturnedStructuralValue(SemanticValue);

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
}

struct NativeRun {
    program: ProgramAuthority,
    installer: ExecutableInstaller,
    config: JitConfig,
    object: Option<CodeObject>,
    require_pre_entry_stack_check: bool,
    total_compile_time: Duration,
    optimization_time: Duration,
    native_entries: u64,
    direct_native_calls: u64,
    poll_calls: u64,
    native_invocations: u64,
    metrics_started: Option<Instant>,
    time_to_first_native_entry: Option<Duration>,
    first_native_call: Option<Duration>,
    native_execution: Duration,
    diagnostic_bytes: u64,
    lists: Option<lkjscript_core::SegmentedListArena<Value>>,
    region_products: Option<lkjscript_core::RegionProductArena<Value>>,
    runtime_heap_attempts: u64,
    runtime_heap_successes: u64,
    resource_runtime_calls: u64,
    unique_runtime_calls: u64,
    structural_runtime_calls: u64,
    native_resources: NativeResourceStats,
    native_unique: NativeUniqueStats,
    native_structural: NativeStructuralStats,
    returned_unique: Option<Vec<u8>>,
    returned_structural: Option<ReturnedStructuralValue>,
    next_resource_scope: u64,
    peak_native_frame_depth: usize,
    peak_native_stack_bytes: usize,
    last_runtime_trap: Option<String>,
    last_runtime_resource: Option<ResourceLimitKind>,
    last_runtime_failure: Option<NativeServiceError>,
    last_lowering_and_encoding: Duration,
    last_installation: Duration,
}

#[derive(Clone, Copy)]
struct JitValueLimits {
    allocations: Option<u64>,
    runtime_bytes: Option<u64>,
}

struct JitValueServices<'a> {
    lists: &'a mut lkjscript_core::SegmentedListArena<Value>,
    region_products: &'a mut lkjscript_core::RegionProductArena<Value>,
    list_allocations: u64,
    region_product_allocations: u64,
    max_list_allocations: Option<u64>,
    max_runtime_bytes: Option<u64>,
    last_trap: Option<String>,
    last_resource: Option<ResourceLimitKind>,
}
