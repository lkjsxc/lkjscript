use std::collections::{BTreeMap, BTreeSet};

use crate::{CpuSet, HardwareTopology, ResourceError, ResourceResult, WorkerGroupId, WorkerId};

mod planner;
pub use planner::ResourcePlanner;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlacementMode {
    KernelManaged,
    CpuPinned,
    LlcDomainMasked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlanCaps {
    pub task_cap: usize,
    pub request_cap: usize,
    pub profile_cap: usize,
    pub queue_bytes_per_worker: u64,
    pub scratch_bytes_per_worker: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerPlacement {
    pub worker: WorkerId,
    pub group: WorkerGroupId,
    pub exact_mask: CpuSet,
    pub elastic_locality: CpuSet,
    pub queue_bytes: u64,
    pub scratch_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerGroup {
    pub id: WorkerGroupId,
    pub mask: CpuSet,
    pub workers: Vec<WorkerId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResourcePlan {
    pub mode: PlacementMode,
    pub workers: Vec<WorkerPlacement>,
    pub groups: Vec<WorkerGroup>,
    pub sequential_fallback: bool,
}

impl ExecutionResourcePlan {
    pub fn validate(&self, topology: &HardwareTopology, caps: PlanCaps) -> ResourceResult<()> {
        if self.workers.is_empty()
            || self.workers.len() > caps.task_cap.min(caps.request_cap).min(caps.profile_cap)
        {
            return Err(ResourceError::new(
                "plan-workers",
                "worker count violates caps",
            ));
        }
        let groups: BTreeMap<WorkerGroupId, &WorkerGroup> =
            self.groups.iter().map(|group| (group.id, group)).collect();
        if groups.len() != self.groups.len() {
            return Err(ResourceError::new("plan-group", "duplicate group"));
        }
        let mut worker_ids = BTreeSet::new();
        let mut pinned = BTreeSet::new();
        for worker in &self.workers {
            if !worker_ids.insert(worker.worker)
                || !groups.contains_key(&worker.group)
                || worker.queue_bytes > caps.queue_bytes_per_worker
                || worker.scratch_bytes > caps.scratch_bytes_per_worker
                || worker
                    .exact_mask
                    .as_slice()
                    .iter()
                    .any(|cpu| !topology.allowed.contains(*cpu))
                || worker
                    .elastic_locality
                    .as_slice()
                    .iter()
                    .any(|cpu| !topology.allowed.contains(*cpu))
            {
                return Err(ResourceError::new(
                    "plan-placement",
                    "invalid worker placement",
                ));
            }
            if self.mode == PlacementMode::CpuPinned {
                for cpu in worker.exact_mask.as_slice() {
                    if !pinned.insert(*cpu) {
                        return Err(ResourceError::new("plan-mask", "CPU pinned twice"));
                    }
                }
            }
        }
        for group in &self.groups {
            if group
                .workers
                .iter()
                .any(|worker| !worker_ids.contains(worker))
                || group
                    .mask
                    .as_slice()
                    .iter()
                    .any(|cpu| !topology.allowed.contains(*cpu))
            {
                return Err(ResourceError::new("plan-group", "invalid group membership"));
            }
        }
        Ok(())
    }
}
