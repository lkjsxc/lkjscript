//! Artifact foreground commands, repository differential commands and graph-owned test runners.

use super::capability::NormalizedCapabilities;
use super::codec::{decode_value_with_control, encode_typed_with_control, require_json_encoding};
use super::prepare::{NormalizedProgram, NormalizedTarget};
use super::reference::{
    NormalizedReferenceBinding, NormalizedReferenceInterpreter, NormalizedReferenceObservation,
    NormalizedReferenceOwnerRead, NormalizedReferenceRead, NormalizedReferenceReadWork,
    reference_equal,
};
use super::value::NormalizedValue;
use super::vm::{NormalizedRunObservation, NormalizedRunPolicy, NormalizedVm, normalized_equal};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::json::{JsonLimits, decode_strict};
use crate::platform::kernel::{ComparisonPolicy, Name, OwnerKey, TypeForm, TypeObjectDigest};
use crate::platform::package::RunnerKind;
use crate::platform::publication::RepositoryView;
use crate::platform::semantic_id::RevisionId;
use futures_util::FutureExt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedCommandReceipt {
    pub target: Name,
    pub revision: Option<RevisionId>,
    pub result_json: Vec<u8>,
    pub production: NormalizedRunObservation,
    pub reference: NormalizedReferenceObservation,
    pub differential: &'static str,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NormalizedCommandPolicy {
    pub execution: NormalizedRunPolicy,
    pub json: JsonLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedTestReceipt {
    pub revision: Option<RevisionId>,
    pub passed: u64,
    pub failed: u64,
    pub production_instructions: u64,
    pub reference_expressions: u64,
    pub differential: &'static str,
}

pub(crate) struct PreparedCommandInvocation {
    artifact_manifest: crate::platform::compiler::ArtifactManifestDigest,
    pub(crate) task: bool,
    pub(crate) target: NormalizedTarget,
    pub(crate) arguments: Vec<NormalizedValue>,
    pub(crate) result_type: TypeObjectDigest,
}

impl NormalizedReferenceRead for RepositoryView {
    fn schema(&self) -> Result<std::sync::Arc<super::NormalizedReferenceSchema>, ExecutionError> {
        let read = self
            .reconstruct_full_oracle()
            .map_err(repository_execution_error)?;
        let mut schema = super::NormalizedReferenceSchema::reconstruct([&read.value])?;
        schema.work.map_pages_read = read.work.map.pages_read;
        schema.work.objects_read = read.work.store.objects_read;
        schema.work.bytes_read = read
            .work
            .map
            .bytes_read
            .saturating_add(read.work.store.bytes_read);
        Ok(std::sync::Arc::new(schema))
    }

    fn blob(
        &self,
        digest: crate::platform::kernel::BlobObjectDigest,
    ) -> Result<Vec<u8>, ExecutionError> {
        self.reference_blob(digest)
            .map_err(repository_execution_error)
    }

    fn binding(&self) -> Result<NormalizedReferenceBinding, ExecutionError> {
        Ok(NormalizedReferenceBinding {
            repository: self.current().head.repository_id,
            package: self.package(),
            revision: Some(self.revision()),
            semantic_state: Some(self.current().revision.core.semantic_state),
        })
    }

    fn owner(&self, owner: OwnerKey) -> Result<NormalizedReferenceOwnerRead, ExecutionError> {
        let read = RepositoryView::owner(self, owner).map_err(repository_execution_error)?;
        Ok(NormalizedReferenceOwnerRead {
            record: read.value,
            work: NormalizedReferenceReadWork {
                owner_reads: 1,
                map_pages_read: read.work.map.pages_read,
                objects_read: read.work.store.objects_read,
                bytes_read: read
                    .work
                    .map
                    .bytes_read
                    .saturating_add(read.work.store.bytes_read),
            },
        })
    }
}

/// Runs a pure command-like target through both implementation-disjoint execution tiers.
///
/// Effectful targets deliberately reject here: running production and reference tiers against one
/// live adapter would duplicate externally visible work. Deployment runners must instead use one
/// production tier and separately retained deterministic adapter evidence.
pub fn run_pure_command(
    authority: &dyn NormalizedReferenceRead,
    program: &NormalizedProgram,
    target_name: &Name,
    arguments_json: &[u8],
    policy: NormalizedCommandPolicy,
    control: &ExecutionControl,
) -> Result<NormalizedCommandReceipt, Diagnostic> {
    let authority_binding = authority.binding().map_err(execution_diagnostic)?;
    validate_authority_binding(program, authority_binding)?;
    let invocation =
        prepare_command_invocation(program, target_name, arguments_json, policy.json, control)?;
    let component = program
        .components
        .get(invocation.target.component.0 as usize)
        .ok_or_else(|| {
            runner_error(
                DiagnosticClass::Corrupt,
                "normalized_runner_component",
                "selected target component escaped the prepared runtime table",
            )
        })?;
    if invocation.task || !component.requirements.is_empty() {
        return Err(runner_error(
            DiagnosticClass::Capability,
            "normalized_runner_grants_required",
            "effectful target requires one production execution with exact deployment grants",
        ));
    }

    let production = NormalizedVm::new(program, policy.execution)
        .invoke_root_target(target_name, invocation.arguments.clone(), None, control)
        .map_err(execution_diagnostic)?;
    let reference =
        NormalizedReferenceInterpreter::from_reader(authority, program, policy.execution)
            .invoke_root_target(target_name, invocation.arguments, None, control)
            .map_err(execution_diagnostic)?;
    if production.0 != reference.0 {
        return Err(runner_error(
            DiagnosticClass::Infrastructure,
            "normalized_runner_differential",
            "production and reference execution disagree for the selected pure target",
        ));
    }
    let result_json = encode_typed_with_control(
        program,
        &production.0,
        invocation.result_type,
        policy.json,
        control,
    )?;
    Ok(NormalizedCommandReceipt {
        target: target_name.clone(),
        revision: authority_binding.revision,
        result_json,
        production: production.1,
        reference: reference.1,
        differential: "equal",
    })
}

#[derive(Debug)]
pub(crate) struct NormalizedForegroundReceipt {
    pub result_json: Vec<u8>,
    pub production: NormalizedRunObservation,
    pub invocation_nanoseconds: u64,
    pub shutdown: crate::platform::runtime::ShutdownReceipt,
}

async fn foreground_outcome<T>(
    mut running: std::pin::Pin<&mut impl std::future::Future<Output = T>>,
    cancellation: impl std::future::Future<Output = ()>,
) -> Option<T> {
    tokio::select! {
        biased;
        result = running.as_mut() => Some(result),
        () = cancellation => running.now_or_never(),
    }
}

/// One production invocation under the common runtime/resource/adapter lifecycle.
pub(crate) async fn run_foreground_command(
    resident: super::resident::NormalizedResidentDeployment,
    invocation: PreparedCommandInvocation,
    cancellation: impl std::future::Future<Output = ()>,
) -> Result<NormalizedForegroundReceipt, Diagnostic> {
    let admitted = invocation.target == *resident.target()
        && invocation.artifact_manifest == resident.program().artifact().manifest_digest
        && invocation.target.runner == RunnerKind::Command;
    let outcome = if !admitted {
        Err(runner_error(
            DiagnosticClass::Corrupt,
            "normalized_runner_grant_component",
            "foreground preflight and prepared deployment select different exact command bindings",
        ))
    } else {
        let running = resident.invoke(invocation.arguments);
        tokio::pin!(running);
        match foreground_outcome(running.as_mut(), cancellation).await {
            Some(outcome) => outcome.map_err(execution_diagnostic),
            None => {
                resident.cancel();
                let grace = std::time::Duration::from_millis(
                    resident.limits().cancellation_grace_milliseconds,
                );
                let joined = tokio::time::timeout(grace, &mut running).await;
                let mut error = runner_error(
                    DiagnosticClass::Cancelled,
                    "execution_cancelled",
                    "foreground execution was cancelled by its owning process",
                );
                if joined.is_err() {
                    error
                        .notes
                        .push("invocation did not join within cancellation grace".to_owned());
                }
                Err(error)
            }
        }
    };
    let encoded = outcome.and_then(|receipt| {
        let bytes = encode_typed_with_control(
            resident.program(),
            &receipt.value,
            invocation.result_type,
            JsonLimits::default(),
            &ExecutionControl::uncancelled(),
        )?;
        Ok((bytes, receipt))
    });
    let shutdown = resident.shutdown().await;
    let mut result = encoded.map(|(result_json, receipt)| NormalizedForegroundReceipt {
        result_json,
        production: receipt.execution,
        invocation_nanoseconds: receipt.execution_nanoseconds,
        shutdown: shutdown.clone(),
    });
    if (shutdown.remaining_tasks != 0 || !shutdown.cleanup_failures.is_empty()) && result.is_ok() {
        result = Err(runner_error(
            DiagnosticClass::Infrastructure,
            "foreground_cleanup",
            "foreground cleanup did not complete successfully",
        ));
    }
    if let Err(error) = &mut result {
        error.notes.push(format!(
            "foreground cleanup: admission-stopped={} remaining-owned-tasks={} failures={}",
            shutdown.admission_stopped,
            shutdown.remaining_tasks,
            shutdown.cleanup_failures.len()
        ));
        if invocation.task && resident.observe().admitted != 0 {
            crate::platform::deployment::foreground_visibility(error);
        }
        if shutdown.remaining_tasks != 0 {
            error.notes.push(format!(
                "{} owned tasks remain after shutdown",
                shutdown.remaining_tasks
            ));
        }
        for failure in &shutdown.cleanup_failures {
            error.notes.push(format!(
                "adapter cleanup failed with safe code '{}'",
                failure.code
            ));
        }
    }
    result
}

/// Runs each independently inventoried canonical graph test through dense and canonical execution.
///
/// A supplied capability set must be deterministic and replayable because each test executes once
/// per tier. Production deployment adapters are not appropriate here.
pub fn run_graph_tests(
    authority: &dyn NormalizedReferenceRead,
    program: &NormalizedProgram,
    capabilities: Option<&NormalizedCapabilities>,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
) -> Result<NormalizedTestReceipt, Diagnostic> {
    let authority_binding = authority.binding().map_err(execution_diagnostic)?;
    validate_authority_binding(program, authority_binding)?;
    let vm = NormalizedVm::new(program, policy);
    let reference = NormalizedReferenceInterpreter::from_reader(authority, program, policy);
    let schema = authority.schema().map_err(execution_diagnostic)?;
    let prepared_tests = program
        .tests()
        .map(|test| (test.declaration, test.comparison))
        .collect::<std::collections::BTreeMap<_, _>>();
    if prepared_tests != schema.tests {
        return Err(runner_error(
            DiagnosticClass::Corrupt,
            "normalized_test_inventory_differential",
            "compiled tests differ from the independent complete canonical test inventory",
        ));
    }
    let mut receipt = NormalizedTestReceipt {
        revision: authority_binding.revision,
        passed: 0,
        failed: 0,
        production_instructions: 0,
        reference_expressions: 0,
        differential: "equal",
    };
    for (declaration, comparison) in &schema.tests {
        let production = vm
            .invoke_test(*declaration, capabilities, control)
            .map_err(execution_diagnostic)?;
        let oracle = reference
            .invoke_test(*declaration, capabilities, control)
            .map_err(execution_diagnostic)?;
        receipt.production_instructions = receipt
            .production_instructions
            .saturating_add(production.0.1.instructions)
            .saturating_add(production.1.1.instructions);
        receipt.reference_expressions = receipt
            .reference_expressions
            .saturating_add(oracle.0.1.expressions)
            .saturating_add(oracle.1.1.expressions);
        if production.0.0 != oracle.0.0 || production.1.0 != oracle.1.0 {
            return Err(runner_error(
                DiagnosticClass::Infrastructure,
                "normalized_test_differential",
                format!(
                    "production and reference execution disagree for exact test {:?}",
                    declaration
                ),
            ));
        }
        let (production_equal, reference_equal) = match comparison {
            ComparisonPolicy::Exact => (
                normalized_equal(&production.0.0, &production.1.0).map_err(execution_diagnostic)?,
                reference_equal(&oracle.0.0, &oracle.1.0).map_err(execution_diagnostic)?,
            ),
        };
        if production_equal != reference_equal {
            return Err(runner_error(
                DiagnosticClass::Infrastructure,
                "normalized_test_comparison_differential",
                "production and reference equality semantics disagree",
            ));
        }
        if !production_equal {
            let mut diagnostic = runner_error(
                DiagnosticClass::Semantic,
                "normalized_test_failed",
                format!("exact graph-owned test {:?} failed", declaration),
            );
            diagnostic.notes.push(format!(
                "{} earlier graph-owned tests passed before this failure",
                receipt.passed
            ));
            return Err(diagnostic);
        }
        receipt.passed = receipt.passed.saturating_add(1);
    }
    Ok(receipt)
}

pub(crate) fn prepare_command_invocation(
    program: &NormalizedProgram,
    target_name: &Name,
    arguments_json: &[u8],
    json_limits: JsonLimits,
    control: &ExecutionControl,
) -> Result<PreparedCommandInvocation, Diagnostic> {
    let target = program.root_target(target_name).ok_or_else(|| {
        runner_error(
            DiagnosticClass::Source,
            "normalized_runner_target_missing",
            "root artifact package has no target with the exact selected name",
        )
    })?;
    if !matches!(
        target.runner,
        RunnerKind::Command | RunnerKind::Batch | RunnerKind::Test
    ) {
        return Err(runner_error(
            DiagnosticClass::Source,
            "normalized_runner_kind",
            "selected target is not a command, batch, or test runner",
        ));
    }
    let port_index = target.port.ok_or_else(|| {
        runner_error(
            DiagnosticClass::Corrupt,
            "normalized_runner_target_port",
            "selected non-HTTP target has no exact port",
        )
    })?;
    let port = program.ports.get(port_index.0 as usize).ok_or_else(|| {
        runner_error(
            DiagnosticClass::Corrupt,
            "normalized_runner_port",
            "selected target port escaped the prepared runtime table",
        )
    })?;
    if port.component != target.component {
        return Err(runner_error(
            DiagnosticClass::Corrupt,
            "normalized_runner_port_component",
            "selected target and port disagree on their exact component",
        ));
    }
    let (parameter_types, result_type, task) = function_type(program, port.function_type)?;
    require_json_encoding(program, result_type, false, json_limits, control)?;
    let arguments = decode_strict(arguments_json, json_limits)?;
    let arguments = arguments.as_array().ok_or_else(|| {
        runner_error(
            DiagnosticClass::Source,
            "normalized_runner_arguments_array",
            "target arguments must be one JSON array",
        )
    })?;
    if arguments.len() != parameter_types.len() {
        return Err(runner_error(
            DiagnosticClass::Source,
            "normalized_runner_argument_count",
            format!(
                "target expects {} arguments; {} were supplied",
                parameter_types.len(),
                arguments.len()
            ),
        ));
    }
    let arguments = arguments
        .iter()
        .zip(parameter_types)
        .map(|(value, ty)| decode_value_with_control(program, value, ty, json_limits, control))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PreparedCommandInvocation {
        artifact_manifest: program.artifact().manifest_digest,
        target: target.clone(),
        arguments,
        result_type,
        task,
    })
}

