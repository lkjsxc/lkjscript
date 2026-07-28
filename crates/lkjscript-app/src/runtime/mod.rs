use std::collections::BTreeSet;
use std::process::ExitCode;

use lkjscript_contracts::SEMANTIC_RESOURCE_PLANE_DIGEST;
use lkjscript_sys::{discover_linux_host, ConfigValue, LinuxHostSnapshot};

mod args;
mod json;
mod json_support;
mod plan;

pub fn command(arguments: &[String]) -> Result<ExitCode, String> {
    let command = args::parse(arguments)?;
    let snapshot = discover_linux_host().map_err(|error| error.to_string())?;
    match command {
        args::RuntimeCommand::Topology { json: true, .. } => {
            println!("{}", json::topology(&snapshot));
        }
        args::RuntimeCommand::Topology {
            explain: Some(identity),
            ..
        } => explain(&snapshot, &identity)?,
        args::RuntimeCommand::Topology { .. } => print_topology(&snapshot),
        args::RuntimeCommand::HostScheduler { json: true } => {
            println!("{}", json::scheduler(&snapshot));
        }
        args::RuntimeCommand::HostScheduler { json: false } => print_scheduler(&snapshot),
        args::RuntimeCommand::Plan(options) => {
            let resource_plan = plan::build(&snapshot, &options)?;
            if options.json {
                println!("{}", json::plan(&snapshot, &resource_plan, &options.policy));
            } else {
                print_identity(&snapshot);
                plan::print_text(&resource_plan, &options.policy);
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn print_identity(snapshot: &LinuxHostSnapshot) {
    println!("contract: {SEMANTIC_RESOURCE_PLANE_DIGEST}");
    println!("snapshot: {}", hex(&snapshot.digest));
}

fn print_topology(snapshot: &LinuxHostSnapshot) {
    print_identity(snapshot);
    let cores: BTreeSet<_> = snapshot
        .cpus
        .iter()
        .filter_map(|cpu| Some((cpu.package.value?, cpu.die.value?, cpu.core.value?)))
        .collect();
    let max_cache = snapshot.caches.iter().map(|cache| cache.level).max();
    let llcs = snapshot
        .caches
        .iter()
        .filter(|cache| Some(cache.level) == max_cache)
        .count();
    println!("processing-units: {}", snapshot.cpus.len());
    println!("allowed-cpus: {:?}", snapshot.topology.allowed.as_slice());
    println!("physical-cores-known: {}", cores.len());
    println!("llc-domains: {llcs}");
    println!("numa-nodes: {}", snapshot.numa.len());
    println!("certainty: {:?}", snapshot.topology.certainty);
}

fn print_scheduler(snapshot: &LinuxHostSnapshot) {
    print_identity(snapshot);
    let scheduler = &snapshot.scheduler;
    println!(
        "kernel-release: {}",
        scheduler
            .kernel_release
            .value
            .as_deref()
            .unwrap_or("unknown")
    );
    println!(
        "process-affinity: {:?}",
        scheduler.process_affinity.value.as_slice()
    );
    println!(
        "thread-affinity: {:?}",
        scheduler.thread_affinity.value.as_slice()
    );
    println!(
        "cpuset-effective: {:?}",
        scheduler
            .cpuset_effective
            .value
            .as_ref()
            .map(lkjscript_resource::CpuSet::as_slice)
    );
    println!(
        "config-sched-cache: {} source={:?} certain={}",
        config(scheduler.sched_cache.value),
        scheduler.sched_cache.source,
        scheduler.sched_cache.certain,
    );
    println!("numa-balancing: {}", config(scheduler.numa_balancing.value));
    println!("sched-ext: {:?}", scheduler.sched_ext_state.value);
    println!(
        "active-sched-ext: {}",
        scheduler
            .active_sched_ext
            .value
            .as_deref()
            .unwrap_or("none")
    );
    println!("process-policy: {:?}", scheduler.process_policy.value);
}

fn explain(snapshot: &LinuxHostSnapshot, identity: &str) -> Result<(), String> {
    print_identity(snapshot);
    if identity == "snapshot" {
        print_topology(snapshot);
        return Ok(());
    }
    if let Some(value) = identity.strip_prefix("cpu:") {
        let id = value.parse::<u32>().map_err(|_| "invalid CPU identity")?;
        let cpu = snapshot
            .cpus
            .iter()
            .find(|cpu| cpu.cpu == id)
            .ok_or_else(|| "unknown CPU identity".to_string())?;
        println!("{cpu:#?}");
        return Ok(());
    }
    if let Some(value) = identity.strip_prefix("numa:") {
        let id = value.parse::<u32>().map_err(|_| "invalid NUMA identity")?;
        let node = snapshot
            .numa
            .iter()
            .find(|node| node.id == id)
            .ok_or_else(|| "unknown NUMA identity".to_string())?;
        println!("{node:#?}");
        return Ok(());
    }
    if let Some(value) = identity.strip_prefix("cache:") {
        let mut parts = value.split(':');
        let level = parts
            .next()
            .and_then(|value| value.parse::<u8>().ok())
            .ok_or_else(|| "invalid cache identity".to_string())?;
        let id = parts
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .filter(|_| parts.next().is_none())
            .ok_or_else(|| "invalid cache identity".to_string())?;
        let cache = snapshot
            .caches
            .iter()
            .find(|cache| cache.level == level && cache.id == id)
            .ok_or_else(|| "unknown cache identity".to_string())?;
        println!("{cache:#?}");
        return Ok(());
    }
    Err("topology identity must be snapshot, cpu:N, cache:LEVEL:ID, or numa:N".into())
}

fn config(value: ConfigValue) -> &'static str {
    match value {
        ConfigValue::Enabled => "enabled",
        ConfigValue::Disabled => "disabled",
        ConfigValue::Unknown => "unknown",
    }
}

fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
