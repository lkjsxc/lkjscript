//! Real public host preparation crossing, independent of selfhost generations.
//! This component control does not replace the public disk/C2 acceptance witness.

use super::*;
use crate::platform::DiagnosticClass;
use crate::platform::execution::normalized::prepare::{
    NormalizedPreparationLimits, NormalizedPreparationWork,
};

fn preparation_observation(work: &NormalizedPreparationWork) -> Value {
    json!({
        "type_derivation_steps": work.type_derivation_steps.to_string(),
        "type_derivation_saturated": work.type_derivation_saturated,
        "type_metadata_bytes": work.type_metadata_bytes.to_string(),
        "type_metadata_saturated": work.type_metadata_saturated,
        "admission_bytes": work.admission_bytes.to_string(),
        "instructions": work.instructions.to_string(),
        "compiler_units": work.compiler_units.to_string(),
        "runtime_owners": work.runtime_owners.to_string(),
        "scope": "modeled cumulative preparation work and metadata; not RSS or a quota"
    })
}

fn selected_limits(limits: NormalizedPreparationLimits) -> Value {
    json!({
        "maximum_type_derivation_steps":
            limits.maximum_type_derivation_steps.map(|value| value.to_string()),
        "maximum_type_metadata_bytes":
            limits.maximum_type_metadata_bytes.map(|value| value.to_string())
    })
}

fn preparation_unsaturated(program: &NormalizedProgram) -> Check<()> {
    ensure(
        !program.work.type_derivation_saturated && !program.work.type_metadata_saturated,
        "finite preparation observations unexpectedly saturated",
    )
}

fn vm_42(program: &NormalizedProgram, case: &InputCase) -> Check<NormalizedRunObservation> {
    let entry = function_named(&case.source, "sequence-wide", "entry")?;
    let (value, observation) = NormalizedVm::for_test(program, NormalizedRunPolicy::foreground())
        .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
        .map_err(|error| format!("Sequence preparation VM: {error:?}"))?;
    equal_value(
        &value,
        &NormalizedValue::I64(42),
        "Sequence preparation VM result",
    )?;
    reclaimed(&observation, "Sequence preparation VM cleanup")?;
    Ok(observation)
}

