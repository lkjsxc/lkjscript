use std::path::Path;

use lkjscript_resource::{
    FactCertainty, FactSource, HardwareTopology, HostSchedulerRecord, ObservedFact,
};

use super::cache::discover_caches;
use super::error::{HostResult, LinuxHostError};
use super::numa::discover_numa;
use super::parse::intersect;
use super::root::AnchoredRoot;
use super::scheduler::discover_scheduler;
use super::topology::{discover_cpus, TOPOLOGY_SOURCE};
use super::types::{LinuxFactSource, LinuxHostSnapshot};

pub fn discover_linux_host() -> HostResult<LinuxHostSnapshot> {
    discover_linux_host_at("/")
}

pub fn discover_linux_host_at(root: impl AsRef<Path>) -> HostResult<LinuxHostSnapshot> {
    let root = AnchoredRoot::new(root.as_ref())?;
    let mut cpu = discover_cpus(&root)?;
    let (caches, resource_caches) = discover_caches(&root, &mut cpu.details)?;
    let (numa, resource_numa) = discover_numa(&root, &mut cpu.details)?;
    let scheduler = discover_scheduler(&root)?;
    validate_scheduler_sets(
        &cpu.online,
        &scheduler.observation.process_affinity.value,
        scheduler.observation.cpuset_effective.value.as_ref(),
    )?;
    let mut sets = vec![&cpu.online, &scheduler.observation.process_affinity.value];
    if let Some(cpuset) = &scheduler.observation.cpuset_effective.value {
        sets.push(cpuset);
    }
    let allowed = intersect(&sets)?;
    let topology = HardwareTopology {
        units: cpu.units,
        caches: resource_caches,
        numa_nodes: resource_numa,
        allowed,
        source: TOPOLOGY_SOURCE,
        certainty: cpu.certainty,
    };
    topology.validate()?;
    let mut snapshot = LinuxHostSnapshot {
        topology,
        cpus: cpu.details,
        caches,
        numa,
        scheduler: scheduler.observation,
        digest: [0; 32],
        quota_workers: scheduler.quota_workers,
    };
    snapshot.digest = digest(&snapshot);
    snapshot.scheduler_record().validate()?;
    Ok(snapshot)
}

fn validate_scheduler_sets(
    online: &lkjscript_resource::CpuSet,
    process: &lkjscript_resource::CpuSet,
    cpuset: Option<&lkjscript_resource::CpuSet>,
) -> HostResult<()> {
    if process.as_slice().iter().any(|cpu| !online.contains(*cpu)) {
        return Err(LinuxHostError::new(
            "affinity-offline",
            "process affinity contains an offline CPU",
        ));
    }
    if cpuset.is_some_and(|set| set.as_slice().iter().any(|cpu| !online.contains(*cpu))) {
        return Err(LinuxHostError::new(
            "cpuset-offline",
            "effective cpuset contains an offline CPU",
        ));
    }
    Ok(())
}

fn digest(snapshot: &LinuxHostSnapshot) -> [u8; 32] {
    let bytes = format!(
        "lkjscript.linux-host.v1\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{:?}",
        snapshot.topology,
        snapshot.cpus,
        snapshot.caches,
        snapshot.numa,
        snapshot.scheduler,
        snapshot.quota_workers
    );
    lkjscript_contracts::sha256(bytes.as_bytes())
}

pub(crate) fn scheduler_record(snapshot: &LinuxHostSnapshot) -> HostSchedulerRecord {
    HostSchedulerRecord {
        online: ObservedFact {
            value: lkjscript_resource::CpuSet::new(
                snapshot
                    .cpus
                    .iter()
                    .filter(|cpu| cpu.online)
                    .map(|cpu| cpu.cpu),
            )
            .unwrap_or_else(|_| snapshot.topology.allowed.clone()),
            source: FactSource::LinuxSysfs,
            certainty: FactCertainty::Reported,
        },
        allowed: ObservedFact {
            value: snapshot.topology.allowed.clone(),
            source: FactSource::LinuxProcfs,
            certainty: FactCertainty::Observed,
        },
        quota_workers: ObservedFact {
            value: snapshot.quota_workers,
            source: if snapshot.scheduler.cpuset_effective.source == LinuxFactSource::Cgroup {
                FactSource::LinuxProcfs
            } else {
                FactSource::Unknown
            },
            certainty: if snapshot.quota_workers.is_some() {
                FactCertainty::Reported
            } else {
                FactCertainty::Unknown
            },
        },
        topology: ObservedFact {
            value: snapshot.topology.clone(),
            source: snapshot.topology.source,
            certainty: snapshot.topology.certainty,
        },
    }
}
