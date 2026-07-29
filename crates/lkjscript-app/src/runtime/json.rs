use std::fmt::Write;

use lkjscript_linux_host::LinuxHostSnapshot;
use lkjscript_resource::ExecutionResourcePlan;

use super::json_support::*;

pub(super) fn topology(snapshot: &LinuxHostSnapshot) -> String {
    let mut output = header("lkjscript.runtime-topology", snapshot);
    let _ = write!(
        output,
        ",\"allowed_cpus\":{},\"topology_certain\":{},\"cpus\":[",
        cpu_set(&snapshot.topology.allowed),
        snapshot.topology.certainty != lkjscript_resource::FactCertainty::Unknown
    );
    for (index, cpu) in snapshot.cpus.iter().enumerate() {
        comma(&mut output, index);
        let _ = write!(
            output,
            "{{\"cpu\":{},\"online\":{},\"package\":{},\"die\":{},\"core\":{},\"smt\":{},\"llc\":{},\"numa\":{}}}",
            cpu.cpu,
            cpu.online,
            optional_u32(cpu.package.value),
            optional_u32(cpu.die.value),
            optional_u32(cpu.core.value),
            cpu_set(&cpu.smt_siblings),
            optional_u32(cpu.llc_domain),
            optional_u32(cpu.numa_node),
        );
    }
    output.push_str("],\"caches\":[");
    for (index, cache) in snapshot.caches.iter().enumerate() {
        comma(&mut output, index);
        let _ = write!(
            output,
            concat!(
                "{{\"id\":{},\"level\":{},\"kind\":\"{:?}\",",
                "\"size_bytes\":{},\"line_bytes\":{},\"associativity\":{},",
                "\"shared_cpus\":{}}}"
            ),
            cache.id,
            cache.level,
            cache.kind,
            optional_u64(cache.size_bytes.value),
            optional_u32(cache.line_bytes.value),
            optional_u32(cache.associativity.value),
            cpu_set(&cache.shared_cpus),
        );
    }
    output.push_str("],\"numa_nodes\":[");
    for (index, node) in snapshot.numa.iter().enumerate() {
        comma(&mut output, index);
        let distances = node
            .distances
            .iter()
            .map(|(id, distance)| format!("[{},{}]", id, distance))
            .collect::<Vec<_>>()
            .join(",");
        let _ = write!(
            output,
            "{{\"id\":{},\"cpus\":{},\"capacity_bytes\":{},\"distances\":[{}]}}",
            node.id,
            cpu_set(&node.cpus),
            optional_u64(node.capacity_bytes.value),
            distances,
        );
    }
    output.push_str("]}");
    output
}

pub(super) fn scheduler(snapshot: &LinuxHostSnapshot) -> String {
    let value = &snapshot.scheduler;
    let mut output = header("lkjscript.host-scheduler", snapshot);
    let _ = write!(
        output,
        concat!(
            ",\"kernel_release\":{},\"process_affinity\":{},",
            "\"thread_affinity\":{},\"cpuset_effective\":{},",
            "\"config_sched_cache\":\"{}\",\"scheduler_debug\":{},",
            "\"numa_balancing\":\"{}\",\"sched_ext_state\":\"{}\",",
            "\"active_sched_ext\":{},\"process_policy\":\"{:?}\",",
            "\"sources\":{{\"kernel\":\"{}\",\"sched_cache\":\"{}\"}},",
            "\"certainty\":{{\"kernel\":{},\"sched_cache\":{}}}}}"
        ),
        optional_string(value.kernel_release.value.as_deref()),
        cpu_set(&value.process_affinity.value),
        cpu_set(&value.thread_affinity.value),
        value
            .cpuset_effective
            .value
            .as_ref()
            .map_or_else(|| "null".into(), cpu_set),
        config(value.sched_cache.value),
        optional_string(value.scheduler_debug.value.as_deref()),
        config(value.numa_balancing.value),
        sched_ext(value.sched_ext_state.value),
        optional_string(value.active_sched_ext.value.as_deref()),
        value.process_policy.value,
        source(value.kernel_release.source),
        source(value.sched_cache.source),
        value.kernel_release.certain,
        value.sched_cache.certain,
    );
    output
}

pub(super) fn plan(
    snapshot: &LinuxHostSnapshot,
    plan: &ExecutionResourcePlan,
    policy: &str,
) -> String {
    let mut output = header("lkjscript.execution-resource-plan", snapshot);
    let _ = write!(
        output,
        ",\"policy\":{},\"affinity\":\"{}\",\"sequential_fallback\":{},\"workers\":[",
        quote(policy),
        placement(plan.mode),
        plan.sequential_fallback,
    );
    for (index, worker) in plan.workers.iter().enumerate() {
        comma(&mut output, index);
        let _ = write!(
            output,
            concat!(
                "{{\"id\":{},\"generation\":{},\"group\":{},\"mask\":{},",
                "\"elastic_locality\":{},\"queue_bytes\":{},\"scratch_bytes\":{}}}"
            ),
            worker.worker.slot,
            worker.worker.generation,
            worker.group.slot,
            cpu_set(&worker.exact_mask),
            cpu_set(&worker.elastic_locality),
            worker.queue_bytes,
            worker.scratch_bytes,
        );
    }
    output.push_str("]}");
    output
}
