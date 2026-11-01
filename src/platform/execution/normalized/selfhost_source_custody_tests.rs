//! Direct ByteBuffer source custody, nested under the compound source controls.
//! Requests contain only a complete source carrier and caller-selected TargetId.
//! Host preparation is an independent oracle after source-driven compilation.
use super::*;

#[path = "selfhost_source_custody_evidence_tests.rs"]
mod evidence;

const CUSTODY_SOURCE: &str =
    include_str!("../../../../tests/fixtures/selfhost-source-custody.lkjc");

fn custody_fixture() -> &'static KernelSnapshot {
    static SOURCE: OnceLock<KernelSnapshot> = OnceLock::new();
    SOURCE.get_or_init(|| {
        let source = byte_buffer_tests::author_only(CUSTODY_SOURCE).unwrap();
        assert!(kernel::memory_reference::accepts(&source));
        source
    })
}

fn custody_binding(source: &KernelSnapshot, function: &str, name: &str) -> BindingId {
    let E::Let { bindings, .. } =
        &expression(source, fixture_function(source, function).body).operation
    else {
        panic!("custody function must have a root Let");
    };
    bindings
        .iter()
        .find_map(
            |binding| match &source.owners[&OwnerKey::Binding(*binding)] {
                OwnerRecord::Binding(record) if record.name.as_str() == name => Some(*binding),
                _ => None,
            },
        )
        .unwrap()
}

fn custody_let_body(source: &KernelSnapshot, function: &str) -> ExpressionId {
    let E::Let { body, .. } =
        &expression(source, fixture_function(source, function).body).operation
    else {
        panic!("custody function must have a root Let");
    };
    *body
}

fn custody_last(source: &KernelSnapshot, sequence: ExpressionId) -> ExpressionId {
    let E::Sequence { items } = &expression(source, sequence).operation else {
        panic!("custody sequence");
    };
    *items.last().unwrap()
}

fn custody_argument(source: &KernelSnapshot, call: ExpressionId, index: usize) -> ExpressionId {
    let E::Call { arguments, .. } = &expression(source, call).operation else {
        panic!("custody call");
    };
    arguments[index]
}

fn custody_redirect(source: &mut KernelSnapshot, expression_id: ExpressionId, binding: BindingId) {
    assert!(matches!(
        expression(source, expression_id).operation,
        E::Local { .. }
    ));
    expression_mut(source, expression_id).operation = E::Local {
        value: kernel::LocalValueReference::LexicalBinding(binding),
    };
}

fn custody_wrap_local(source: &mut KernelSnapshot, expression_id: ExpressionId, seed: &[u8]) {
    let original = expression(source, expression_id).operation.clone();
    assert!(matches!(original, E::Local { .. }));
    let child = ExpressionId::migrate(seed, 0);
    assert!(
        source
            .owners
            .insert(
                OwnerKey::Expression(child),
                OwnerRecord::Expression(kernel::ExpressionRecord::new(child, original).unwrap()),
            )
            .is_none()
    );
    expression_mut(source, expression_id).operation = E::Sequence { items: vec![child] };
}

fn custody_function_parameter(
    source: &KernelSnapshot,
    function: &str,
    index: usize,
) -> crate::platform::semantic_id::ParameterId {
    fixture_function(source, function).parameters[index]
}

fn custody_parameter_mode(
    source: &mut KernelSnapshot,
    parameter: crate::platform::semantic_id::ParameterId,
    mode: kernel::ParameterUse,
) {
    let OwnerRecord::Parameter(record) = source
        .owners
        .get_mut(&OwnerKey::Parameter(parameter))
        .unwrap()
    else {
        panic!("custody parameter");
    };
    record.use_mode = mode;
}

fn custody_rejection(
    name: &'static str,
    source: KernelSnapshot,
    diagnostic: i64,
    memory_rejects: bool,
) -> Case {
    if memory_rejects {
        assert!(
            !kernel::memory_reference::accepts(&source),
            "independent memory oracle accepted {name}"
        );
    }
    Case::new(name, source, diagnostic, None)
}

