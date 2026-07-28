//! Verification and repository-structure tooling for lkjscript.

mod agent;
mod documentation;
mod documentation_status;
mod model;
mod sha256;
#[cfg(test)]
mod sha256_tests;
mod source_checks;
mod structure;
mod tracing_ratchet;
mod util;
mod verification;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let root = PathBuf::from(".");
    let code = match args.first().map(String::as_str) {
        Some("agent") => agent::run(&root, &args[1..]),
        Some("check-docs") => documentation::check(&root),
        Some("check-tree") => source_checks::check_tree(&root),
        Some("check-sources") => source_checks::check_sources(&root),
        Some("quiet") => verification::quiet(&root, &args[1..]),
        Some("structure") => structure::run(&root, &args[1..]),
        _ => {
            eprintln!(
                "usage: lkjscript-xtask [agent ...|check-docs|check-tree|check-sources|quiet ...|structure ...]"
            );
            2
        }
    };
    ExitCode::from(code as u8)
}
