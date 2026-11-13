//! The compiler core consumes declared source bytes and returns artifacts or diagnostics.
//! Filesystem access, process startup and bootstrap input packing belong to the host.
use crate::ast::{Diagnostic, Program, Span};
use crate::k::CheckedProgram;
use crate::source::SourceBundle;
use crate::{check, elaborate, lower, runtime::Runtime, source, wasm};

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

/// Compile the declared closure to the current memory64 WebAssembly target.
/// This entry performs no host I/O and has no hidden compilation service.
pub fn compile(bundle: &SourceBundle, runtime: &Runtime) -> Result<Vec<u8>, Diagnostic> {
    let checked = check_bundle(bundle)?;
    lower::lower(&checked, runtime)
        .and_then(|program| wasm::encode(&program))
        .map_err(|message| {
            let mut problem = Diagnostic::new(Span::default(), "E_EMIT", &message);
            problem.path = bundle.root.clone();
            problem
        })
}
