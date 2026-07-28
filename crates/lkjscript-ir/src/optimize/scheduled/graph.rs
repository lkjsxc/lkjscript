use lkjscript_resource::{
    AccessMode, AccessRecord, AccessRecordId, DataOwnerId, ExecutionResourcePlan, GraphLimits,
    TaskClassId, TaskGraphBuilder, TaskGraphVerifier, TaskId, TaskNode, TaskResultId, TaskScope,
    TaskScopeId, VerifiedTaskGraph,
};

use crate::{Function, InstructionKind, Program, Terminator};

pub(super) fn task_graph(
    program: &Program,
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
    for (slot, function) in program.functions.iter().enumerate() {
        let task = TaskId::new(slot as u32, 1);
        let local_compute = discovery_weight(function);
        let local_scratch = discovery_scratch(function);
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
            max_tasks: program.functions.len(),
            max_dependencies: 0,
            max_accesses: program.functions.len().saturating_mul(2),
            max_compute_units: compute,
            max_scratch_bytes: scratch,
        },
    )
}

pub(super) fn validate_admission(
    graph: &VerifiedTaskGraph,
    plan: &ExecutionResourcePlan,
) -> Result<(), &'static str> {
    if plan.workers.is_empty() {
        return Err("scheduled discovery requires a worker");
    }
    let mut queued = vec![0_u64; plan.workers.len()];
    let descriptor_bytes = std::mem::size_of::<TaskId>() as u64;
    for task in graph.tasks() {
        let worker = task.id.slot as usize % plan.workers.len();
        queued[worker] = queued[worker].saturating_add(descriptor_bytes);
        if task.scratch_bytes > plan.workers[worker].scratch_bytes {
            return Err("scheduled discovery scratch reservation exceeds worker plan");
        }
    }
    if queued
        .iter()
        .zip(&plan.workers)
        .any(|(bytes, worker)| *bytes > worker.queue_bytes)
    {
        return Err("scheduled discovery queue reservation exceeds worker plan");
    }
    Ok(())
}

fn discovery_scratch(function: &Function) -> u64 {
    let blocks = function.blocks.len() as u64;
    let values = function
        .blocks
        .iter()
        .map(|block| {
            block
                .parameters
                .len()
                .saturating_add(block.instructions.len()) as u64
        })
        .sum::<u64>();
    let words = blocks.saturating_add(63) / 64;
    values
        .saturating_mul(192)
        .saturating_add(blocks.saturating_mul(words).saturating_mul(8))
        .saturating_add(blocks.saturating_mul(64))
        .max(1)
}

pub(super) fn discovery_weight(function: &Function) -> u64 {
    let blocks = function.blocks.len() as u64;
    let instructions = function
        .blocks
        .iter()
        .map(|block| block.instructions.len() as u64)
        .sum::<u64>();
    let parameters = function
        .blocks
        .iter()
        .map(|block| block.parameters.len() as u64)
        .sum::<u64>();
    let operands = function
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .map(|instruction| match &instruction.kind {
            InstructionKind::Runtime { arguments, .. }
            | InstructionKind::Call { arguments, .. } => arguments.len() as u64,
            _ => 0,
        })
        .sum::<u64>();
    let edges = function
        .blocks
        .iter()
        .map(|block| match block.terminator {
            Terminator::Branch { .. } => 1,
            Terminator::ConditionalBranch { .. } => 2,
            _ => 0,
        })
        .sum::<u64>();
    let words = blocks.saturating_add(63) / 64;
    let dominance_scan = blocks.saturating_add(edges).saturating_mul(words);
    let dominance = blocks
        .saturating_mul(words.saturating_add(3))
        .saturating_add(edges)
        .saturating_add(blocks.saturating_mul(blocks))
        .saturating_add(
            blocks
                .saturating_mul(blocks)
                .saturating_add(1)
                .saturating_mul(dominance_scan),
        );
    1_u64
        .saturating_add(parameters.saturating_add(instructions).saturating_mul(2))
        .saturating_add(instructions.saturating_mul(4))
        .saturating_add(operands.saturating_mul(3))
        .saturating_add(dominance)
        .saturating_add(instructions.saturating_mul(instructions))
}
