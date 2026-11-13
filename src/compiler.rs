//! The compiler core consumes declared source bytes and returns artifacts or diagnostics.
//! Filesystem access, process startup and bootstrap input packing belong to the host.
use crate::ast::{Diagnostic, Program};
use crate::k::CheckedProgram;
use crate::source::SourceBundle;
use crate::{check, elaborate, kc, runtime::Runtime, source};

fn locate(program: &Program, mut problem: Diagnostic) -> Diagnostic {
    if problem.path.is_empty() {
        if let Some(file) = program.files.get(problem.file) {
            problem.path = file.path.clone();
        }
    }
    problem
}

/// Resolve and check a parsed source closure through the production pipeline.
pub fn check_source(program: &Program) -> Result<CheckedProgram, Diagnostic> {
    let meaning = elaborate::elaborate(program).map_err(|problem| locate(program, problem))?;
    check::check(meaning).map_err(|problem| locate(program, problem))
}

/// A bundle is closed over its source and library inputs; missing inputs are errors.
pub fn check_bundle(bundle: &SourceBundle) -> Result<CheckedProgram, Diagnostic> {
    let program = source::parse_bundle(bundle)?;
    check_source(&program)
}

/// Compile declared source and a checked runtime to native Linux x86-64 bytes.
/// This entry performs no host I/O and has no hidden compilation service.
pub fn compile(bundle: &SourceBundle, runtime: &Runtime) -> Result<Vec<u8>, Diagnostic> {
    let checked = check_bundle(bundle)?;
    let mut entry = "probe";
    for function in &checked.program().functions {
        if function.name == "main" {
            entry = "main";
            break;
        }
    }
    kc::emit(&checked, runtime, entry).map_err(|mut problem| {
        if problem.path.is_empty() {
            problem.path = bundle.root.clone();
        }
        problem
    })
}
