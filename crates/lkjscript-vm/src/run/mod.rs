//! Bytecode interpreter over a validated immutable chunk.
mod calls;
mod control;
mod data;
mod dispatch;
mod enum_value;
mod ext_ops;
mod failure_cleanup;
mod host_ops;
mod numeric;
mod product;
mod state;
mod structural_ops;
#[cfg(test)]
#[path = "data/test_support.rs"]
mod test_support;
pub(crate) mod unique;
#[cfg(test)]
pub(crate) use test_support::test_chunk;
mod unique_ops;

use std::time::{Duration, Instant};

use lkjscript_core::{
    CleanupFailures, CleanupPhase, CleanupSubject, Constant, EnumFieldRef, EnumId, EnumVariantRef,
    Error, ErrorClass, ExecutionOutcome, ExecutionPolicy, FailureCleanupAction, HostError, Op,
    ProductFieldRef, ProductId, ResourceLimitKind, Result, Trap, ValidatedChunk, Value, VariantId,
};
#[cfg(feature = "jit")]
use lkjscript_jit::{
    EngineError, EntryDecision, FunctionId, JitSession, JitStats, NativeValue, ScalarInvocation,
    ScalarSignature, TrapCode,
};

use crate::host::{flush_out, read_byte, write_byte, write_output, write_str};
use crate::host_ext::ResourceTable;
use crate::ExecutionInputs;
use calls::{call, car, cdr, make_closure};

pub(crate) struct Frame {
    pub proto: Option<usize>,
    pub ip: usize,
    pub instruction_offset: usize,
    pub stack_base: usize,
    pub locals_base: usize,
    pub unique_places: Vec<unique::RuntimePlace>,
    pub borrowed_resources: Vec<Value>,
    pub memory_witnesses: Vec<lkjscript_core::MemoryWitnessBinding>,
}

enum Stop {
    Returned(Value),
    Exited(i32),
}

pub trait RuntimeTier {
    #[cfg(feature = "jit")]
    fn observe_function_entry(&mut self, prototype: u64) -> EntryDecision;
    #[cfg(feature = "jit")]
    fn scalar_signature(&self, function: FunctionId) -> Option<ScalarSignature>;
    #[cfg(feature = "jit")]
    fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionPolicy,
    ) -> std::result::Result<ScalarInvocation, EngineError>;
    #[cfg(feature = "jit")]
    fn trap_message(&self, function: FunctionId, trap: TrapCode, site: Option<u64>) -> String;
}

#[derive(Debug, Default)]
pub struct NoTier;

impl RuntimeTier for NoTier {
    #[cfg(feature = "jit")]
    fn observe_function_entry(&mut self, _prototype: u64) -> EntryDecision {
        EntryDecision::Interpret
    }

    #[cfg(feature = "jit")]
    fn scalar_signature(&self, _function: FunctionId) -> Option<ScalarSignature> {
        None
    }

    #[cfg(feature = "jit")]
    fn invoke_scalar(
        &mut self,
        function: FunctionId,
        _arguments: &[NativeValue],
        _execution: &ExecutionPolicy,
    ) -> std::result::Result<ScalarInvocation, EngineError> {
        Err(EngineError::new_unavailable(function))
    }

    #[cfg(feature = "jit")]
    fn trap_message(&self, _function: FunctionId, _trap: TrapCode, _site: Option<u64>) -> String {
        "native tier is unavailable".to_string()
    }
}

#[cfg(feature = "jit")]
impl RuntimeTier for JitSession {
    fn observe_function_entry(&mut self, prototype: u64) -> EntryDecision {
        JitSession::observe_function_entry(self, prototype)
    }

    fn scalar_signature(&self, function: FunctionId) -> Option<ScalarSignature> {
        JitSession::scalar_signature(self, function)
    }

    fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionPolicy,
    ) -> std::result::Result<ScalarInvocation, EngineError> {
        JitSession::invoke_scalar(self, function, arguments, execution)
    }

    fn trap_message(&self, function: FunctionId, trap: TrapCode, site: Option<u64>) -> String {
        self.trap_message_for(function, trap, site)
    }
}

pub struct Vm<'a, J: RuntimeTier> {
    pub(crate) chunk: &'a ValidatedChunk,
    pub(crate) globals: Vec<Value>,
    pub(crate) stack: Vec<Value>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) lists: Option<lkjscript_core::SegmentedListArena<Value>>,
    pub(crate) region_products: Option<lkjscript_core::RegionProductArena<Value>>,
    pub(crate) jit: J,
    pub(crate) exit_code: Option<i32>,
    pub(crate) inputs: ExecutionInputs,
    pub(crate) resources: ResourceTable,
    pub(crate) unique: unique::UniqueRuntime,
    pub(crate) structural: Option<structural_ops::StructuralInvocation>,
    structural_initialization_error: Option<Error>,
    global_initialization_error: Option<Error>,
    list_initialization_error: Option<Error>,
    region_product_initialization_error: Option<Error>,
    config: ExecutionPolicy,
    fuel_remaining: Option<u64>,
    output_bytes: usize,
    allocation_error: Option<Error>,
    cleanup_failures: CleanupFailures,
    list_allocations: u64,
    region_product_allocations: u64,
    started: Instant,
}

fn outcome_from_error(error: Error) -> ExecutionOutcome {
    match error.class() {
        ErrorClass::Ordinary => ExecutionOutcome::Trapped(Trap::new(error.to_string())),
        ErrorClass::Deadline => ExecutionOutcome::DeadlineExceeded,
        ErrorClass::Resource(kind) => ExecutionOutcome::ResourceLimitExceeded(kind),
        ErrorClass::BytecodePolicy | ErrorClass::Host => {
            ExecutionOutcome::HostFailure(HostError::new(error.to_string()))
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::field_reassign_with_default)]
mod tests;
