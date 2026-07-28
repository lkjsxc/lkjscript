use std::collections::BTreeMap;

use super::super::fixtures::*;
use crate::*;
use lkjscript_resource::{
    CacheDomain, CpuSet, FactCertainty, FactSource, HardwareTopology, NumaNode, PlacementMode,
    PlanCaps, ProcessingUnit, ResourcePlanner,
};

#[test]
fn scheduled_discovery_is_exactly_sequential_and_uses_verified_tasks() {
    let input = verify(widened_optimization_program()).expect("verify scheduled input");
    let limits = OptimizationLimits::default();
    let sequential = optimize(&input, limits).expect("sequential optimize");
    let topology = two_core_topology();
    let plan = ResourcePlanner::plan(
        &topology,
        PlacementMode::KernelManaged,
        PlanCaps {
            task_cap: 2,
            request_cap: 2,
            profile_cap: 2,
            queue_bytes_per_worker: 4_096,
            scratch_bytes_per_worker: 64 * 1024,
        },
        2,
    )
    .expect("plan two workers");
    let scheduled = optimize_scheduled(&input, limits, &plan).expect("scheduled optimize");
    assert_eq!(scheduled.optimized(), &sequential);
    assert_eq!(scheduled.task_count, 2);
    assert_eq!(scheduled.worker_count, 2);
    assert_eq!(scheduled.runtime.executed, 2);
    assert_eq!(
        evaluate(&input, &EvalConfig::default()),
        evaluate(
            scheduled.optimized().verified_program(),
            &EvalConfig::default()
        )
    );

    let repeated = optimize_scheduled(&input, limits, &plan).expect("repeat scheduled optimize");
    assert_eq!(repeated.optimized(), scheduled.optimized());
    assert_eq!(repeated.task_graph, scheduled.task_graph);
}

fn widened_optimization_program() -> Program {
    let mut program = optimizable_checked_program();
    for function in &mut program.functions {
        let block = &mut function.blocks[0];
        let start = block
            .parameters
            .len()
            .saturating_add(block.instructions.len()) as u32;
        for id in start..start + 80 {
            block.instructions.push(Instruction {
                id: ValueId::new(id),
                ty: SsaType::I64,
                kind: InstructionKind::Constant(Constant::I64(i64::from(id))),
                metadata: metadata(EffectSet::PURE),
            });
        }
    }
    program
}

fn two_core_topology() -> HardwareTopology {
    let cpus = CpuSet::new([0, 1]).expect("two CPU set");
    HardwareTopology {
        units: vec![
            ProcessingUnit {
                cpu: 0,
                package: 0,
                die: 0,
                core: 0,
                online: true,
            },
            ProcessingUnit {
                cpu: 1,
                package: 0,
                die: 0,
                core: 1,
                online: true,
            },
        ],
        caches: vec![CacheDomain {
            id: 0,
            level: 3,
            cpus: cpus.clone(),
        }],
        numa_nodes: vec![NumaNode {
            id: 0,
            cpus: cpus.clone(),
            distances: BTreeMap::from([(0, 10)]),
        }],
        allowed: cpus,
        source: FactSource::Synthetic,
        certainty: FactCertainty::Observed,
    }
}
