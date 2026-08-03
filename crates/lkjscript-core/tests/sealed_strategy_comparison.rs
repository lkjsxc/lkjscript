#![allow(clippy::expect_used)]

#[path = "sealed_strategy_comparison/model.rs"]
mod model;
use model::{Candidate, Workload};

#[test]
fn coarse_sharing_beats_per_node_ownership_without_hiding_fusion() {
    let workloads = [
        Workload {
            name: "two-owner",
            nodes: 2_048,
            owners: 2,
            process: false,
            fusion: false,
        },
        Workload {
            name: "four-owner",
            nodes: 2_048,
            owners: 4,
            process: false,
            fusion: false,
        },
        Workload {
            name: "branch",
            nodes: 2_048,
            owners: 2,
            process: false,
            fusion: false,
        },
        Workload {
            name: "process",
            nodes: 2_048,
            owners: 2,
            process: true,
            fusion: false,
        },
        Workload {
            name: "single",
            nodes: 2_048,
            owners: 1,
            process: false,
            fusion: true,
        },
        Workload {
            name: "borrow",
            nodes: 2_048,
            owners: 1,
            process: false,
            fusion: true,
        },
        Workload {
            name: "move",
            nodes: 2_048,
            owners: 1,
            process: false,
            fusion: true,
        },
    ];
    for workload in workloads {
        let mut eligible = Vec::new();
        for candidate in Candidate::ALL {
            if let Some(value) = candidate.measure(workload) {
                println!(
                    "strategy={} workload={} metrics={value:?}",
                    candidate.name(),
                    workload.name
                );
                eligible.push((candidate, value));
            }
        }
        let coarse = eligible
            .iter()
            .find(|(candidate, _)| *candidate == Candidate::CoarseSealed)
            .map(|(_, value)| *value)
            .expect("coarse");
        let node_rc = eligible
            .iter()
            .find(|(candidate, _)| *candidate == Candidate::PerNodeRc)
            .map(|(_, value)| *value)
            .expect("per-node");
        let best_p99 = eligible
            .iter()
            .map(|(_, item)| item.p99_ns)
            .min()
            .expect("p99");
        let best_bytes = eligible
            .iter()
            .map(|(_, item)| item.logical_bytes)
            .min()
            .expect("bytes");
        if !cfg!(debug_assertions) {
            assert!(coarse.p99_ns <= best_p99 * 2);
        }
        assert!(coarse.logical_bytes * 2 <= best_bytes * 3);
        assert!(coarse.copied_bytes <= node_rc.copied_bytes);
        if workload.owners > 1 {
            assert!(coarse.per_node_ops < node_rc.per_node_ops);
            assert!(coarse.release_work * 2 < node_rc.release_work);
            assert_eq!(coarse.atomics, 0);
        } else {
            let fusion = Candidate::UniqueFusion.measure(workload).expect("fusion");
            assert!(fusion.allocations < coarse.allocations);
            assert!(fusion.owner_ops < coarse.owner_ops);
        }
        assert!(coarse.p99_ns > 0);
        assert!(coarse.logical_bytes > 0);
    }
}
