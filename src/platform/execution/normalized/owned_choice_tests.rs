//! Owned sums return the selected payload, including a rejected original owner.
use super::*;

const SOURCE: &str = r#"declarations.begin
(units
  (module create recoverable
    (external create create-cell (visibility private) (implementation core.cell.create)
      (parameter create n (type I64)) (returns OwnedI64Cell))
    (external create extract (visibility private) (implementation core.cell.extract)
      (parameter create c (type OwnedI64Cell) (use consume)) (returns I64))
    (function create attempt (visibility public) (effect pure)
      (parameter create allowed (type Bool))
      (parameter create input (type OwnedI64Cell) (use consume))
      (returns (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
      (body (if (local allowed)
        (choose-owned (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
          (case accepted) (call extract (local input)))
        (choose-owned (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
          (case rejected) (local input)))))
    (function create main (visibility public) (effect pure)
      (parameter create n (type I64)) (parameter create allowed (type Bool)) (returns I64)
      (body (let
        (binding c (type OwnedI64Cell) (call create-cell (local n)))
        (binding outcome (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
          (call attempt (local allowed) (local c)))
        (in (match-owned (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
          (local outcome)
          (case accepted (binding value (type I64)) (in (local value)))
          (case rejected (binding original (type OwnedI64Cell)) (in (call extract (local original)))))))))
    (function create abandoned (visibility public) (effect pure) (returns Unit)
      (body (let
        (binding c (type OwnedI64Cell) (call create-cell (i64 42)))
        (binding outcome (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell)))
          (call attempt (bool false) (local c)))
        (in (unit)))))))
declarations.end"#;

pub(super) fn source() -> crate::platform::kernel::KernelSnapshot {
    byte_buffer_tests::author_only(SOURCE).unwrap()
}

#[test]
fn owned_choice_both_cases_preserve_result_and_release_storage() {
    let source = source();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    crate::platform::kernel::validate_full(&source).unwrap();
    let program = prepare_snapshot(&source);
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        let composites = super::super::owned_product::StorageObservation::start();
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        for n in [i64::MIN, 0, 128, i64::MAX] {
            for allowed in [false, true] {
                let entry = declaration_named(&source, "main");
                let args = vec![NormalizedValue::I64(n), NormalizedValue::Bool(allowed)];
                let actual = if reference {
                    NormalizedReferenceInterpreter::new(
                        &source,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .invoke(entry, args, None, &control)
                    .unwrap()
                    .0
                } else {
                    NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                        .invoke(entry, args, None, &control)
                        .unwrap()
                        .0
                };
                assert_eq!(actual, NormalizedValue::I64(n));
                assert_eq!(composites.live(), (0, 0));
                assert_eq!(cells.live(), (0, 0));
                composites.assert_transfers_preserve_allocations();
            }
        }
        let entry = declaration_named(&source, "abandoned");
        let actual = if reference {
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(entry, vec![], None, &control)
            .unwrap()
            .0
        } else {
            NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                .invoke(entry, vec![], None, &control)
                .unwrap()
                .0
        };
        assert_eq!(actual, NormalizedValue::Unit);
        assert_eq!(composites.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }
}

#[test]
fn owned_choice_rejects_missing_case_duplicate_use_and_foreign_scope() {
    for input in [
        SOURCE.replace(
            "(case rejected (binding original (type OwnedI64Cell)) (in (call extract (local original))))",
            "",
        ),
        SOURCE.replace(
            "(case rejected) (local input)",
            "(case rejected) (call create-cell (i64 5))",
        ),
        SOURCE.replace(
            "(in (call extract (local original)))",
            "(in (sequence (call extract (local original)) (call extract (local original))))",
        ),
        SOURCE.replace("(in (local value))", "(in (call extract (local original)))"),
        SOURCE.replace(
            "(case accepted (binding value (type I64))",
            "(case unknown (binding value (type I64))",
        ),
    ] {
        assert_ne!(input, SOURCE);
        assert!(byte_buffer_tests::author_only(&input).is_err(), "{input}");
    }
}

#[test]
fn owned_choice_signature_type_cannot_hide_in_graph_19() {
    let input = r#"declarations.begin
(units (module create old
  (function create relay (visibility private) (effect pure)
    (parameter create value (type (owned-choice (case accepted I64) (case rejected OwnedI64Cell))) (use consume))
    (returns (owned-choice (case accepted I64) (case rejected OwnedI64Cell))) (body (local value)))))
declarations.end"#;
    let mut source = byte_buffer_tests::author_only(input).unwrap();
    crate::platform::kernel::validate_full(&source).unwrap();
    source.root.graph_contract_version = 19;
    for owner in source.owners.values_mut() {
        owner.set_encoding_for_edit(19);
        crate::platform::kernel::encode_owner(owner).unwrap();
    }
    let errors = crate::platform::kernel::validate_full(&source).unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "kernel_choice_generation"),
        "{errors:?}"
    );
    assert!(!crate::platform::kernel::memory_reference::accepts(&source));
}

#[test]
fn owned_choice_selected_arm_failure_cleans_up_and_does_not_poison_reused_execution() {
    let literal = SOURCE
        .replacen(
            "    (function create attempt",
            "    (external create divide (visibility private) (implementation core.i64.divide)\n      (parameter create lhs (type I64)) (parameter create rhs (type I64)) (returns I64))\n    (function create attempt",
            1,
        )
        .replacen(
            "(parameter create allowed (type Bool)) (returns I64)",
            "(parameter create allowed (type Bool)) (parameter create fail (type Bool)) (returns I64)",
            1,
        )
        .replacen(
            "(in (local value))",
            "(in (if (local fail) (call divide (local value) (i64 0)) (local value)))",
            1,
        )
        .replacen(
            "(in (call extract (local original)))",
            "(in (if (local fail) (call divide (i64 1) (i64 0)) (call extract (local original))))",
            1,
        );
    let source = byte_buffer_tests::author_only(&literal).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "main");
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::foreground();
    let vm = NormalizedVm::new(&program, policy);
    let interpreter = NormalizedReferenceInterpreter::new(&source, &program, policy);
    for reference in [false, true] {
        let composites = super::super::owned_product::StorageObservation::start();
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        for n in [i64::MIN, 0, i64::MAX] {
            for allowed in [false, true] {
                // The same prepared executor succeeds immediately after either arm fails.
                // In the rejected arm, failure precedes extraction of the live owned cell.
                for fail in [true, false] {
                    let arguments = vec![
                        NormalizedValue::I64(n),
                        NormalizedValue::Bool(allowed),
                        NormalizedValue::Bool(fail),
                    ];
                    let result = if reference {
                        interpreter
                            .invoke(entry, arguments, None, &control)
                            .map(|p| p.0)
                    } else {
                        vm.invoke(entry, arguments, None, &control).map(|p| p.0)
                    };
                    if fail {
                        assert_eq!(
                            result.unwrap_err().code,
                            if reference {
                                "reference_integer_division"
                            } else {
                                "normalized_integer_division"
                            }
                        );
                    } else {
                        assert_eq!(result.unwrap(), NormalizedValue::I64(n));
                    }
                    assert_eq!(cells.live(), (0, 0));
                    assert_eq!(composites.live(), (0, 0));
                    composites.assert_transfers_preserve_allocations();
                }
            }
        }
    }
}
