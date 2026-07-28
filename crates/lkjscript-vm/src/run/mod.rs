//! Bytecode interpreter over a validated immutable chunk.

mod calls;
mod control;
mod data;
mod dispatch;
mod enum_value;
mod ext_ops;
mod host_ops;
mod limits;
mod numeric;
mod product;
mod state;
mod unique;
mod unique_ops;

use std::time::{Duration, Instant};

use lkjscript_core::{
    CleanupFailures, CleanupPhase, CleanupSubject, Constant, EnumConstructionRef, EnumFieldRef,
    EnumId, EnumVariantRef, Error, ErrorClass, ExecutionConfig, ExecutionOutcome, GcConfig,
    GcHeap as Arena, HeapObj, HostError, Op, ProductFieldRef, ProductId, ResourceLimitKind, Result,
    Trap, ValidatedChunk, Value, VariantId, MAX_LIST_EQUAL_STEPS, MAX_PRODUCT_FIELDS,
};
use lkjscript_jit::{
    EngineError, EntryDecision, FunctionId, JitSession, JitStats, NativeValue, ScalarInvocation,
    ScalarSignature, TrapCode,
};

use crate::host::{display_value, flush_out, read_byte, write_byte, write_output, write_str};
use crate::host_ext::ResourceTable;
use crate::ExecutionInputs;
use calls::{call, car, cdr, make_closure};

pub(crate) struct Frame {
    pub proto: u32,
    pub ip: usize,
    pub stack_base: usize,
    pub locals_base: usize,
    pub unique_places: Vec<unique::RuntimePlace>,
}

enum Stop {
    Returned(Value),
    Exited(i32),
}

pub trait RuntimeTier {
    fn observe_function_entry(&mut self, prototype: u32) -> EntryDecision;
    fn scalar_signature(&self, function: FunctionId) -> Option<ScalarSignature>;
    fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionConfig,
    ) -> std::result::Result<ScalarInvocation, EngineError>;
    fn trap_message(&self, function: FunctionId, trap: TrapCode, site: Option<u32>) -> String;
    fn record_invocation_failure(&mut self, function: FunctionId);
}

#[derive(Debug, Default)]
pub struct NoTier;

impl RuntimeTier for NoTier {
    fn observe_function_entry(&mut self, _prototype: u32) -> EntryDecision {
        EntryDecision::Interpret
    }

    fn scalar_signature(&self, _function: FunctionId) -> Option<ScalarSignature> {
        None
    }

    fn invoke_scalar(
        &mut self,
        function: FunctionId,
        _arguments: &[NativeValue],
        _execution: &ExecutionConfig,
    ) -> std::result::Result<ScalarInvocation, EngineError> {
        Err(EngineError::new_unavailable(function))
    }

    fn trap_message(&self, _function: FunctionId, _trap: TrapCode, _site: Option<u32>) -> String {
        "native tier is unavailable".to_string()
    }

    fn record_invocation_failure(&mut self, _function: FunctionId) {}
}

impl RuntimeTier for JitSession {
    fn observe_function_entry(&mut self, prototype: u32) -> EntryDecision {
        JitSession::observe_function_entry(self, prototype)
    }

    fn scalar_signature(&self, function: FunctionId) -> Option<ScalarSignature> {
        JitSession::scalar_signature(self, function)
    }

    fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionConfig,
    ) -> std::result::Result<ScalarInvocation, EngineError> {
        JitSession::invoke_scalar(self, function, arguments, execution)
    }

    fn trap_message(&self, function: FunctionId, trap: TrapCode, site: Option<u32>) -> String {
        self.trap_message_for(function, trap, site)
    }

    fn record_invocation_failure(&mut self, function: FunctionId) {
        JitSession::record_invocation_failure(self, function);
    }
}

pub struct Vm<'a, J: RuntimeTier> {
    pub(crate) chunk: &'a ValidatedChunk,
    pub(crate) globals: Vec<Value>,
    pub(crate) stack: Vec<Value>,
    pub(crate) frames: Vec<Frame>,
    pub(crate) arena: Arena,
    pub(crate) jit: J,
    pub(crate) exit_code: Option<i32>,
    pub(crate) inputs: ExecutionInputs,
    pub(crate) resources: ResourceTable,
    pub(crate) unique: unique::UniqueRuntime,
    config: ExecutionConfig,
    fuel_remaining: u64,
    output_bytes: usize,
    allocation_error: Option<Error>,
    logical_aggregate_constructions: u64,
    started: Instant,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) fn test_chunk() -> ValidatedChunk {
    let mut chunk = lkjscript_core::Chunk::new();
    chunk.main.emit(lkjscript_core::Op::Unit);
    chunk.main.emit(lkjscript_core::Op::Return);
    lkjscript_core::validate_chunk(chunk, &lkjscript_core::ValidationLimits::default())
        .expect("VM unit-test chunk validates")
}

fn outcome_from_error(error: Error) -> ExecutionOutcome {
    match error.class() {
        ErrorClass::Ordinary => ExecutionOutcome::Trapped(Trap::new(error.to_string())),
        ErrorClass::Deadline => ExecutionOutcome::DeadlineExceeded,
        ErrorClass::Resource(kind) => ExecutionOutcome::ResourceLimitExceeded(kind),
        ErrorClass::Host => ExecutionOutcome::HostFailure(HostError::new(error.to_string())),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::field_reassign_with_default)]
mod tests;
