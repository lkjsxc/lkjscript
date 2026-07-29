use crate::{DataOwnerId, TaskId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoLiveLoanProof {
    pub(super) owner: DataOwnerId,
    pub(super) epoch: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OwnerMetrics {
    pub transfers: u64,
    pub remote_releases: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemoteRelease {
    pub owner: DataOwnerId,
    pub from_task: TaskId,
    pub(super) epoch: u64,
}

impl RemoteRelease {
    pub const fn new(owner: DataOwnerId, from_task: TaskId) -> Self {
        Self {
            owner,
            from_task,
            epoch: 0,
        }
    }
}
