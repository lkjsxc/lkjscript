use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::{
    CallTarget, Constant, EnumId, FunctionId, Instruction, InstructionKind, ProductId,
    RuntimeLayoutId, RuntimeOp, StructuredOutcome, Terminator, ValueId, VariantId, VerifiedProgram,
};

pub use config::{EvalConfig, EvalResourcePolicy};
pub use resources::EvalResource;

include!("value.rs");

pub use outcome::EvalOutcome;
pub(crate) use outcome::Flow;

pub fn evaluate(program: &VerifiedProgram, config: &EvalConfig) -> EvalOutcome {
    let arguments = match capabilities::main_arguments(program, config) {
        Ok(arguments) => arguments,
        Err(message) => return EvalOutcome::HostFailure(message),
    };
    let Some(unique) = unique::EvalUniqueRuntime::new(config) else {
        return EvalOutcome::HostFailure("invalid evaluator unique-store limits".into());
    };
    let resources = match resources::EvalResources::new(
        config.max_resources,
        config.resource_policy,
        config.cleanup_failure_limits,
    ) {
        Ok(resources) => resources,
        Err(message) => return EvalOutcome::HostFailure(message),
    };
    let mut evaluator = Evaluator {
        static_bytes: collect_static_bytes(program),
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
    let (primary, unique_cleanup_error) = match evaluator.call(program.program().main, arguments, 0)
    {
        Ok(value @ EvalValue::ByteVector(_)) => match evaluator.unique.export_owner(value) {
            Ok(bytes) => (
                EvalOutcome::Returned(EvalValue::ReturnedByteVector(bytes)),
                None,
            ),
            Err(flow) => (
                flow.outcome(),
                evaluator.unique.cleanup().err().map(Flow::detail),
            ),
        },
        Ok(value @ EvalValue::Bytes(_)) => match evaluator.unique.export_owner(value) {
            Ok(bytes) => (EvalOutcome::Returned(EvalValue::ReturnedBytes(bytes)), None),
            Err(flow) => (
                flow.outcome(),
                evaluator.unique.cleanup().err().map(Flow::detail),
            ),
        },
        Ok(EvalValue::StaticBytes(index)) => (
            evaluator
                .static_bytes
                .get(index as usize)
                .map(|bytes| EvalOutcome::Returned(EvalValue::ReturnedBytes(bytes.to_vec())))
                .unwrap_or_else(|| EvalOutcome::Trapped("invalid returned static bytes".into())),
            None,
        ),
        Ok(value) => match evaluator.unique.verify_empty() {
            Ok(()) => (EvalOutcome::Returned(value), None),
            Err(flow) => (
                flow.outcome(),
                evaluator.unique.cleanup().err().map(Flow::detail),
            ),
        },
        Err(flow) => (
            flow.outcome(),
            evaluator.unique.cleanup().err().map(Flow::detail),
        ),
    };
    let mut cleanup_failures = lkjscript_core::CleanupFailures::new(config.cleanup_failure_limits);
    if let Some(error) = unique_cleanup_error {
        cleanup_failures.push(
            lkjscript_core::CleanupPhase::Emergency,
            lkjscript_core::CleanupSubject::UniqueStorage,
            error,
        );
    }
    resources::finish_evaluation(&mut evaluator.resources, primary, cleanup_failures)
}

pub(crate) struct Evaluator<'a> {
    pub(crate) program: &'a VerifiedProgram,
    pub(crate) static_bytes: Vec<Box<[u8]>>,
    pub(crate) config: &'a EvalConfig,
    pub(crate) fuel: u64,
    pub(crate) allocations: u64,
    pub(crate) logical_aggregate_constructions: u64,
    pub(crate) heap_bytes: usize,
    pub(crate) next_buffer_id: u64,
    pub(crate) resources: resources::EvalResources,
    pub(crate) unique: unique::EvalUniqueRuntime,
}

mod allocation;
#[cfg(test)]
mod boundary_tests;
mod capabilities;
mod config;
mod control;
mod instruction;
mod numeric_conversion;
mod outcome;
mod resources;
mod runtime;
mod unique;
mod values;

pub(crate) use values::*;

fn collect_static_bytes(program: &VerifiedProgram) -> Vec<Box<[u8]>> {
    let mut output: Vec<Box<[u8]>> = Vec::new();
    for constant in program
        .program()
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.kind {
            InstructionKind::Constant(Constant::StaticBytes(bytes)) => Some(bytes),
            _ => None,
        })
    {
        if !output.iter().any(|known| known.as_ref() == constant) {
            output.push(constant.clone().into_boxed_slice());
        }
    }
    output
}
