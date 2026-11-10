//! Reuse the complete independent scalar/compound/custody case inventory on
//! actual emitted compiler generations. No host-admitted target IR is supplied.
use super::artifact_support::run_native;
use super::*;

pub(super) fn verify(stage: &str, compiler: &[u8]) -> Check<()> {
    active_controls(stage, compiler)?;
    super::bulk::verify_stage(stage, compiler)?;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    super::callable::verify_stage(stage, compiler)?;
    Ok(())
}

fn active_controls(stage: &str, compiler: &[u8]) -> Check<()> {
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
            let output = run_filter(
                &format!("{stage}/{label}"),
                compiler,
                &request,
                case.diagnostic,
                WHOLE_SOURCE_DEADLINE,
            )?;
            if case.diagnostic != 0 {
                continue;
            }
            match mode {
                SourceOutputMode::CanonicalArtifact => {
                    let loaded = load_artifact(&output)
                        .map_err(|error| format!("{stage}/{label}: strict load: {error:?}"))?;
                    loaded
                        .require_current_admission(|| Ok(()))
                        .map_err(|error| format!("{stage}/{label}: admission: {error:?}"))?;
                }
                SourceOutputMode::Elf => {
                    let actual = run_native(
                        &format!("{stage}/{label}/execute"),
                        &output,
                        &[],
                        WHOLE_SOURCE_DEADLINE,
                    )?;
                    match case
                        .result
                        .ok_or_else(|| format!("{label}: missing behavioral expectation"))?
                    {
                        ScalarResult::Value(expected) => {
                            equal(actual.status.code(), Some(0), &label)?;
                            equal_bytes(&actual.stdout, expected.to_string().as_bytes(), &label)?;
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
    ensure(
        positive > 0 && negative > 0,
        "empty active-profile stage coverage",
    )?;
    eprintln!(
        "raw source active-profile stage={stage} cases={} positives={positive} negatives={negative} modes=elf,artifact complete=true input=raw-source host_target_ir=false",
        names.len()
    );
    binary_controls(stage, compiler)?;
    Ok(())
}

fn binary_controls(stage: &str, compiler: &[u8]) -> Check<()> {
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
        let output = raw_outputs(
            &format!("{stage}/{label}-binary"),
            compiler,
            &case,
            target,
            WHOLE_SOURCE_DEADLINE,
        )?;
        // Compile the independent C0 oracle after the source compiler finishes.
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
            let program_image = run_filter(
                &format!("{stage}/{target}"),
                compiler,
                &request,
                0,
                WHOLE_SOURCE_DEADLINE,
            )?;
            run_scalar(
                &format!("{stage}/{target}/execute"),
                &program_image,
                expected,
                WHOLE_SOURCE_DEADLINE,
            )?;
        }
        let entry = super::super::declaration_reference(&case.source, "binary-entry");
        let host_native = native_image(&host.program, entry)?;
        let loaded = strict_artifact(
            &format!("{stage}/{label}-binary"),
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
                expected
                    .extend_from_slice(&blake3::derive_key("lkjscript compound control", input));
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
                (stage, output.elf.as_slice()),
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
    Ok(())
}

#[test]
fn selfhost_source_artifact_active_profile_controls_with_bootstrap() -> Check<()> {
    let image = bootstrap_compiler()?;
    active_controls("C1", image)
}
