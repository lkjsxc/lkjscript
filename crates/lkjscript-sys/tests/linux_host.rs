#[allow(dead_code)]
#[path = "linux_host/fixture.rs"]
mod fixture;

use fixture::Fixture;
use lkjscript_resource::FactCertainty;
use lkjscript_sys::{
    current_thread_affinity, discover_linux_host, discover_linux_host_at, AffinityGuard,
    ConfigValue, SchedExtState,
};

fn rich_fixture() -> std::io::Result<Fixture> {
    let fixture = Fixture::new(&[0, 1, 2, 3], "0-2", "0-2")?;
    fixture.write("sys/devices/system/cpu/cpu3/online", "0")?;
    for (cpu, core, siblings, cache) in [
        (0, 0, "0-1", 10),
        (1, 0, "0-1", 10),
        (2, 1, "2-3", 11),
        (3, 1, "2-3", 11),
    ] {
        fixture.cpu(cpu, 0, 0, core, siblings)?;
        let shared = if cache == 10 { "0-1" } else { "2-3" };
        fixture.cache(cpu, 0, cache, 3, "Unified", shared)?;
    }
    fixture.node(0, "0-1", "10 21", 1_024)?;
    fixture.node(1, "2-3", "21 10", 2_048)?;
    fixture.write("sys/fs/cgroup/fixture/cpuset.cpus.effective", "0,2")?;
    fixture.write("sys/fs/cgroup/fixture/cpu.max", "150000 100000")?;
    fixture.write("boot/config-fixture-kernel", "CONFIG_SCHED_CACHE=y\n")?;
    fixture.write("sys/kernel/debug/sched/features", "GENTLE_FAIR_SLEEPERS")?;
    fixture.write("sys/kernel/sched_ext/state", "enabled")?;
    fixture.write("sys/kernel/sched_ext/root/ops", "fixture_ops")?;
    Ok(fixture)
}

#[test]
fn discovers_smt_llc_numa_cpuset_offline_and_scheduler_facts(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = rich_fixture()?;
    let first = discover_linux_host_at(fixture.path())?;
    let second = discover_linux_host_at(fixture.path())?;
    assert_eq!(first.digest, second.digest);
    assert_eq!(first.topology.allowed.as_slice(), &[0, 2]);
    assert_eq!(first.topology.units.len(), 4);
    assert!(!first.topology.unit(3).ok_or("missing CPU 3")?.online);
    assert_eq!(first.topology.caches.len(), 2);
    assert_eq!(first.numa.len(), 2);
    assert_eq!(first.numa[1].capacity_bytes.value, Some(2_048 * 1_024));
    assert_eq!(first.cpus[0].smt_siblings.as_slice(), &[0, 1]);
    assert_eq!(first.cpus[2].llc_domain, Some(11));
    assert_eq!(first.cpus[3].numa_node, Some(1));
    assert_eq!(first.scheduler.sched_cache.value, ConfigValue::Enabled);
    assert!(first.scheduler.sched_cache.certain);
    assert_eq!(
        first.scheduler.sched_ext_state.value,
        SchedExtState::Enabled
    );
    assert_eq!(
        first.scheduler.active_sched_ext.value.as_deref(),
        Some("fixture_ops")
    );
    assert_eq!(first.scheduler_record().quota_workers.value, Some(2));
    first.scheduler_record().validate()?;
    Ok(())
}

#[test]
fn missing_die_and_config_are_explicitly_unknown_and_n_is_not_y(
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture = Fixture::new(&[0], "0", "0")?;
    fixture.remove("sys/devices/system/cpu/cpu0/topology/die_id")?;
    let unknown = discover_linux_host_at(fixture.path())?;
    assert_eq!(unknown.cpus[0].die.value, None);
    assert!(!unknown.cpus[0].die.certain);
    assert_eq!(unknown.topology.certainty, FactCertainty::Unknown);
    assert_eq!(unknown.scheduler.sched_cache.value, ConfigValue::Unknown);
    assert!(!unknown.scheduler.sched_cache.certain);
    fixture.write("boot/config-fixture-kernel", "CONFIG_SCHED_CACHE=n\n")?;
    let disabled = discover_linux_host_at(fixture.path())?;
    assert_eq!(disabled.scheduler.sched_cache.value, ConfigValue::Disabled);
    assert!(disabled.scheduler.sched_cache.certain);
    Ok(())
}

#[test]
fn host_discovery_smoke_is_read_only() -> Result<(), Box<dyn std::error::Error>> {
    let snapshot = discover_linux_host()?;
    assert!(!snapshot.topology.allowed.is_empty());
    assert_eq!(
        snapshot.scheduler.thread_affinity.value,
        current_thread_affinity()?
    );
    snapshot.scheduler_record().validate()?;
    Ok(())
}

#[test]
fn affinity_guard_sets_reads_back_and_restores_or_reports_host_denial(
) -> Result<(), Box<dyn std::error::Error>> {
    let saved = current_thread_affinity()?;
    let cpu = *saved.as_slice().first().ok_or("host affinity is empty")?;
    let one = lkjscript_resource::CpuSet::new([cpu])?;
    match AffinityGuard::bind(&one) {
        Ok(guard) => {
            assert_eq!(current_thread_affinity()?, one);
            guard.restore()?;
            assert_eq!(current_thread_affinity()?, saved);
        }
        Err(error)
            if error.code == "affinity-write"
                && (error.detail == "errno 1" || error.detail == "errno 13") =>
        {
            assert_eq!(current_thread_affinity()?, saved);
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[test]
fn rejects_numa_distance_and_llc_sharing_conflicts() -> Result<(), Box<dyn std::error::Error>> {
    let distance = rich_fixture()?;
    distance.write("sys/devices/system/node/node0/distance", "10")?;
    assert_eq!(
        discover_linux_host_at(distance.path())
            .err()
            .ok_or("accepted distances")?
            .code,
        "numa-distance-mismatch"
    );

    let cache = rich_fixture()?;
    cache.cache(2, 1, 12, 3, "Unified", "1-2")?;
    assert_eq!(
        discover_linux_host_at(cache.path())
            .err()
            .ok_or("accepted LLC conflict")?
            .code,
        "llc-sharing-conflict"
    );
    Ok(())
}
