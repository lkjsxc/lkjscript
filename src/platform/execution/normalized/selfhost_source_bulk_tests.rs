//! New byte primitives are proven on independent programs before compiler adoption.
use super::artifact_support::run_native;
use super::*;
const SOURCE: &str = include_str!("../../../../tests/fixtures/bulk-byte-intrinsics.lkjc");

pub(super) fn verify_stages(
    left_name: &str,
    left: &[u8],
    right_name: &str,
    right: &[u8],
) -> Check<()> {
    let source = byte_buffer_tests::author_only(SOURCE)?;
    let case = case_from_snapshot(&source)?;
    // Obtain the first emitted outputs before creating an independent C0 oracle.
    let first = raw_outputs(
        &format!("{left_name}/bulk-minimum"),
        left,
        &case,
        "bulk-minimum",
        FOCUSED_DEADLINE,
    )?;
    let host = host_oracle(&case)?;
    for (index, (name, expected)) in [
        ("minimum", Some(i64::MIN)),
        ("negative", Some(-1)),
        ("zero", Some(0)),
        ("positive", Some(0x0102030405060708)),
        ("maximum", Some(i64::MAX)),
        ("negative-start", None),
        ("reversed", None),
        ("beyond-end", None),
        ("maximum-start", None),
        ("word-negative", None),
        ("word-truncated", None),
        ("word-overflow", None),
    ]
    .into_iter()
    .enumerate()
    {
        let target = format!("bulk-{name}");
        let left_output = if index == 0 {
            RawOutputs {
                elf: first.elf.clone(),
                artifact: first.artifact.clone(),
            }
        } else {
            raw_outputs(
                &format!("{left_name}/{target}"),
                left,
                &case,
                &target,
                FOCUSED_DEADLINE,
            )?
        };
        let right_output = raw_outputs(
            &format!("{right_name}/{target}"),
            right,
            &case,
            &target,
            FOCUSED_DEADLINE,
        )?;
        equal_bytes(
            &left_output.elf,
            &right_output.elf,
            "bulk stage ELF identity",
        )?;
        equal_bytes(
            &left_output.artifact,
            &right_output.artifact,
            "bulk stage artifact identity",
        )?;
        let entry = function_named(&case.source, "bulk-bytes", name)?;
        let host_native = native_image(&host.program, entry)?;
        for (producer, output) in [(left_name, left_output), (right_name, right_output)] {
            let label = format!("{producer}/{target}");
            let loaded = strict_artifact(&label, &output.artifact, &case, &host)?;
            let program = NormalizedProgram::prepare(loaded)
                .map_err(|error| format!("bulk artifact: {error:?}"))?;
            let control = ExecutionControl::uncancelled();
            let policy = NormalizedRunPolicy::foreground();
            let vm = NormalizedVm::for_test(&program, policy)
                .invoke(entry, vec![], None, &control)
                .map(|r| r.0);
            let reference =
                NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
                    .invoke(entry, vec![], None, &control)
                    .map(|r| r.0);
            for result in [vm, reference] {
                if let Some(expected) = expected {
                    equal(
                        result.map_err(|error| format!("{label}: evaluator: {error:?}"))?,
                        NormalizedValue::I64(expected),
                        &label,
                    )?;
                } else {
                    let error = result
                        .err()
                        .ok_or_else(|| format!("{label}: invalid bounds accepted"))?;
                    equal(
                        error.class,
                        crate::platform::execution::ExecutionFailureClass::Trap,
                        &label,
                    )?;
                    equal(error.code.as_str(), "normalized_bytes_range", &label)?;
                }
            }
            for (engine, image) in [
                (producer, output.elf.as_slice()),
                ("C0", host_native.as_slice()),
            ] {
                let actual = run_native(
                    &format!("{engine}/{target}/execute"),
                    image,
                    &[],
                    FOCUSED_DEADLINE,
                )?;
                if let Some(expected) = expected {
                    equal(actual.status.code(), Some(0), &label)?;
                    equal_bytes(&actual.stdout, expected.to_string().as_bytes(), &label)?;
                    equal_bytes(&actual.stderr, &[], &label)?;
                } else {
                    equal(actual.status.code(), Some(70), &label)?;
                    equal_bytes(&actual.stdout, &[], &label)?;
                    equal_bytes(&actual.stderr, b"normalized_bytes_range\n", &label)?;
                }
            }
        }
    }
    let target = "bulk-binary";
    let output_left = raw_outputs(
        &format!("{left_name}/{target}"),
        left,
        &case,
        target,
        FOCUSED_DEADLINE,
    )?;
    let output_right = raw_outputs(
        &format!("{right_name}/{target}"),
        right,
        &case,
        target,
        FOCUSED_DEADLINE,
    )?;
    equal_bytes(
        &output_left.elf,
        &output_right.elf,
        "bulk binary ELF identity",
    )?;
    let entry = function_named(&case.source, "bulk-bytes", "binary")?;
    let c0 = native_image(&host.program, entry)?;
    for (producer, output) in [(left_name, output_left), (right_name, output_right)] {
        let loaded = strict_artifact(
            &format!("{producer}/{target}"),
            &output.artifact,
            &case,
            &host,
        )?;
        let program = NormalizedProgram::prepare(loaded)
            .map_err(|error| format!("binary artifact: {error:?}"))?;
        for input in [
            Vec::new(),
            vec![0, 255, 128, 42],
            (0..131_073).map(|i| (i % 251) as u8).collect(),
        ] {
            let mut expected = i64::MIN.to_le_bytes().to_vec();
            expected.extend_from_slice(&input);
            expected.extend_from_slice(&i64::MAX.to_le_bytes());
            let control = ExecutionControl::uncancelled();
            let policy = NormalizedRunPolicy::foreground();
            let args = vec![NormalizedValue::bytes(input.as_slice())];
            let vm = NormalizedVm::for_test(&program, policy)
                .invoke(entry, args.clone(), None, &control)
                .map_err(|error| format!("binary VM: {error:?}"))?;
            let reference =
                NormalizedReferenceInterpreter::new(&case.source, &host.program, policy)
                    .invoke(entry, args, None, &control)
                    .map_err(|error| format!("binary reference: {error:?}"))?;
            for value in [vm.0, reference.0] {
                equal(
                    filter_value(&value)?,
                    (0, expected.as_slice()),
                    "bulk binary expected bytes",
                )?;
            }
            for image in [output.elf.as_slice(), c0.as_slice()] {
                equal_bytes(
                    &run_filter("bulk binary execution", image, &input, 0, FOCUSED_DEADLINE)?,
                    &expected,
                    "bulk binary native bytes",
                )?;
            }
        }
    }
    eprintln!(
        "raw source bulk intrinsic stages={left_name},{right_name} scalar_cases=12 binary_sizes=3 output_modes=2 independent_engines=vm,canonical-reference,c0-native,source-native"
    );
    Ok(())
}

