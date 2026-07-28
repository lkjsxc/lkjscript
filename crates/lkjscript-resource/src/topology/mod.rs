use std::collections::{BTreeMap, BTreeSet};

use crate::{CpuSet, ResourceError, ResourceResult};

mod host;
pub use host::{HostSchedulerRecord, ObservedFact};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactCertainty {
    Unknown,
    Reported,
    Observed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FactSource {
    Unknown,
    LinuxSysfs,
    LinuxProcfs,
    Request,
    Synthetic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessingUnit {
    pub cpu: u32,
    pub package: u32,
    pub die: u32,
    pub core: u32,
    pub online: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CacheDomain {
    pub id: u32,
    pub level: u8,
    pub cpus: CpuSet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumaNode {
    pub id: u32,
    pub cpus: CpuSet,
    pub distances: BTreeMap<u32, u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardwareTopology {
    pub units: Vec<ProcessingUnit>,
    pub caches: Vec<CacheDomain>,
    pub numa_nodes: Vec<NumaNode>,
    pub allowed: CpuSet,
    pub source: FactSource,
    pub certainty: FactCertainty,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Locality {
    SameCpu,
    SmtSibling,
    SameLastLevelCache,
    SameNumaNode,
    SamePackage,
    Remote,
}

impl HardwareTopology {
    pub fn validate(&self) -> ResourceResult<()> {
        if self.allowed.is_empty() {
            return Err(ResourceError::new(
                "topology-empty",
                "allowed CPU set is empty",
            ));
        }
        let mut known = BTreeSet::new();
        for unit in &self.units {
            if !known.insert(unit.cpu) {
                return Err(ResourceError::new(
                    "topology-duplicate",
                    "duplicate processing unit",
                ));
            }
        }
        for cpu in self.allowed.as_slice() {
            let unit = self.unit(*cpu).ok_or_else(|| {
                ResourceError::new("topology-unknown", "allowed set contains unknown CPU")
            })?;
            if !unit.online {
                return Err(ResourceError::new(
                    "topology-offline",
                    "offline CPU selected",
                ));
            }
        }
        let mut cache_keys = BTreeSet::new();
        for cache in &self.caches {
            if cache.level == 0 || !cache_keys.insert((cache.level, cache.id)) {
                return Err(ResourceError::new(
                    "cache-duplicate",
                    "duplicate or invalid cache",
                ));
            }
            if cache.cpus.as_slice().iter().any(|cpu| !known.contains(cpu)) {
                return Err(ResourceError::new(
                    "cache-unknown",
                    "cache contains unknown CPU",
                ));
            }
        }
        let mut assigned = BTreeSet::new();
        let node_ids: BTreeSet<u32> = self.numa_nodes.iter().map(|node| node.id).collect();
        if node_ids.len() != self.numa_nodes.len() {
            return Err(ResourceError::new("numa-duplicate", "duplicate NUMA node"));
        }
        for node in &self.numa_nodes {
            for cpu in node.cpus.as_slice() {
                if !known.contains(cpu) || !assigned.insert(*cpu) {
                    return Err(ResourceError::new(
                        "numa-membership",
                        "invalid NUMA membership",
                    ));
                }
            }
            if node.distances.keys().any(|id| !node_ids.contains(id)) {
                return Err(ResourceError::new(
                    "numa-distance",
                    "distance names unknown node",
                ));
            }
        }
        Ok(())
    }

    pub fn unit(&self, cpu: u32) -> Option<&ProcessingUnit> {
        self.units.iter().find(|unit| unit.cpu == cpu)
    }

    pub fn locality(&self, left: u32, right: u32) -> ResourceResult<Locality> {
        let a = self
            .unit(left)
            .ok_or_else(|| ResourceError::new("cpu-unknown", "left CPU"))?;
        let b = self
            .unit(right)
            .ok_or_else(|| ResourceError::new("cpu-unknown", "right CPU"))?;
        if left == right {
            return Ok(Locality::SameCpu);
        }
        if (a.package, a.die, a.core) == (b.package, b.die, b.core) {
            return Ok(Locality::SmtSibling);
        }
        let max_level = self.caches.iter().map(|cache| cache.level).max();
        if self.caches.iter().any(|cache| {
            Some(cache.level) == max_level
                && cache.cpus.contains(left)
                && cache.cpus.contains(right)
        }) {
            return Ok(Locality::SameLastLevelCache);
        }
        if self
            .numa_nodes
            .iter()
            .any(|node| node.cpus.contains(left) && node.cpus.contains(right))
        {
            return Ok(Locality::SameNumaNode);
        }
        if a.package == b.package {
            return Ok(Locality::SamePackage);
        }
        Ok(Locality::Remote)
    }

    pub fn smt_siblings(&self, cpu: u32) -> ResourceResult<CpuSet> {
        let unit = self
            .unit(cpu)
            .ok_or_else(|| ResourceError::new("cpu-unknown", "CPU"))?;
        CpuSet::new(
            self.units
                .iter()
                .filter(|other| {
                    (other.package, other.die, other.core) == (unit.package, unit.die, unit.core)
                })
                .map(|other| other.cpu),
        )
    }
}