fn function_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<(Vec<TypeObjectDigest>, TypeObjectDigest, bool), Diagnostic> {
    let object = program.types.get(&ty).ok_or_else(|| {
        runner_error(
            DiagnosticClass::Corrupt,
            "normalized_runner_function_type_missing",
            "selected port function type is absent from the exact artifact closure",
        )
    })?;
    match &object.form {
        TypeForm::Function { parameters, result } => Ok((parameters.clone(), *result, false)),
        TypeForm::TaskFunction {
            parameters,
            result,
            effect,
        } if effect.is_closed() => Ok((parameters.clone(), *result, true)),
        _ => Err(runner_error(
            DiagnosticClass::Corrupt,
            "normalized_runner_port_type",
            "selected port does not have an exact function type",
        )),
    }
}

fn validate_authority_binding(
    program: &NormalizedProgram,
    binding: NormalizedReferenceBinding,
) -> Result<(), Diagnostic> {
    if !binding.matches(program) {
        return Err(runner_error(
            DiagnosticClass::Infrastructure,
            "normalized_reference_authority_binding",
            "reference authority and executable artifact do not bind one exact accepted root",
        ));
    }
    Ok(())
}

pub(crate) fn execution_diagnostic(error: ExecutionError) -> Diagnostic {
    let class = match error.class {
        ExecutionFailureClass::Trap => DiagnosticClass::Semantic,
        ExecutionFailureClass::Capability | ExecutionFailureClass::PossibleVisibility => {
            DiagnosticClass::Capability
        }
        ExecutionFailureClass::Resource => DiagnosticClass::Resource,
        ExecutionFailureClass::Cancelled => DiagnosticClass::Cancelled,
        ExecutionFailureClass::Infrastructure => DiagnosticClass::Infrastructure,
    };
    let mut diagnostic = Diagnostic::new(class, error.code, error.message);
    if error.possibly_visible {
        diagnostic
            .notes
            .push("external effects may already be visible".to_owned());
    }
    diagnostic
}

