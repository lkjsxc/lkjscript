use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::{
    CallTarget, Constant, EnumId, FunctionId, Instruction, InstructionKind, ProductId,
    RuntimeLayoutId, RuntimeOp, StructuredOutcome, Terminator, ValueId, VariantId, VerifiedProgram,
};

pub use config::EvalConfig;
pub use resources::EvalResource;

include!("value.rs");

#[derive(Debug, Clone, PartialEq)]
pub enum EvalOutcome {
    Returned(EvalValue),
    Exited(i64),
    Trapped(String),
    UnsupportedOperation(RuntimeOp),
    DeadlineExceeded,
    ResourceLimitExceeded(String),
    HostFailure(String),
}

pub fn evaluate(program: &VerifiedProgram, config: &EvalConfig) -> EvalOutcome {
    let arguments = match capabilities::main_arguments(program, config) {
        Ok(arguments) => arguments,
        Err(message) => return EvalOutcome::HostFailure(message),
    };
    let Some(unique) = unique::EvalUniqueRuntime::new(config) else {
        return EvalOutcome::HostFailure("invalid evaluator unique-store limits".into());
    };
    let resources = match resources::EvalResources::new(config.max_resources) {
        Ok(resources) => resources,
        Err(message) => return EvalOutcome::HostFailure(message),
    };
    let mut evaluator = Evaluator {
        program,
        config,
        fuel: config.fuel,
        allocations: 0,
        logical_aggregate_constructions: 0,
        heap_bytes: 0,
        next_buffer_id: 1,
        resources,
        unique,
    };
    let primary = match evaluator.call(program.program().main, arguments, 0) {
        Ok(value @ EvalValue::ByteVector(_)) => match evaluator.unique.export_owner(value) {
            Ok(bytes) => EvalOutcome::Returned(EvalValue::ReturnedByteVector(bytes)),
            Err(flow) => flow.outcome(),
        },
        Ok(value) => match evaluator.unique.verify_empty() {
            Ok(()) => EvalOutcome::Returned(value),
            Err(flow) => {
                let _cleanup = evaluator.unique.cleanup();
                flow.outcome()
            }
        },
        Err(flow) => match evaluator.unique.cleanup() {
            Ok(()) => flow.outcome(),
            Err(cleanup) => cleanup.outcome(),
        },
    };
    resources::finish_evaluation(&mut evaluator.resources, primary)
}

pub(crate) struct Evaluator<'a> {
    pub(crate) program: &'a VerifiedProgram,
    pub(crate) config: &'a EvalConfig,
    pub(crate) fuel: u64,
    pub(crate) allocations: u64,
    pub(crate) logical_aggregate_constructions: u64,
    pub(crate) heap_bytes: usize,
    pub(crate) next_buffer_id: u64,
    pub(crate) resources: resources::EvalResources,
    pub(crate) unique: unique::EvalUniqueRuntime,
}

#[derive(Debug)]
pub(crate) enum Flow {
    Exit(i64),
    Trap(String),
    Unsupported(RuntimeOp),
    Deadline,
    Resource(String),
    HostFailure(String),
}

impl Flow {
    fn outcome(self) -> EvalOutcome {
        match self {
            Self::Exit(code) => EvalOutcome::Exited(code),
            Self::Trap(message) => EvalOutcome::Trapped(message),
            Self::Unsupported(operation) => EvalOutcome::UnsupportedOperation(operation),
            Self::Deadline => EvalOutcome::DeadlineExceeded,
            Self::Resource(kind) => EvalOutcome::ResourceLimitExceeded(kind),
            Self::HostFailure(message) => EvalOutcome::HostFailure(message),
        }
    }
}

mod allocation;
#[cfg(test)]
mod boundary_tests;
mod capabilities;
mod config;
mod control;
mod instruction;
mod numeric_conversion;
mod resources;
mod runtime;
mod unique;
mod values;

pub(crate) use values::*;
