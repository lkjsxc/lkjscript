use lkjscript_contracts::ContractDigest;
use lkjscript_resource::{
    ExecutionResourcePlan, NoopWorkerBinder, RuntimeConfig, RuntimeMetrics, ScopedRuntime,
    WorkerBinder, WorkerDescriptor,
};

use super::api::verify_optimization_with_budget;
use crate::optimize::*;
use crate::VerifiedProgram;

mod executor;
mod graph;
mod merge;

use executor::DiscoveryExecutor;

pub struct ScheduledOptimizationReport {
    optimized: VerifiedOptimizedProgram,
    pub task_graph: ContractDigest,
    pub runtime: RuntimeMetrics,
    pub task_count: usize,
    pub worker_count: usize,
}

impl ScheduledOptimizationReport {
    pub fn optimized(&self) -> &VerifiedOptimizedProgram {
        &self.optimized
    }

    pub fn into_optimized(self) -> VerifiedOptimizedProgram {
        self.optimized
    }
}

pub fn optimize_scheduled(
    input: &VerifiedProgram,
    limits: OptimizationLimits,
    plan: &ExecutionResourcePlan,
) -> Result<ScheduledOptimizationReport, OptimizationError> {
    optimize_scheduled_with_binder(input, limits, plan, &NoopWorkerBinder)
}

pub fn optimize_scheduled_with_binder<B: WorkerBinder>(
    input: &VerifiedProgram,
    limits: OptimizationLimits,
    plan: &ExecutionResourcePlan,
    binder: &B,
) -> Result<ScheduledOptimizationReport, OptimizationError> {
    let mut budget = Budget::new(limits);
    let input_shape = preflight_program(input.program(), &mut budget)?;
    budget.set_input_instructions(input_shape.instructions);
    budget.discovery_passes = budget.discovery_passes.saturating_add(1);

    let ranges = graph::ranges(input.program(), plan.workers.len());
    let task_graph = graph::task_graph(input.program(), &ranges).map_err(resource_error)?;
    let executor = DiscoveryExecutor {
        program: input.program(),
        ranges: &ranges,
        limits,
    };
    let config = RuntimeConfig {
        workers: plan
            .workers
            .iter()
            .map(|worker| WorkerDescriptor {
                id: worker.worker,
                allowed: worker.exact_mask.clone(),
            })
            .collect(),
        queue_capacity: ranges.len().max(1),
        spin_limit: 64,
    };
    let report =
        ScopedRuntime::run(&task_graph, config, &executor, binder).map_err(resource_error)?;
    if let Some((_, failure)) = report.selected_failure {
        return Err(failure);
    }
    let runtime = report.metrics;
    let (discovery, certificate) =
        merge::merge_discovery(report.outputs, input.program().functions.len(), &mut budget)?;
    let candidate = discovery_reconstruct(input.program(), &certificate, &discovery, &mut budget)?;
    let candidate_shape = preflight_program(&candidate, &mut budget)?;
    budget.check_growth(candidate_shape.instructions)?;
    let optimized = verify_optimization_with_budget(
        input,
        candidate,
        candidate_shape,
        certificate,
        input_shape,
        &mut budget,
    )?;
    Ok(ScheduledOptimizationReport {
        optimized,
        task_graph: task_graph.id(),
        runtime,
        task_count: ranges.len(),
        worker_count: plan.workers.len(),
    })
}

fn resource_error(error: lkjscript_resource::ResourceError) -> OptimizationError {
    OptimizationError::new(
        OptimizationFailureCode::InputVerification,
        format!("semantic resource plane {}: {}", error.code, error.detail),
    )
}
