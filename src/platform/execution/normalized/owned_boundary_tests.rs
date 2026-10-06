//! Raw, persistence and descriptor entry cannot fabricate ownership or static witnesses.
use super::super::{owned_i64_cell::OwnedI64Cell, value::ValueOrigin};
use super::*;
#[test]
fn owned_cell_raw_ingress_results_and_persistence_fail_closed() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-generics.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    let schema =
        super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&source]).unwrap();
    let cell_type = crate::platform::kernel::encode_type_object(
        &TypeObject::new(TypeForm::OwnedI64Cell).unwrap(),
    )
    .unwrap()
    .0;
    let entry = declaration_named(&source, "extract");
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        let storage = super::super::owned_i64_cell::StorageObservation::start();
        for mode in ["live", "inert", "borrow", "stale", "wrong-carrier"] {
            let owner = OwnedI64Cell::new(ValueOrigin::fresh().unwrap(), i64::MIN);
            let (raw, retained) = match mode {
                "live" => (NormalizedValue::OwnedI64Cell(owner), None),
                "inert" => (NormalizedValue::OwnedI64Cell(owner.clone()), Some(owner)),
                "borrow" => (
                    NormalizedValue::OwnedI64Cell(owner.borrow().unwrap()),
                    Some(owner),
                ),
                "stale" => {
                    let stale = owner.clone();
                    drop(owner);
                    (NormalizedValue::OwnedI64Cell(stale), None)
                }
                "wrong-carrier" => {
                    drop(owner);
                    (
                        NormalizedValue::ByteBuffer(super::super::byte_buffer::ByteBuffer::empty(
                            ValueOrigin::fresh().unwrap(),
                        )),
                        None,
                    )
                }
                _ => unreachable!(),
            };
            assert!(
                super::super::codec::encode_typed(
                    &program,
                    &raw,
                    cell_type,
                    crate::platform::json::JsonLimits::default()
                )
                .is_err()
            );
            assert!(super::super::data_codec::encode_typed(&program, &raw, cell_type).is_err());
            assert!(
                super::super::data_codec_reference::encode_typed(&program, &raw, cell_type)
                    .is_err()
            );
            let result = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, vec![raw], None, &control)
                .map(|v| v.0)
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, vec![raw], None, &control)
                    .map(|v| v.0)
            };
            assert!(result.is_err(), "{reference} {mode}");
            drop(retained);
            assert_eq!(storage.live(), (0, 0));
        }
        let create = declaration_named(&source, "new");
        let result = if reference {
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(create, vec![NormalizedValue::I64(128)], None, &control)
            .map(|v| v.0)
        } else {
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .invoke(create, vec![NormalizedValue::I64(128)], None, &control)
                .map(|v| v.0)
        };
        assert!(
            result.is_err(),
            "owned results cannot escape an ordinary raw entry"
        );
        assert_eq!(storage.live(), (0, 0));
    }
    for input in [b"null".as_slice(), b"128", b"{\"$cell\":128}"] {
        for types in [
            &program as &dyn super::super::value_schema::NormalizedValueSchema,
            &schema,
        ] {
            assert!(
                super::super::codec::decode_typed(
                    types,
                    input,
                    cell_type,
                    crate::platform::json::JsonLimits::default()
                )
                .is_err()
            );
        }
        assert!(super::super::data_codec::decode_typed(&program, input, cell_type).is_err());
        assert!(
            super::super::data_codec_reference::decode_typed(&program, input, cell_type).is_err()
        );
    }
}

#[test]
fn owned_static_witness_templates_cannot_be_erased_at_raw_entry() {
    let source = byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell-consumer.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap();
    let program = prepare_snapshot(&source);
    let d = declaration_named(&source, "produce");
    let cell = crate::platform::kernel::encode_type_object(
        &TypeObject::new(TypeForm::OwnedI64Cell).unwrap(),
    )
    .unwrap()
    .0;
    let control = ExecutionControl::uncancelled();
    assert!(
        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke_entry(
                super::super::prepare::NormalizedEntryPoint::InstantiatedFunction(
                    program.function(d).unwrap(),
                    Arc::from([cell])
                ),
                vec![NormalizedValue::I64(128)],
                None,
                &control
            )
            .is_err()
    );
    assert!(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke_instantiated(d, &[cell], vec![NormalizedValue::I64(128)], &control)
            .is_err()
    );
}

#[test]
fn owned_vm_rechecks_closed_witness_self_and_binding_arity() {
    let source = byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell-consumer.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "main");
    let scalar = declaration_named(&source, "Scalar");
    let i64_type =
        crate::platform::kernel::encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
            .unwrap()
            .0;
    for missing in [false, true] {
        // Deliberately bypass preparation, exercising VM admission itself.
        let mut forged = program.clone();
        let mut changed = 0;
        for function in Arc::make_mut(&mut forged.functions) {
            if function
                .implementation_arguments
                .iter()
                .any(|application| application.implementation == scalar)
            {
                changed += 1;
                function.implementation_arguments = if missing {
                    Arc::from([])
                } else {
                    Arc::from([Arc::new(
                        super::super::prepare::NormalizedImplementationApplication {
                            identity: 0,
                            depth: 0,
                            implementation: scalar,
                            contract: function.implementation_parameters[0].contract,
                            implementation_type_arguments: Arc::from([]),
                            self_type: i64_type,
                            type_arguments: Arc::from([]),
                            implementations: Arc::from([]),
                            prerequisites: Arc::from([]),
                        },
                    )])
                };
            }
        }
        assert!(changed > 0);
        let storage = super::super::owned_i64_cell::StorageObservation::start();
        let failure = NormalizedVm::for_test(&forged, NormalizedRunPolicy::foreground())
            .invoke(
                entry,
                vec![NormalizedValue::I64(128)],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap_err();
        assert_eq!(failure.class, ExecutionFailureClass::Infrastructure);
        assert_eq!(failure.code, "normalized_runtime_type");
        assert!(
            failure.message.contains(if missing {
                "unbound static implementation"
            } else {
                "implementation witness differs from its admitted identity"
            }),
            "{failure:?}"
        );
        assert_eq!(storage.live(), (0, 0));
    }
}
