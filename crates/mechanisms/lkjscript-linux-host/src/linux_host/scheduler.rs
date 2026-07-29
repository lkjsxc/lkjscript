use super::affinity::{current_process_affinity, current_thread_affinity};
use super::error::{HostResult, LinuxHostError};
use super::linux_abi;
use super::parse::{named_cpu_sets, number};
use super::root::AnchoredRoot;
use super::scheduler_controls::{config_sched_cache, discover_cgroup};
use super::types::{
    ConfigValue, Evidence, HostSchedulerObservation, LinuxFactSource, SchedExtState,
    SchedulerPolicy,
};

pub(crate) struct SchedulerDiscovery {
    pub observation: HostSchedulerObservation,
    pub quota_workers: Option<usize>,
}

pub(crate) fn discover_scheduler(root: &AnchoredRoot) -> HostResult<SchedulerDiscovery> {
    let process_status = root.read("proc/self/status")?;
    let (status_process, _) = named_cpu_sets(&process_status)?;
    let thread_status = root
        .read_optional("proc/thread-self/status")?
        .unwrap_or_else(|| process_status.clone());
    let (status_thread, _) = named_cpu_sets(&thread_status)?;
    let (process, thread, policy, affinity_source) = if root.is_fixture() {
        (
            status_process,
            status_thread,
            policy_from_proc(root)?,
            LinuxFactSource::Procfs,
        )
    } else {
        let process = current_process_affinity()?;
        let thread = current_thread_affinity()?;
        if process != status_process || thread != status_thread {
            return Err(LinuxHostError::new(
                "affinity-mismatch",
                "kernel ABI and proc affinity differ",
            ));
        }
        (
            process,
            thread,
            policy_from_abi()?,
            LinuxFactSource::KernelAbi,
        )
    };
    let (cpuset, quota_workers) = discover_cgroup(root)?;
    let release = root.read_optional("proc/sys/kernel/osrelease")?;
    let sched_cache = config_sched_cache(root, release.as_deref())?;
    Ok(SchedulerDiscovery {
        observation: HostSchedulerObservation {
            kernel_release: optional_fact(release, LinuxFactSource::Procfs),
            process_affinity: Evidence::reported(process, affinity_source),
            thread_affinity: Evidence::reported(thread, affinity_source),
            cpuset_effective: optional_fact(cpuset, LinuxFactSource::Cgroup),
            sched_cache,
            scheduler_debug: optional_fact(
                root.read_optional("sys/kernel/debug/sched/features")?,
                LinuxFactSource::Sysfs,
            ),
            numa_balancing: switch_fact(
                root.read_optional("proc/sys/kernel/numa_balancing")?,
                LinuxFactSource::Procfs,
            )?,
            sched_ext_state: sched_ext_fact(root.read_optional("sys/kernel/sched_ext/state")?),
            active_sched_ext: optional_fact(
                root.read_optional("sys/kernel/sched_ext/root/ops")?,
                LinuxFactSource::Sysfs,
            ),
            process_policy: Evidence::reported(policy, affinity_source),
        },
        quota_workers,
    })
}

fn switch_fact(
    value: Option<String>,
    source: LinuxFactSource,
) -> HostResult<Evidence<ConfigValue>> {
    match value.as_deref() {
        Some("1") => Ok(Evidence::reported(ConfigValue::Enabled, source)),
        Some("0") => Ok(Evidence::reported(ConfigValue::Disabled, source)),
        Some(_) => Err(LinuxHostError::new("kernel-switch", "expected zero or one")),
        None => Ok(Evidence::unknown(ConfigValue::Unknown)),
    }
}

fn sched_ext_fact(value: Option<String>) -> Evidence<SchedExtState> {
    match value.as_deref() {
        Some("enabled") => Evidence::reported(SchedExtState::Enabled, LinuxFactSource::Sysfs),
        Some("disabled") => Evidence::reported(SchedExtState::Disabled, LinuxFactSource::Sysfs),
        _ => Evidence::unknown(SchedExtState::Unknown),
    }
}

fn policy_from_abi() -> HostResult<SchedulerPolicy> {
    linux_abi::scheduler(linux_abi::process_id())
        .map(policy)
        .map_err(|errno| LinuxHostError::new("scheduler-policy", format!("errno {errno}")))
}

fn policy_from_proc(root: &AnchoredRoot) -> HostResult<SchedulerPolicy> {
    let text = root.read("proc/self/sched")?;
    let value = text
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("policy")
                .and_then(|rest| rest.trim().strip_prefix(':'))
        })
        .ok_or_else(|| LinuxHostError::new("scheduler-policy", "missing policy"))?;
    Ok(policy(number(value, "scheduler-policy")?))
}

fn policy(value: i32) -> SchedulerPolicy {
    match value {
        0 => SchedulerPolicy::Other,
        1 => SchedulerPolicy::Fifo,
        2 => SchedulerPolicy::RoundRobin,
        3 => SchedulerPolicy::Batch,
        5 => SchedulerPolicy::Idle,
        6 => SchedulerPolicy::Deadline,
        7 => SchedulerPolicy::Ext,
        other => SchedulerPolicy::Unknown(other),
    }
}

fn optional_fact<T>(value: Option<T>, source: LinuxFactSource) -> Evidence<Option<T>> {
    if value.is_some() {
        Evidence::reported(value, source)
    } else {
        Evidence::unknown(None)
    }
}
