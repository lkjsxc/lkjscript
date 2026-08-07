#![allow(clippy::panic, clippy::unwrap_used)]

use lkjscript_core::Constant;

fn canonical_source(source: &str) -> String {
    source.to_string()
}

fn compile_source(source: &str, path: &str) -> lkjscript_core::Result<crate::ExecutableProgram> {
    super::compile_source(&canonical_source(source), path)
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

#[test]
fn preparation_binds_the_same_identity_to_verified_ssa_and_validated_bytecode() {
    let executable = compile_source(&unit_main(""), "prepared-identity.lkjscript")
        .expect("unit source must compile");
    let identity = executable.prepared_identity();
    assert!(executable.ssa().require_prepared_identity(identity).is_ok());
    assert!(executable
        .bytecode()
        .require_prepared_identity(identity)
        .is_ok());
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
mod match_nested;
mod matches;
mod numeric;
mod operations;
#[path = "structural_cases/ownership_boundaries.rs"]
mod ownership_boundaries;
mod ownership_control;
mod ownership_flow;
mod paths;
#[path = "structural_cases/bytecode.rs"]
mod structural_bytecode;
