use std::collections::{BTreeMap, BTreeSet};

use lkjscript_resource::{CpuSet, FactCertainty, FactSource, ProcessingUnit, MAX_CPU, MAX_CPUS};

use super::error::{HostResult, LinuxHostError};
use super::parse::{cpu_list, indexed_name, number};
use super::root::AnchoredRoot;
use super::types::{Evidence, LinuxCpuObservation, LinuxFactSource};

pub(crate) struct CpuDiscovery {
    pub online: CpuSet,
    pub details: Vec<LinuxCpuObservation>,
    pub units: Vec<ProcessingUnit>,
    pub certainty: FactCertainty,
}

pub(crate) fn discover_cpus(root: &AnchoredRoot) -> HostResult<CpuDiscovery> {
    let online = cpu_list(&root.read("sys/devices/system/cpu/online")?, "cpu-online")?;
    let mut ids = BTreeSet::new();
    for name in root.entries("sys/devices/system/cpu")? {
        if let Some(cpu) = indexed_name(&name, "cpu") {
            if cpu > MAX_CPU || !ids.insert(cpu) || ids.len() > MAX_CPUS {
                return Err(LinuxHostError::new(
                    "cpu-count",
                    "duplicate or excessive CPUs",
                ));
            }
        }
    }
    if ids.is_empty() || online.as_slice().iter().any(|cpu| !ids.contains(cpu)) {
        return Err(LinuxHostError::new(
            "cpu-membership",
            "online CPU has no cpu directory",
        ));
    }
    let mut details = Vec::new();
    let mut units = Vec::new();
    let mut complete = true;
    for cpu in ids.iter().copied() {
        let base = format!("sys/devices/system/cpu/cpu{cpu}");
        if root.is_fixture() {
            validate_online_file(root, &base, cpu, online.contains(cpu))?;
        }
        let package = optional_u32(root, &format!("{base}/topology/physical_package_id"))?;
        let die = optional_u32(root, &format!("{base}/topology/die_id"))?;
        let core = optional_u32(root, &format!("{base}/topology/core_id"))?;
        complete &= package.is_some() && die.is_some() && core.is_some();
        let siblings = match root.read_optional(&format!("{base}/topology/thread_siblings_list"))? {
            Some(text) => cpu_list(&text, "smt-siblings")?,
            None => CpuSet::new([cpu])?,
        };
        if !siblings.contains(cpu) || siblings.as_slice().iter().any(|item| !ids.contains(item)) {
            return Err(LinuxHostError::new("smt-membership", format!("CPU {cpu}")));
        }
        details.push(LinuxCpuObservation {
            cpu,
            online: online.contains(cpu),
            package: fact(package),
            die: fact(die),
            core: fact(core),
            smt_siblings: siblings,
            llc_domain: None,
            numa_node: None,
        });
        units.push(ProcessingUnit {
            cpu,
            package: package.unwrap_or(0),
            die: die.unwrap_or(0),
            core: core.unwrap_or(cpu),
            online: online.contains(cpu),
        });
    }
    validate_siblings(&details)?;
    Ok(CpuDiscovery {
        online,
        details,
        units,
        certainty: if complete {
            FactCertainty::Reported
        } else {
            FactCertainty::Unknown
        },
    })
}

fn optional_u32(root: &AnchoredRoot, path: &str) -> HostResult<Option<u32>> {
    root.read_optional(path)?
        .map(|text| number::<u32>(&text, "topology-id"))
        .transpose()
}

fn fact(value: Option<u32>) -> Evidence<Option<u32>> {
    if value.is_some() {
        Evidence::reported(value, LinuxFactSource::Sysfs)
    } else {
        Evidence::unknown(None)
    }
}

fn validate_online_file(root: &AnchoredRoot, base: &str, cpu: u32, online: bool) -> HostResult<()> {
    let Some(text) = root.read_optional(&format!("{base}/online"))? else {
        return Ok(());
    };
    let local = match text.as_str() {
        "0" => false,
        "1" => true,
        _ => return Err(LinuxHostError::new("cpu-online", format!("CPU {cpu}"))),
    };
    if local != online {
        return Err(LinuxHostError::new(
            "cpu-online-conflict",
            format!("CPU {cpu}"),
        ));
    }
    Ok(())
}

fn validate_siblings(cpus: &[LinuxCpuObservation]) -> HostResult<()> {
    let by_id: BTreeMap<u32, &CpuSet> = cpus
        .iter()
        .map(|cpu| (cpu.cpu, &cpu.smt_siblings))
        .collect();
    for cpu in cpus {
        for sibling in cpu.smt_siblings.as_slice() {
            if by_id.get(sibling) != Some(&&cpu.smt_siblings) {
                return Err(LinuxHostError::new(
                    "smt-conflict",
                    format!("CPU {}", cpu.cpu),
                ));
            }
        }
    }
    Ok(())
}

pub(crate) const TOPOLOGY_SOURCE: FactSource = FactSource::LinuxSysfs;
