//! Copyable compound source controls, nested under selfhost_source_scalar_tests.
//! The native source compiler receives only the complete canonical carrier and
//! caller-selected TargetId. Independent Rust target preparation occurs AFTER
//! source-driven compilation and is never supplied to that compiler.
use super::*;

#[path = "selfhost_source_custody_tests.rs"]
mod custody;

const COMPOUND_SOURCE: &str =
    include_str!("../../../../tests/fixtures/selfhost-source-compound.lkjc");

fn compound_fixture() -> &'static KernelSnapshot {
    static SOURCE: OnceLock<KernelSnapshot> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let source = byte_buffer_tests::author_only(COMPOUND_SOURCE).unwrap();
        // This unannotated intermediate must be inferred by the source compiler.
        // Supplying only declared TypeRecords cannot serve as its type registry.
        let inferred = kernel::TypeObject::new(kernel::TypeForm::StructuralRecord {
            fields: vec![
                kernel::StructuralTypeField {
                    name: kernel::Name::new("transient-count").unwrap(),
                    ty: scalar_type(&source, kernel::TypeForm::I64),
                },
                kernel::StructuralTypeField {
                    name: kernel::Name::new("transient-data").unwrap(),
                    ty: scalar_type(&source, kernel::TypeForm::Bytes),
                },
            ],
        })
        .unwrap();
        let digest = kernel::encode_type_object(&inferred).unwrap().0;
        assert!(
            !source.types.contains_key(&digest),
            "intermediate record control accidentally supplied its inferred descriptor"
        );
        source
    })
}

fn compound_parameter_mut<'a>(
    source: &'a mut KernelSnapshot,
    external: &str,
    index: usize,
) -> &'a mut kernel::ParameterRecord {
    let parameter = external_mut(source, external).parameters[index];
    let OwnerRecord::Parameter(record) = source
        .owners
        .get_mut(&OwnerKey::Parameter(parameter))
        .unwrap()
    else {
        panic!("compound external parameter");
    };
    record
}

fn compound_call_arguments(source: &KernelSnapshot, function: &str) -> Vec<ExpressionId> {
    let body = fixture_function(source, function).body;
    let E::Call { arguments, .. } = &expression(source, body).operation else {
        panic!("compound fixture direct call");
    };
    arguments.clone()
}

