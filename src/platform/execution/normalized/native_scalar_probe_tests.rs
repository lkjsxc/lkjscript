//! Opt-in Tier 1 measurement: one normalized entry, three runtime modes.
//! No raw compiler generations, emitted-instruction patches or performance gate.
use super::super::*;
use super::{AllocationObservation, PhasePolicy, caller::Mode, verify_observed};
use crate::platform::execution::normalized::prepare::{
    NormalizedFunctionBody, NormalizedInstruction,
};
use crate::platform::execution::normalized::tests::selfhost_tests::{
    artifact, compiler, input, source::source_fixture,
};

const PROBE_SOURCE: &str = include_str!("../../../../tests/fixtures/native-scalar-probe.lkjc");
const SEED: i64 = 4096;
const STEP: i64 = 1024;

fn settings() -> Check<([i64; 2], Duration)> {
    let selected = std::env::var("LKJSCRIPT_NATIVE_SCALAR_PROBE_COUNTS").map_err(|_| {
        "set two explicit counts in LKJSCRIPT_NATIVE_SCALAR_PROBE_COUNTS".to_owned()
    })?;
    let values = selected
        .split(',')
        .map(|word| word.trim().parse::<i64>().map_err(|e| e.to_string()))
        .collect::<Check<Vec<_>>>()?;
    let counts: [i64; 2] = values
        .try_into()
        .map_err(|_| "the probe requires exactly two input-selected counts".to_owned())?;
    ensure(
        counts[0] > 0 && counts[1] > counts[0],
        "probe counts must be positive and increasing",
    )?;
    for count in counts {
        SEED.checked_add(count)
            .ok_or("probe cursor overflows I64")?;
        STEP.checked_mul(count)
            .and_then(|value| SEED.checked_add(value))
            .ok_or("probe expected result overflows I64")?;
    }
    let seconds = std::env::var("LKJSCRIPT_NATIVE_SCALAR_PROBE_DEADLINE_SECONDS")
        .map_err(|_| "set an explicit native probe caller deadline".to_owned())?
        .parse::<u64>()
        .map_err(|e| format!("probe caller deadline: {e}"))?;
    ensure(seconds > 0, "probe caller deadline must be positive")?;
    Ok((counts, Duration::from_secs(seconds)))
}

fn output(value: &NormalizedValue) -> Check<Vec<u8>> {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        return Err("probe oracle result is not a structural record".to_owned());
    };
    equal(fields.len(), 2, "probe oracle result fields")?;
    let diagnostic = fields
        .iter()
        .find(|(name, _)| name.as_str() == "diagnostic")
        .ok_or("probe oracle diagnostic absent")?;
    ensure(
        matches!(diagnostic.1, NormalizedValue::I64(0)),
        "probe oracle diagnostic",
    )?;
    let image = fields
        .iter()
        .find(|(name, _)| name.as_str() == "image")
        .ok_or("probe oracle image absent")?;
    let NormalizedValue::Bytes(bytes) = &image.1 else {
        return Err("probe oracle image is not Bytes".to_owned());
    };
    Ok(bytes.as_ref().to_vec())
}

fn frame_record(row: &AllocationObservation) -> serde_json::Value {
    serde_json::json!({
        "frame": row.frame,
        "status": row.status.to_string(),
        "diagnostic": row.diagnostic.to_string(),
        "actual_output": row.output,
        "actual_output_sha256": sha256(&row.output),
        "allocator_requests_by_handle": row.attempts.map(|value| value.to_string()),
        "attempt_saturation_by_handle": row.attempt_saturation.map(|value| value.to_string()),
        "owned_mapped_bytes": row.owned_mapped_bytes.to_string(),
        "output_slabs_by_handle": row.output_slabs.map(|value| value.to_string()),
        "output_mapped_bytes_by_handle": row.output_bytes.map(|value| value.to_string()),
        "output_roots_live_by_handle": row.output_roots_live,
    })
}

fn invocation_requests(rows: &[AllocationObservation]) -> Check<[u64; 3]> {
    let mut requests = [0; 3];
    for (slot, (frame, owner)) in [(0_u64, 0_usize), (1, 1), (8, 1)].into_iter().enumerate() {
        let row = rows
            .iter()
            .find(|row| row.frame == frame)
            .ok_or("probe invocation absent")?;
        equal(
            row.attempt_saturation,
            [0, 0],
            "probe request observations must be exact",
        )?;
        ensure(row.output_roots_live[owner], "probe output owner absent")?;
        requests[slot] = row.attempts[owner];
    }
    Ok(requests)
}

