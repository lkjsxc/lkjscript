use std::fs;
use std::path::Path;

use crate::model::{Audit, Policy, ProvenanceFile};
use crate::public_facts::Registry;

pub fn graph(
    root: &Path,
    audit: &Audit,
    policy: &Policy,
    registry: Option<&Registry>,
    flag: Option<&str>,
) -> i32 {
    if !matches!(flag, None | Some("--json") | Some("--dot")) {
        eprintln!("usage: structure graph [--json|--dot]");
        return 2;
    }
    let Some(registry) = registry else {
        eprintln!("public facts unavailable");
        return 1;
    };
    let graph = super::graph::build_with_facts(root, audit, policy, registry);
    let output = root.join("target/lkjscript/structure");
    if let Err(error) = fs::create_dir_all(&output) {
        eprintln!("create {}: {error}", output.display());
        return 1;
    }
    let json = match serde_json::to_string_pretty(&graph) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("serialize graph: {error}");
            return 1;
        }
    };
    let dot = super::graph::dot(&graph);
    if let Err(error) = fs::write(output.join("graph.json"), format!("{json}\n"))
        .and_then(|()| fs::write(output.join("graph.dot"), &dot))
    {
        eprintln!("write generated graph: {error}");
        return 1;
    }
    match flag {
        Some("--json") => println!("{json}"),
        Some("--dot") => print!("{dot}"),
        _ => println!(
            "generated {} nodes and {} edges",
            graph.nodes.len(),
            graph.edges.len()
        ),
    }
    0
}

pub(crate) fn agent_context(
    root: &Path,
    targets: &[String],
    profile: &str,
) -> Result<Vec<crate::model::QueryResult>, String> {
    let policy: Policy = super::repository::load_json(&root.join(super::INPUTS.0))?;
    let provenance: ProvenanceFile = super::repository::load_json(&root.join(super::INPUTS.1))?;
    if !super::current_structure_contract(&policy, &provenance) {
        return Err("structure policy or provenance contract mismatch".into());
    }
    let snapshot = super::repository::capture(root, &provenance.entries)?;
    let audit = super::rules::audit(root, &policy, provenance.entries, snapshot);
    let registry = crate::public_facts::load(root)
        .map_err(|error| format!("public facts unavailable: {error}"))?;
    let graph = super::graph::build_with_facts(root, &audit, &policy, &registry);
    let results: Vec<_> = targets
        .iter()
        .map(|target| super::query::run("context", target, Some(profile), &graph, &policy))
        .collect();
    let output_limit = if profile == "weak" {
        policy.limits.query_bytes.min(32_768)
    } else {
        policy.limits.query_bytes
    };
    for result in &results {
        if crate::util::json_pretty_len(result)? > output_limit {
            return Err("serialized repository context exceeds query byte limit".into());
        }
    }
    Ok(results)
}

pub fn query(
    command: &str,
    root: &Path,
    audit: &Audit,
    policy: &Policy,
    registry: Option<&Registry>,
    args: &[String],
) -> i32 {
    let Some(target) = args.first() else {
        eprintln!("usage: structure {command} <target>");
        return 2;
    };
    let profile = if command == "context" {
        match args.get(1).map(String::as_str) {
            None => Some("weak"),
            Some("--profile") => match args.get(2).map(String::as_str) {
                Some(value @ ("weak" | "strong")) if args.len() == 3 => Some(value),
                _ => {
                    eprintln!("profile must be weak or strong");
                    return 2;
                }
            },
            _ => {
                eprintln!("usage: structure context <target> [--profile weak|strong]");
                return 2;
            }
        }
    } else if args.len() == 1 {
        None
    } else {
        eprintln!("usage: structure {command} <target>");
        return 2;
    };
    let Some(registry) = registry else {
        eprintln!("public facts unavailable");
        return 1;
    };
    let graph = super::graph::build_with_facts(root, audit, policy, registry);
    let result = super::query::run(command, target, profile, &graph, policy);
    let output_limit = if profile == Some("weak") {
        policy.limits.query_bytes.min(32_768)
    } else {
        policy.limits.query_bytes
    };
    if let Err(error) = crate::util::print_json_bounded(&result, output_limit) {
        eprintln!("{error}");
        1
    } else {
        0
    }
}
