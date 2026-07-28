use lkjscript_resource::{
    AccessMode, AccessRecord, AccessRecordId, DataOwnerId, GraphLimits, TaskClassId,
    TaskGraphBuilder, TaskGraphVerifier, TaskId, TaskNode, TaskResultId, TaskScope, TaskScopeId,
    VerifiedTaskGraph,
};

use crate::Program;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct FunctionRange {
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn ranges(program: &Program, workers: usize) -> Vec<FunctionRange> {
    let total = program
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .map(|block| block.instructions.len())
        .sum::<usize>();
    let target = total
        .checked_div(workers.max(1).saturating_mul(4))
        .unwrap_or(0)
        .max(64);
    let mut output = Vec::new();
    let mut start = 0;
    let mut work = 0_usize;
    for (index, function) in program.functions.iter().enumerate() {
        work = work.saturating_add(
            function
                .blocks
                .iter()
                .map(|block| block.instructions.len())
                .sum::<usize>(),
        );
        if work >= target {
            output.push(FunctionRange {
                start,
                end: index + 1,
            });
            start = index + 1;
            work = 0;
        }
    }
    if start < program.functions.len() {
        output.push(FunctionRange {
            start,
            end: program.functions.len(),
        });
    }
    output
}

pub(super) fn task_graph(
    program: &Program,
    ranges: &[FunctionRange],
) -> Result<VerifiedTaskGraph, lkjscript_resource::ResourceError> {
    let scope = TaskScopeId::new(0, 1);
    let mut builder = TaskGraphBuilder::new();
    builder.add_scope(TaskScope {
        id: scope,
        parent: None,
    })?;
    let input = DataOwnerId::new(0, 1);
    let mut compute = 0_u64;
    let mut scratch = 0_u64;
    for (slot, range) in ranges.iter().enumerate() {
        let task = TaskId::new(slot as u32, 1);
        let local_compute = program.functions[range.start..range.end]
            .iter()
            .flat_map(|function| &function.blocks)
            .map(|block| block.instructions.len() as u64)
            .sum::<u64>()
            .max(1);
        let local_scratch = local_compute.saturating_mul(64);
        compute = compute.saturating_add(local_compute);
        scratch = scratch.saturating_add(local_scratch);
        let output = DataOwnerId::new(slot as u32 + 1, 1);
        builder.add_task(TaskNode {
            id: task,
            class: TaskClassId::from_name("proof-edit-discovery"),
            scope,
            result: TaskResultId::new(slot as u32, 1),
            result_owner: output,
            dependencies: Vec::new(),
            accesses: vec![
                AccessRecord {
                    id: AccessRecordId::new((slot * 2) as u32, 1),
                    owner: input,
                    mode: AccessMode::Read,
                    range: None,
                },
                AccessRecord {
                    id: AccessRecordId::new((slot * 2 + 1) as u32, 1),
                    owner: output,
                    mode: AccessMode::Produce,
                    range: None,
                },
            ],
            blocking: false,
            portable: true,
            compute_units: local_compute,
            scratch_bytes: local_scratch,
            cleanup: true,
        })?;
    }
    TaskGraphVerifier::verify(
        builder.build(),
        GraphLimits {
            max_tasks: ranges.len(),
            max_dependencies: 0,
            max_accesses: ranges.len().saturating_mul(2),
            max_compute_units: compute,
            max_scratch_bytes: scratch,
        },
    )
}
