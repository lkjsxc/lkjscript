use std::path::Path;

use lkjscript_core::{BudgetLedger, Limits, ResourceProfile, Result};

use crate::analyze::analyze_program_with_budget;
use crate::source::validate_for_compiler_with_budget;
use crate::ExecutableProgram;

use super::common::{compile_analyzed, finish};

pub fn compile_source(source: &str, path: &str, limits: &Limits) -> Result<ExecutableProgram> {
    compile_source_with_profile(source, path, limits, ResourceProfile::default())
}

pub fn compile_source_with_profile(
    source: &str,
    path: &str,
    limits: &Limits,
    profile: ResourceProfile,
) -> Result<ExecutableProgram> {
    let mut ledger = BudgetLedger::new(profile);
    compile_source_with_ledger(source, path, limits, &mut ledger)
}

pub fn compile_source_with_ledger(
    source: &str,
    path: &str,
    limits: &Limits,
    ledger: &mut BudgetLedger,
) -> Result<ExecutableProgram> {
    let result = (|| {
        crate::ensure_source_path(Path::new(path))?;
        let program = validate_for_compiler_with_budget(source, path, limits, ledger)?;
        let projection = program
            .module_scoped_projection()
            .map_err(crate::source::SourceDiagnostic::into_core)?;
        let analyzed = analyze_program_with_budget(&projection, ledger)?;
        compile_analyzed(&analyzed, limits, ledger)
    })();
    finish(result, ledger)
}

pub fn validate_source(source: &str, path: &str, limits: &Limits) -> Result<()> {
    validate_source_with_profile(source, path, limits, ResourceProfile::default())
}

pub fn validate_source_with_profile(
    source: &str,
    path: &str,
    limits: &Limits,
    profile: ResourceProfile,
) -> Result<()> {
    let mut ledger = BudgetLedger::new(profile);
    validate_source_with_ledger(source, path, limits, &mut ledger)
}

pub fn validate_source_with_ledger(
    source: &str,
    path: &str,
    limits: &Limits,
    ledger: &mut BudgetLedger,
) -> Result<()> {
    let result = (|| {
        crate::ensure_source_path(Path::new(path))?;
        validate_for_compiler_with_budget(source, path, limits, ledger).map(|_| ())
    })();
    finish(result, ledger)
}
