use crate::model::{Edge, Node};
use crate::public_facts::{Authority, LocatedFact, Registry};

use super::{declared_node, edge};
use crate::structure::graph::Budget;

pub fn add(
    registry: Option<&Registry>,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    budget: &mut Budget,
) {
    let Some(registry) = registry else {
        return;
    };
    for located in registry.facts.values() {
        add_fact(located, nodes, edges, budget);
        if budget.truncated {
            break;
        }
    }
}

fn add_fact(
    located: &LocatedFact,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    budget: &mut Budget,
) {
    let fact = &located.fact;
    let fact_id = format!("fact:{}", fact.id);
    let authority = format!("meta/config/public-facts/{}", located.shard);
    add_node(
        nodes,
        &fact_id,
        "public-fact",
        &fact.interface,
        &authority,
        budget,
    );
    add_file_edge(
        &fact_id,
        &format!("file:{authority}"),
        "defined-by",
        located,
        edges,
        budget,
    );
    let status_id = format!("fact-status:{}", fact.status.name());
    add_node(
        nodes,
        &status_id,
        "fact-status",
        fact.status.name(),
        "docs/operations/status-authority.md",
        budget,
    );
    add_file_edge(&fact_id, &status_id, "has-status", located, edges, budget);
    let interface_id = format!("fact-interface:{}", fact.id);
    add_node(
        nodes,
        &interface_id,
        "fact-interface",
        &fact.interface,
        &authority,
        budget,
    );
    add_file_edge(&fact_id, &interface_id, "exposes", located, edges, budget);
    for exclusion in &fact.exclusions {
        let id = format!("fact-exclusion:{}:{}", fact.id, exclusion.id);
        add_node(
            nodes,
            &id,
            "fact-exclusion",
            &exclusion.interface,
            &authority,
            budget,
        );
        add_file_edge(&fact_id, &id, "excludes", located, edges, budget);
    }
    match &fact.authority {
        Authority::RepositoryPath { path } => {
            add_file_edge(
                &fact_id,
                &format!("file:{path}"),
                "specified-by",
                located,
                edges,
                budget,
            );
        }
        Authority::MachineSource { source } => {
            let id = format!("machine-source:{}", source.name());
            add_node(
                nodes,
                &id,
                "machine-source",
                source.name(),
                source.name(),
                budget,
            );
            add_file_edge(&fact_id, &id, "derived-from", located, edges, budget);
        }
    }
    for path in &fact.implementation_anchors {
        add_file_edge(
            &fact_id,
            &format!("file:{path}"),
            "implemented-by",
            located,
            edges,
            budget,
        );
    }
    for (index, evidence) in fact.evidence.iter().enumerate() {
        super::public_fact_evidence::add(&fact_id, index, evidence, located, nodes, edges, budget);
    }
    for projection in &fact.projections {
        add_file_edge(
            &format!("file:{projection}"),
            &fact_id,
            "projects",
            located,
            edges,
            budget,
        );
    }
    for dependency in &fact.dependencies {
        add_file_edge(
            &fact_id,
            &format!("fact:{dependency}"),
            "depends-on",
            located,
            edges,
            budget,
        );
    }
    for invalidating in &fact.invalidated_by {
        add_file_edge(
            &fact_id,
            &format!("fact:{invalidating}"),
            "invalidated-by",
            located,
            edges,
            budget,
        );
    }
}

pub(super) fn add_node(
    nodes: &mut Vec<Node>,
    id: &str,
    kind: &str,
    label: &str,
    authority: &str,
    budget: &mut Budget,
) {
    let bytes = id
        .len()
        .checked_add(label.len())
        .and_then(|value| value.checked_add(authority.len()));
    if bytes.is_some_and(|bytes| budget.charge(1, bytes as u64)) {
        declared_node(nodes, id, kind, label, authority);
    }
}

pub(super) fn add_file_edge(
    from: &str,
    to: &str,
    kind: &str,
    located: &LocatedFact,
    edges: &mut Vec<Edge>,
    budget: &mut Budget,
) {
    let bytes = from
        .len()
        .checked_add(to.len())
        .and_then(|value| value.checked_add(kind.len()))
        .and_then(|value| value.checked_add(located.digest.len()));
    if bytes.is_some_and(|bytes| budget.charge(1, bytes as u64)) {
        edge(edges, from, to, kind, &located.digest, "declared");
    }
}
