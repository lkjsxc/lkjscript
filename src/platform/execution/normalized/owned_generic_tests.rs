//! Native owned generics, independently expected scalar results, and source execution.
use super::*;

#[test]
fn owned_generics_cell_scalar_extremes() {
    let cell_storage = super::super::owned_i64_cell::StorageObservation::start();
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-generics.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    let d = declaration_named(&source, "main");
    let control = ExecutionControl::uncancelled();
    for n in [0, 255, 128, -257, i64::MIN, i64::MAX] {
        let (actual, work) = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke(d, vec![NormalizedValue::I64(n)], None, &control)
            .unwrap();
        assert_eq!(actual, NormalizedValue::I64(n));
        assert_eq!(work.live_call_frames_after, 0);
        assert_eq!(cell_storage.live(), (0, 0));
        let (actual, work) = NormalizedReferenceInterpreter::new(
            &source,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(d, vec![NormalizedValue::I64(n)], None, &control)
        .unwrap();
        assert_eq!(actual, NormalizedValue::I64(n));
        assert_eq!(work.live_call_frames_after, 0);
    }
}

#[test]
fn owned_checks_preserve_uncalled_ordinary_generic_aggregate_entry() {
    let source = byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create ordinary
  (function create size (visibility public)
    (type-parameter create T)
    (parameter create xs (type (list T)))
    (returns I64) (effect pure) (body (i64 7)))
  (function create keep (visibility public)
    (type-parameter create T)
    (parameter create xs (type (list T)))
    (returns (list T)) (effect pure) (body (local xs)))))
declarations.end
"#,
    )
    .unwrap();
    let program = prepare_snapshot(&source);
    let i64_type =
        crate::platform::kernel::encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
            .unwrap()
            .0;
    let list_type = crate::platform::kernel::encode_type_object(
        &TypeObject::new(TypeForm::List { item: i64_type }).unwrap(),
    )
    .unwrap()
    .0;
    assert!(
        !program.types.contains_key(&list_type),
        "raw applications are not enumerated by preparation"
    );
    let control = ExecutionControl::uncancelled();
    for (name, expected) in [
        ("size", NormalizedValue::I64(7)),
        (
            "keep",
            NormalizedValue::list(vec![NormalizedValue::I64(1)]).unwrap(),
        ),
    ] {
        let d = declaration_named(&source, name);
        let argument = || vec![NormalizedValue::list(vec![NormalizedValue::I64(1)]).unwrap()];
        assert_eq!(
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .invoke_entry(
                    super::super::prepare::NormalizedEntryPoint::InstantiatedFunction(
                        program.function(d).unwrap(),
                        Arc::from([i64_type])
                    ),
                    argument(),
                    None,
                    &control
                )
                .unwrap()
                .0,
            expected
        );
        assert_eq!(
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground()
            )
            .invoke_instantiated(d, &[i64_type], argument(), &control)
            .unwrap()
            .0,
            expected
        );
    }
}

#[test]
fn owned_witness_cell_selects_exact_same_self_implementation() {
    let text = [
        include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
        include_str!("../../../../tests/fixtures/owned-witness-cell.lkjc"),
        include_str!("../../../../tests/fixtures/owned-witness-cell-consumer.lkjc"),
    ]
    .join("\n");
    let source = byte_buffer_tests::author_only(&text).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let control = ExecutionControl::uncancelled();
    let main = declaration_named(&source, "main");
    let alternate = declaration_named(&source, "alternate");
    for n in [128, i64::MIN, i64::MAX] {
        assert_eq!(
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .invoke(main, vec![NormalizedValue::I64(n)], None, &control)
                .unwrap()
                .0,
            NormalizedValue::I64(n)
        );
        assert_eq!(
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground()
            )
            .invoke(main, vec![NormalizedValue::I64(n)], None, &control)
            .unwrap()
            .0,
            NormalizedValue::I64(n)
        );
    }
    assert_eq!(
        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke(alternate, vec![], None, &control)
            .unwrap()
            .0,
        NormalizedValue::I64(99)
    );
    assert_eq!(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(alternate, vec![], None, &control)
            .unwrap()
            .0,
        NormalizedValue::I64(99)
    );
}

#[test]
fn owned_witness_deep_tail_forwarding_restores_exact_selection() {
    let source = byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-recursion.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-recursion-consumer.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "deep-main");
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        let observation = super::super::owned_i64_cell::StorageObservation::start();
        let (value, depth, transfers) = if reference {
            let (value, work) = NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(entry, vec![NormalizedValue::I64(i64::MIN)], None, &control)
            .unwrap();
            (value, work.maximum_call_depth, work.tail_transfers)
        } else {
            let (value, work) = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .invoke(entry, vec![NormalizedValue::I64(i64::MIN)], None, &control)
                .unwrap();
            (value, work.maximum_call_depth, work.tail_transfers)
        };
        assert_eq!(value, NormalizedValue::I64(i64::MIN));
        assert!(depth < 16, "reference={reference} depth={depth}");
        assert!(
            transfers >= 16384,
            "reference={reference} transfers={transfers}"
        );
        assert_eq!(observation.live(), (0, 0));
    }
}

