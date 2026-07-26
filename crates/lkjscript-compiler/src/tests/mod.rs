#![allow(clippy::panic, clippy::unwrap_used)]

use std::path::Path;

use lkjscript_core::{Constant, Limits};

use super::{ensure_source_path, validate_source};

const EDITION2: &str = "edition/\n2\n/edition\n";

fn edition2(source: &str) -> String {
    if source.starts_with(EDITION2) {
        source.to_string()
    } else {
        format!("{EDITION2}{source}")
    }
}

fn compile_source(
    source: &str,
    path: &str,
    limits: &Limits,
) -> lkjscript_core::Result<crate::ExecutableProgram> {
    super::compile_source(&edition2(source), path, limits)
}

fn compile_source_with_profile(
    source: &str,
    path: &str,
    limits: &Limits,
    profile: crate::ResourceProfile,
) -> lkjscript_core::Result<crate::ExecutableProgram> {
    super::compile_source_with_profile(&edition2(source), path, limits, profile)
}

fn unit_main(body: &str) -> String {
    format!("main/\nsig/\n->\nUnit\n/sig\ndo/\n{body}\nunit\n/do\n/main\n")
}

fn ownership_source(body: &str, result: &str) -> String {
    let result = result.replace(' ', "\n");
    format!("main/\nsig/\n->\n{result}\n/sig\n{body}\n/main\n")
}

mod constants;
mod ledger_hir;
mod ledger_phases;
mod match_nested;
mod match_resources;
mod matches;
mod numeric;
mod operations;
mod ownership_boundaries;
mod ownership_control;
mod ownership_flow;
mod paths;
mod resources;
