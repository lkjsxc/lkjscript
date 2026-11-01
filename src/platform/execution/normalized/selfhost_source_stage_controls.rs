//! Reuse the complete independent scalar/compound/custody case inventory on
//! actual emitted compiler generations. No host-admitted target IR is supplied.
use super::artifact_support::run_native;
use super::*;

pub(super) fn verify(left_name: &str, left: &[u8], right_name: &str, right: &[u8]) -> Check<()> {
    let cases = super::super::cases()
        .iter()
        .chain(super::super::compound::active_stage_cases());
    let mut names = BTreeSet::new();
    let mut positive = 0usize;
    let mut negative = 0usize;
    for case in cases {
        ensure(names.insert(case.name), "duplicate emitted-stage case name")?;
        if case.diagnostic == 0 {
            positive += 1;
        } else {
            negative += 1;
        }
        for mode in [SourceOutputMode::Elf, SourceOutputMode::CanonicalArtifact] {
            let request = raw_source::request(&case.carrier, case.selected_target, mode)
                .map_err(|error| format!("{}: stage request: {error:?}", case.name))?;
            let label = format!("active-profile/{}/{mode:?}", case.name);
            // Both producers finish before any strict output loader is invoked.
            let outputs = [
                run_filter(
                    &format!("{left_name}/{label}"),
                    left,
                    &request,
                    case.diagnostic,
                    WHOLE_SOURCE_DEADLINE,
                )?,
                run_filter(
                    &format!("{right_name}/{label}"),
                    right,
                    &request,
                    case.diagnostic,
                    WHOLE_SOURCE_DEADLINE,
                )?,
            ];
            equal_bytes(
                &outputs[0],
                &outputs[1],
                &format!("{label}: generation equality"),
            )?;
            if case.diagnostic != 0 {
                continue;
            }
            for (name, output) in [left_name, right_name].into_iter().zip(&outputs) {
                match mode {
                    SourceOutputMode::CanonicalArtifact => {
                        let loaded = load_artifact(output)
                            .map_err(|error| format!("{name}/{label}: strict load: {error:?}"))?;
                        loaded
                            .require_current_admission(|| Ok(()))
                            .map_err(|error| format!("{name}/{label}: admission: {error:?}"))?;
                    }
                    SourceOutputMode::Elf => {
                        let actual = run_native(
                            &format!("{name}/{label}/execute"),
                            output,
                            &[],
                            WHOLE_SOURCE_DEADLINE,
                        )?;
                        match case
                            .result
                            .ok_or_else(|| format!("{label}: missing behavioral expectation"))?
                        {
                            ScalarResult::Value(expected) => {
                                equal(actual.status.code(), Some(0), &label)?;
                                equal_bytes(
                                    &actual.stdout,
                                    expected.to_string().as_bytes(),
                                    &label,
                                )?;
                                equal_bytes(&actual.stderr, &[], &label)?;
                            }
                            ScalarResult::Trap(code) => {
                                equal(actual.status.code(), Some(70), &label)?;
                                equal_bytes(&actual.stdout, &[], &label)?;
                                equal_bytes(
                                    &actual.stderr,
                                    format!("normalized_{code}\n").as_bytes(),
                                    &label,
                                )?;
                            }
                        }
                    }
                }
            }
        }
    }
    ensure(
        positive > 0 && negative > 0,
        "empty active-profile stage coverage",
    )?;
    eprintln!(
        "raw source active-profile stages={left_name},{right_name} cases={} positives={positive} negatives={negative} modes=elf,artifact complete=true input=raw-source host_target_ir=false",
        names.len()
    );
    binary_controls(left_name, left, right_name, right)?;
    super::bulk::verify_stages(left_name, left, right_name, right)
}

