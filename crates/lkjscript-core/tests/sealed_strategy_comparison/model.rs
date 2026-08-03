use std::hint::black_box;
use std::time::Instant;

const RUNS: usize = 257;
const WARMUP_RUNS: usize = 8;
const NODE_BYTES: u64 = 24;

#[derive(Clone, Copy)]
pub struct Workload {
    pub name: &'static str,
    pub nodes: u64,
    pub owners: u64,
    pub process: bool,
    pub fusion: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Measurement {
    pub allocations: u64,
    pub logical_bytes: u64,
    pub copied_bytes: u64,
    pub owner_ops: u64,
    pub per_node_ops: u64,
    pub atomics: u64,
    pub release_work: u64,
    pub p99_ns: u128,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum Candidate {
    DetachedClone,
    CoarseSealed,
    NodeDomains,
    PerNodeRc,
    UniqueFusion,
}

impl Candidate {
    pub const ALL: [Self; 5] = [
        Self::DetachedClone,
        Self::CoarseSealed,
        Self::NodeDomains,
        Self::PerNodeRc,
        Self::UniqueFusion,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::DetachedClone => "detached-clone",
            Self::CoarseSealed => "coarse-sealed",
            Self::NodeDomains => "one-node-domains",
            Self::PerNodeRc => "private-per-node-rc",
            Self::UniqueFusion => "unique-fusion",
        }
    }

    pub fn measure(self, workload: Workload) -> Option<Measurement> {
        if matches!(self, Self::UniqueFusion) && !workload.fusion {
            return None;
        }
        for _ in 0..WARMUP_RUNS {
            black_box(run_candidate(self, workload));
        }
        let mut samples = Vec::with_capacity(RUNS);
        for _ in 0..RUNS {
            let started = Instant::now();
            black_box(run_candidate(self, workload));
            samples.push(started.elapsed().as_nanos());
        }
        samples.sort_unstable();
        let (allocations, logical_bytes, copied_bytes, owner_ops, per_node_ops, release_work) =
            counts(self, workload);
        Some(Measurement {
            allocations,
            logical_bytes,
            copied_bytes,
            owner_ops,
            per_node_ops,
            atomics: 0,
            release_work,
            p99_ns: samples[(RUNS * 99).div_ceil(100) - 1],
        })
    }
}

fn run_candidate(candidate: Candidate, workload: Workload) -> u64 {
    let nodes = usize::try_from(workload.nodes).expect("bounded nodes");
    let owners = usize::try_from(workload.owners).expect("bounded owners");
    let image: Vec<u64> = (0..workload.nodes).collect();
    match candidate {
        Candidate::DetachedClone => {
            let copies: Vec<_> = (0..owners).map(|_| image.clone()).collect();
            copies.iter().flatten().copied().sum()
        }
        Candidate::CoarseSealed => {
            let mut count = 1_u64;
            count += workload.owners - 1;
            for _ in 0..workload.owners {
                count -= 1;
            }
            image.iter().copied().sum::<u64>() + count
        }
        Candidate::NodeDomains => {
            let mut domains = vec![owners as u32; nodes];
            domains.fill(0);
            image.iter().copied().sum::<u64>() + u64::from(domains[0])
        }
        Candidate::PerNodeRc => {
            let mut counts = vec![1_u32; nodes];
            for _ in 1..owners {
                for count in &mut counts {
                    *count += 1;
                }
            }
            for _ in 0..owners {
                for count in &mut counts {
                    *count -= 1;
                }
            }
            image.iter().copied().sum::<u64>() + u64::from(counts[0])
        }
        Candidate::UniqueFusion => image.iter().copied().sum(),
    }
}

fn counts(candidate: Candidate, workload: Workload) -> (u64, u64, u64, u64, u64, u64) {
    let bytes = workload.nodes * NODE_BYTES;
    let process_copy = u64::from(workload.process) * bytes * 2;
    match candidate {
        Candidate::DetachedClone => (
            workload.owners,
            bytes * workload.owners,
            bytes * workload.owners + process_copy,
            0,
            0,
            workload.nodes * workload.owners,
        ),
        Candidate::CoarseSealed => (
            1,
            bytes,
            process_copy,
            workload.owners * 2 - 1,
            0,
            workload.owners,
        ),
        Candidate::NodeDomains => (
            workload.nodes,
            bytes,
            process_copy,
            0,
            workload.nodes * workload.owners * 2,
            workload.nodes,
        ),
        Candidate::PerNodeRc => (
            workload.nodes,
            bytes + workload.nodes * 4,
            process_copy,
            0,
            workload.nodes * (workload.owners * 2 - 1),
            workload.nodes,
        ),
        Candidate::UniqueFusion => (0, bytes, process_copy, 0, 0, 1),
    }
}