pub(super) fn custody_cases() -> &'static [Case] {
    static CASES: OnceLock<Vec<Case>> = OnceLock::new();
    CASES.get_or_init(|| {
        let base = custody_fixture();
        let mut cases = vec![Case::new("custody-baseline", base.clone(), 0, Some(42))];

        let mut source = base.clone();
        let first = custody_binding(&source, "unused-double-consume", "first");
        let last = custody_last(&source, custody_let_body(&source, "unused-double-consume"));
        let argument = custody_argument(&source, last, 0);
        custody_redirect(&mut source, argument, first);
        cases.push(custody_rejection("unused-double-consume", source, 19, true));

        let mut source = base.clone();
        let first = custody_binding(&source, "unused-after-freeze", "first");
        let argument =
            custody_argument(&source, custody_let_body(&source, "unused-after-freeze"), 0);
        custody_redirect(&mut source, argument, first);
        cases.push(custody_rejection(
            "unused-borrow-after-freeze",
            source,
            19,
            true,
        ));

        let mut source = base.clone();
        let first = custody_binding(&source, "unused-maybe-moved", "first");
        let argument =
            custody_argument(&source, custody_let_body(&source, "unused-maybe-moved"), 0);
        custody_redirect(&mut source, argument, first);
        cases.push(custody_rejection(
            "unused-borrow-after-branch-maybe-move",
            source,
            19,
            true,
        ));

        for (function, name) in [
            ("unused-nonlocal-consume", "non-local-consume-argument"),
            ("unused-nonlocal-borrow", "non-local-borrow-argument"),
        ] {
            let mut source = base.clone();
            let argument = custody_argument(&source, custody_let_body(&source, function), 0);
            custody_wrap_local(&mut source, argument, name.as_bytes());
            cases.push(custody_rejection(name, source, 19, true));
        }

        let mut source = base.clone();
        let binding = custody_binding(&source, "unused-annotation", "buffer");
        let OwnerRecord::Binding(record) =
            source.owners.get_mut(&OwnerKey::Binding(binding)).unwrap()
        else {
            panic!("custody annotation binding");
        };
        record.declared_type = None;
        cases.push(custody_rejection(
            "unannotated-owned-binding",
            source,
            19,
            true,
        ));

        let mut source = base.clone();
        let original = custody_binding(&source, "unused-return-transfer", "original");
        let argument = custody_argument(
            &source,
            custody_let_body(&source, "unused-return-transfer"),
            0,
        );
        custody_redirect(&mut source, argument, original);
        cases.push(custody_rejection(
            "returned-owner-does-not-revive-source",
            source,
            19,
            true,
        ));

        let mut source = base.clone();
        let first = custody_binding(&source, "unused-alias", "first");
        let second_argument =
            custody_argument(&source, custody_let_body(&source, "unused-alias"), 1);
        custody_redirect(&mut source, second_argument, first);
        cases.push(custody_rejection(
            "same-local-in-two-consuming-arguments",
            source,
            19,
            true,
        ));

        let mut source = base.clone();
        let first = custody_binding(&source, "unused-untaken", "first");
        let E::If { when_false, .. } =
            &expression(&source, custody_let_body(&source, "unused-untaken")).operation
        else {
            panic!("custody untaken branch");
        };
        let last = custody_last(&source, *when_false);
        let argument = custody_argument(&source, last, 0);
        custody_redirect(&mut source, argument, first);
        cases.push(custody_rejection(
            "untaken-branch-double-consume",
            source,
            19,
            true,
        ));

        let mut source = base.clone();
        let E::If { when_false, .. } =
            &expression(&source, fixture_function(&source, "pick").body).operation
        else {
            panic!("custody owner selection");
        };
        let false_arm = *when_false;
        expression_mut(&mut source, false_arm).operation = E::Unit {};
        cases.push(custody_rejection(
            "branch-result-owner-category-mismatch",
            source,
            19,
            true,
        ));

        let mut source = base.clone();
        let parameter = custody_function_parameter(&source, "unused-owner-result", 0);
        custody_parameter_mode(&mut source, parameter, kernel::ParameterUse::Unrestricted);
        cases.push(custody_rejection(
            "unrestricted-owned-parameter",
            source,
            19,
            true,
        ));

        let mut source = base.clone();
        let buffer = scalar_type(&source, kernel::TypeForm::ByteBuffer);
        external_mut(&mut source, "unused-size").result = buffer;
        cases.push(custody_rejection(
            "borrow-intrinsic-forged-owned-result",
            source,
            19,
            false,
        ));

        let mut source = base.clone();
        let parameter = external_mut(&mut source, "unused-size").parameters[0];
        custody_parameter_mode(&mut source, parameter, kernel::ParameterUse::Consume);
        cases.push(custody_rejection(
            "borrow-intrinsic-forged-consume-mode",
            source,
            19,
            false,
        ));

        // Arbitrary borrowed helper signatures remain outside this active slice.
        // This is an explicit unsupported-family control, not evidence that the
        // compiler admits borrowed results or the wider P_self provenance family.
        let mut source = base.clone();
        let parameter = custody_function_parameter(&source, "unused-owner-result", 0);
        custody_parameter_mode(&mut source, parameter, kernel::ParameterUse::Borrow);
        cases.push(custody_rejection(
            "borrowed-helper-result-unsupported",
            source,
            13,
            true,
        ));
        cases
    })
}

