use crate::{
    CallTarget, Constant, EnumId, FunctionId, Instruction, InstructionKind, ProductId,
    RuntimeLayoutId, RuntimeOp, StructuredOutcome, Terminator, ValueId, VariantId, VerifiedProgram,
};

pub use config::{EvalConfig, EvalResourcePolicy};
pub use resources::EvalResource;

include!("value.rs");

pub use outcome::EvalOutcome;
pub(crate) use outcome::Flow;
pub use values::EvalStructuralObservation;

mod failure_cleanup;

pub fn evaluate(program: &VerifiedProgram, config: &EvalConfig) -> EvalOutcome {
    evaluate_observed(program, config).0
}

pub fn evaluate_observed(
    program: &VerifiedProgram,
    config: &EvalConfig,
) -> (EvalOutcome, EvalStructuralObservation) {
    let arguments = match capabilities::main_arguments(program, config) {
        Ok(arguments) => arguments,
        Err(message) => return (EvalOutcome::HostFailure(message), Default::default()),
    };
    let Some(unique) = unique::EvalUniqueRuntime::new(config) else {
        return (
            EvalOutcome::HostFailure("invalid evaluator unique-store limits".into()),
            Default::default(),
        );
    };
    let lists = match lkjscript_core::SegmentedListArena::new(
        lkjscript_core::SegmentedListArenaLimits::for_allocation_policy(config.max_allocations),
    ) {
        Ok(lists) => lists,
        Err(error) => {
            return (
                EvalOutcome::HostFailure(format!("segmented-list arena: {error:?}")),
                Default::default(),
            );
        }
    };
    let region_limit = u32::try_from(config.max_allocations)
        .unwrap_or(u32::MAX)
        .max(1);
    let region_products =
        match lkjscript_core::RegionProductArena::new(lkjscript_core::RegionProductLimits {
            max_records: std::num::NonZeroU32::new(region_limit)
                .unwrap_or(std::num::NonZeroU32::MIN),
            max_fields: std::num::NonZeroU32::new(region_limit.saturating_mul(16))
                .unwrap_or(std::num::NonZeroU32::MIN),
        }) {
            Ok(arena) => arena,
            Err(error) => {
                return (
                    EvalOutcome::HostFailure(format!("region-product arena: {error:?}")),
                    Default::default(),
                );
            }
        };
    let structural =
        match EvaluatorStructuralSession::new(program.program(), config.structural_limits) {
            Ok(structural) => structural,
            Err(message) => return (EvalOutcome::HostFailure(message), Default::default()),
        };
    let resources = match resources::EvalResources::new(
        config.max_resources,
        config.resource_policy,
        config.cleanup_failure_limits,
    ) {
        Ok(resources) => resources,
        Err(message) => return (EvalOutcome::HostFailure(message), Default::default()),
    };
    let mut evaluator = Evaluator {
        static_bytes: collect_static_bytes(program),
        program,
        config,
        fuel: config.fuel,
        allocations: 0,
        logical_aggregate_constructions: 0,
        heap_bytes: 0,
        cleanup_failures: lkjscript_core::CleanupFailures::new(config.cleanup_failure_limits),
        resources,
        unique,
        structural,
        lists,
        list_owned: Vec::new(),
        region_products,
    };
    let result = evaluator.call(program.program().main, arguments, Vec::new(), 0);
    let primary = match result {
        Ok(value) => evaluator.adapt_return(value),
        Err(flow) => Err(flow),
    }
    .unwrap_or_else(Flow::outcome);
    let list_owned = std::mem::take(&mut evaluator.list_owned);
    for value in list_owned.into_iter().rev() {
        if let Err(cleanup) = evaluator.cleanup_eval_value(value) {
            evaluator.note_structural_cleanup_failure(cleanup.detail());
        }
    }
    if evaluator.unique.verify_empty().is_err() {
        if let Err(cleanup) = evaluator.unique.cleanup() {
            evaluator.note_structural_cleanup_failure(cleanup.detail());
        }
    }
    let final_empty = evaluator.structural.runtime.verify_empty().is_ok();
    if !final_empty {
        evaluator.note_structural_cleanup_failure(
            "structural runtime retained roots, views, or destinations".into(),
        );
    }
    let observation = EvalStructuralObservation::capture(&evaluator.structural, final_empty);
    let cleanup_failures = std::mem::replace(
        &mut evaluator.cleanup_failures,
        lkjscript_core::CleanupFailures::new(config.cleanup_failure_limits),
    );
    let outcome = resources::finish_evaluation(&mut evaluator.resources, primary, cleanup_failures);
    (outcome, observation)
}

pub(crate) struct Evaluator<'a> {
    pub(crate) program: &'a VerifiedProgram,
    pub(crate) static_bytes: Vec<Box<[u8]>>,
    pub(crate) config: &'a EvalConfig,
    pub(crate) fuel: u64,
    pub(crate) allocations: u64,
    pub(crate) logical_aggregate_constructions: u64,
    pub(crate) heap_bytes: usize,
    pub(crate) cleanup_failures: lkjscript_core::CleanupFailures,
    pub(crate) resources: resources::EvalResources,
    pub(crate) unique: unique::EvalUniqueRuntime,
    pub(crate) structural: EvaluatorStructuralSession,
    pub(crate) lists: lkjscript_core::SegmentedListArena<EvalValue>,
    pub(crate) list_owned: Vec<EvalValue>,
    pub(crate) region_products: lkjscript_core::RegionProductArena<EvalValue>,
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
