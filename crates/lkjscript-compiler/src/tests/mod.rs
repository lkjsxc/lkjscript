#![allow(clippy::panic, clippy::unwrap_used)]

use std::path::Path;

use lkjscript_core::{Constant, Limits};

use super::{compile_source, ensure_source_path, validate_source};

fn unit_main(body: &str) -> String {
    format!("main/\nsig/\n->\nUnit\n/sig\ndo/\n{body}\nunit\n/do\n/main\n")
}

fn ownership_source(body: &str, result: &str) -> String {
    let result = result.replace(' ', "\n");
    format!("main/\nsig/\n->\n{result}\n/sig\n{body}\n/main\n")
}

mod constants;
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
