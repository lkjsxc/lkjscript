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
pub(crate) mod unique;
mod unique_ops;

use std::time::{Duration, Instant};

use lkjscript_core::{
    CleanupFailures, CleanupPhase, CleanupSubject, Constant, EnumConstructionRef, EnumFieldRef,
    EnumId, EnumVariantRef, Error, ErrorClass, ExecutionConfig, ExecutionOutcome,
    FailureCleanupAction, GcConfig, GcHeap as Arena, HeapObj, HostError, Op, ProductFieldRef,
    ProductId, ResourceLimitKind, Result, Trap, ValidatedChunk, Value, VariantId,
    MAX_LIST_EQUAL_STEPS, MAX_PRODUCT_FIELDS,
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
    pub proto: u32,
    pub ip: usize,
    pub stack_base: usize,
    pub locals_base: usize,
    pub unique_places: Vec<unique::RuntimePlace>,
    pub borrowed_resources: Vec<Value>,
}

enum Stop {
    Returned(Value),
    Exited(i32),
}

pub trait RuntimeTier {
    #[cfg(feature = "jit")]
    fn observe_function_entry(&mut self, prototype: u32) -> EntryDecision;
    #[cfg(feature = "jit")]
    fn scalar_signature(&self, function: FunctionId) -> Option<ScalarSignature>;
    #[cfg(feature = "jit")]
    fn invoke_scalar(
        &mut self,
        function: FunctionId,
        arguments: &[NativeValue],
        execution: &ExecutionConfig,
    ) -> std::result::Result<ScalarInvocation, EngineError>;
    #[cfg(feature = "jit")]
    fn trap_message(&self, function: FunctionId, trap: TrapCode, site: Option<u32>) -> String;
}

#[derive(Debug, Default)]
pub struct NoTier;

impl RuntimeTier for NoTier {
    #[cfg(feature = "jit")]
    fn observe_function_entry(&mut self, _prototype: u32) -> EntryDecision {
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
        _execution: &ExecutionConfig,
    ) -> std::result::Result<ScalarInvocation, EngineError> {
        Err(EngineError::new_unavailable(function))
    }

    #[cfg(feature = "jit")]
    fn trap_message(&self, _function: FunctionId, _trap: TrapCode, _site: Option<u32>) -> String {
        "native tier is unavailable".to_string()
    }
}

#[cfg(feature = "jit")]
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
    pub(crate) structural: Option<structural_ops::StructuralInvocation>,
    structural_initialization_error: Option<Error>,
    config: ExecutionConfig,
    fuel_remaining: u64,
    output_bytes: usize,
    allocation_error: Option<Error>,
    cleanup_failures: CleanupFailures,
    logical_aggregate_constructions: u64,
    started: Instant,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) fn test_chunk() -> ValidatedChunk {
    let mut chunk = lkjscript_core::Chunk::new();
    chunk.main.emit(lkjscript_core::Op::Unit);
    chunk.main.emit(lkjscript_core::Op::Return);
    chunk.protos.push(lkjscript_core::FunctionProto {
        name: "test-function".into(),
        arity: 0,
        locals: 0,
        memory_plan: None,
        parameter_structurals: Vec::new(),
        parameter_structural_places: Vec::new(),
        return_structural: None,
        parameter_resources: Vec::new(),
        parameter_resource_places: Vec::new(),
        return_resource: None,
        parameter_uniques: Vec::new(),
        parameter_unique_places: Vec::new(),
        return_unique: None,
        unique_places: 0,
        failure_cleanups: Vec::new(),
        failure_cleanup_ranges: Vec::new(),
        code: vec![
            lkjscript_core::Op::Unit as u8,
            lkjscript_core::Op::Return as u8,
        ],
    });
    lkjscript_core::validate_chunk(chunk, &lkjscript_core::ValidationLimits::default())
        .expect("VM unit-test chunk validates")
}

fn is_structural_runtime_value(value: Value) -> bool {
    value.as_structural_root().is_some()
        || value.as_structural_view().is_some()
        || value.as_structural_destination().is_some()
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
