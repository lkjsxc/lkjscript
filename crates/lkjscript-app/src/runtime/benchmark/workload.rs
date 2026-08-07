use std::sync::atomic::{AtomicU64, Ordering};

use lkjscript_resource::{DataOwnerId, PartitionedUniqueStore, TaskExecutor, TaskId, WorkerId};

pub(super) const TASKS: usize = 256;
const CHUNK: usize = 1024;

#[derive(Clone, Copy)]
pub(super) enum Workload {
    Reuse,
    Streaming,
    Imbalanced,
    FalseSharing,
    PaddedMetadata,
    OwnerTransfer,
}

impl Workload {
    pub(super) const ALL: [Self; 6] = [
        Self::Reuse,
        Self::Streaming,
        Self::Imbalanced,
        Self::FalseSharing,
        Self::PaddedMetadata,
        Self::OwnerTransfer,
    ];

    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Reuse => "reuse",
            Self::Streaming => "streaming",
            Self::Imbalanced => "imbalanced",
            Self::FalseSharing => "false-sharing",
            Self::PaddedMetadata => "padded-metadata",
            Self::OwnerTransfer => "owner-transfer",
        }
    }
}

#[repr(align(64))]
struct Padded(AtomicU64);

pub(super) struct WorkloadExecutor {
    workload: Workload,
    data: Vec<u64>,
    adjacent: Vec<AtomicU64>,
    padded: Vec<Padded>,
    homes: PartitionedUniqueStore,
}

impl WorkloadExecutor {
    pub(super) fn new(workload: Workload) -> std::io::Result<Self> {
        Ok(Self {
            workload,
            data: (0..TASKS * CHUNK).map(|value| value as u64).collect(),
            adjacent: (0..TASKS).map(|_| AtomicU64::new(0)).collect(),
            padded: (0..TASKS).map(|_| Padded(AtomicU64::new(0))).collect(),
            homes: PartitionedUniqueStore::new(100, 4, TASKS, TASKS)
                .map_err(std::io::Error::other)?,
        })
    }

    pub(super) fn home_metrics(&self) -> lkjscript_resource::ResourceResult<[u64; 5]> {
        let (owner, unique) = self.homes.metrics()?;
        Ok([
            owner.transfers,
            owner.remote_releases,
            unique.allocated_bytes,
            unique.peak_live_bytes,
            u64::from(unique.live_objects),
        ])
    }
}

impl TaskExecutor for WorkloadExecutor {
    type Output = u64;
    type Error = String;

    fn execute(&self, task: TaskId, worker: WorkerId) -> Result<Self::Output, Self::Error> {
        let slot = task.slot as usize;
        match self.workload {
            Workload::Reuse => Ok((0..4).fold(0_u64, |sum, _| {
                self.data[..CHUNK]
                    .iter()
                    .fold(sum, |value, item| value.wrapping_add(*item))
            })),
            Workload::Streaming => Ok(self.data[slot * CHUNK..(slot + 1) * CHUNK]
                .iter()
                .fold(0_u64, |sum, item| sum.wrapping_add(*item))),
            Workload::Imbalanced => {
                let rounds = (slot % 31 + 1) * 1024;
                Ok((0..rounds).fold(slot as u64, |value, item| {
                    value.rotate_left(7) ^ item as u64
                }))
            }
            Workload::FalseSharing => {
                for _ in 0..4096 {
                    self.adjacent[slot].fetch_add(1, Ordering::Relaxed);
                }
                Ok(self.adjacent[slot].load(Ordering::Relaxed))
            }
            Workload::PaddedMetadata => {
                for _ in 0..4096 {
                    self.padded[slot].0.fetch_add(1, Ordering::Relaxed);
                }
                Ok(self.padded[slot].0.load(Ordering::Relaxed))
            }
            Workload::OwnerTransfer => {
                let destination = WorkerId::new((worker.slot + 1) % 4, 1);
                let value = self
                    .homes
                    .allocate_byte_vector(
                        DataOwnerId::new(task.slot + 1, 1),
                        worker,
                        vec![task.slot as u8; 256],
                    )
                    .map_err(|error| error.to_string())?;
                let proof = self
                    .homes
                    .prove_no_live_loan(value)
                    .map_err(|error| error.to_string())?;
                self.homes
                    .move_home(value, destination, proof)
                    .map_err(|error| error.to_string())?;
                let checksum = self
                    .homes
                    .checksum(value)
                    .map_err(|error| error.to_string())?;
                self.homes
                    .release(worker, task, value)
                    .map_err(|error| error.to_string())?;
                self.homes
                    .drain_remote(destination, 1)
                    .map_err(|error| error.to_string())?;
                Ok(checksum)
            }
        }
    }
}
