//! Grant-free embedding for ordinary native development tools.
//!
//! Prepared immutable code is reusable; every invocation has fresh execution state.
//! This API loads the ordinary strict artifact format, accepts bounded typed JSON,
//! and executes one production pure Command target. It never opens an authoring
//! project, supplies grants, runs a subprocess, or bypasses task identity checks.
//! Artifact integrity is not trust: callers select trusted code and own cancellation.
//! Foreground execution has no cumulative fuel quota and is not a hostile-code sandbox.

use super::compiler::load_artifact;
use super::diagnostic::Diagnostic;
use super::execution::ExecutionControl;
use super::execution::normalized::{
    NormalizedCommandPolicy, NormalizedProgram, NormalizedRunPolicy, execution_diagnostic,
    run_pure_artifact_command,
};
use super::json::JsonLimits;
use super::kernel::Name;

pub struct PureTool {
    program: NormalizedProgram,
}

impl PureTool {
    /// Independently admit a complete ordinary bundle before preparing any target.
    pub fn load(artifact: &[u8], control: &ExecutionControl) -> Result<Self, Diagnostic> {
        control.check().map_err(execution_diagnostic)?;
        let artifact = load_artifact(artifact)?;
        let program = NormalizedProgram::prepare_with_control(artifact, control)?;
        Ok(Self { program })
    }

    /// Run once with no ambient authority. JSON limits apply to both input and output.
    /// A failed or cancelled call does not poison the immutable prepared program.
    pub fn run_json(
        &self,
        target: &str,
        arguments: &[u8],
        limits: JsonLimits,
        control: &ExecutionControl,
    ) -> Result<Vec<u8>, Diagnostic> {
        control.check().map_err(execution_diagnostic)?;
        let target = Name::new(target)?;
        run_pure_artifact_command(
            &self.program,
            &target,
            arguments,
            NormalizedCommandPolicy {
                execution: NormalizedRunPolicy::foreground(),
                json: limits,
            },
            control,
        )
    }
}
