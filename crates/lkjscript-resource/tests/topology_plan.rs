mod common;

use common::*;
use lkjscript_resource::*;

#[test]
fn cpu_list_and_map_are_exact_and_bounded() -> ResourceResult<()> {
    assert_eq!(CpuSet::parse_list("0,2-4,9")?.as_slice(), &[0, 2, 3, 4, 9]);
    assert_eq!(
        CpuSet::parse_map("00000001,00000003")?.as_slice(),
        &[0, 1, 32]
    );
    for malformed in [
        "",
        "1,",
        ",1",
        "1,,2",
        "3-1",
        "1-2-3",
        "01",
        "1 2",
        "1-2,2",
        "4294967296",
        "1048576",
    ] {
        assert!(
            CpuSet::parse_list(malformed).is_err(),
            "accepted {malformed}"
        );
    }
    for malformed in ["", "0", "gg", "1,", "123456789", " 1"] {
        assert!(
            CpuSet::parse_map(malformed).is_err(),
            "accepted {malformed}"
        );
    }
    let too_many_ranges = (0..=MAX_RANGES)
        .map(|index| (index * 2).to_string())
        .collect::<Vec<_>>()
        .join(",");
    assert_eq!(
        CpuSet::parse_list(&too_many_ranges).map_err(|error| error.code),
        Err("cpu-ranges")
    );
    let too_many_cpus = format!("0-{}", MAX_CPUS);
    assert_eq!(
        CpuSet::parse_list(&too_many_cpus).map_err(|error| error.code),
        Err("cpu-limit")
    );
    Ok(())
}

#[test]
fn topology_models_smt_llc_numa_and_restriction() -> ResourceResult<()> {
    let mut topo = topology()?;
    topo.validate()?;
    assert_eq!(topo.smt_siblings(0)?.as_slice(), &[0, 1]);
    assert_eq!(topo.locality(0, 1)?, Locality::SmtSibling);
    assert_eq!(topo.locality(0, 2)?, Locality::SameLastLevelCache);
    assert_eq!(topo.locality(0, 4)?, Locality::SamePackage);
    topo.allowed = cpus(&[2, 3])?;
    topo.validate()?;
    let record = HostSchedulerRecord {
        online: ObservedFact {
            value: cpus(&[0, 1, 2, 3])?,
            source: FactSource::LinuxSysfs,
            certainty: FactCertainty::Observed,
        },
        allowed: ObservedFact {
            value: cpus(&[2, 3])?,
            source: FactSource::LinuxProcfs,
            certainty: FactCertainty::Observed,
        },
        quota_workers: ObservedFact {
            value: Some(1),
            source: FactSource::LinuxProcfs,
            certainty: FactCertainty::Reported,
        },
        topology: ObservedFact {
            value: topo,
            source: FactSource::Synthetic,
            certainty: FactCertainty::Observed,
        },
    };
    record.validate()?;
    Ok(())
}

#[test]
fn malformed_topologies_are_rejected() -> ResourceResult<()> {
    let mut duplicate = topology()?;
    duplicate.units.push(duplicate.units[0].clone());
    assert_eq!(
        duplicate.validate().map_err(|error| error.code),
        Err("topology-duplicate")
    );
    let mut cache_unknown = topology()?;
    cache_unknown.caches[0].cpus = cpus(&[0, 99])?;
    assert_eq!(
        cache_unknown.validate().map_err(|error| error.code),
        Err("cache-unknown")
    );
    let mut numa_duplicate = topology()?;
    numa_duplicate.numa_nodes[1].cpus = cpus(&[0, 4])?;
    assert_eq!(
        numa_duplicate.validate().map_err(|error| error.code),
        Err("numa-membership")
    );
    let mut offline = topology()?;
    offline.units[0].online = false;
    assert_eq!(
        offline.validate().map_err(|error| error.code),
        Err("topology-offline")
    );
    Ok(())
}

#[test]
fn planner_uses_physical_cores_compact_llc_and_modes() -> ResourceResult<()> {
    let topo = topology()?;
    for mode in [
        PlacementMode::KernelManaged,
        PlacementMode::CpuPinned,
        PlacementMode::LlcDomainMasked,
    ] {
        let plan = ResourcePlanner::plan(&topo, mode, caps(), 2)?;
        assert_eq!(plan.mode, mode);
        assert_eq!(plan.workers.len(), 2);
        assert_eq!(
            plan.workers[0].elastic_locality,
            plan.workers[1].elastic_locality
        );
        assert_eq!(plan.workers[0].worker, WorkerId::new(0, 1));
        assert!(plan.workers[0].exact_mask.contains(0));
        assert!(plan.workers[1].exact_mask.contains(2));
        plan.validate(&topo, caps())?;
    }
    let llc = ResourcePlanner::plan(&topo, PlacementMode::LlcDomainMasked, caps(), 2)?;
    assert_eq!(llc.workers[0].exact_mask.as_slice(), &[0, 1, 2, 3]);
    assert_eq!(llc.workers[0].exact_mask, llc.workers[1].exact_mask);
    let three = ResourcePlanner::plan(&topo, PlacementMode::CpuPinned, caps(), 3)?;
    let selected: Vec<u32> = three
        .workers
        .iter()
        .map(|worker| worker.exact_mask.as_slice()[0])
        .collect();
    assert_eq!(selected, vec![0, 2, 4]);
    Ok(())
}

#[test]
fn planner_falls_back_for_single_or_unknown_topology() -> ResourceResult<()> {
    let mut single = topology()?;
    single.allowed = cpus(&[3])?;
    let plan = ResourcePlanner::plan(&single, PlacementMode::CpuPinned, caps(), 8)?;
    assert!(plan.sequential_fallback);
    assert_eq!(plan.mode, PlacementMode::KernelManaged);
    assert_eq!(plan.workers.len(), 1);
    let mut unknown = topology()?;
    unknown.certainty = FactCertainty::Unknown;
    let plan = ResourcePlanner::plan(&unknown, PlacementMode::LlcDomainMasked, caps(), 8)?;
    assert!(plan.sequential_fallback);
    assert_eq!(plan.workers.len(), 1);
    Ok(())
}
