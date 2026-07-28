use std::collections::BTreeMap;

use lkjscript_contracts::ContractDigest;

use crate::{TaskId, VerifiedTaskGraph, WorkerId};

mod reference_execution;
mod replay;
mod state;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskState {
    Created,
    Waiting,
    Ready,
    Running,
    Completed,
    Failed,
    Cancelled,
    Retired,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SchedulePolicy {
    Sequential,
    StaticPartition,
    GlobalFifo,
    #[default]
    LocalWorkStealing,
    HierarchicalLocality,
    OwnerCompute,
}

impl SchedulePolicy {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sequential => "sequential",
            Self::StaticPartition => "static-partition",
            Self::GlobalFifo => "global-fifo",
            Self::LocalWorkStealing => "local-work-stealing",
            Self::HierarchicalLocality => "hierarchical-locality",
            Self::OwnerCompute => "owner-compute",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceEvent {
    pub task: TaskId,
    pub from: TaskState,
    pub to: TaskState,
    pub worker: Option<WorkerId>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchedulingTrace {
    pub graph: ContractDigest,
    pub events: Vec<TraceEvent>,
    pub truncated: bool,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceReport {
    pub states: BTreeMap<TaskId, TaskState>,
    pub failure: Option<(TaskId, String)>,
    pub trace: SchedulingTrace,
}

pub trait SchedulingPolicy {
    fn rank(&self, graph: &VerifiedTaskGraph, task: TaskId, worker_count: usize) -> (u64, TaskId);
}
macro_rules! policy {
    ($name:ident, $body:expr) => {
        #[derive(Clone, Copy, Debug, Default)]
        pub struct $name;
        impl SchedulingPolicy for $name {
            fn rank(
                &self,
                graph: &VerifiedTaskGraph,
                task: TaskId,
                workers: usize,
            ) -> (u64, TaskId) {
                ($body)(graph, task, workers)
            }
        }
    };
}
policy!(Sequential, |_g: &VerifiedTaskGraph,
                     task: TaskId,
                     _w: usize| (
    task.slot as u64,
    task
));
policy!(StaticPartition, |_g: &VerifiedTaskGraph,
                          task: TaskId,
                          w: usize| (
    (task.slot as usize % w) as u64,
    task
));
policy!(GlobalFifo, |_g: &VerifiedTaskGraph,
                     task: TaskId,
                     _w: usize| (
    task.slot as u64,
    task
));
policy!(LocalWorkStealing, |_g: &VerifiedTaskGraph,
                            task: TaskId,
                            w: usize| (
    ((task.slot as usize + 1) % w) as u64,
    task
));
policy!(
    HierarchicalLocality,
    |g: &VerifiedTaskGraph, task: TaskId, _w: usize| {
        let scope = g
            .tasks()
            .iter()
            .find(|node| node.id == task)
            .map_or(0, |node| node.scope.slot);
        (scope as u64, task)
    }
);
policy!(OwnerCompute, |g: &VerifiedTaskGraph,
                       task: TaskId,
                       _w: usize| {
    let owner = g
        .tasks()
        .iter()
        .find(|node| node.id == task)
        .map_or(0, |node| node.result_owner.slot);
    (owner as u64, task)
});

impl SchedulingPolicy for SchedulePolicy {
    fn rank(&self, graph: &VerifiedTaskGraph, task: TaskId, workers: usize) -> (u64, TaskId) {
        match self {
            Self::Sequential => Sequential.rank(graph, task, workers),
            Self::StaticPartition => StaticPartition.rank(graph, task, workers),
            Self::GlobalFifo => GlobalFifo.rank(graph, task, workers),
            Self::LocalWorkStealing => LocalWorkStealing.rank(graph, task, workers),
            Self::HierarchicalLocality => HierarchicalLocality.rank(graph, task, workers),
            Self::OwnerCompute => OwnerCompute.rank(graph, task, workers),
        }
    }
}

pub struct ReferenceScheduler;
