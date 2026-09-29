//! Raw byte indexing through independently admitted source and both evaluators.
use super::*;

#[path = "bytes_range_tests.rs"]
mod ranges;

fn fixture() -> crate::platform::kernel::KernelSnapshot {
    let temporary = tempfile::tempdir().unwrap();
    let empty = empty_normalized_snapshot(b"native-byte-index");
    let repository = GraphRepository::create(&temporary.path().join("bytes"), &empty, None)
        .unwrap()
        .repository;
    let base = repository.view_current().unwrap().revision();
    let request = format!(
        r#"request base={base}
declarations.begin
(units
  (module create octets
    (external create get (visibility public) (implementation core.bytes.get)
      (parameter create bytes (type Bytes))
      (parameter create index (type I64))
      (returns I64))
    (external create slice (visibility public) (implementation core.bytes.slice)
      (parameter create bytes (type Bytes))
      (parameter create start (type I64))
      (parameter create end (type I64))
      (returns Bytes))
    (external create copy (visibility public) (implementation core.bytes.copy)
      (parameter create bytes (type Bytes)) (returns Bytes))))
declarations.end
"#
    );
    let decoded =
        crate::platform::control::decode_compact_change("bytes", request.as_bytes()).unwrap();
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

#[test]
fn bytes_slice_exhaustive_small_ranges_agree_with_independent_values() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "slice");
    let control = ExecutionControl::uncancelled();
    let original = (0..=255).collect::<Vec<u8>>();
    for length in 0..=16 {
        let bytes = NormalizedValue::bytes(original[..length].to_vec());
        let NormalizedValue::Bytes(payload) = &bytes else {
            unreachable!()
        };
        for start in 0..=length {
            for end in start..=length {
                let arguments = vec![
                    bytes.clone(),
                    NormalizedValue::I64(start as i64),
                    NormalizedValue::I64(end as i64),
                ];
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
                let expected = NormalizedValue::bytes(original[start..end].to_vec());
                assert_eq!(actual, expected, "{length} {start} {end}");
                assert_eq!(reference, expected, "reference {length} {start} {end}");
                if start != end {
                    let NormalizedValue::Bytes(window) = &actual else {
                        unreachable!()
                    };
                    assert_eq!(window.as_ptr(), payload[start..].as_ptr());
                }
            }
        }
        assert_eq!(bytes, NormalizedValue::bytes(original[..length].to_vec()));
    }
    // Byte ranges are not Unicode ranges: a split UTF-8 scalar is still valid Bytes.
    for (start, end, expected) in [
        (0, 1, vec![0xe7]),
        (1, 3, vec![0x8c, 0xab]),
        (2, 4, vec![0xab, 0]),
    ] {
        let actual = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
            .invoke(
                declaration,
                vec![
                    NormalizedValue::bytes(vec![0xe7, 0x8c, 0xab, 0, 255]),
                    NormalizedValue::I64(start),
                    NormalizedValue::I64(end),
                ],
                None,
                &control,
            )
            .unwrap()
            .0;
        assert_eq!(actual, NormalizedValue::bytes(expected));
    }
}

#[test]
fn bytes_slice_rejects_invalid_ranges_in_both_evaluators() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "slice");
    let control = ExecutionControl::uncancelled();
    for length in [0, 1, 3] {
        for start in [i64::MIN, -1, 0, 1, 3, 4, i64::MAX] {
            for end in [i64::MIN, -1, 0, 1, 3, 4, i64::MAX] {
                if 0 <= start && start <= end && end <= length {
                    continue;
                }
                let arguments = vec![
                    NormalizedValue::bytes(vec![255; length as usize]),
                    NormalizedValue::I64(start),
                    NormalizedValue::I64(end),
                ];
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
                assert_eq!(
                    actual.code, "normalized_bytes_range",
                    "{length} {start} {end}"
                );
                assert_eq!(
                    reference.code, "reference_bytes_range",
                    "{length} {start} {end}"
                );
            }
        }
    }
}

#[test]
fn bytes_get_returns_every_unsigned_octet_without_text_interpretation() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "get");
    let control = ExecutionControl::uncancelled();
    let bytes = NormalizedValue::bytes((0..=255).collect::<Vec<u8>>());
    for expected in 0..=255_i64 {
        let arguments = vec![bytes.clone(), NormalizedValue::I64(expected)];
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
        assert_eq!(actual, NormalizedValue::I64(expected));
        assert_eq!(reference, NormalizedValue::I64(expected));
    }
    // The same retained carrier still contains every byte after every read.
    assert_eq!(
        bytes,
        NormalizedValue::bytes((0..=255).collect::<Vec<u8>>())
    );
}

#[test]
fn bytes_get_rejects_empty_negative_and_excessive_indices_in_both_evaluators() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "get");
    let control = ExecutionControl::uncancelled();
    for bytes in [vec![], vec![0], vec![0, 255, 128]] {
        for index in [i64::MIN, -1, bytes.len() as i64, i64::MAX] {
            let arguments = vec![
                NormalizedValue::bytes(bytes.clone()),
                NormalizedValue::I64(index),
            ];
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
            assert_eq!(actual.code, "normalized_bytes_index");
            assert_eq!(reference.code, "reference_bytes_index");
        }
    }
}
