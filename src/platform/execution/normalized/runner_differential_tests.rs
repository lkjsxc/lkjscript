//! Explicit differential oracle for tests of the source command path.

use super::*;

#[derive(Debug)]
pub(crate) struct DifferentialCommandReceipt {
    pub target: Name,
    pub revision: Option<RevisionId>,
    pub result_json: Vec<u8>,
    pub production: NormalizedRunObservation,
    pub reference: NormalizedReferenceObservation,
    pub differential: &'static str,
}

/// Runs a pure command-like target through both implementation-disjoint execution tiers.
///
/// Effectful targets deliberately reject here: running production and reference tiers against one
/// live adapter would duplicate externally visible work. Deployment runners must instead use one
/// production tier and separately retained deterministic adapter evidence.
pub(crate) fn run_pure_command(
    authority: &dyn NormalizedReferenceRead,
    program: &Arc<NormalizedProgram>,
    target_name: &Name,
    arguments_json: &[u8],
    policy: NormalizedCommandPolicy,
    control: &ExecutionControl,
) -> Result<DifferentialCommandReceipt, Diagnostic> {
    let authority_binding = authority.binding().map_err(execution_diagnostic)?;
    validate_authority_binding(program, authority_binding)?;
    let invocation =
        prepare_command_invocation(program, target_name, arguments_json, policy.json, control)?;
    require_pure_invocation(program, &invocation)?;

    let mut executor = StructuredExecutor::new();
    let production_observer = std::sync::Mutex::new(None);
    let reference_observer = std::sync::Mutex::new(None);
    let outcome = (|| {
        let production = NormalizedVm::new(program, policy.execution, &executor.handle())
            .observing_checked(&production_observer)
            .invoke_root_target(target_name, invocation.arguments.clone(), None, control)
            .map_err(execution_diagnostic)?;
        let reference =
            NormalizedReferenceInterpreter::from_reader(authority, program, policy.execution)
                .observing_checked(&reference_observer)
                .invoke_root_target(target_name, invocation.arguments, None, control)
                .map_err(execution_diagnostic)?;
        if !raw_structural_equal(&production.0, &reference.0, control)
            .map_err(execution_diagnostic)?
        {
            return Err(runner_error(
                DiagnosticClass::Infrastructure,
                "normalized_runner_differential",
                "production and reference execution disagree for the selected pure target",
            ));
        }
        let result_json = encode_typed_with_control(
            program.as_ref(),
            &production.0,
            invocation.result_type,
            policy.json,
            control,
        )?;
        Ok(DifferentialCommandReceipt {
            target: target_name.clone(),
            revision: authority_binding.revision,
            result_json,
            production: production.1,
            reference: reference.1,
            differential: "equal",
        })
    })();
    let cleanup = executor.shutdown().map_err(execution_diagnostic);
    match (outcome, cleanup) {
        (Ok(command), Ok(_)) => Ok(command),
        (Ok(command), Err(mut cleanup)) => {
            append_source_observation_json(
                &mut cleanup,
                "production-observation",
                &command.production,
            );
            append_source_observation_json(
                &mut cleanup,
                "reference-observation",
                &command.reference,
            );
            Err(cleanup)
        }
        (Err(mut primary), cleanup) => {
            append_source_observation(&mut primary, "production-observation", production_observer);
            append_source_observation(&mut primary, "reference-observation", reference_observer);
            match cleanup {
                Ok(executor) => {
                    append_source_observation_json(&mut primary, "executor-observation", &executor)
                }
                Err(cleanup) => primary.notes.push(format!(
                    "source structured cleanup failed with safe code '{}': {}",
                    cleanup.code, cleanup.message
                )),
            }
            Err(primary)
        }
    }
}
