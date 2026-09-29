//! Real native meaning, independent interpretation, owned-box identity and cancellation.
use super::super::prepare::{NormalizedFunctionBody, NormalizedInstruction};
use super::*;
use std::sync::Arc;

fn fixture() -> crate::platform::kernel::KernelSnapshot {
    let temporary = tempfile::tempdir().unwrap();
    let empty = empty_normalized_snapshot(b"terminal-local-values");
    let repository = GraphRepository::create(&temporary.path().join("values"), &empty, None)
        .unwrap()
        .repository;
    let base = repository.view_current().unwrap().revision();
    let request = format!(
        r#"request base={base}
declarations.begin
(units
  (module create values
    (variant create Envelope (visibility public)
      (case create leaf (payload I64))
      (case create wrap (payload Envelope)))
    (function create keep (visibility public)
      (parameter create value (type Envelope))
      (returns Envelope) (effect pure) (body (local value)))
    (function create forever (visibility public)
      (parameter create value (type Envelope))
      (returns Envelope) (effect pure) (body (call forever (local value))))))
declarations.end
"#
    );
    let decoded =
        crate::platform::control::decode_compact_change("values", request.as_bytes()).unwrap();
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

fn nested(program: &NormalizedProgram, depth: usize) -> NormalizedValue {
    let (index, layout) = program
        .variants
        .iter()
        .enumerate()
        .find(|(_, layout)| layout.cases.iter().any(|case| case.name.as_str() == "wrap"))
        .unwrap();
    let leaf = layout
        .cases
        .iter()
        .position(|case| case.name.as_str() == "leaf")
        .unwrap() as u32;
    let wrap = layout
        .cases
        .iter()
        .position(|case| case.name.as_str() == "wrap")
        .unwrap() as u32;
    let layout = super::super::value::VariantLayoutIndex(index as u32, program.value_origin);
    let mut value = NormalizedValue::Variant {
        layout,
        case: leaf,
        payload: Some(Box::new(NormalizedValue::I64(42))),
    };
    for _ in 0..depth {
        value = NormalizedValue::Variant {
            layout,
            case: wrap,
            payload: Some(Box::new(value)),
        };
    }
    value
}

fn payload_addresses(mut value: &NormalizedValue) -> Vec<*const NormalizedValue> {
    let mut addresses = Vec::new();
    while let NormalizedValue::Variant {
        payload: Some(child),
        ..
    } = value
    {
        addresses.push(&**child as *const NormalizedValue);
        value = child;
    }
    assert_eq!(*value, NormalizedValue::I64(42));
    addresses
}

#[test]
fn terminal_move_preserves_owned_box_and_matches_canonical_and_copying_execution() {
    let snapshot = fixture();
    let prepared = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "keep");
    for optimized in [false, true] {
        let mut program = prepared.clone();
        if !optimized {
            // Remove only the derived optimization, retaining the same admitted meaning.
            let index = program.function(declaration).unwrap().0 as usize;
            let NormalizedFunctionBody::Code(code) =
                &mut Arc::make_mut(&mut program.functions)[index].body
            else {
                panic!("native keep must be graph code");
            };
            assert!(
                code.instructions
                    .iter()
                    .any(|i| matches!(i, NormalizedInstruction::MoveLocal(0)))
            );
            for instruction in Arc::make_mut(&mut code.instructions) {
                if let NormalizedInstruction::MoveLocal(local) = instruction {
                    *instruction = NormalizedInstruction::LoadLocal {
                        local: *local,
                        use_mode: crate::platform::kernel::ParameterUse::Unrestricted,
                    };
                }
            }
        }
        let argument = nested(&program, 64);
        let addresses = payload_addresses(&argument);
        assert_eq!(addresses.len(), 65);
        let (actual, observation) = NormalizedVm::new(&program, NormalizedRunPolicy::default())
            .invoke(
                declaration,
                vec![argument],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
        assert_eq!(actual, nested(&program, 64));
        let result_addresses = payload_addresses(&actual);
        assert_eq!(result_addresses.len(), addresses.len());
        assert!(
            result_addresses
                .iter()
                .zip(&addresses)
                .all(|(after, before)| { (after == before) == optimized })
        );
        assert_eq!(
            observation.value_work.local_value_moves,
            u64::from(optimized)
        );
        assert_eq!(
            observation.value_work.local_value_copies,
            u64::from(!optimized)
        );
        assert_eq!(observation.live_locals_after, 0);
        let reference = NormalizedReferenceInterpreter::new(
            &snapshot,
            &program,
            NormalizedRunPolicy::default(),
        )
        .invoke(
            declaration,
            vec![nested(&program, 64)],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap()
        .0;
        assert_eq!(actual, reference);
    }
}

#[test]
fn cancellation_joins_moved_frames_and_the_same_program_remains_reusable() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "forever");
    let sink = std::sync::Mutex::new(None);
    let error = NormalizedVm::new(&program, NormalizedRunPolicy::default())
        .observing_checked(&sink)
        .invoke(
            declaration,
            vec![nested(&program, 32)],
            None,
            &ExecutionControl::cancel_after_checks(1024),
        )
        .unwrap_err();
    assert_eq!(error.code, "execution_cancelled");
    let observation = sink.into_inner().unwrap().unwrap();
    assert!(observation.value_work.local_value_moves > 0);
    assert_eq!(observation.value_work.local_value_copies, 0);
    assert!(observation.tail_transfers > 0);
    assert_eq!(
        (
            observation.live_locals_after,
            observation.live_operands_after,
            observation.live_call_frames_after
        ),
        (0, 0, 0)
    );
    let declaration = declaration_named(&snapshot, "keep");
    let actual = NormalizedVm::new(&program, NormalizedRunPolicy::default())
        .invoke(
            declaration,
            vec![nested(&program, 32)],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap()
        .0;
    assert_eq!(actual, nested(&program, 32));
}

#[test]
fn terminal_move_never_admits_a_foreign_prepared_value() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let foreign = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "keep");
    let sink = std::sync::Mutex::new(None);
    let error = NormalizedVm::new(&program, NormalizedRunPolicy::default())
        .observing_checked(&sink)
        .invoke(
            declaration,
            vec![nested(&foreign, 4)],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
    assert_eq!(error.code, "normalized_value_admission");
    let observation = sink.into_inner().unwrap().unwrap();
    assert_eq!(observation.value_work.local_value_moves, 0);
    assert_eq!(observation.live_locals_after, 0);
}
