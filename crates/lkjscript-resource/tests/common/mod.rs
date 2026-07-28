#![allow(dead_code)]
use std::collections::BTreeMap;

use lkjscript_resource::*;

pub fn id(slot: u32) -> TaskId {
    TaskId::new(slot, 1)
}
pub fn scope(slot: u32) -> TaskScopeId {
    TaskScopeId::new(slot, 1)
}
pub fn owner(slot: u32) -> DataOwnerId {
    DataOwnerId::new(slot, 1)
}

pub fn node(slot: u32, dependencies: Vec<TaskId>) -> TaskNode {
    TaskNode {
        id: id(slot),
        class: TaskClassId::from_name("compute"),
        scope: scope(0),
        result: TaskResultId::new(slot, 1),
        result_owner: owner(100 + slot),
        dependencies,
        accesses: vec![AccessRecord {
            id: AccessRecordId::new(slot, 1),
            owner: owner(100 + slot),
            mode: AccessMode::Produce,
            range: None,
        }],
        blocking: false,
        portable: true,
        compute_units: 1,
        scratch_bytes: 8,
        cleanup: true,
    }
}

pub fn graph(dependencies: &[Vec<TaskId>]) -> ResourceResult<VerifiedTaskGraph> {
    let mut builder = TaskGraphBuilder::new();
    builder.add_scope(TaskScope {
        id: scope(0),
        parent: None,
    })?;
    for (slot, deps) in dependencies.iter().enumerate() {
        builder.add_task(node(slot as u32, deps.clone()))?;
    }
    TaskGraphVerifier::verify(builder.build(), GraphLimits::default())
}

pub fn topology() -> ResourceResult<HardwareTopology> {
    let units = (0..8)
        .map(|cpu| ProcessingUnit {
            cpu,
            package: 0,
            die: 0,
            core: cpu / 2,
            online: true,
        })
        .collect();
    Ok(HardwareTopology {
        units,
        caches: vec![
            CacheDomain {
                id: 0,
                level: 3,
                cpus: cpus(&[0, 1, 2, 3])?,
            },
            CacheDomain {
                id: 1,
                level: 3,
                cpus: cpus(&[4, 5, 6, 7])?,
            },
        ],
        numa_nodes: vec![
            NumaNode {
                id: 0,
                cpus: cpus(&[0, 1, 2, 3])?,
                distances: BTreeMap::from([(0, 10), (1, 20)]),
            },
            NumaNode {
                id: 1,
                cpus: cpus(&[4, 5, 6, 7])?,
                distances: BTreeMap::from([(0, 20), (1, 10)]),
            },
        ],
        allowed: cpus(&[0, 1, 2, 3, 4, 5, 6, 7])?,
        source: FactSource::Synthetic,
        certainty: FactCertainty::Observed,
    })
}

pub fn cpus(values: &[u32]) -> ResourceResult<CpuSet> {
    CpuSet::new(values.iter().copied())
}

pub fn caps() -> PlanCaps {
    PlanCaps {
        task_cap: 8,
        request_cap: 8,
        profile_cap: 8,
        queue_bytes_per_worker: 1024,
        scratch_bytes_per_worker: 2048,
    }
}