#[test]
fn selfhost_source_sequence_preparation_optional_policies_use_real_artifact() -> Check<()> {
    let started = Instant::now();
    let text = proposal(3, 42, false)?;
    let case = public_case(&text)?;
    let host = host_oracle(&case)?;
    preparation_unsaturated(&host.program)?;
    let observed = host.program.work;
    ensure(
        observed.type_derivation_steps > 0 && observed.type_metadata_bytes > 0,
        "tiny artifact must perform actual type preparation",
    )?;
    let defaults = NormalizedPreparationLimits::default();
    equal(
        defaults,
        NormalizedPreparationLimits {
            maximum_type_derivation_steps: None,
            maximum_type_metadata_bytes: None,
        },
        "ordinary preparation policies are unbounded",
    )?;
    vm_42(&host.program, &case)?;
    let healthy = ExecutionControl::uncancelled();
    let mut outcomes = Vec::new();
    for (label, limits, expected_code) in [
        (
            "zero_work",
            NormalizedPreparationLimits {
                maximum_type_derivation_steps: Some(0),
                maximum_type_metadata_bytes: None,
            },
            "normalized_instantiation_work",
        ),
        (
            "zero_metadata",
            NormalizedPreparationLimits {
                maximum_type_derivation_steps: None,
                maximum_type_metadata_bytes: Some(0),
            },
            "normalized_instantiation_storage",
        ),
    ] {
        let error = NormalizedProgram::prepare_with_limits(host.loaded.clone(), limits, &healthy)
            .err()
            .ok_or_else(|| format!("{label}: zero preparation allowance unexpectedly succeeded"))?;
        equal(
            error.class,
            DiagnosticClass::Resource,
            "finite preparation refusal class",
        )?;
        equal(
            error.code.as_str(),
            expected_code,
            "finite preparation refusal code",
        )?;
        outcomes.push(json!({
            "policy": label, "limits": selected_limits(limits),
            "outcome": "resource_refused", "diagnostic": error.code
        }));
    }
    for (label, limits) in [
        (
            "exact",
            NormalizedPreparationLimits {
                maximum_type_derivation_steps: Some(observed.type_derivation_steps),
                maximum_type_metadata_bytes: Some(observed.type_metadata_bytes),
            },
        ),
        (
            "finite_u64_max",
            NormalizedPreparationLimits {
                maximum_type_derivation_steps: Some(u64::MAX),
                maximum_type_metadata_bytes: Some(u64::MAX),
            },
        ),
    ] {
        let program = NormalizedProgram::prepare_with_limits(host.loaded.clone(), limits, &healthy)
            .map_err(|error| format!("{label}: real artifact preparation: {error:?}"))?;
        preparation_unsaturated(&program)?;
        equal(
            (
                program.work.type_derivation_steps,
                program.work.type_metadata_bytes,
            ),
            (observed.type_derivation_steps, observed.type_metadata_bytes),
            "finite allowance preserves complete preparation",
        )?;
        vm_42(&program, &case)?;
        outcomes.push(json!({
            "policy": label, "limits": selected_limits(limits),
            "outcome": "prepared_and_executed", "expected": 42,
            "preparation": preparation_observation(&program.work)
        }));
    }
    let cancelled = ExecutionControl::cancel_after_checks(0);
    let error = NormalizedProgram::prepare_with_limits(host.loaded.clone(), defaults, &cancelled)
        .err()
        .ok_or("cancelled real artifact preparation unexpectedly succeeded")?;
    equal(
        error.class,
        DiagnosticClass::Cancelled,
        "preparation cancellation remains distinct from finite refusal",
    )?;
    let retry = NormalizedProgram::prepare(host.loaded.clone())
        .map_err(|error| format!("healthy preparation after refusal/cancellation: {error:?}"))?;
    preparation_unsaturated(&retry)?;
    equal(
        (
            retry.work.type_derivation_steps,
            retry.work.type_metadata_bytes,
        ),
        (observed.type_derivation_steps, observed.type_metadata_bytes),
        "healthy retry retains complete preparation",
    )?;
    vm_42(&retry, &case)?;
    eprintln!(
        "SEQUENCE_PREPARATION {}",
        json!({
            "event": "optional_policies_passed", "items": 3, "expected": 42,
            "carrier_sha256": sha256(&case.carrier),
            "artifact_sha256": sha256(&host.bytes),
            "default_limits": selected_limits(defaults),
            "default_preparation": preparation_observation(&observed),
            "policies": outcomes, "cancellation": "cancelled",
            "healthy_retry": "prepared_and_executed",
            "elapsed_ms": started.elapsed().as_millis().to_string(),
            "scope": "actual host artifact preparation and VM integration; no selfhost generation"
        })
    );
    Ok(())
}

