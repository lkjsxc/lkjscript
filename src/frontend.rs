//! Pure source-to-K boundary over declared source and library bytes.
use crate::ast::{Diagnostic, Program};
use crate::k::CheckedProgram;
use crate::source::SourceBundle;
use crate::{check, elaborate, k_text, source};

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

/// Serialize the checked meaning of exactly the supplied source bundle.
pub fn source_to_k(bundle: &SourceBundle) -> Result<String, Diagnostic> {
    let checked = check_bundle(bundle)?;
    Ok(k_text::write(checked.program()))
}
