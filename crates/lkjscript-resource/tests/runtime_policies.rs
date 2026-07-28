mod common;

use common::*;
use lkjscript_resource::*;

struct PolicyExecutor;
impl TaskExecutor for PolicyExecutor {
    type Output = u32;
    type Error = String;

    fn execute(&self, task: TaskId, _worker: WorkerId) -> Result<Self::Output, Self::Error> {
        Ok(task.slot)
    }
}

#[test]
fn every_real_runtime_policy_completes_the_same_verified_graph() -> ResourceResult<()> {
    let dag = graph(&[
        vec![],
        vec![],
        vec![],
        vec![id(0)],
        vec![id(1)],
        vec![id(2)],
        vec![id(3), id(4), id(5)],
    ])?;
    for policy in [
        SchedulePolicy::Sequential,
        SchedulePolicy::StaticPartition,
        SchedulePolicy::GlobalFifo,
        SchedulePolicy::LocalWorkStealing,
        SchedulePolicy::HierarchicalLocality,
        SchedulePolicy::OwnerCompute,
    ] {
        let workers = if policy == SchedulePolicy::Sequential {
            1
        } else {
            3
        };
        let config = RuntimeConfig {
            workers: (0..workers)
                .map(|slot| {
                    Ok(WorkerDescriptor {
                        id: WorkerId::new(slot as u32, 1),
                        allowed: CpuSet::new([slot as u32])?,
                        group: WorkerGroupId::new((slot / 2) as u32, 1),
                        numa_node: Some((slot / 2) as u32),
                    })
                })
                .collect::<ResourceResult<Vec<_>>>()?,
            queue_capacity: 8,
            spin_limit: 0,
            policy,
        };
        let report = ScopedRuntime::run(&dag, config, &PolicyExecutor, &NoopWorkerBinder)?;
        assert!(report.failures.is_empty(), "policy={}", policy.name());
        assert_eq!(report.outputs.len(), 7, "policy={}", policy.name());
        assert_eq!(report.metrics.executed, 7, "policy={}", policy.name());
        assert_eq!(report.metrics.active_workers, 0);
    }
    Ok(())
}
