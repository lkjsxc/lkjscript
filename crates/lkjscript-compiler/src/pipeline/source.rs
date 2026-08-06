use std::path::Path;

use lkjscript_core::{Limits, Result};

use crate::analyze::analyze_program;
use crate::source::validate_for_compiler;
use crate::ExecutableProgram;

use super::common::compile_analyzed;

pub fn compile_source(source: &str, path: &str, limits: &Limits) -> Result<ExecutableProgram> {
    crate::ensure_source_path(Path::new(path))?;
    let program = validate_for_compiler(source, path, limits)?;
    let source_identity = program
        .files()
        .first()
        .ok_or_else(|| lkjscript_core::Error::msg("development source closure is empty"))?
        .exact_source_sha256;
    let projection = program
        .module_scoped_projection()
        .map_err(crate::source::SourceDiagnostic::into_core)?;
    let analyzed = analyze_program(&projection)?;
    compile_analyzed(&analyzed, limits, |plan| {
        Ok(crate::package::program::development(
            source_identity,
            path,
            plan,
        ))
    })
}

pub fn validate_source(source: &str, path: &str, limits: &Limits) -> Result<()> {
    crate::ensure_source_path(Path::new(path))?;
    validate_for_compiler(source, path, limits).map(|_| ())
}
