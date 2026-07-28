use crate::{CpuSet, SchedulePolicy, TaskId, WorkerGroupId, WorkerId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerDescriptor {
    pub id: WorkerId,
    pub allowed: CpuSet,
    pub group: WorkerGroupId,
    pub numa_node: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeConfig {
    pub workers: Vec<WorkerDescriptor>,
    pub queue_capacity: usize,
    pub spin_limit: usize,
    pub policy: SchedulePolicy,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RuntimeMetrics {
    pub executed: u64,
    pub steals: u64,
    pub spins: u64,
    pub parks: u64,
    pub same_group_steals: u64,
    pub cross_group_steals: u64,
    pub cross_numa_steals: u64,
    pub queue_high_water: usize,
    pub active_workers: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeReport<O, E> {
    pub outputs: Vec<(TaskId, O)>,
    pub failures: Vec<(TaskId, E)>,
    pub selected_failure: Option<(TaskId, E)>,
    pub cancelled: Vec<TaskId>,
    pub metrics: RuntimeMetrics,
}
