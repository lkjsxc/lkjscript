//! Verification and repository-structure tooling for lkjscript.

mod documentation;
mod model;
mod sha256;
mod source_checks;
mod structure;
mod util;
mod verification;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    let root = PathBuf::from(".");
    let code = match args.first().map(String::as_str) {
        Some("check-docs") => documentation::check(&root),
        Some("check-tree") => source_checks::check_tree(&root),
        Some("check-sources") => source_checks::check_sources(&root),
        Some("quiet") => verification::quiet(&root, &args[1..]),
        Some("structure") => structure::run(&root, &args[1..]),
        _ => {
            eprintln!(
                "usage: lkjscript-xtask [check-docs|check-tree|check-sources|quiet ...|structure ...]"
            );
            2
        }
    };
    ExitCode::from(code as u8)
}
