use lkjscript_core::Result;

use crate::{ExecutableProgram, WorkspaceSnapshot};

use super::common::compile_snapshot;

pub fn compile_source(source: &str, path: &str) -> Result<ExecutableProgram> {
    let snapshot = crate::workspace::import_source(source, path)?;
    compile_snapshot(&snapshot)
}

pub fn validate_source(source: &str, path: &str) -> Result<WorkspaceSnapshot> {
    crate::workspace::import_source(source, path)
}
