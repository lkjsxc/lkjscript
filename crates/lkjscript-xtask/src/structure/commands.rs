use std::fs;
use std::path::Path;

use crate::model::{Audit, Policy};

pub fn graph(root: &Path, audit: &Audit, policy: &Policy, flag: Option<&str>) -> i32 {
    if !matches!(flag, None | Some("--json") | Some("--dot")) {
        eprintln!("usage: structure graph [--json|--dot]");
        return 2;
    }
    let graph = crate::structure_graph::build(audit, policy);
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
    let dot = crate::structure_graph::dot(&graph);
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

pub fn query(command: &str, audit: &Audit, policy: &Policy, args: &[String]) -> i32 {
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
    let graph = crate::structure_graph::build(audit, policy);
    let result = crate::structure_query::run(command, target, profile, &graph, policy);
    if let Err(error) = crate::util::print_json(&result) {
        eprintln!("{error}");
        1
    } else {
        0
    }
}