#[test]
fn owned_checks_reject_phantom_external_arguments_in_both_evaluators() {
    use super::super::prepare::NormalizedEntryPoint;
    let source = byte_buffer_tests::author_only(&format!("{}\n{}", include_str!("../../../../tests/fixtures/owned-generics.lkjc"), r#"declarations.begin
(units (module create external-phantom
  (external create phantom-add (visibility public) (implementation core.i64.add)
    (type-parameter create T)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (function create unused-buffer (visibility private) (parameter create b (type ByteBuffer) (use consume)) (returns Unit) (effect pure) (body (unit)))))
declarations.end"#)).unwrap();
    let program = prepare_snapshot(&source);
    let d = declaration_named(&source, "phantom-add");
    let control = ExecutionControl::uncancelled();
    for (ty, accepted) in [
        (TypeForm::I64, true),
        (TypeForm::ByteBuffer, false),
        (TypeForm::OwnedI64Cell, false),
    ] {
        let ty = crate::platform::kernel::encode_type_object(&TypeObject::new(ty).unwrap())
            .unwrap()
            .0;
        let args = || vec![NormalizedValue::I64(1), NormalizedValue::I64(2)];
        let actual = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke_entry(
                NormalizedEntryPoint::InstantiatedFunction(
                    program.function(d).unwrap(),
                    Arc::from([ty]),
                ),
                args(),
                None,
                &control,
            );
        let reference = NormalizedReferenceInterpreter::new(
            &source,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke_instantiated(d, &[ty], args(), &control);
        if accepted {
            assert_eq!(actual.unwrap().0, NormalizedValue::I64(3));
            assert_eq!(reference.unwrap().0, NormalizedValue::I64(3));
        } else {
            assert!(actual.is_err());
            assert!(reference.is_err());
        }
    }
}

#[test]
fn owned_witness_oracle_and_independent_reference_reject_unused_method_faults() {
    use crate::platform::kernel::*;
    let source = byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell-consumer.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap();
    assert!(memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let schema = Arc::new(
        super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&source]).unwrap(),
    );
    let entry = declaration_named(&source, "alternate"); // No update method invoked here.
    let selected = declaration_named(&source, "Alternate");
    let update = declaration_named(&source, "update");
    let read = declaration_named(&source, "read");
    for fault in [
        "missing",
        "wrong-method",
        "wrong-self",
        "task",
        "wrong-mode",
        "wrong-result",
        "wrong-constraint",
    ] {
        let mut invalid = source.clone();
        if ["missing", "wrong-method", "wrong-self"].contains(&fault) {
            let OwnerRecord::Declaration(d) = invalid
                .owners
                .get_mut(&OwnerKey::Declaration(selected.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
                unreachable!()
            };
            match fault {
                "missing" => {
                    i.methods.remove(1);
                }
                "wrong-method" => i.methods[1].function = read,
                "wrong-self" => {
                    i.self_type = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
                        .unwrap()
                        .0
                }
                _ => unreachable!(),
            }
        } else if fault == "wrong-constraint" {
            for owner in invalid.owners.values_mut() {
                if let OwnerRecord::TypeParameter(p) = owner
                    && p.name.as_str() == "Self"
                {
                    p.constraints = TypeParameterConstraints::None;
                }
            }
        } else {
            let OwnerRecord::Declaration(d) = invalid
                .owners
                .get_mut(&OwnerKey::Declaration(update.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            let DeclarationPayload::Function(f) = &mut d.payload else {
                unreachable!()
            };
            if fault == "task" {
                f.effect = FunctionEffect::Task {
                    effect_parameters: vec![],
                    requirements: vec![],
                };
            } else if fault == "wrong-result" {
                f.result = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
                    .unwrap()
                    .0;
            } else {
                let id = f.parameters[1];
                let OwnerRecord::Parameter(p) =
                    invalid.owners.get_mut(&OwnerKey::Parameter(id)).unwrap()
                else {
                    unreachable!()
                };
                p.use_mode = ParameterUse::Borrow;
            }
        }
        assert!(
            !memory_reference::accepts(&invalid),
            "independent oracle: {fault}"
        );
        assert!(validate_full(&invalid).is_err(), "kernel: {fault}");
        // Preserve the admitted schema to exercise source dispatch itself. This test-only
        // corruption reader intentionally bypasses production source admission.
        let reader = FaultedReferenceRead {
            source: &invalid,
            schema: Arc::clone(&schema),
        };
        let failure = NormalizedReferenceInterpreter::from_reader(
            &reader,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(entry, vec![], None, &ExecutionControl::uncancelled());
        assert!(
            failure.is_err(),
            "independent source dispatch accepted {fault}"
        );
    }
}