#[test]
#[ignore = "explicit million-item public host preparation and independent execution"]
fn selfhost_source_sequence_preparation_crosses_metadata_and_work_defaults() -> Check<()> {
    let total = Instant::now();
    let count = 1_000_001;
    let started = Instant::now();
    let text = proposal(count, 42, false)?;
    let proposal_ms = started.elapsed().as_millis().to_string();
    let started = Instant::now();
    let case = public_case(&text)?;
    let public_input_ms = started.elapsed().as_millis().to_string();
    let source_binding = json!({
        "repository": case.source.root.repository_id.to_string(),
        "package": case.source.root.package_id.to_string(),
        "package_revision": case.revision_digest.to_string(),
        "proposal_bytes": text.len(), "proposal_sha256": sha256(text.as_bytes()),
        "carrier_bytes": case.carrier.len(), "carrier_sha256": sha256(&case.carrier),
        "core_sha256": sha256(&case.canonical_core),
        "owners": case.source.owners.len(), "supplied_types": case.source.types.len(),
        "scope": "complete public in-memory fixture carrier; not maintained compiler S"
    });
    drop(text);
    eprintln!(
        "SEQUENCE_PREPARATION {}",
        json!({
            "event": "public_input_ready", "items": count,
            "source_binding": &source_binding,
            "proposal_ms": &proposal_ms, "public_input_ms": &public_input_ms
        })
    );
    let started = Instant::now();
    // The existing oracle performs real build, link, strict load and default
    // NormalizedProgram::prepare exactly once. Its stages share this elapsed time.
    let host = host_oracle(&case)?;
    let host_ms = started.elapsed().as_millis().to_string();
    preparation_unsaturated(&host.program)?;
    let started = Instant::now();
    let inventory = code_inventory(&case, &host, count, false)?;
    let inventory_ms = started.elapsed().as_millis().to_string();
    let work = host.program.work;
    ensure(
        work.type_metadata_bytes > 256 * 1024 * 1024,
        "real preparation did not cross the former 256 MiB metadata default",
    )?;
    ensure(
        work.type_derivation_steps > 10_000_000,
        "real preparation did not cross the former ten-million work default",
    )?;
    eprintln!(
        "SEQUENCE_PREPARATION {}",
        json!({
            "event": "host_prepared", "items": count,
            "inventory": &inventory, "preparation": preparation_observation(&work),
            "host_build_link_strict_load_prepare_ms": &host_ms,
            "inventory_ms": &inventory_ms
        })
    );
    let started = Instant::now();
    let vm = vm_42(&host.program, &case)?;
    let vm_ms = started.elapsed().as_millis().to_string();
    let entry = function_named(&case.source, "sequence-wide", "entry")?;
    let started = Instant::now();
    let reference = NormalizedReferenceInterpreter::new(
        &case.source,
        &host.program,
        NormalizedRunPolicy::foreground(),
    )
    .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
    .map_err(|error| format!("Sequence preparation reference: {error:?}"))?;
    let reference_ms = started.elapsed().as_millis().to_string();
    equal_value(
        &reference.0,
        &NormalizedValue::I64(42),
        "Sequence preparation independent reference result",
    )?;
    reference_reclaimed(&reference.1, "Sequence preparation reference cleanup")?;
    let started = Instant::now();
    let native = native_image(&host.program, entry)?;
    let native_emission_ms = started.elapsed().as_millis().to_string();
    let started = Instant::now();
    run_scalar(
        "SequencePreparation/C0/million",
        &native,
        42,
        FOCUSED_DEADLINE,
    )?;
    let native_execution_ms = started.elapsed().as_millis().to_string();
    eprintln!(
        "SEQUENCE_PREPARATION {}",
        json!({
            "event": "crossing_passed", "items": count, "locals": false, "expected": 42,
            "source_binding": source_binding, "inventory": inventory,
            "preparation": preparation_observation(&work),
            "limits": selected_limits(NormalizedPreparationLimits::default()),
            "elapsed_ms": {
                "proposal": proposal_ms, "public_input": public_input_ms,
                "host_build_link_strict_load_prepare": host_ms, "inventory": inventory_ms,
                "vm": vm_ms, "reference": reference_ms,
                "C0_emission": native_emission_ms, "C0_execution": native_execution_ms,
                "total": total.elapsed().as_millis().to_string()
            },
            "vm": vm, "reference": reference.1,
            "C0": {
                "image_bytes": native.len(), "image_sha256": sha256(&native),
                "exit_code": 0, "stdout": "42", "stderr_bytes": 0,
                "process_joined": true
            },
            "cleanup": {
                "vm_and_reference_live_counters_zero": true,
                "native_scope": "joined process exit; no same-process arena recovery assertion"
            },
            "scope": "public in-memory GraphRepository authoring/export, host build/link/strict load/preparation, VM/reference/C0 execution",
            "public_cli_file_authoring": false, "public_disk_fixture": false,
            "selfhost_generations_or_emission": false,
            "full_capacity_acceptance": false
        })
    );
    Ok(())
}
