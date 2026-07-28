use crate::{ContractItem, ContractItemKind};

use super::fact;

pub(super) fn topology() -> ContractItem {
    ContractItem::new("hardware-topology", ContractItemKind::Type)
        .fact(fact(
            "nodes",
            "nodes",
            "machine package die chiplet NUMA LLC cache core processing-unit memory-node",
        ))
        .fact(fact(
            "relations",
            "locality",
            "SMT core private-cache L2 LLC chiplet NUMA package remote-NUMA",
        ))
        .fact(fact(
            "constraints",
            "process constraints",
            "online affinity cpuset and memory-node masks",
        ))
        .fact(fact(
            "observation",
            "host scheduler",
            "kernel affinity cpuset sched-cache NUMA and sched-ext facts with certainty",
        ))
        .fact(fact(
            "unknown",
            "unknown facts",
            "preserved and never invented",
        ))
}

pub(super) fn plan() -> ContractItem {
    ContractItem::new("execution-resource-plan", ContractItemKind::Type)
        .fact(fact(
            "workers",
            "workers",
            "physical-core-first bounded selection",
        ))
        .fact(fact(
            "groups",
            "worker groups",
            "LLC or closest reliable domain",
        ))
        .fact(fact("queues", "queues", "bounded exact capacity"))
        .fact(fact(
            "scratch",
            "scratch",
            "bounded worker-local first-touch storage",
        ))
        .fact(fact(
            "affinity",
            "affinity",
            "kernel-managed cpu-pinned or llc-domain-masked with readback",
        ))
        .fact(fact(
            "elastic",
            "elastic locality",
            "soft compact demand-sized bounded worker set",
        ))
}

pub(super) fn policies() -> ContractItem {
    let mut item =
        ContractItem::new("schedule-policies", ContractItemKind::Variant).semantic_order();
    for (id, name_value) in [
        ("sequential", "sequential"),
        ("static-partition", "static-partition"),
        ("global-fifo", "global-fifo"),
        ("local-stealing", "local-work-stealing"),
        ("hierarchical", "hierarchical-locality"),
        ("owner-compute", "owner-compute"),
    ] {
        item.facts
            .push(fact(id, name_value, "complete typed policy"));
    }
    item
}

pub(super) fn runtime() -> ContractItem {
    ContractItem::new("worker-runtime", ContractItemKind::Type)
        .fact(fact(
            "lifetime",
            "lifetime",
            "session-owned scoped joined workers",
        ))
        .fact(fact("queue", "queue", "bounded lock-protected local deque"))
        .fact(fact("wake", "wakeup", "bounded spin then park"))
        .fact(fact(
            "states",
            "task states",
            "closed checked transition machine",
        ))
        .fact(fact(
            "failure",
            "failure",
            "stable primary and bounded attachments",
        ))
        .fact(fact(
            "reference",
            "reference scheduler",
            "single-thread trace and replay",
        ))
        .fact(fact(
            "shutdown",
            "shutdown",
            "zero live task result owner and release state",
        ))
}

pub(super) fn memory_homes() -> ContractItem {
    ContractItem::new("memory-homes", ContractItemKind::Type)
        .fact(fact(
            "home",
            "owner home",
            "worker group LLC chiplet and NUMA facts",
        ))
        .fact(fact(
            "transfer",
            "owner transfer",
            "boundary move with no-live-loan proof",
        ))
        .fact(fact(
            "release",
            "remote release",
            "bounded exactly-once home release",
        ))
        .fact(fact("heap", "control storage", "never allocated in GcHeap"))
}

pub(super) fn metrics() -> ContractItem {
    ContractItem::new("scheduler-metrics", ContractItemKind::Type)
        .fact(fact(
            "tasks",
            "task lifecycle",
            "created admitted executed outcomes peaks",
        ))
        .fact(fact(
            "schedule",
            "scheduling",
            "locality steals migrations parks wakeups",
        ))
        .fact(fact(
            "memory",
            "memory locality",
            "home hits transfers releases copied bytes",
        ))
        .fact(fact(
            "overhead",
            "overhead",
            "graph verification queue and scheduler work",
        ))
        .fact(fact(
            "availability",
            "optional host facts",
            "source certainty or unavailable",
        ))
}
