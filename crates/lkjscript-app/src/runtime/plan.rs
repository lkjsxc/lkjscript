use lkjscript_resource::{ExecutionResourcePlan, PlanCaps, ResourcePlanner};
use lkjscript_sys::LinuxHostSnapshot;

use super::args::PlanOptions;

pub(super) fn build(
    snapshot: &LinuxHostSnapshot,
    options: &PlanOptions,
) -> Result<ExecutionResourcePlan, String> {
    let tasks = options
        .tasks
        .unwrap_or(snapshot.topology.allowed.len())
        .max(1);
    let requested = if options.policy == "sequential" {
        1
    } else {
        options
            .parallelism
            .unwrap_or(snapshot.topology.allowed.len())
            .max(1)
    };
    let cap = snapshot.topology.allowed.len().clamp(1, 64);
    ResourcePlanner::plan(
        &snapshot.topology,
        options.affinity,
        PlanCaps {
            task_cap: tasks,
            request_cap: requested,
            profile_cap: cap,
            queue_bytes_per_worker: 64 * 1024,
            scratch_bytes_per_worker: 1024 * 1024,
        },
        requested,
    )
    .map_err(|error| error.to_string())
}

pub(super) fn print_text(plan: &ExecutionResourcePlan, policy: &str) {
    println!("policy: {policy}");
    println!("affinity: {:?}", plan.mode);
    println!("workers: {}", plan.workers.len());
    println!("worker-groups: {}", plan.groups.len());
    println!("sequential-fallback: {}", plan.sequential_fallback);
    for worker in &plan.workers {
        println!(
            "worker:{} group:{} mask:{:?} locality:{:?}",
            worker.worker.slot,
            worker.group.slot,
            worker.exact_mask.as_slice(),
            worker.elastic_locality.as_slice(),
        );
    }
}