#[test]
fn selfhost_source_custody_e2_checks_unused_and_untaken_ownership() {
    for case in custody_cases() {
        case.assert_e1();
        let value = super::super::super::invoke(
            "source-scalar",
            "inspect",
            vec![NormalizedValue::bytes(case.carrier.as_slice())],
        );
        assert_eq!(
            super::super::super::field(&value, "diagnostic"),
            &NormalizedValue::I64(case.diagnostic),
            "{}",
            case.name
        );
        let NormalizedValue::Bytes(image) = super::super::super::field(&value, "image") else {
            panic!("custody inspector bytes");
        };
        let expected: &[u8] = if case.diagnostic == 0 {
            &case.carrier[48..80]
        } else {
            &[]
        };
        assert_eq!(image.as_ref(), expected, "{}", case.name);
        assert_eq!(
            run_filter(native_inspector(), case, &case.carrier, "LKJSG002"),
            expected
        );
        if case.diagnostic != 0 {
            assert!(
                run_filter(native_compiler(), case, &case.request, "LKJSRQ01/profile2").is_empty()
            );
        }
    }
}

#[test]
fn selfhost_source_custody_e3_executes_buffer_lifetimes_and_source_mutation() {
    let source = custody_fixture();
    let baseline = &custody_cases()[0];
    let baseline_image = run_filter(
        native_compiler(),
        baseline,
        &baseline.request,
        "LKJSRQ01/profile2",
    );
    assert_command_oracles(source, "entry", 42, &baseline_image);
    for (name, target, function, expected) in [
        (
            "custody-lexical-cleanup",
            "custody-scopes",
            "scope-cleanup",
            42,
        ),
        (
            "custody-borrow-tail-fallback",
            "custody-borrowed",
            "fallback-size",
            1,
        ),
        (
            "custody-sequence-temporary-drop",
            "custody-sequence",
            "sequence-cleanup",
            42,
        ),
    ] {
        let case =
            Case::new(name, source.clone(), 0, Some(expected)).select(target_id(source, target));
        let image = run_filter(native_compiler(), &case, &case.request, "LKJSRQ01/profile2");
        let storage =
            crate::platform::execution::normalized::byte_buffer::StorageObservation::start();
        assert_command_oracles(source, function, expected, &image);
        // Only evaluator-owned ByteBuffer storage is observed here. This is not
        // a claim about native mapped arenas, RSS, or phase reclamation.
        assert_eq!(storage.live(), (0, 0), "{function}");
    }

    let mut changed = source.clone();
    let initializer = entry_initializer(&changed, "one");
    let byte = custody_argument(&changed, initializer, 0);
    expression_mut(&mut changed, byte).operation = E::I64 { value: 41 };
    let case = Case::new("custody-source-mutation", changed.clone(), 0, Some(43));
    assert_ne!(case.carrier_sha256, baseline.carrier_sha256);
    let image = run_filter(native_compiler(), &case, &case.request, "LKJSRQ01/profile2");
    assert_ne!(image, baseline_image);
    assert_command_oracles(&changed, "entry", 43, &image);
}

