//! Verification and repository-structure tooling for lkjscript.

#[path = "structure/capsule_actual.rs"]
mod capsule_actual;
#[path = "structure/graph_edges.rs"]
mod graph_edges;
mod legacy_docs;
mod legacy_run;
mod legacy_sources;
mod model;
#[path = "structure/repository.rs"]
mod repository;
#[path = "structure/repository_support.rs"]
mod repository_support;
#[cfg(test)]
#[path = "structure/repository_tests.rs"]
mod repository_tests;
mod sha256;
mod structure;
#[path = "structure/commands.rs"]
mod structure_commands;
#[path = "structure/detector.rs"]
mod structure_detector;
#[path = "structure/graph.rs"]
mod structure_graph;
#[path = "structure/query.rs"]
mod structure_query;
#[path = "structure/rules.rs"]
mod structure_rules;
#[path = "structure/validation.rs"]
mod structure_validation;
mod util;
#[cfg(test)]
#[path = "structure/validation_tests.rs"]
mod validation_tests;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let root = PathBuf::from(".");
    let code = match args.first().map(String::as_str) {
        Some("check-docs") => legacy_docs::check(&root),
        Some("check-tree") => legacy_sources::check_tree(&root),
        Some("check-sources") => legacy_sources::check_sources(&root),
        Some("quiet") => legacy_run::quiet(&root, &args[1..]),
        Some("structure") => structure::run(&root, &args[1..]),
        _ => {
            eprintln!("usage: lkjscript-xtask [check-docs|check-tree|check-sources|quiet ...|structure ...]");
            2
        }
    };
    ExitCode::from(code as u8)
}
