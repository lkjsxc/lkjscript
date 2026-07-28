use std::collections::{BTreeMap, BTreeSet};

use lkjscript_resource::{NumaNode, MAX_CPUS};

use super::error::{HostResult, LinuxHostError};
use super::parse::{cpu_list, indexed_name, number};
use super::root::AnchoredRoot;
use super::types::{Evidence, LinuxCpuObservation, LinuxFactSource, LinuxNumaObservation};

const MAX_NODES: usize = 1_024;
const MAX_DISTANCES: usize = 65_536;

pub(crate) fn discover_numa(
    root: &AnchoredRoot,
    cpus: &mut [LinuxCpuObservation],
) -> HostResult<(Vec<LinuxNumaObservation>, Vec<NumaNode>)> {
    let directory = "sys/devices/system/node";
    if !root.exists(directory)? {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut ids = BTreeSet::new();
    for name in root.entries(directory)? {
        if let Some(id) = indexed_name(&name, "node") {
            if !ids.insert(id) || ids.len() > MAX_NODES {
                return Err(LinuxHostError::new(
                    "numa-count",
                    "duplicate or excessive nodes",
                ));
            }
        }
    }
    if ids.len().saturating_mul(ids.len()) > MAX_DISTANCES {
        return Err(LinuxHostError::new(
            "numa-distance-bound",
            "distance cap exceeded",
        ));
    }
    let known: BTreeSet<u32> = cpus.iter().map(|cpu| cpu.cpu).collect();
    let ordered: Vec<u32> = ids.iter().copied().collect();
    let mut assigned = BTreeSet::new();
    let mut details = Vec::new();
    for id in &ordered {
        let base = format!("{directory}/node{id}");
        let members = cpu_list(&root.read(&format!("{base}/cpulist"))?, "numa-cpus")?;
        if members.len() > MAX_CPUS
            || members
                .as_slice()
                .iter()
                .any(|cpu| !known.contains(cpu) || !assigned.insert(*cpu))
        {
            return Err(LinuxHostError::new("numa-membership", format!("node {id}")));
        }
        for cpu in members.as_slice() {
            if let Some(detail) = cpus.iter_mut().find(|detail| detail.cpu == *cpu) {
                detail.numa_node = Some(*id);
            }
        }
        let distances = read_distances(root, &base, &ordered)?;
        let capacity_bytes = read_capacity(root, &base, *id)?;
        details.push(LinuxNumaObservation {
            id: *id,
            cpus: members,
            capacity_bytes,
            distances,
        });
    }
    let resource = details
        .iter()
        .map(|node| NumaNode {
            id: node.id,
            cpus: node.cpus.clone(),
            distances: node.distances.clone(),
        })
        .collect();
    Ok((details, resource))
}

fn read_distances(root: &AnchoredRoot, base: &str, ids: &[u32]) -> HostResult<BTreeMap<u32, u32>> {
    let text = root.read(&format!("{base}/distance"))?;
    let values: Vec<&str> = text.split_ascii_whitespace().collect();
    if values.len() != ids.len() {
        return Err(LinuxHostError::new(
            "numa-distance-mismatch",
            format!("{} values for {} nodes", values.len(), ids.len()),
        ));
    }
    ids.iter()
        .zip(values)
        .map(|(id, value)| Ok((*id, number(value, "numa-distance")?)))
        .collect()
}

fn read_capacity(root: &AnchoredRoot, base: &str, id: u32) -> HostResult<Evidence<Option<u64>>> {
    let Some(text) = root.read_optional(&format!("{base}/meminfo"))? else {
        return Ok(Evidence::unknown(None));
    };
    let prefix = format!("Node {id} MemTotal:");
    let value = text
        .lines()
        .find_map(|line| line.trim().strip_prefix(&prefix));
    let Some(value) = value else {
        return Ok(Evidence::unknown(None));
    };
    let mut fields = value.split_ascii_whitespace();
    let kib = fields
        .next()
        .ok_or_else(|| LinuxHostError::new("numa-capacity", "missing value"))?;
    if fields.next() != Some("kB") || fields.next().is_some() {
        return Err(LinuxHostError::new("numa-capacity", "expected kB value"));
    }
    let bytes = number::<u64>(kib, "numa-capacity")?
        .checked_mul(1_024)
        .ok_or_else(|| LinuxHostError::new("numa-capacity", "capacity overflow"))?;
    Ok(Evidence::reported(Some(bytes), LinuxFactSource::Sysfs))
}
