use std::error::Error;
use std::time::Instant;

use lkjscript_resource::*;
use lkjscript_sys::{discover_linux_host, LinuxWorkerBinder};

#[path = "benchmark/graph.rs"]
mod graph;
#[path = "benchmark/stats.rs"]
mod stats;
#[path = "benchmark/workload.rs"]
mod workload;
use stats::Samples;
use workload::{Workload, WorkloadExecutor, TASKS};

const POLICIES: [SchedulePolicy; 6] = [
    SchedulePolicy::Sequential,
    SchedulePolicy::StaticPartition,
    SchedulePolicy::GlobalFifo,
    SchedulePolicy::LocalWorkStealing,
    SchedulePolicy::HierarchicalLocality,
    SchedulePolicy::OwnerCompute,
];

fn main() -> Result<(), Box<dyn Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let samples = argument_usize(&arguments, 0, 12)?;
    let requested_workers = argument_usize(&arguments, 1, 4)?;
    let mode = parse_mode(
        arguments
            .get(2)
            .map(String::as_str)
            .unwrap_or("kernel-managed"),
    )?;
    let host = discover_linux_host()?;
    println!(concat!(
        "workload\tpolicy\taffinity\tworkers\tsamples\t",
        "p50-ns\tp95-ns\tp99-ns\ttasks-per-second\tsteals\t",
        "same-group-steals\tcross-group-steals\tcross-numa-steals\tparks\tchecksum"
    ));
    for workload in Workload::ALL {
        let graph = graph::graph(workload)?;
        for policy in POLICIES {
            let worker_request = if policy == SchedulePolicy::Sequential {
                1
            } else {
                requested_workers
            };
            let plan = ResourcePlanner::plan(
                &host.topology,
                mode,
                PlanCaps {
                    task_cap: TASKS,
                    request_cap: requested_workers,
                    profile_cap: requested_workers,
                    queue_bytes_per_worker: u64::try_from(TASKS * std::mem::size_of::<TaskId>())?,
                    scratch_bytes_per_worker: 0,
                },
                worker_request,
            )?;
            let config = RuntimeConfig {
                workers: plan
                    .workers
                    .iter()
                    .map(|worker| WorkerDescriptor {
                        id: worker.worker,
                        allowed: worker.exact_mask.clone(),
                        group: worker.group,
                        numa_node: host
                            .topology
                            .numa_nodes
                            .iter()
                            .find(|node| {
                                worker
                                    .exact_mask
                                    .as_slice()
                                    .iter()
                                    .any(|cpu| node.cpus.contains(*cpu))
                            })
                            .map(|node| node.id),
                    })
                    .collect(),
                queue_capacity: TASKS,
                spin_limit: 256,
                policy,
            };
            let binder = LinuxWorkerBinder::for_mode(mode);
            let mut measurements = Samples::default();
            for sample in 0..samples + 2 {
                let executor = WorkloadExecutor::new(workload);
                let start = Instant::now();
                let report = ScopedRuntime::run(&graph, config.clone(), &executor, &binder)?;
                let elapsed = start.elapsed().as_nanos();
                if !report.failures.is_empty() || report.outputs.len() != TASKS {
                    return Err(format!("incomplete {} {}", workload.name(), policy.name()).into());
                }
                if sample >= 2 {
                    let checksum = report
                        .outputs
                        .iter()
                        .fold(0_u64, |sum, (_, value)| sum.wrapping_add(*value));
                    measurements.push(
                        elapsed,
                        [
                            report.metrics.steals,
                            report.metrics.same_group_steals,
                            report.metrics.cross_group_steals,
                            report.metrics.cross_numa_steals,
                        ],
                        report.metrics.parks,
                        checksum,
                    );
                }
            }
            measurements.print(
                workload.name(),
                policy.name(),
                mode_name(mode),
                config.workers.len(),
            );
        }
    }
    Ok(())
}

fn argument_usize(
    arguments: &[String],
    index: usize,
    default: usize,
) -> Result<usize, Box<dyn Error>> {
    match arguments.get(index) {
        Some(value) => Ok(value.parse()?),
        None => Ok(default),
    }
}

fn parse_mode(value: &str) -> Result<PlacementMode, Box<dyn Error>> {
    match value {
        "kernel-managed" => Ok(PlacementMode::KernelManaged),
        "cpu-pinned" => Ok(PlacementMode::CpuPinned),
        "llc-domain-masked" => Ok(PlacementMode::LlcDomainMasked),
        _ => Err(format!("unknown affinity mode {value}").into()),
    }
}

const fn mode_name(mode: PlacementMode) -> &'static str {
    match mode {
        PlacementMode::KernelManaged => "kernel-managed",
        PlacementMode::CpuPinned => "cpu-pinned",
        PlacementMode::LlcDomainMasked => "llc-domain-masked",
    }
}
