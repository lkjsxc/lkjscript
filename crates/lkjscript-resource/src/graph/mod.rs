use lkjscript_contracts::ContractDigest;

mod content;
use content::content_id;
pub(crate) use content::task_map;

use crate::{
    AccessRecordId, DataOwnerId, ResourceError, ResourceResult, TaskClassId, TaskId, TaskResultId,
    TaskScopeId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessMode {
    Read,
    Write,
    Consume,
    Produce,
    IdentityOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckedRange {
    pub start: u64,
    pub end: u64,
}

impl CheckedRange {
    pub fn new(start: u64, end: u64) -> ResourceResult<Self> {
        if start >= end {
            return Err(ResourceError::new("range-empty", "range must increase"));
        }
        Ok(Self { start, end })
    }
    pub fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessRecord {
    pub id: AccessRecordId,
    pub owner: DataOwnerId,
    pub mode: AccessMode,
    pub range: Option<CheckedRange>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskScope {
    pub id: TaskScopeId,
    pub parent: Option<TaskScopeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskNode {
    pub id: TaskId,
    pub class: TaskClassId,
    pub scope: TaskScopeId,
    pub result: TaskResultId,
    pub result_owner: DataOwnerId,
    pub dependencies: Vec<TaskId>,
    pub accesses: Vec<AccessRecord>,
    pub blocking: bool,
    pub portable: bool,
    pub compute_units: u64,
    pub scratch_bytes: u64,
    pub cleanup: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphLimits {
    pub max_tasks: usize,
    pub max_dependencies: usize,
    pub max_accesses: usize,
    pub max_compute_units: u64,
    pub max_scratch_bytes: u64,
}

impl Default for GraphLimits {
    fn default() -> Self {
        Self {
            max_tasks: 4_096,
            max_dependencies: 16_384,
            max_accesses: 16_384,
            max_compute_units: 1_000_000,
            max_scratch_bytes: 1 << 30,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnverifiedTaskGraph {
    pub scopes: Vec<TaskScope>,
    pub tasks: Vec<TaskNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedTaskGraph {
    scopes: Vec<TaskScope>,
    tasks: Vec<TaskNode>,
    id: ContractDigest,
}

impl VerifiedTaskGraph {
    pub fn tasks(&self) -> &[TaskNode] {
        &self.tasks
    }
    pub fn scopes(&self) -> &[TaskScope] {
        &self.scopes
    }
    pub fn id(&self) -> ContractDigest {
        self.id
    }
    pub(crate) fn new(scopes: Vec<TaskScope>, tasks: Vec<TaskNode>) -> Self {
        let id = content_id(&scopes, &tasks);
        Self { scopes, tasks, id }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TaskGraphBuilder {
    scopes: Vec<TaskScope>,
    tasks: Vec<TaskNode>,
}

impl TaskGraphBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_scope(&mut self, scope: TaskScope) -> ResourceResult<&mut Self> {
        if self.scopes.iter().any(|item| item.id == scope.id) {
            return Err(ResourceError::new(
                "scope-duplicate",
                "scope identifier repeated",
            ));
        }
        self.scopes.push(scope);
        Ok(self)
    }
    pub fn add_task(&mut self, task: TaskNode) -> ResourceResult<&mut Self> {
        if self.tasks.iter().any(|item| item.id == task.id) {
            return Err(ResourceError::new(
                "task-duplicate",
                "task identifier repeated",
            ));
        }
        self.tasks.push(task);
        Ok(self)
    }
    pub fn build(self) -> UnverifiedTaskGraph {
        UnverifiedTaskGraph {
            scopes: self.scopes,
            tasks: self.tasks,
        }
    }
}