#[test]
fn selfhost_source_custody_e3_transfers_owned_tail_arguments() {
    let source = custody_fixture();
    let case = Case::new("custody-owned-tail", source.clone(), 0, Some(42))
        .select(target_id(source, "custody-tail"));
    let image = run_filter(native_compiler(), &case, &case.request, "LKJSRQ01/profile2");
    let output = execute(&image, &[]);
    assert_eq!(output.status.code(), Some(0), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    assert_eq!(output.stdout, b"42");

    let program = prepare_snapshot(source);
    let entry = declaration_reference(source, "tail-entry");
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        let storage =
            crate::platform::execution::normalized::byte_buffer::StorageObservation::start();
        if reference {
            let (value, observed) = NormalizedReferenceInterpreter::new(source, &program, policy)
                .invoke(entry, vec![], None, &control)
                .unwrap();
            assert_eq!(value, NormalizedValue::I64(42));
            assert!(observed.maximum_call_depth <= 16);
            assert!(observed.tail_transfers >= 16383);
            assert_eq!(observed.live_call_frames_after, 0);
        } else {
            let (value, observed) = NormalizedVm::for_test(&program, policy)
                .invoke(entry, vec![], None, &control)
                .unwrap();
            assert_eq!(value, NormalizedValue::I64(42));
            assert!(observed.maximum_call_depth <= 16);
            assert!(observed.tail_transfers >= 16383);
            assert_eq!(observed.live_call_frames_after, 0);
        }
        assert_eq!(storage.live(), (0, 0));
    }
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
fn selfhost_source_custody_e3_preserves_returned_owners_and_both_branch_values() {
    let source = custody_fixture();
    let selected_target = target_id(source, "custody-binary");
    let carrier = graph::packet(source);
    let request = source_request(&carrier, selected_target);
    let case = Case {
        name: "custody-binary",
        carrier_sha256: sha256(&carrier),
        request_sha256: sha256(&request),
        carrier,
        selected_target,
        request,
        diagnostic: 0,
        result: None,
        e1_diagnostic: 0,
        e1: OnceLock::new(),
    };
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
    for input in [&b""[..], &b"\0\xff*"[..]] {
        let mut expected = vec![0, 255, 0, 255];
        expected.extend_from_slice(input);
        for reference in [false, true] {
            let storage =
                crate::platform::execution::normalized::byte_buffer::StorageObservation::start();
            let arguments = vec![NormalizedValue::bytes(input)];
            let value = if reference {
                NormalizedReferenceInterpreter::new(source, &program, policy)
                    .invoke(entry, arguments, None, &control)
                    .unwrap()
                    .0
            } else {
                NormalizedVm::for_test(&program, policy)
                    .invoke(entry, arguments, None, &control)
                    .unwrap()
                    .0
            };
            assert_eq!(
                super::super::super::field(&value, "diagnostic"),
                &NormalizedValue::I64(0)
            );
            let NormalizedValue::Bytes(bytes) = super::super::super::field(&value, "image") else {
                panic!("custody binary result");
            };
            assert_eq!(bytes.as_ref(), expected.as_slice());
            drop(value);
            assert_eq!(storage.live(), (0, 0));
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

#[test]
fn selfhost_source_custody_e2_keeps_mixed_borrow_helpers_explicitly_unsupported() {
    // This is independently valid source. Profile6 cannot express the exact
    // provenance needed to move an unrelated owner after a borrowed operand.
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-custody-mixed.lkjc"
    ))
    .unwrap();
    assert!(kernel::memory_reference::accepts(&source));
    let case = Case::new("mixed-borrow-helper-unsupported", source, 13, None);
    case.assert_e1();
    let value = super::super::super::invoke(
        "source-scalar",
        "inspect",
        vec![NormalizedValue::bytes(case.carrier.as_slice())],
    );
    assert_eq!(
        super::super::super::field(&value, "diagnostic"),
        &NormalizedValue::I64(13)
    );
    assert!(run_filter(native_inspector(), &case, &case.carrier, "LKJSG002").is_empty());
    assert!(run_filter(native_compiler(), &case, &case.request, "LKJSRQ01/profile2").is_empty());
}