fn compound_cases() -> &'static [Case] {
    static CASES: OnceLock<Vec<Case>> = OnceLock::new();
    CASES.get_or_init(|| {
        let base = compound_fixture();
        let mut cases = vec![Case::new("compound-baseline", base.clone(), 0, Some(42))];

        let mut source = base.clone();
        external_mut(&mut source, "unused-bytes-get").implementation =
            kernel::ImplementationName::new("core.bytes.unknown").unwrap();
        cases.push(Case::new(
            "unused-compound-unknown-intrinsic",
            source,
            13,
            None,
        ));

        let mut source = base.clone();
        let boolean = scalar_type(&source, kernel::TypeForm::Bool);
        external_mut(&mut source, "unused-bytes-get").result = boolean;
        cases.push(Case::new("unused-compound-wrong-result", source, 19, None));

        let mut source = base.clone();
        let integer = scalar_type(&source, kernel::TypeForm::I64);
        compound_parameter_mut(&mut source, "unused-bytes-get", 0).ty = integer;
        cases.push(Case::new(
            "unused-compound-wrong-parameter",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let removed = external_mut(&mut source, "unused-bytes-get")
            .parameters
            .pop()
            .unwrap();
        assert!(
            source
                .owners
                .remove(&OwnerKey::Parameter(removed))
                .is_some()
        );
        cases.push(Case::new("unused-compound-wrong-arity", source, 19, None));

        for (name, mode) in [
            ("unused-copyable-borrow", kernel::ParameterUse::Borrow),
            ("unused-copyable-consume", kernel::ParameterUse::Consume),
        ] {
            let mut source = base.clone();
            compound_parameter_mut(&mut source, "unused-bytes-get", 0).use_mode = mode;
            cases.push(Case::new(name, source, 19, None));
        }

        let mut source = base.clone();
        let boolean = scalar_type(&source, kernel::TypeForm::Bool);
        external_mut(&mut source, "unused-list-get").result = boolean;
        cases.push(Case::new(
            "unused-generic-template-wrong-result",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let integer = scalar_type(&source, kernel::TypeForm::I64);
        let list = scalar_type(&source, kernel::TypeForm::List { item: integer });
        compound_parameter_mut(&mut source, "unused-list-get", 0).ty = list;
        cases.push(Case::new(
            "unused-generic-template-erased-binder",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let binder = external_mut(&mut source, "unused-list-size").type_parameters[0];
        let parameter_type = scalar_type(
            &source,
            kernel::TypeForm::TypeParameter { parameter: binder },
        );
        external_mut(&mut source, "unused-list-size").result = parameter_type;
        cases.push(Case::new(
            "unused-generic-size-result-is-parameter",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let body = fixture_function(&source, "unused-list-call").body;
        let E::Call { type_arguments, .. } = &mut expression_mut(&mut source, body).operation
        else {
            panic!("generic call");
        };
        type_arguments.clear();
        cases.push(Case::new(
            "unused-generic-call-missing-substitution",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let integer = scalar_type(&source, kernel::TypeForm::I64);
        let body = fixture_function(&source, "unused-list-call").body;
        let E::Call { type_arguments, .. } = &mut expression_mut(&mut source, body).operation
        else {
            panic!("generic call");
        };
        type_arguments.push(integer);
        cases.push(Case::new(
            "unused-generic-call-extra-substitution",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let boolean = scalar_type(&source, kernel::TypeForm::Bool);
        let body = fixture_function(&source, "unused-list-call").body;
        let E::Call { type_arguments, .. } = &mut expression_mut(&mut source, body).operation
        else {
            panic!("generic call");
        };
        type_arguments[0] = boolean;
        cases.push(Case::new(
            "unused-generic-call-outside-closed-i64-profile",
            source,
            13,
            None,
        ));

        let mut source = base.clone();
        let arguments = compound_call_arguments(&source, "unused-list-call");
        expression_mut(&mut source, arguments[1]).operation = E::Bool { value: false };
        cases.push(Case::new(
            "unused-generic-call-wrong-index",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let arguments = compound_call_arguments(&source, "unused-list-call");
        let E::List { items, .. } = &expression(&source, arguments[0]).operation else {
            panic!("generic call list");
        };
        let first = items[0];
        expression_mut(&mut source, first).operation = E::Bool { value: false };
        cases.push(Case::new(
            "unused-list-element-type-mismatch",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let arguments = compound_call_arguments(&source, "unused-bytes-call");
        let E::Call { arguments, .. } = &expression(&source, arguments[0]).operation else {
            panic!("text conversion call");
        };
        let literal = arguments[0];
        let E::Text { value } = expression(&source, literal).operation.clone() else {
            panic!("text conversion literal");
        };
        expression_mut(&mut source, literal).operation = E::StaticText { value };
        cases.push(Case::new(
            "static-text-is-not-text-argument",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let body = fixture_function(&source, "unused-field").body;
        let E::Field { selector, .. } = &mut expression_mut(&mut source, body).operation else {
            panic!("unused structural field");
        };
        *selector = kernel::FieldSelector::Structural(kernel::Name::new("absent").unwrap());
        cases.push(Case::new(
            "unused-structural-field-missing",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let initial = entry_initializer(&source, "label");
        expression_mut(&mut source, initial).operation = E::I64 { value: 7 };
        cases.push(Case::new(
            "compound-local-type-reaches-call",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        let entry = fixture_function(&source, "entry").body;
        let E::Let { body, .. } = &expression(&source, entry).operation else {
            panic!("compound entry let");
        };
        let E::Sequence { items } = &expression(&source, *body).operation else {
            panic!("compound entry sequence");
        };
        let E::If { when_false, .. } = &expression(&source, *items.last().unwrap()).operation
        else {
            panic!("compound entry branch");
        };
        let untaken = *when_false;
        expression_mut(&mut source, untaken).operation = E::Text {
            value: kernel::TextValue::Inline {
                text: "wrong untaken result".to_owned(),
            },
        };
        cases.push(Case::new(
            "untaken-compound-branch-type-mismatch",
            source,
            19,
            None,
        ));

        let mut source = base.clone();
        insert_unused_type(&mut source, kernel::TypeForm::F64);
        let unsupported =
            Case::new("unused-f64-type-still-unsupported", source, 13, None).rejected_at_e1(13);
        assert_same_state_different_carrier(&cases[0], &unsupported);
        cases.push(unsupported);
        cases
    })
}

pub(super) fn active_stage_cases() -> impl Iterator<Item = &'static Case> {
    compound_cases().iter().chain(custody::custody_cases())
}

#[test]
fn selfhost_source_compound_e2_checks_complete_intrinsic_templates_and_calls() {
    let inspector = native_inspector();
    for case in compound_cases() {
        case.assert_e1();
        let value = super::super::invoke(
            "source-scalar",
            "inspect",
            vec![NormalizedValue::bytes(case.carrier.as_slice())],
        );
        assert_eq!(
            super::super::field(&value, "diagnostic"),
            &NormalizedValue::I64(case.diagnostic),
            "case={}",
            case.name
        );
        let NormalizedValue::Bytes(image) = super::super::field(&value, "image") else {
            panic!("compound inspector result");
        };
        let expected: &[u8] = if case.diagnostic == 0 {
            &case.carrier[48..80]
        } else {
            &[]
        };
        assert_eq!(image.as_ref(), expected, "case={}", case.name);
        assert_eq!(
            run_filter(inspector, case, &case.carrier, "LKJSG002"),
            expected
        );
        if case.diagnostic != 0 {
            // Rejection survives the request/entry/lowering boundary and cannot
            // accidentally emit an ELF after a diagnostic-only failure.
            assert!(
                run_filter(native_compiler(), case, &case.request, "LKJSRQ01/profile2").is_empty()
            );
        }
    }
}

fn assert_command_oracles(source: &KernelSnapshot, function: &str, expected: i64, image: &[u8]) {
    let output = execute(image, &[]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, expected.to_string().as_bytes());

    // Host preparation is an independent behavioral oracle, after the native
    // source compiler has already emitted and executed its result.
    let program = prepare_snapshot(source);
    let entry = declaration_reference(source, function);
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(&program, policy)
        .invoke(entry, vec![], None, &control)
        .unwrap()
        .0;
    let reference = NormalizedReferenceInterpreter::new(source, &program, policy)
        .invoke(entry, vec![], None, &control)
        .unwrap()
        .0;
    assert_eq!(vm, NormalizedValue::I64(expected));
    assert_eq!(reference, NormalizedValue::I64(expected));
    let host = crate::platform::compiler::elf_x86_64::compile_function(
        &program,
        program.function(entry).unwrap().0 as usize,
    )
    .unwrap()
    .0;
    let output = execute(&host, &[]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, expected.to_string().as_bytes());
}

#[test]
fn selfhost_source_compound_e3_executes_inferred_records_and_source_mutations() {
    let base = compound_fixture();
    let baseline = &compound_cases()[0];
    let compiler = native_compiler();
    let image = run_filter(compiler, baseline, &baseline.request, "LKJSRQ01/profile2");
    assert_command_oracles(base, "entry", 42, &image);
    let value = super::super::invoke(
        "source-elf",
        "compile",
        vec![NormalizedValue::bytes(baseline.request.as_slice())],
    );
    assert_eq!(
        super::super::field(&value, "diagnostic"),
        &NormalizedValue::I64(0)
    );
    let NormalizedValue::Bytes(expected_image) = super::super::field(&value, "image") else {
        panic!("compound source compiler image");
    };
    assert_eq!(image.as_slice(), expected_image.as_ref());

    let mut changed = base.clone();
    let list = entry_initializer(&changed, "numbers");
    let E::List { items, .. } = &expression(&changed, list).operation else {
        panic!("compound numbers");
    };
    let selected = items[1];
    expression_mut(&mut changed, selected).operation = E::I64 { value: 43 };
    let case = Case::new("compound-source-mutation", changed.clone(), 0, Some(43));
    assert_ne!(case.carrier_sha256, baseline.carrier_sha256);
    let changed_image = run_filter(compiler, &case, &case.request, "LKJSRQ01/profile2");
    assert_ne!(image, changed_image);
    assert_command_oracles(&changed, "entry", 43, &changed_image);

    let other = Case::new("compound-other-target", base.clone(), 0, Some(17))
        .select(target_id(base, "compound-other"));
    assert_eq!(other.carrier, baseline.carrier);
    assert_ne!(other.request, baseline.request);
    let image = run_filter(compiler, &other, &other.request, "LKJSRQ01/profile2");
    assert_command_oracles(base, "other-entry", 17, &image);
}

fn binary_case(source: &KernelSnapshot) -> Case {
    // Binary programs have an explicit byte expectation below, not a fictional
    // scalar result. Preserve the existing scalar constructor's stronger guard.
    let selected_target = target_id(source, "compound-binary");
    let carrier = graph::packet(source);
    let request = source_request(&carrier, selected_target);
    Case {
        name: "compound-binary-target",
        carrier_sha256: sha256(&carrier),
        request_sha256: sha256(&request),
        carrier,
        selected_target,
        request,
        diagnostic: 0,
        result: None,
        e1_diagnostic: 0,
        e1: OnceLock::new(),
    }
}

#[test]
fn selfhost_source_compound_e3_reuses_tail_frames_through_lexical_contexts() {
    let source = compound_fixture();
    let case = Case::new("compound-tail-target", source.clone(), 0, Some(42))
        .select(target_id(source, "compound-tail"));
    case.assert_e1();
    // Both inner If arms run during these 500,000 steps. The recursive Call is
    // reached through an outer If, a Let body, and the final Sequence position.
    // The native run makes accidental ordinary CALL growth observable; the
    // independent evaluator observations also check that their frames stay flat.
    let image = run_filter(native_compiler(), &case, &case.request, "LKJSRQ01/profile2");
    let output = execute(&image, &[]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, b"42");

    let program = prepare_snapshot(source);
    let entry = declaration_reference(source, "tail-entry");
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(&program, policy)
        .invoke(entry, vec![], None, &control)
        .unwrap();
    let reference = NormalizedReferenceInterpreter::new(source, &program, policy)
        .invoke(entry, vec![], None, &control)
        .unwrap();
    assert_eq!(vm.0, NormalizedValue::I64(42));
    assert_eq!(reference.0, NormalizedValue::I64(42));
    assert!(
        vm.1.maximum_call_depth <= 8,
        "VM depth={}",
        vm.1.maximum_call_depth
    );
    assert!(
        reference.1.maximum_call_depth <= 8,
        "reference depth={}",
        reference.1.maximum_call_depth,
    );
    assert_eq!(vm.1.live_call_frames_after, 0);
    assert_eq!(reference.1.live_call_frames_after, 0);

    let host = crate::platform::compiler::elf_x86_64::compile_function(
        &program,
        program.function(entry).unwrap().0 as usize,
    )
    .unwrap()
    .0;
    let output = execute(&host, &[]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, b"42");
}

#[test]
fn selfhost_source_compound_e3_binary_entry_matches_independent_bytes_and_hashes() {
    let source = compound_fixture();
    let case = binary_case(source);
    let image = run_filter(native_compiler(), &case, &case.request, "LKJSRQ01/profile2");
    let program = prepare_snapshot(source);
    let entry = declaration_reference(source, "binary-entry");
    let host = crate::platform::compiler::elf_x86_64::compile_function(
        &program,
        program.function(entry).unwrap().0 as usize,
    )
    .unwrap()
    .0;
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    for input in [&b""[..], &b"\0\xff\xc3\xa9Z"[..]] {
        let mut expected = vec![0, 255];
        expected.extend_from_slice("é".as_bytes());
        expected.extend_from_slice(input);
        expected.extend_from_slice(blake3::hash(input).as_bytes());
        expected.extend_from_slice(&blake3::derive_key("lkjscript compound control", input));
        let arguments = vec![NormalizedValue::bytes(input)];
        let vm = NormalizedVm::for_test(&program, policy)
            .invoke(entry, arguments.clone(), None, &control)
            .unwrap()
            .0;
        let reference = NormalizedReferenceInterpreter::new(source, &program, policy)
            .invoke(entry, arguments, None, &control)
            .unwrap()
            .0;
        for value in [vm, reference] {
            assert_eq!(
                super::super::field(&value, "diagnostic"),
                &NormalizedValue::I64(0)
            );
            let NormalizedValue::Bytes(bytes) = super::super::field(&value, "image") else {
                panic!("compound binary result");
            };
            assert_eq!(bytes.as_ref(), expected.as_slice());
        }
        for (producer, executable) in [("source", image.as_slice()), ("Rust", host.as_slice())] {
            let output = native_filter_tests::execute_filter(executable, input, &[], 3);
            assert_eq!(
                output.status.code(),
                Some(0),
                "{producer}: {:?}",
                output.stderr
            );
            assert!(output.stderr.is_empty(), "{producer}");
            assert_eq!(output.stdout, expected, "{producer}");
        }
    }
}
