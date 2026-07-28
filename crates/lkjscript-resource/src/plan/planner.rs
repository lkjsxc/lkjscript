use std::collections::BTreeSet;

use super::{ExecutionResourcePlan, PlacementMode, PlanCaps, WorkerGroup, WorkerPlacement};
use crate::{
    CpuSet, FactCertainty, HardwareTopology, ResourceError, ResourceResult, WorkerGroupId, WorkerId,
};

pub struct ResourcePlanner;

impl ResourcePlanner {
    pub fn plan(
        topology: &HardwareTopology,
        requested_mode: PlacementMode,
        caps: PlanCaps,
        requested_workers: usize,
    ) -> ResourceResult<ExecutionResourcePlan> {
        topology.validate()?;
        let cap = caps
            .task_cap
            .min(caps.request_cap)
            .min(caps.profile_cap)
            .min(requested_workers.max(1));
        let unknown = topology.certainty == FactCertainty::Unknown;
        let sequential = unknown || topology.allowed.len() == 1 || cap == 1;
        let count = if sequential {
            1
        } else {
            cap.min(topology.allowed.len())
        };
        let selected = select_physical_first(topology, count)?;
        let mode = if sequential || unknown {
            PlacementMode::KernelManaged
        } else {
            requested_mode
        };
        let llc = llc_masks(topology);
        let mut groups_by_mask: Vec<CpuSet> = Vec::new();
        let mut workers = Vec::new();
        for (index, cpu) in selected.iter().enumerate() {
            let locality = match llc.iter().find(|set| set.contains(*cpu)) {
                Some(set) => CpuSet::new(
                    set.as_slice()
                        .iter()
                        .copied()
                        .filter(|item| topology.allowed.contains(*item)),
                )?,
                None => CpuSet::new([*cpu])?,
            };
            let group_index = match groups_by_mask.iter().position(|mask| mask == &locality) {
                Some(found) => found,
                None => {
                    groups_by_mask.push(locality.clone());
                    groups_by_mask.len() - 1
                }
            };
            workers.push(WorkerPlacement {
                worker: WorkerId::new(index as u32, 1),
                group: WorkerGroupId::new(group_index as u32, 1),
                exact_mask: match mode {
                    PlacementMode::KernelManaged => topology.allowed.clone(),
                    PlacementMode::CpuPinned => CpuSet::new([*cpu])?,
                    PlacementMode::LlcDomainMasked => locality.clone(),
                },
                elastic_locality: locality,
                queue_bytes: caps.queue_bytes_per_worker,
                scratch_bytes: caps.scratch_bytes_per_worker,
            });
        }
        let groups = groups_by_mask
            .into_iter()
            .enumerate()
            .map(|(index, mask)| WorkerGroup {
                id: WorkerGroupId::new(index as u32, 1),
                mask,
                workers: workers
                    .iter()
                    .filter(|worker| worker.group.slot == index as u32)
                    .map(|worker| worker.worker)
                    .collect(),
            })
            .collect();
        let plan = ExecutionResourcePlan {
            mode,
            workers,
            groups,
            sequential_fallback: sequential,
        };
        plan.validate(topology, caps)?;
        Ok(plan)
    }
}

fn select_physical_first(topology: &HardwareTopology, count: usize) -> ResourceResult<Vec<u32>> {
    let mut units: Vec<_> = topology
        .units
        .iter()
        .filter(|unit| topology.allowed.contains(unit.cpu))
        .collect();
    units.sort_by_key(|unit| {
        (
            llc_index(topology, unit.cpu),
            unit.package,
            unit.die,
            unit.core,
            unit.cpu,
        )
    });
    let mut cores = BTreeSet::new();
    let mut first = Vec::new();
    let mut siblings = Vec::new();
    for unit in units {
        if cores.insert((unit.package, unit.die, unit.core)) {
            first.push(unit.cpu);
        } else {
            siblings.push(unit.cpu);
        }
    }
    first.extend(siblings);
    first.truncate(count);
    if first.is_empty() {
        return Err(ResourceError::new("plan-empty", "no selectable CPUs"));
    }
    Ok(first)
}

fn llc_index(topology: &HardwareTopology, cpu: u32) -> usize {
    let max = topology.caches.iter().map(|cache| cache.level).max();
    topology
        .caches
        .iter()
        .position(|cache| Some(cache.level) == max && cache.cpus.contains(cpu))
        .unwrap_or(usize::MAX)
}
fn llc_masks(topology: &HardwareTopology) -> Vec<CpuSet> {
    let max = topology.caches.iter().map(|cache| cache.level).max();
    topology
        .caches
        .iter()
        .filter(|cache| Some(cache.level) == max)
        .map(|cache| cache.cpus.clone())
        .collect()
}
