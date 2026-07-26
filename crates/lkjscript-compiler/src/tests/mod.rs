#![allow(clippy::panic, clippy::unwrap_used)]

use std::path::Path;

use lkjscript_core::{Constant, Limits};

use super::{ensure_source_path, validate_source};

fn canonical_source(source: &str) -> String {
    source.to_string()
}

fn compile_source(
    source: &str,
    path: &str,
    limits: &Limits,
) -> lkjscript_core::Result<crate::ExecutableProgram> {
    super::compile_source(&canonical_source(source), path, limits)
}

fn compile_source_with_profile(
    source: &str,
    path: &str,
    limits: &Limits,
    profile: crate::ResourceProfile,
) -> lkjscript_core::Result<crate::ExecutableProgram> {
    super::compile_source_with_profile(&canonical_source(source), path, limits, profile)
}

fn unit_main(body: &str) -> String {
    format!("main/\nsig/\n->\nUnit\n/sig\ndo/\n{body}\nunit\n/do\n/main\n")
}

fn stdio_unit_main(body: &str) -> String {
    format!(
        "main/\nsig/\nCapability/\nStdio\n/Capability\n->\nUnit\n/sig\n\
         params/\nstdio\nCapability/\nStdio\n/Capability\n/params\n\
         do/\n{body}\nunit\n/do\n/main\n"
    )
}

fn ownership_source(body: &str, result: &str) -> String {
    let result = result.replace(' ', "\n");
    format!("main/\nsig/\n->\n{result}\n/sig\n{body}\n/main\n")
}

mod affine_handles;
mod capabilities;
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
