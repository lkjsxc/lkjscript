//! Repository-bound command and graph-owned test runners for normalized Graph 9 artifacts.

use super::capability::NormalizedCapabilities;
use super::codec::{decode_value, encode_typed};
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedCommandReceipt {
    pub target: Name,
    pub revision: Option<RevisionId>,
    pub result_json: Vec<u8>,
    pub production: NormalizedRunObservation,
    pub reference: NormalizedReferenceObservation,
    pub differential: &'static str,
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NormalizedEffectfulCommandReceipt {
    pub target: Name,
    pub revision: Option<RevisionId>,
    pub result_json: Vec<u8>,
    pub production: NormalizedRunObservation,
    pub verification: &'static str,
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

struct PreparedCommandInvocation<'a> {
    target: &'a NormalizedTarget,
    arguments: Vec<NormalizedValue>,
    result_type: TypeObjectDigest,
}

impl NormalizedReferenceRead for RepositoryView {
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
    let invocation = prepare_command_invocation(program, target_name, arguments_json, policy.json)?;
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
    if !component.requirements.is_empty() {
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
    let result_json = encode_typed(program, &production.0, invocation.result_type, policy.json)?;
    Ok(NormalizedCommandReceipt {
        target: target_name.clone(),
        revision: authority_binding.revision,
        result_json,
        production: production.1,
        reference: reference.1,
        differential: "equal",
    })
}

/// Runs one effectful command-like target exactly once through the production tier.
///
/// The authority/artifact binding and complete deployment grant set are checked before execution.
/// The reference tier is intentionally not invoked against live effects.
#[cfg(test)]
pub fn run_effectful_command(
    authority: &dyn NormalizedReferenceRead,
    program: &NormalizedProgram,
    target_name: &Name,
    arguments_json: &[u8],
    capabilities: &NormalizedCapabilities,
    policy: NormalizedCommandPolicy,
    control: &ExecutionControl,
) -> Result<NormalizedEffectfulCommandReceipt, Diagnostic> {
    let authority_binding = authority.binding().map_err(execution_diagnostic)?;
    validate_authority_binding(program, authority_binding)?;
    let invocation = prepare_command_invocation(program, target_name, arguments_json, policy.json)?;
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
    if component.requirements.is_empty() {
        return Err(runner_error(
            DiagnosticClass::Source,
            "normalized_runner_pure_target",
            "pure target must use differential command execution",
        ));
    }
    if capabilities.component() != invocation.target.component {
        return Err(runner_error(
            DiagnosticClass::Capability,
            "normalized_runner_grant_component",
            "deployment grants are bound to another exact component",
        ));
    }

    let production = NormalizedVm::new(program, policy.execution)
        .invoke_root_target(
            target_name,
            invocation.arguments,
            Some(capabilities),
            control,
        )
        .map_err(execution_diagnostic)?;
    let result_json = encode_typed(program, &production.0, invocation.result_type, policy.json)?;
    Ok(NormalizedEffectfulCommandReceipt {
        target: target_name.clone(),
        revision: authority_binding.revision,
        result_json,
        production: production.1,
        verification: "production_only_live_effects",
    })
}

/// Runs every prepared graph-owned test through dense and canonical execution.
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
    let mut receipt = NormalizedTestReceipt {
        revision: authority_binding.revision,
        passed: 0,
        failed: 0,
        production_instructions: 0,
        reference_expressions: 0,
        differential: "equal",
    };
    for test in program.tests() {
        let production = vm
            .invoke_test(test.declaration, capabilities, control)
            .map_err(execution_diagnostic)?;
        let oracle = reference
            .invoke_test(test.declaration, capabilities, control)
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
                    test.declaration
                ),
            ));
        }
        let (production_equal, reference_equal) = match test.comparison {
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
                format!("exact graph-owned test {:?} failed", test.declaration),
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

fn prepare_command_invocation<'a>(
    program: &'a NormalizedProgram,
    target_name: &Name,
    arguments_json: &[u8],
    json_limits: JsonLimits,
) -> Result<PreparedCommandInvocation<'a>, Diagnostic> {
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
    let (parameter_types, result_type) = function_type(program, port.function_type)?;
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
        .map(|(value, ty)| decode_value(program, value, ty, json_limits))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PreparedCommandInvocation {
        target,
        arguments,
        result_type,
    })
}

fn function_type(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> Result<(Vec<TypeObjectDigest>, TypeObjectDigest), Diagnostic> {
    let object = program.types.get(&ty).ok_or_else(|| {
        runner_error(
            DiagnosticClass::Corrupt,
            "normalized_runner_function_type_missing",
            "selected port function type is absent from the exact artifact closure",
        )
    })?;
    let TypeForm::Function { parameters, result } = &object.form else {
        return Err(runner_error(
            DiagnosticClass::Corrupt,
            "normalized_runner_port_type",
            "selected port does not have an exact function type",
        ));
    };
    Ok((parameters.clone(), *result))
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

fn execution_diagnostic(error: ExecutionError) -> Diagnostic {
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