fn repository_execution_error(diagnostic: Diagnostic) -> ExecutionError {
    let class = match diagnostic.class {
        DiagnosticClass::Resource => ExecutionFailureClass::Resource,
        DiagnosticClass::Cancelled => ExecutionFailureClass::Cancelled,
        DiagnosticClass::Capability => ExecutionFailureClass::Capability,
        DiagnosticClass::Source
        | DiagnosticClass::Semantic
        | DiagnosticClass::Corrupt
        | DiagnosticClass::Infrastructure => ExecutionFailureClass::Infrastructure,
    };
    ExecutionError::new(class, diagnostic.code, diagnostic.message)
}

fn runner_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod foreground_race_tests {
    #[tokio::test]
    async fn foreground_completion_ready_at_signal_selection_wins() {
        let completed = std::cell::Cell::new(false);
        let running = std::future::poll_fn(|_| {
            if completed.get() {
                std::task::Poll::Ready(7)
            } else {
                std::task::Poll::Pending
            }
        });
        tokio::pin!(running);
        let result = super::foreground_outcome(running.as_mut(), async {
            completed.set(true);
        })
        .await;
        assert_eq!(result, Some(7));
        let running = std::future::pending::<u8>();
        tokio::pin!(running);
        assert_eq!(
            super::foreground_outcome(running.as_mut(), std::future::ready(())).await,
            None
        );
    }
}
