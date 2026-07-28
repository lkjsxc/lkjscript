use crate::{ContractItem, ContractItemKind};

use super::fact;

pub(super) fn identities() -> ContractItem {
    ContractItem::new("identities", ContractItemKind::Type)
        .fact(fact("plane", "resource-plane", "content identity"))
        .fact(fact("task-class", "task-class", "stable typed identity"))
        .fact(fact("task", "task", "generation-safe runtime identity"))
        .fact(fact(
            "scope",
            "task-scope",
            "generation-safe runtime identity",
        ))
        .fact(fact(
            "result",
            "task-result",
            "generation-safe runtime identity",
        ))
        .fact(fact(
            "owner",
            "data-owner",
            "generation-safe runtime identity",
        ))
        .fact(fact(
            "access",
            "access-record",
            "generation-safe runtime identity",
        ))
        .fact(fact("worker", "worker", "generation-safe runtime identity"))
        .fact(fact(
            "group",
            "worker-group",
            "generation-safe runtime identity",
        ))
        .fact(fact(
            "domain",
            "execution-domain",
            "stable topology identity",
        ))
        .fact(fact("schedule", "schedule-plan", "content identity"))
        .fact(fact("policy", "scheduler-policy", "stable typed identity"))
}

pub(super) fn authority() -> ContractItem {
    ContractItem::new("authority-split", ContractItemKind::Rule)
        .fact(fact(
            "legality",
            "verified-task-graph",
            "dependencies accesses ownership effects capabilities cleanup scope results",
        ))
        .fact(fact(
            "placement",
            "schedule-policy",
            "worker queue affinity steal locality and home heuristics only",
        ))
        .fact(fact(
            "linux",
            "Linux boundary",
            "kernel schedules bounded session-owned workers",
        ))
        .fact(fact(
            "determinism",
            "observable determinism",
            "artifact outcome resource cleanup and failure order independent of schedule",
        ))
}

pub(super) fn accesses() -> ContractItem {
    ContractItem::new("access-modes", ContractItemKind::Variant)
        .fact(fact("read", "read", "shared immutable payload access"))
        .fact(fact(
            "write",
            "write",
            "exclusive overlapping payload access",
        ))
        .fact(fact("consume", "consume", "exclusive ownership transfer"))
        .fact(fact(
            "produce",
            "produce",
            "private until exact publication",
        ))
        .fact(fact(
            "identity-only",
            "identity-only",
            "identity observation without payload authority",
        ))
}

pub(super) fn tasks() -> ContractItem {
    ContractItem::new("verified-task-graph", ContractItemKind::Type)
        .fact(fact(
            "class",
            "task class",
            "closed static internal operation",
        ))
        .fact(fact(
            "edges",
            "dependencies",
            "acyclic exact predecessor set",
        ))
        .fact(fact(
            "data",
            "data accesses",
            "owner mode and optional disjoint range",
        ))
        .fact(fact(
            "ownership",
            "ownership",
            "move only with no live loan",
        ))
        .fact(fact("scope", "structured scope", "no detached child"))
        .fact(fact(
            "result",
            "result owner",
            "one exact publication destination",
        ))
        .fact(fact(
            "resource",
            "admission",
            "bounded pre-reserved records and journal",
        ))
        .fact(fact(
            "verification",
            "verification",
            "independent legality traversal",
        ))
}
