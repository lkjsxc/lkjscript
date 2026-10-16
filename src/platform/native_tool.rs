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
use super::execution::normalized::{
    NormalizedCommandPolicy, NormalizedProgram, NormalizedRunPolicy, execution_diagnostic,
    run_pure_artifact_command, run_pure_artifact_command_observed,
};
use super::execution::{ExecutionControl, RunPolicy};
use super::json::JsonLimits;
use super::kernel::Name;
use std::sync::Arc;

pub use super::execution::normalized::NormalizedRunObservation as PureToolRunObservation;
use super::runtime::structured::StructuredExecutorObservation;

/// A single production run with observations from its own VM and joined workers.
/// Direct pure execution has no resident task owner or adapter cleanup receipt.
#[derive(Debug)]
pub struct PureToolRunReceipt {
    pub result_json: Vec<u8>,
    pub production: PureToolRunObservation,
    pub executor: StructuredExecutorObservation,
}

pub struct PureTool {
    program: Arc<NormalizedProgram>,
}

impl PureTool {
    /// Independently admit a complete ordinary bundle before preparing any target.
    pub fn load(artifact: &[u8], control: &ExecutionControl) -> Result<Self, Diagnostic> {
        control.check().map_err(execution_diagnostic)?;
        let admitted = load_artifact(artifact)?;
        let program =
            super::builtin_standard::prepare_admitted_program(artifact, admitted, control)?;
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

    /// Run with explicit resource policy, retaining actual execution and cleanup evidence.
    /// Typed input and output admission and grant-free purity checks are identical to run_json.
    pub fn run_json_observed(
        &self,
        target: &str,
        arguments: &[u8],
        limits: JsonLimits,
        policy: RunPolicy,
        control: &ExecutionControl,
    ) -> Result<PureToolRunReceipt, Diagnostic> {
        control.check().map_err(execution_diagnostic)?;
        let target = Name::new(target)?;
        let receipt = run_pure_artifact_command_observed(
            &self.program,
            &target,
            arguments,
            NormalizedCommandPolicy {
                execution: policy.into(),
                json: limits,
            },
            control,
        )?;
        Ok(PureToolRunReceipt {
            result_json: receipt.result_json,
            production: receipt.production,
            executor: receipt.executor,
        })
    }
}
