#![forbid(unsafe_code)]
//! Safe, bounded resource topology, graph, planning, scheduling, and scoped execution.

mod cpu;
mod error;
mod graph;
mod ids;
mod journal;
mod owner;
mod plan;
mod runtime;
mod runtime_support;
mod scheduler;
mod topology;
mod verify;

pub use cpu::{CpuSet, MAX_CPU, MAX_CPUS, MAX_RANGES};
pub use error::{ResourceError, ResourceResult};
pub use graph::{
    AccessMode, AccessRecord, CheckedRange, GraphLimits, TaskGraphBuilder, TaskNode, TaskScope,
    UnverifiedTaskGraph, VerifiedTaskGraph,
};
pub use ids::{
    AccessRecordId, DataOwnerId, ExecutionDomainId, GenerationId, GenerationTable, ResourcePlaneId,
    TaskClassId, TaskId, TaskResultId, TaskScopeId, WorkerGroupId, WorkerId,
};
pub use journal::{ResourceJournal, ResourceJournalEntry};
pub use owner::{NoLiveLoanProof, OwnerHomeTable, OwnerMetrics, RemoteRelease};
pub use plan::{
    ExecutionResourcePlan, PlacementMode, PlanCaps, ResourcePlanner, WorkerGroup, WorkerPlacement,
};
pub use runtime::{
    NoopWorkerBinder, RuntimeConfig, RuntimeMetrics, RuntimeReport, ScopedRuntime, TaskExecutor,
    WorkerBinder, WorkerDescriptor,
};
pub use scheduler::{
    GlobalFifo, HierarchicalLocality, LocalWorkStealing, OwnerCompute, ReferenceReport,
    ReferenceScheduler, SchedulingPolicy, SchedulingTrace, Sequential, StaticPartition, TaskState,
    TraceEvent,
};
pub use topology::{
    CacheDomain, FactCertainty, FactSource, HardwareTopology, HostSchedulerRecord, Locality,
    NumaNode, ObservedFact, ProcessingUnit,
};
pub use verify::TaskGraphVerifier;
