use std::sync::atomic::{AtomicU64, Ordering};

use lkjscript_resource::{TaskExecutor, TaskId, WorkerId};

pub(super) const TASKS: usize = 256;
const CHUNK: usize = 1024;

#[derive(Clone, Copy)]
pub(super) enum Workload {
    Reuse,
    Streaming,
    Imbalanced,
    FalseSharing,
    PaddedMetadata,
}

impl Workload {
    pub(super) const ALL: [Self; 5] = [
        Self::Reuse,
        Self::Streaming,
        Self::Imbalanced,
        Self::FalseSharing,
        Self::PaddedMetadata,
    ];

    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Reuse => "reuse",
            Self::Streaming => "streaming",
            Self::Imbalanced => "imbalanced",
            Self::FalseSharing => "false-sharing",
            Self::PaddedMetadata => "padded-metadata",
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
}

impl WorkloadExecutor {
    pub(super) fn new(workload: Workload) -> Self {
        Self {
            workload,
            data: (0..TASKS * CHUNK).map(|value| value as u64).collect(),
            adjacent: (0..TASKS).map(|_| AtomicU64::new(0)).collect(),
            padded: (0..TASKS).map(|_| Padded(AtomicU64::new(0))).collect(),
        }
    }
}

impl TaskExecutor for WorkloadExecutor {
    type Output = u64;
    type Error = String;

    fn execute(&self, task: TaskId, _worker: WorkerId) -> Result<Self::Output, Self::Error> {
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
        }
    }
}