#[test]
fn selfhost_source_bulk_intrinsics_match_four_engines_and_canonical_artifacts() -> Check<()> {
    let image = bootstrap_compiler()?;
    verify_stages("C1-first", image, "C1-repeat", image)
}

#[test]
fn selfhost_source_bulk_intrinsics_reject_unused_forged_contracts_in_both_modes() -> Check<()> {
    let source = byte_buffer_tests::author_only(SOURCE)?;
    for (name, parameter, wrong_mode) in
        [("append", 3, true), ("word", 1, true), ("read", 1, false)]
    {
        let mut changed = source.clone();
        let declaration = function_named(&changed, "bulk-bytes", name)?.declaration;
        let parameter = match changed.owners.get(&OwnerKey::Declaration(declaration)) {
            Some(OwnerRecord::Declaration(record)) => match &record.payload {
                kernel::DeclarationPayload::External(external) => *external
                    .parameters
                    .get(parameter)
                    .ok_or("intrinsic parameter absent")?,
                _ => return Err("intrinsic is not external".to_owned()),
            },
            _ => return Err("intrinsic owner absent".to_owned()),
        };
        let bytes = changed
            .types
            .iter()
            .find_map(|(id, ty)| matches!(ty.form, kernel::TypeForm::Bytes).then_some(*id))
            .ok_or("fixture Bytes missing")?;
        match changed.owners.get_mut(&OwnerKey::Parameter(parameter)) {
            Some(OwnerRecord::Parameter(record)) => {
                if wrong_mode {
                    record.use_mode = kernel::ParameterUse::Unrestricted;
                } else {
                    record.ty = bytes;
                }
            }
            _ => return Err("parameter owner absent".to_owned()),
        }
        let carrier = graph::packet(&changed);
        admission_layer_result("read-nominal", &carrier, 0)?;
        for mode in [SourceOutputMode::Elf, SourceOutputMode::CanonicalArtifact] {
            let request =
                raw_source::request(&carrier, target_named(&source, "bulk-minimum")?, mode)
                    .map_err(|error| format!("forged request: {error:?}"))?;
            run_filter(
                &format!("forged-{name}/{mode:?}"),
                bootstrap_compiler()?,
                &request,
                19,
                FOCUSED_DEADLINE,
            )?;
        }
        ensure(
            kernel::validate_full(&changed).is_err(),
            "independent kernel accepted forged intrinsic contract",
        )?;
    }
    Ok(())
}
