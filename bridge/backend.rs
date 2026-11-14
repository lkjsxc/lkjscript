//! Bootstrap I/O for the separately built production backend.
//! Both IR inputs are explicit paths. No source front end is linked here.
// Core unit tests run once through the library target. Clippy also creates a
// test build for binaries marked test=false; do not relink source-only tests here.
#![cfg(not(test))]
#![allow(dead_code, unused_imports)]
#[path = "../src/backend_lib.rs"]
mod core;
pub(crate) use core::facts;
pub use core::{backend, check, diagnostic, ir, ir_text, native, runtime, x86};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.len() < 2 {
        return Err("usage: lkjscript-backend <program.lkjir> --runtime <runtime.lkjir> [-o output.elf] [--entry main|probe] [--check]".to_string());
    }
    let path = &arguments[1];
    let mut runtime_path = None;
    let mut output = format!("{path}.elf");
    let mut entry = None;
    let mut check_only = false;
    let mut at = 2;
    while at < arguments.len() {
        let option = &arguments[at];
        if option == "--check" {
            check_only = true;
        } else if option == "--runtime" || option == "-o" || option == "--entry" {
            at += 1;
            let value = arguments
                .get(at)
                .ok_or_else(|| format!("{option} needs a value"))?
                .clone();
            match option.as_str() {
                "--runtime" => runtime_path = Some(value),
                "--entry" => entry = Some(value),
                _ => output = value,
            }
        } else {
            return Err(format!("unknown option {option}"));
        }
        at += 1;
    }
    let text = fs::read_to_string(path).map_err(|error| format!("{path}: {error}"))?;
    let report = |path: &str, error: diagnostic::Diagnostic| {
        format!(
            "{path}:{}:{}: {} {}",
            error.line, error.column, error.code, error.message
        )
    };
    let meaning = ir_text::read(&text).map_err(|error| report(path, error))?;
    let checked = check::check(meaning).map_err(|error| report(path, error))?;
    if check_only {
        return Ok(());
    }
    let runtime_path = runtime_path.ok_or("--runtime is required when emitting an artifact")?;
    let runtime_text =
        fs::read_to_string(&runtime_path).map_err(|error| format!("{runtime_path}: {error}"))?;
    let runtime_ir = ir_text::read(&runtime_text).map_err(|error| report(&runtime_path, error))?;
    let runtime_ir = check::check(runtime_ir).map_err(|error| report(&runtime_path, error))?;
    let runtime = runtime::seal(runtime_ir).map_err(|error| report(&runtime_path, error))?;
    let entry = entry.unwrap_or_else(|| {
        if checked
            .program()
            .functions
            .iter()
            .any(|function| function.name == "main")
        {
            "main"
        } else {
            "probe"
        }
        .to_string()
    });
    let bytes = backend::emit(&checked, &runtime, &entry).map_err(|error| report(path, error))?;
    fs::write(&output, bytes).map_err(|error| format!("{output}: {error}"))?;
    fs::set_permissions(&output, fs::Permissions::from_mode(0o755))
        .map_err(|error| format!("{output}: {error}"))
}
fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
