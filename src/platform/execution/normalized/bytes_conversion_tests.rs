//! Byte construction and checked UTF-8 through two independently admitted evaluators.
use super::*;
use crate::platform::execution::normalized::value::NormalizedRecord;

fn fixture() -> crate::platform::kernel::KernelSnapshot {
    let temporary = tempfile::tempdir().unwrap();
    let empty = empty_normalized_snapshot(b"native-byte-conversion");
    let repository = GraphRepository::create(&temporary.path().join("bytes"), &empty, None)
        .unwrap()
        .repository;
    let base = repository.view_current().unwrap().revision();
    let request = format!(
        r#"request base={base}
declarations.begin
(units (module create conversions
  (external create construct (visibility public) (implementation core.bytes.from-list)
    (parameter create octets (type (list I64))) (returns Bytes))
  (external create decode (visibility public) (implementation core.bytes.to-text-result)
    (parameter create bytes (type Bytes)) (returns (record (valid Bool) (value Text))))
  (external create strict (visibility public) (implementation core.bytes.to-text)
    (parameter create bytes (type Bytes)) (returns Text))))
declarations.end
"#
    );
    let decoded =
        crate::platform::control::decode_compact_change("byte-conversions", request.as_bytes())
            .unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value
}

fn decoded(valid: bool, text: &str) -> NormalizedValue {
    NormalizedValue::Record(NormalizedRecord::Structural {
        fields: std::sync::Arc::new(vec![
            (
                Name::new("valid".to_owned()).unwrap(),
                NormalizedValue::Bool(valid),
            ),
            (
                Name::new("value".to_owned()).unwrap(),
                NormalizedValue::text(text),
            ),
        ]),
    })
}

#[test]
fn bytes_construct_preserves_every_octet_and_persistent_list_alias() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "construct");
    let control = ExecutionControl::uncancelled();
    for length in [0, 1, 31, 32, 33, 256, 1023, 1024, 1025] {
        let expected = (0..length).map(|n| (n % 256) as u8).collect::<Vec<_>>();
        let original = NormalizedValue::list(
            expected
                .iter()
                .map(|n| NormalizedValue::I64(i64::from(*n)))
                .collect(),
        )
        .unwrap();
        let argument = original.clone();
        let actual = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
            .invoke(declaration, vec![argument.clone()], None, &control)
            .unwrap()
            .0;
        let reference = NormalizedReferenceInterpreter::new(
            &snapshot,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(declaration, vec![argument.clone()], None, &control)
        .unwrap()
        .0;
        assert_eq!(actual, NormalizedValue::bytes(expected.as_slice()));
        assert_eq!(reference, NormalizedValue::bytes(expected.as_slice()));
        assert_eq!(original, argument);
    }
}

#[test]
fn bytes_construct_rejects_out_of_range_without_clamping_or_wrapping() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "construct");
    let control = ExecutionControl::uncancelled();
    for invalid in [i64::MIN, -1, 256, i64::MAX] {
        for position in [0, 32, 64] {
            let mut input = vec![NormalizedValue::I64(65); 65];
            input[position] = NormalizedValue::I64(invalid);
            let arguments = vec![NormalizedValue::list(input).unwrap()];
            let actual = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                .invoke(declaration, arguments.clone(), None, &control)
                .unwrap_err();
            let reference = NormalizedReferenceInterpreter::new(
                &snapshot,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(declaration, arguments, None, &control)
            .unwrap_err();
            assert_eq!(actual.code, "normalized_bytes_octet");
            assert_eq!(reference.code, "reference_bytes_octet");
        }
    }
}