fn binary_controls(left_name: &str, left: &[u8], right_name: &str, right: &[u8]) -> Check<()> {
    for (label, source, target) in [
        (
            "compound",
            include_str!("../../../../tests/fixtures/selfhost-source-compound.lkjc"),
            "compound-binary",
        ),
        (
            "custody",
            include_str!("../../../../tests/fixtures/selfhost-source-custody.lkjc"),
            "custody-binary",
        ),
    ] {
        let snapshot = byte_buffer_tests::author_only(source)?;
        let case = case_from_snapshot(&snapshot)?;
        let left_output = raw_outputs(
            &format!("{left_name}/{label}-binary"),
            left,
            &case,
            target,
            WHOLE_SOURCE_DEADLINE,
        )?;
        let right_output = raw_outputs(
            &format!("{right_name}/{label}-binary"),
            right,
            &case,
            target,
            WHOLE_SOURCE_DEADLINE,
        )?;
        equal_bytes(
            &left_output.elf,
            &right_output.elf,
            "binary generation ELF equality",
        )?;
        equal_bytes(
            &left_output.artifact,
            &right_output.artifact,
            "binary generation artifact equality",
        )?;
        // C0 target compilation occurs only after both source generations finish.
        let host = host_oracle(&case)?;
        let selections: &[(&str, i64)] = if label == "compound" {
            &[("compound-other", 17), ("compound-tail", 42)]
        } else {
            &[
                ("custody-scopes", 42),
                ("custody-borrowed", 1),
                ("custody-sequence", 42),
                ("custody-tail", 42),
            ]
        };
        for &(target, expected) in selections {
            let request = case.request(target, SourceOutputMode::Elf)?;
            let left_program = run_filter(
                &format!("{left_name}/{target}"),
                left,
                &request,
                0,
                WHOLE_SOURCE_DEADLINE,
            )?;
            let right_program = run_filter(
                &format!("{right_name}/{target}"),
                right,
                &request,
                0,
                WHOLE_SOURCE_DEADLINE,
            )?;
            equal_bytes(
                &left_program,
                &right_program,
                "selected target generation equality",
            )?;
            for (name, image) in [(left_name, left_program), (right_name, right_program)] {
                run_scalar(
                    &format!("{name}/{target}/execute"),
                    &image,
                    expected,
                    WHOLE_SOURCE_DEADLINE,
                )?;
            }
        }
        let entry = super::super::declaration_reference(&case.source, "binary-entry");
        let host_native = native_image(&host.program, entry)?;
        for (name, output) in [(left_name, left_output), (right_name, right_output)] {
            let loaded = strict_artifact(
                &format!("{name}/{label}-binary"),
                &output.artifact,
                &case,
                &host,
            )?;
            let program = NormalizedProgram::prepare(loaded)
                .map_err(|error| format!("{label}: artifact preparation: {error:?}"))?;
            for input in [&b""[..], &b"\0\xff\xc3\xa9Z"[..]] {
                let mut expected = vec![0, 255];
                if label == "compound" {
                    expected.extend_from_slice("é".as_bytes());
                    expected.extend_from_slice(input);
                    expected.extend_from_slice(blake3::hash(input).as_bytes());
                    expected.extend_from_slice(&blake3::derive_key(
                        "lkjscript compound control",
                        input,
                    ));
                } else {
                    expected.extend_from_slice(&[0, 255]);
                    expected.extend_from_slice(input);
                }
                let arguments = vec![NormalizedValue::bytes(input)];
                let control = ExecutionControl::uncancelled();
                let policy = NormalizedRunPolicy::foreground();
                let vm = NormalizedVm::for_test(&program, policy)
                    .invoke(entry, arguments.clone(), None, &control)
                    .map_err(|error| format!("{label}: artifact VM: {error:?}"))?;
                let reference =
                    NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
                        .invoke(entry, arguments, None, &control)
                        .map_err(|error| format!("{label}: canonical reference: {error:?}"))?;
                for (value, frames) in [
                    (vm.0, vm.1.live_call_frames_after),
                    (reference.0, reference.1.live_call_frames_after),
                ] {
                    equal(
                        filter_value(&value)?,
                        (0, expected.as_slice()),
                        "binary expected evaluator bytes",
                    )?;
                    equal(frames, 0, "binary frames reclaimed")?;
                }
                for (producer, executable) in [
                    (name, output.elf.as_slice()),
                    ("C0", host_native.as_slice()),
                ] {
                    let actual = run_filter(
                        &format!("{producer}/{label}-binary/execute"),
                        executable,
                        input,
                        0,
                        WHOLE_SOURCE_DEADLINE,
                    )?;
                    equal_bytes(&actual, &expected, "binary expected native bytes")?;
                }
            }
        }
    }
    Ok(())
}

#[test]
fn selfhost_source_artifact_active_profile_controls_with_bootstrap() -> Check<()> {
    // A focused harness check, explicitly not evidence of a maintained C2/C3
    // fixed point. The whole-source test calls verify with actual C2 and C3.
    let image = bootstrap_compiler()?;
    verify("C1-first", image, "C1-repeat", image)
}
