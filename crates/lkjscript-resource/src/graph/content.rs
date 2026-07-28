use std::collections::BTreeMap;

use lkjscript_contracts::{sha256, ContractDigest};

use super::{AccessMode, TaskNode, TaskScope};
use crate::TaskId;

pub(super) fn content_id(scopes: &[TaskScope], tasks: &[TaskNode]) -> ContractDigest {
    let mut bytes = Vec::new();
    let mut sorted_scopes = scopes.to_vec();
    sorted_scopes.sort_by_key(|scope| scope.id);
    for scope in sorted_scopes {
        push_id(&mut bytes, scope.id.slot, scope.id.generation);
        if let Some(parent) = scope.parent {
            push_id(&mut bytes, parent.slot, parent.generation);
        } else {
            bytes.extend_from_slice(&u64::MAX.to_be_bytes());
        }
    }
    let mut sorted_tasks = tasks.to_vec();
    sorted_tasks.sort_by_key(|task| task.id);
    for task in sorted_tasks {
        push_id(&mut bytes, task.id.slot, task.id.generation);
        bytes.extend_from_slice(&task.class.0.to_be_bytes());
        push_id(&mut bytes, task.scope.slot, task.scope.generation);
        push_id(&mut bytes, task.result.slot, task.result.generation);
        push_id(
            &mut bytes,
            task.result_owner.slot,
            task.result_owner.generation,
        );
        let mut deps = task.dependencies;
        deps.sort();
        for dep in deps {
            push_id(&mut bytes, dep.slot, dep.generation);
        }
        let mut accesses = task.accesses;
        accesses.sort_by_key(|access| access.id);
        for access in accesses {
            push_id(&mut bytes, access.id.slot, access.id.generation);
            push_id(&mut bytes, access.owner.slot, access.owner.generation);
            bytes.push(access_mode_byte(access.mode));
            if let Some(range) = access.range {
                bytes.extend_from_slice(&range.start.to_be_bytes());
                bytes.extend_from_slice(&range.end.to_be_bytes());
            } else {
                bytes.extend_from_slice(&[0xff; 16]);
            }
        }
        bytes.extend_from_slice(&task.compute_units.to_be_bytes());
        bytes.extend_from_slice(&task.scratch_bytes.to_be_bytes());
        bytes.extend_from_slice(&[task.blocking as u8, task.portable as u8, task.cleanup as u8]);
    }
    ContractDigest::from_bytes(sha256(&bytes))
}

fn push_id(bytes: &mut Vec<u8>, slot: u32, generation: u32) {
    bytes.extend_from_slice(&slot.to_be_bytes());
    bytes.extend_from_slice(&generation.to_be_bytes());
}
fn access_mode_byte(mode: AccessMode) -> u8 {
    match mode {
        AccessMode::Read => 0,
        AccessMode::Write => 1,
        AccessMode::Consume => 2,
        AccessMode::Produce => 3,
        AccessMode::IdentityOnly => 4,
    }
}

pub(crate) fn task_map(tasks: &[TaskNode]) -> BTreeMap<TaskId, usize> {
    tasks
        .iter()
        .enumerate()
        .map(|(index, task)| (task.id, index))
        .collect()
}
