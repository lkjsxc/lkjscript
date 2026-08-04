use std::fs;

use super::super::{graph, query};
use super::support::{fixture, policy, root};

#[test]
fn graph_exhaustion_and_query_completion_are_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let root = root();
    let audit = fixture(&root, &"d".repeat(40));
    let error = match graph::build(&root, &audit, &policy(4, 4000, 100_000, 1_000_000)) {
        Err(error) => error,
        Ok(_) => {
            return Err(std::io::Error::other("node exhaustion published a complete graph").into())
        }
    };
    assert_eq!(error.dimension, "nodes");
    assert!(error.attempted > error.limit);
    let full = graph::build(&root, &audit, &policy(1000, 4000, 100_000, 1_000_000))?;
    let exact_failures = [
        (
            "nodes",
            policy(
                u64::try_from(full.nodes.len())
                    .unwrap_or(u64::MAX)
                    .saturating_sub(1),
                4000,
                100_000,
                1_000_000,
            ),
        ),
        (
            "edges",
            policy(
                1000,
                u64::try_from(full.edges.len())
                    .unwrap_or(u64::MAX)
                    .saturating_sub(1),
                100_000,
                1_000_000,
            ),
        ),
        (
            "work",
            policy(1000, 4000, full.work_used.saturating_sub(1), 1_000_000),
        ),
        (
            "retained-bytes",
            policy(1000, 4000, 100_000, full.bytes_used.saturating_sub(1)),
        ),
    ];
    for (dimension, constrained) in exact_failures {
        assert_eq!(
            graph::build(&root, &audit, &constrained)
                .err()
                .as_ref()
                .map(|error| error.dimension.as_str()),
            Some(dimension)
        );
    }
    let limits = policy(1000, 4000, 100_000, 1_000_000);
    let adjacency_work = u64::try_from(full.edges.len()).unwrap_or(u64::MAX);
    let mut node_limited = limits.clone();
    node_limited.limits.query_work = adjacency_work;
    let node_frontier = query::run("impact", "capsule:x", None, &full, &node_limited);
    assert_eq!(
        node_frontier.completion.omitted_frontier,
        ["capsule:x".to_owned()]
    );
    let mut expected_neighbors = full
        .edges
        .iter()
        .filter(|edge| edge.to == "capsule:x")
        .map(|edge| edge.from.clone())
        .collect::<Vec<_>>();
    expected_neighbors.sort();
    expected_neighbors.dedup();
    assert!(!expected_neighbors.is_empty());
    let mut edge_limited = limits.clone();
    edge_limited.limits.query_work = adjacency_work.saturating_add(1);
    let edge_frontier = query::run("impact", "capsule:x", None, &full, &edge_limited);
    assert_eq!(
        edge_frontier.completion.omitted_frontier,
        expected_neighbors
    );
    let context = query::run("context", "x", Some("strong"), &full, &limits);
    let serialized = serde_json::to_string_pretty(&context).unwrap_or_default();
    assert!(u64::try_from(serialized.len()).unwrap_or(u64::MAX) <= limits.limits.query_bytes);
    assert_eq!(
        context
            .sections
            .first()
            .map(|section| section.name.as_str()),
        Some("goal")
    );
    assert_eq!(
        context.sections.last().map(|section| section.name.as_str()),
        Some("omissions")
    );
    let mut tight = limits.clone();
    tight.limits.query_bytes = 2_048;
    let bounded = query::run("context", "x", Some("strong"), &full, &tight);
    assert!(bounded.nodes.len() > 1);
    assert_eq!(bounded.completion.status, "bounded");
    assert!(!bounded.completion.stop_reasons.is_empty());
    assert!(!bounded.completion.continuation_supported);
    assert!(bounded.edges.iter().all(|edge| {
        bounded.nodes.iter().any(|node| node.id == edge.from)
            && bounded.nodes.iter().any(|node| node.id == edge.to)
    }));
    let mut no_work = limits.clone();
    no_work.limits.query_work = 0;
    let work_bounded = query::run("context", "x", Some("strong"), &full, &no_work);
    assert_eq!(work_bounded.completion.status, "bounded");
    assert!(work_bounded
        .completion
        .stop_reasons
        .iter()
        .any(|reason| reason == "work-budget"));
    assert!(!work_bounded.completion.omitted_frontier.is_empty());
    let impact = query::run("impact", "core", None, &full, &limits);
    assert!(impact.nodes.iter().any(|node| node.id == "capsule:x"));
    let tests = query::run("tests", "x", None, &full, &limits);
    assert!(tests.nodes.iter().any(|node| node.kind == "test"));
    fs::remove_file(root.join("docs/decision.md"))?;
    assert_eq!(
        graph::build(&root, &audit, &limits)
            .err()
            .as_ref()
            .map(|error| error.dimension.as_str()),
        Some("source-read")
    );
    fs::remove_dir_all(root)?;
    Ok(())
}
