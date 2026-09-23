//! Raw byte indexing through independently admitted source and both evaluators.
use super::*;

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
      (returns I64))))
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