#[test]
fn bytes_decode_preserves_valid_scalars_bom_and_controls_and_rejects_whole_invalid_input() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "decode");
    let strict = declaration_named(&snapshot, "strict");
    let control = ExecutionControl::uncancelled();
    for text in [
        "",
        "A\0Z\r\n",
        "é",
        "日本語",
        "\u{feff}BOM",
        "😀",
        "\u{7f}\u{80}\u{7ff}\u{800}\u{d7ff}\u{e000}\u{ffff}\u{10000}\u{10ffff}",
    ] {
        let arguments = vec![NormalizedValue::bytes(text.as_bytes())];
        let actual = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
            .invoke(declaration, arguments.clone(), None, &control)
            .unwrap()
            .0;
        let reference = NormalizedReferenceInterpreter::new(
            &snapshot,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(declaration, arguments, None, &control)
        .unwrap()
        .0;
        assert_eq!(actual, decoded(true, text));
        assert_eq!(reference, decoded(true, text));
    }
    let invalid: &[&[u8]] = &[
        &[0x80],
        &[0xff],
        &[0xc0, 0xaf],
        &[0xc1, 0xbf],
        &[0xc2],
        &[0xc2, 0x20],
        &[0xe0, 0x80, 0x80],
        &[0xed, 0xa0, 0x80],
        &[0xed, 0xbf, 0xbf],
        &[0xe2, 0x82],
        &[0xf0, 0x80, 0x80, 0x80],
        &[0xf4, 0x90, 0x80, 0x80],
        &[0xf5, 0x80, 0x80, 0x80],
        &[0xf0, 0x90, 0x80],
        &[0x41, 0x80, 0x42],
        &[0x41, 0xe2, 0x82],
    ];
    for bytes in invalid {
        let arguments = vec![NormalizedValue::bytes(*bytes)];
        let actual = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
            .invoke(declaration, arguments.clone(), None, &control)
            .unwrap()
            .0;
        let reference = NormalizedReferenceInterpreter::new(
            &snapshot,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(declaration, arguments.clone(), None, &control)
        .unwrap()
        .0;
        assert_eq!(actual, decoded(false, ""));
        assert_eq!(reference, decoded(false, ""));
        assert_eq!(
            NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                .invoke(strict, arguments.clone(), None, &control)
                .unwrap_err()
                .code,
            "normalized_bytes_utf8"
        );
        assert_eq!(
            NormalizedReferenceInterpreter::new(
                &snapshot,
                &program,
                NormalizedRunPolicy::foreground()
            )
            .invoke(strict, arguments, None, &control)
            .unwrap_err()
            .code,
            "reference_bytes_utf8"
        );
    }
}

#[test]
fn byte_conversions_keep_cancellation_and_allocation_failures_as_errors() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    for (name, input) in [
        (
            "construct",
            NormalizedValue::list(vec![NormalizedValue::I64(65); 2048]).unwrap(),
        ),
        ("decode", NormalizedValue::bytes(vec![65; 2048])),
    ] {
        let declaration = declaration_named(&snapshot, name);
        let control = ExecutionControl::uncancelled();
        control.cancel();
        assert!(
            NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                .invoke(declaration, vec![input.clone()], None, &control)
                .is_err()
        );
        assert!(
            NormalizedReferenceInterpreter::new(
                &snapshot,
                &program,
                NormalizedRunPolicy::foreground()
            )
            .invoke(declaration, vec![input.clone()], None, &control)
            .is_err()
        );
        let mut policy = NormalizedRunPolicy::foreground();
        policy.maximum_allocated_bytes = Some(4096);
        let control = ExecutionControl::uncancelled();
        let actual = NormalizedVm::new(&program, policy)
            .invoke(declaration, vec![input.clone()], None, &control)
            .unwrap_err();
        let reference = NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
            .invoke(declaration, vec![input], None, &control)
            .unwrap_err();
        assert!(actual.code.contains("allocation"), "{}", actual.code);
        assert!(reference.code.contains("allocation"), "{}", reference.code);
    }
}

#[test]
fn bytes_conversion_explicit_core_hosts_agree_with_checked_paths() {
    use crate::platform::execution::normalized::reference::CoreNormalizedReferenceHost;
    use crate::platform::execution::normalized::vm::CoreNormalizedHost;
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let control = ExecutionControl::uncancelled();
    for (name, input, expected) in [
        (
            "construct",
            NormalizedValue::list(vec![NormalizedValue::I64(0), NormalizedValue::I64(255)])
                .unwrap(),
            NormalizedValue::bytes(vec![0, 255]),
        ),
        (
            "decode",
            NormalizedValue::bytes("猫".as_bytes()),
            decoded(true, "猫"),
        ),
        (
            "decode",
            NormalizedValue::bytes(vec![65, 128, 66]),
            decoded(false, ""),
        ),
    ] {
        let declaration = declaration_named(&snapshot, name);
        let vm_observer = std::sync::Mutex::new(None);
        let reference_observer = std::sync::Mutex::new(None);
        let actual = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
            .observing(&vm_observer, &CoreNormalizedHost)
            .invoke(declaration, vec![input.clone()], None, &control)
            .unwrap()
            .0;
        let reference = NormalizedReferenceInterpreter::new(
            &snapshot,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .observing(&reference_observer, &CoreNormalizedReferenceHost)
        .invoke(declaration, vec![input], None, &control)
        .unwrap()
        .0;
        assert_eq!(actual, expected);
        assert_eq!(reference, expected);
    }
}