#[test]
#[ignore = "explicit native allocation measurement; requires caller-selected counts and deadline"]
fn native_scalar_bytes_and_direct_call_allocation_measurement() -> Check<()> {
    let (counts, deadline) = settings()?;
    let preparation = Instant::now();
    let source = byte_buffer_tests::author_only(PROBE_SOURCE)?;
    let program = prepare_snapshot(&source);
    let entry = function_named(&source, "native-scalar-probe", "entry")?;
    let entry_index = program.function(entry).ok_or("probe entry absent")?;
    let identity = program
        .function(function_named(&source, "native-scalar-probe", "identity")?)
        .ok_or("probe identity absent")?;
    let loop_index = program
        .function(function_named(&source, "native-scalar-probe", "loop")?)
        .ok_or("probe loop absent")?;
    let NormalizedFunctionBody::Code(body) = &program.functions[loop_index.0 as usize].body else {
        return Err("probe loop is not code".to_owned());
    };
    let ordinary_identity_calls = body
        .instructions
        .iter()
        .filter(|instruction| {
            matches!(instruction, NormalizedInstruction::Call { function, .. } if *function == identity)
        })
        .count();
    equal(
        ordinary_identity_calls,
        1,
        "probe identity remains an ordinary non-tail call",
    )?;
    let normalized = input::export(&program, entry_index.0 as usize)
        .map_err(|e| format!("probe normalized export: {e:?}"))?;
    let rust_native = native_image(&program, entry)?;
    let maintained_native = artifact::selfhost_native_probe(
        &program,
        entry_index.0 as usize,
        "native-scalar-probe::entry",
    );
    let backend = artifact::selfhost_native_backend();
    // Existing lazy public export checks every owner, supplied type and retirement
    // against the cached accepted compiler. It binds context; it is not compiled here.
    let compiler_carrier = source_fixture::packet(&compiler().0);
    let context = serde_json::json!({
        "schema": "lkjscript.native-scalar-probe-context.v1",
        "scope": "Tier1-normalized-input; no raw-source generations",
        "fixture": {"bytes": PROBE_SOURCE.len(), "sha256": sha256(PROBE_SOURCE.as_bytes())},
        "complete_maintained_source_carrier": {
            "bytes": compiler_carrier.len(), "sha256": sha256(&compiler_carrier),
            "use": "context binding only; complete public inventory includes unused supplied types",
        },
        "normalized_input": {"bytes": normalized.len(), "sha256": sha256(&normalized)},
        "maintained_backend": {"bytes": backend.len(), "sha256": sha256(backend)},
        "rust_image": {"bytes": rust_native.len(), "sha256": sha256(&rust_native)},
        "maintained_image": {"bytes": maintained_native.len(), "sha256": sha256(&maintained_native)},
        "counts": counts.map(|value| value.to_string()),
        "input_seed": SEED.to_string(), "input_step": STEP.to_string(),
        "ordinary_non_tail_identity_call_sites": ordinary_identity_calls,
        "address_space_allowance": null,
        "explicit_caller_deadline_seconds": deadline.as_secs().to_string(),
        "preparation_wall_nanoseconds": preparation.elapsed().as_nanos().to_string(),
        "preparation_scope": "authoring, normalization, emitter/cache work and full carrier context export",
    });
    let context_bytes = serde_json::to_vec(&context).map_err(|e| e.to_string())?;
    let context_sha = sha256(&context_bytes);
    println!("NATIVE_SCALAR_PROBE_CONTEXT {context}");
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let phase_policy = PhasePolicy {
        address_space: None,
        deadline,
    };
    let mut cases = Vec::new();
    let mut comparisons = Vec::new();
    for count in counts {
        let expected_value = STEP
            .checked_mul(count)
            .and_then(|value| SEED.checked_add(value))
            .ok_or("probe expected result extent")?;
        let expected = expected_value.to_le_bytes();
        let mut requests_by_mode = Vec::new();
        for (mode, name) in [(0_i64, "scalar"), (1, "bytes"), (2, "direct-call")] {
            // The complete request is identical across emitters. Mode and count
            // are runtime data; neither changes the compiled entry or its image.
            let mut request = Vec::new();
            for value in [count, SEED, STEP, mode] {
                request.extend_from_slice(&value.to_le_bytes());
            }
            request.push(173); // Complete 17-byte window at [16,33).
            let arguments = vec![NormalizedValue::bytes(request.as_slice())];
            let oracle_started = Instant::now();
            let (vm, vm_work) = NormalizedVm::for_test(&program, policy)
                .invoke(entry, arguments.clone(), None, &control)
                .map_err(|e| format!("probe VM: {e:?}"))?;
            let vm_output = output(&vm)?;
            equal_bytes(
                &vm_output,
                &expected,
                "probe independent VM arithmetic oracle",
            )?;
            let (reference, reference_work) =
                NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .invoke(entry, arguments, None, &control)
                    .map_err(|e| format!("probe reference: {e:?}"))?;
            let reference_output = output(&reference)?;
            equal_bytes(
                &reference_output,
                &expected,
                "probe independent reference arithmetic oracle",
            )?;
            equal(vm_work.live_call_frames_after, 0, "probe VM frame cleanup")?;
            equal(
                reference_work.live_call_frames_after,
                0,
                "probe reference frame cleanup",
            )?;
            let oracle_ns = oracle_started.elapsed().as_nanos().to_string();
            let mut per_emitter = Vec::new();
            let mut mode_requests = Vec::new();
            for (emitter, image) in [
                ("Rust", rust_native.as_slice()),
                ("maintained", maintained_native.as_ref()),
            ] {
                let label = format!("native-scalar-probe/{emitter}/{name}/count-{count}");
                let started = Instant::now();
                let observed = verify_observed(
                    &label,
                    image,
                    &request,
                    &expected,
                    Mode::AllocationIdentity,
                    phase_policy,
                    None,
                )?;
                let elapsed = started.elapsed().as_nanos().to_string();
                let frames: Vec<_> = observed.iter().map(|row| row.frame).collect();
                equal(
                    frames,
                    vec![100, 0, 1, 2, 3, 8, 9, 30, 31, 10, 11],
                    "probe phase frame inventory",
                )?;
                let requests = invocation_requests(&observed)?;
                let final_observation = observed.last().ok_or("probe final observation absent")?;
                equal(
                    final_observation.owned_mapped_bytes,
                    0,
                    "probe final owned mappings",
                )?;
                equal(
                    final_observation.output_slabs,
                    [0, 0],
                    "probe final output slabs",
                )?;
                equal(
                    final_observation.output_bytes,
                    [0, 0],
                    "probe final output mappings",
                )?;
                per_emitter.push(serde_json::json!({
                    "emitter": emitter, "case_label": label, "image_sha256": sha256(image),
                    "context_sha256": context_sha, "input_sha256": sha256(&request),
                    "input": request, "expected_output": expected,
                    "observations": observed.iter().map(frame_record).collect::<Vec<_>>(),
                    "invocation_frames": [0, 1, 8],
                    "invocation_handles": [0, 1, 1],
                    "allocator_requests": requests.map(|value| value.to_string()),
                    "control_wall_nanoseconds": elapsed,
                    "timing_scope": "three invocations, startup, framing, handshakes, independent map walks and successful joining; not invocation-only time",
                    "joined_successfully": true, "stderr_empty": true,
                    "independent_maps_and_output_survival": "passed",
                    "final_owned_mapped_bytes": final_observation.owned_mapped_bytes.to_string(),
                    "production_instructions": "unchanged",
                }));
                mode_requests.push(requests);
            }
            cases.push(serde_json::json!({
                "mode": name, "count": count.to_string(), "context_sha256": context_sha,
                "input": request, "input_sha256": sha256(&request),
                "independent_expected_value": expected_value.to_string(),
                "vm_actual_output": vm_output, "reference_actual_output": reference_output,
                "oracle_wall_nanoseconds": oracle_ns,
                "native": per_emitter,
            }));
            requests_by_mode.push(mode_requests);
        }
        for (emitter_index, emitter) in ["Rust", "maintained"].into_iter().enumerate() {
            let scalar = requests_by_mode[0][emitter_index];
            let delta = |mode: usize| {
                std::array::from_fn::<_, 3, _>(|run| {
                    (i128::from(requests_by_mode[mode][emitter_index][run])
                        - i128::from(scalar[run]))
                    .to_string()
                })
            };
            comparisons.push(serde_json::json!({
                "emitter": emitter, "count": count.to_string(),
                "bytes_minus_scalar_requests": delta(1),
                "direct_call_minus_scalar_requests": delta(2),
                "interpretation": "observed request differences; no performance threshold or byte attribution",
            }));
        }
    }
    println!(
        "NATIVE_SCALAR_PROBE_COMPLETE {}",
        serde_json::json!({
            "schema": "lkjscript.native-scalar-probe.v1", "status": "passed",
            "context_sha256": context_sha, "cases": cases, "comparisons": comparisons,
            "scope": "Tier1-normalized-input; three runtime modes; two explicit counts; two emitters",
            "processes": 12, "invocations_per_process": 3, "frames_per_process": 11,
            "limitations": "no whole-compiler allocation attribution, peak scratch measurement, raw fixed-point proof or deep non-tail stack claim",
        })
    );
    Ok(())
}
