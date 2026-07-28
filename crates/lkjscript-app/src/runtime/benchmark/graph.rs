use lkjscript_resource::*;

use super::workload::{Workload, TASKS};

pub(super) fn graph(workload: Workload) -> ResourceResult<VerifiedTaskGraph> {
    let scope = TaskScopeId::new(0, 1);
    let shared = DataOwnerId::new(0, 1);
    let mut builder = TaskGraphBuilder::new();
    builder.add_scope(TaskScope {
        id: scope,
        parent: None,
    })?;
    for slot in 0..TASKS as u32 {
        let metadata = matches!(workload, Workload::FalseSharing | Workload::PaddedMetadata);
        let shared_access = AccessRecord {
            id: AccessRecordId::new(slot * 2, 1),
            owner: shared,
            mode: if metadata {
                AccessMode::Write
            } else {
                AccessMode::Read
            },
            range: metadata.then_some(CheckedRange {
                start: u64::from(slot) * 8,
                end: u64::from(slot + 1) * 8,
            }),
        };
        let owner = DataOwnerId::new(slot + 1, 1);
        builder.add_task(TaskNode {
            id: TaskId::new(slot, 1),
            class: TaskClassId::from_name(workload.name()),
            scope,
            result: TaskResultId::new(slot, 1),
            result_owner: owner,
            dependencies: Vec::new(),
            accesses: vec![
                shared_access,
                AccessRecord {
                    id: AccessRecordId::new(slot * 2 + 1, 1),
                    owner,
                    mode: AccessMode::Produce,
                    range: None,
                },
            ],
            blocking: false,
            portable: true,
            compute_units: if matches!(workload, Workload::Imbalanced) {
                u64::from(slot % 31 + 1)
            } else {
                1
            },
            scratch_bytes: 0,
            cleanup: true,
        })?;
    }
    TaskGraphVerifier::verify(builder.build(), GraphLimits::default())
}
