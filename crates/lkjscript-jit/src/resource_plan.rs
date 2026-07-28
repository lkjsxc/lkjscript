use lkjscript_resource::{
    CpuSet, ExecutionResourcePlan, PlacementMode, ResourceResult, WorkerGroup, WorkerGroupId,
    WorkerId, WorkerPlacement,
};

pub(crate) fn proof_discovery_plan(workers: u16) -> ResourceResult<ExecutionResourcePlan> {
    let count = usize::from(workers.max(1));
    let allowed = CpuSet::new([0])?;
    let placements = (0..count)
        .map(|slot| WorkerPlacement {
            worker: WorkerId::new(slot as u32, 1),
            group: WorkerGroupId::new(0, 1),
            exact_mask: allowed.clone(),
            elastic_locality: allowed.clone(),
            queue_bytes: 64 * 1024,
            scratch_bytes: 1024 * 1024,
        })
        .collect::<Vec<_>>();
    Ok(ExecutionResourcePlan {
        mode: PlacementMode::KernelManaged,
        groups: vec![WorkerGroup {
            id: WorkerGroupId::new(0, 1),
            mask: allowed,
            workers: placements.iter().map(|worker| worker.worker).collect(),
        }],
        workers: placements,
        sequential_fallback: count == 1,
    })
}
