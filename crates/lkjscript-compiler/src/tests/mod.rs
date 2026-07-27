#![allow(clippy::panic, clippy::unwrap_used)]

use lkjscript_core::{Constant, Limits};

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
    format!("main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\ndo/\n{body}\nunit\n/do\n/main\n")
}

fn stdio_unit_main(body: &str) -> String {
    format!(
        "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\noutput/\nunit\n/output\n/sig\n\
         params/\nstdio\ncapability/\nstdio\n/capability\n/params\n\
         do/\n{body}\nunit\n/do\n/main\n"
    )
}

fn ownership_source(body: &str, result: &str) -> String {
    let result = match result.split_once(' ') {
        Some((constructor, inner)) => {
            format!("{constructor}/\n{inner}\n/{constructor}")
        }
        None => result.to_string(),
    };
    format!("main/\nsig/\ninputs/\n/inputs\noutput/\n{result}\n/output\n/sig\n{body}\n/main\n")
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
