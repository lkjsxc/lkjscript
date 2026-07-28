use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::{
    CallTarget, Constant, EnumId, FunctionId, Instruction, InstructionKind, ProductId,
    RuntimeLayoutId, RuntimeOp, StructuredOutcome, Terminator, ValueId, VariantId, VerifiedProgram,
};

pub use config::EvalConfig;
pub use resources::EvalResource;

#[derive(Clone)]
pub struct EvalBuffer {
    id: u64,
    bytes: Rc<RefCell<Vec<u8>>>,
}

impl fmt::Debug for EvalBuffer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let length = self.bytes.try_borrow().map_or(0, |bytes| bytes.len());
        formatter
            .debug_struct("EvalBuffer")
            .field("id", &self.id)
            .field("length", &length)
            .finish()
    }
}

impl PartialEq for EvalBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Clone)]
pub enum EvalValue {
    Unit,
    Bool(bool),
    I64(i64),
    F64(f64),
    Str(String),
    Symbol(String),
    Buf(EvalBuffer),
    Path(Vec<u8>),
    Capability(lkjscript_contracts::CapabilityKind),
    Resource(EvalResource),
    Product(ProductId, Vec<Self>),
    Enum {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        physical_tag: u16,
        payload: Vec<Self>,
    },
    List(Vec<Self>),
    Function(FunctionId),
}

impl PartialEq for EvalValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unit, Self::Unit) => true,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::I64(left), Self::I64(right)) => left == right,
            (Self::F64(left), Self::F64(right)) => left.to_bits() == right.to_bits(),
            (Self::Str(left), Self::Str(right)) | (Self::Symbol(left), Self::Symbol(right)) => {
                left == right
            }
            (Self::Buf(left), Self::Buf(right)) => left == right,
            (Self::Path(left), Self::Path(right)) => left == right,
            (Self::Capability(left), Self::Capability(right)) => left == right,
            (Self::Resource(_), Self::Resource(_)) => false,
            (Self::Product(left_id, left), Self::Product(right_id, right)) => {
                left_id == right_id && left == right
            }
            (
                Self::Enum {
                    enum_id: le,
                    variant: lv,
                    layout: ll,
                    physical_tag: lt,
                    payload: lp,
                },
                Self::Enum {
                    enum_id: re,
                    variant: rv,
                    layout: rl,
                    physical_tag: rt,
                    payload: rp,
                },
            ) => le == re && lv == rv && ll == rl && lt == rt && lp == rp,
            (Self::List(left), Self::List(right)) => left == right,
            (Self::Function(left), Self::Function(right)) => left == right,
            _ => false,
        }
    }
}

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
    };
    let primary = match evaluator.call(program.program().main, arguments, 0) {
        Ok(value) => EvalOutcome::Returned(value),
        Err(flow) => flow.outcome(),
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
mod values;

pub(crate) use values::*;
