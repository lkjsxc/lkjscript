use std::collections::BTreeMap;

use lkjscript_resource::{CpuSet, HardwareTopology, HostSchedulerRecord};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LinuxFactSource {
    Unknown,
    Sysfs,
    Procfs,
    Cgroup,
    BootConfig,
    KernelAbi,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evidence<T> {
    pub value: T,
    pub source: LinuxFactSource,
    pub certain: bool,
}

impl<T> Evidence<T> {
    pub(crate) fn unknown(value: T) -> Self {
        Self {
            value,
            source: LinuxFactSource::Unknown,
            certain: false,
        }
    }

    pub(crate) fn reported(value: T, source: LinuxFactSource) -> Self {
        Self {
            value,
            source,
            certain: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfigValue {
    Enabled,
    Disabled,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedExtState {
    Enabled,
    Disabled,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerPolicy {
    Other,
    Fifo,
    RoundRobin,
    Batch,
    Idle,
    Deadline,
    Ext,
    Unknown(i32),
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum CacheKind {
    Data,
    Instruction,
    Unified,
    Other,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxCpuObservation {
    pub cpu: u32,
    pub online: bool,
    pub package: Evidence<Option<u32>>,
    pub die: Evidence<Option<u32>>,
    pub core: Evidence<Option<u32>>,
    pub smt_siblings: CpuSet,
    pub llc_domain: Option<u32>,
    pub numa_node: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxCacheObservation {
    pub id: u32,
    pub level: u8,
    pub kind: CacheKind,
    pub size_bytes: Evidence<Option<u64>>,
    pub line_bytes: Evidence<Option<u32>>,
    pub associativity: Evidence<Option<u32>>,
    pub shared_cpus: CpuSet,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxNumaObservation {
    pub id: u32,
    pub cpus: CpuSet,
    pub capacity_bytes: Evidence<Option<u64>>,
    pub distances: BTreeMap<u32, u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostSchedulerObservation {
    pub kernel_release: Evidence<Option<String>>,
    pub process_affinity: Evidence<CpuSet>,
    pub thread_affinity: Evidence<CpuSet>,
    pub cpuset_effective: Evidence<Option<CpuSet>>,
    pub sched_cache: Evidence<ConfigValue>,
    pub scheduler_debug: Evidence<Option<String>>,
    pub numa_balancing: Evidence<ConfigValue>,
    pub sched_ext_state: Evidence<SchedExtState>,
    pub active_sched_ext: Evidence<Option<String>>,
    pub process_policy: Evidence<SchedulerPolicy>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LinuxHostSnapshot {
    pub topology: HardwareTopology,
    pub cpus: Vec<LinuxCpuObservation>,
    pub caches: Vec<LinuxCacheObservation>,
    pub numa: Vec<LinuxNumaObservation>,
    pub scheduler: HostSchedulerObservation,
    pub digest: [u8; 32],
    pub(crate) quota_workers: Option<usize>,
}

impl LinuxHostSnapshot {
    pub fn scheduler_record(&self) -> HostSchedulerRecord {
        super::discover::scheduler_record(self)
    }
}
