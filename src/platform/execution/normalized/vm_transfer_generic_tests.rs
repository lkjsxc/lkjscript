//! Two valid actual applications can share a target and result but not return custody.
use super::*;

#[test]
fn transfer_application_resolves_wide_structural_signatures_with_only_application_storage() {
    use crate::platform::kernel::{
        Name, StructuralTypeField, TypeObject, TypeParameterConstraints, encode_type_object,
    };
    use crate::platform::semantic_id::TypeParameterId;

    let (prepared, functions) = fixture();
    let mut program = (*prepared).clone();
    let function = functions["fixed-result"];
    let parameter = program.functions[function.0 as usize].type_parameters[0];
    let symbolic_object = TypeObject::new(TypeForm::TypeParameter { parameter }).unwrap();
    let symbolic = encode_type_object(&symbolic_object).unwrap().0;
    program.types.insert(symbolic, symbolic_object);
    let scalar = program
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::Text).then_some(*ty))
        .unwrap();
    let fields = |ty| {
        (0..256)
            .map(|index| StructuralTypeField {
                name: Name::new(format!(
                    "field_{index:03}_metadata_whose_name_storage_is_already_prepared"
                ))
                .unwrap(),
                ty,
            })
            .collect()
    };
    let template_object = TypeObject::new(TypeForm::StructuralRecord {
        fields: fields(symbolic),
    })
    .unwrap();
    let template = encode_type_object(&template_object).unwrap().0;
    program.types.insert(template, template_object);
    let concrete_object = TypeObject::new(TypeForm::StructuralRecord {
        fields: fields(scalar),
    })
    .unwrap();
    let concrete = encode_type_object(&concrete_object).unwrap().0;
    program.types.insert(concrete, concrete_object);
    program.comparable_types.insert(concrete);
    let target = &mut Arc::make_mut(&mut program.functions)[function.0 as usize];
    target.type_parameter_constraints = Arc::from([TypeParameterConstraints::Transferable]);
    Arc::make_mut(&mut target.parameters)[0].ty = template;
    Arc::make_mut(&mut target.parameters)[0].use_mode = ParameterUse::Unrestricted;
    target.result = template;
    let control = ExecutionControl::uncancelled();
    let retained = std::mem::size_of::<TypeObjectDigest>() as u64;
    let binding = (std::mem::size_of::<(TypeParameterId, TypeObjectDigest)>()
        + 3 * std::mem::size_of::<usize>()) as u64;
    let required = binding + retained;
    for quota in [required, required - 1] {
        let mut used = 0;
        let result = TaskApplication::bind(
            &program,
            function,
            Arc::from([scalar]),
            &control,
            &mut |bytes| {
                let next = used + bytes;
                if next > quota {
                    return Err(ExecutionError::resource(
                        "test_metadata_budget",
                        "application storage reservation denied",
                    ));
                }
                used = next;
                Ok(())
            },
        );
        if quota == required {
            let application = result.unwrap();
            assert_eq!(application.result(), concrete);
            assert_eq!(application.parameter(&program, 0).unwrap(), concrete);
            assert_eq!(used, required);
        } else {
            assert_eq!(result.err().unwrap().class, ExecutionFailureClass::Resource);
            assert_eq!(used, binding);
        }
    }
    let bindings = BTreeMap::from([(parameter, scalar)]);
    assert_eq!(
        resolve_type(&program, template, &bindings, &control).unwrap(),
        concrete
    );
    assert_eq!(
        resolve_type(
            &program,
            symbolic,
            &bindings,
            &ExecutionControl::cancel_after_checks(2)
        )
        .unwrap(),
        scalar
    );
    assert!(resolve_type(&program, template, &BTreeMap::new(), &control).is_err());
    assert_eq!(
        resolve_type(
            &program,
            template,
            &bindings,
            &ExecutionControl::cancel_after_checks(1)
        )
        .unwrap_err()
        .class,
        ExecutionFailureClass::Cancelled
    );
}

#[test]
fn sealed_transfer_generic_result_binds_even_phantom_type_arguments() {
    let (program, functions) = fixture();
    let function = functions["fixed-result"];
    let text = program
        .types
        .iter()
        .find(|(_, t)| matches!(t.form, TypeForm::Text))
        .unwrap()
        .0;
    let unit = program
        .types
        .iter()
        .find(|(_, t)| matches!(t.form, TypeForm::Unit))
        .unwrap()
        .0;
    let ty = program.functions[function.0 as usize].result;
    let control = ExecutionControl::uncancelled();
    for wrong in [false, true] {
        let buffers = Buffers::start();
        let source = ValueOrigin::fresh().unwrap();
        let destination = ValueOrigin::fresh().unwrap();
        let mut reserved = 0;
        let application = TaskApplication::bind(
            &program,
            function,
            Arc::from([*text]),
            &control,
            &mut |bytes| {
                reserved += bytes;
                Ok(())
            },
        )
        .unwrap();
        assert!(reserved > 0);
        let other = TaskApplication::bind(
            &program,
            function,
            Arc::from([*unit]),
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
        assert_eq!(application.result(), other.result());
        assert_eq!(application.function(), other.function());
        let buffer = ByteBuffer::empty(source)
            .push(197, &control, &mut |_| Ok(()))
            .unwrap();
        let identity = buffer.allocation_identity();
        let result = TransferResult::seal_applied(
            &program,
            source,
            destination,
            application,
            NormalizedValue::ByteBuffer(buffer),
            &control,
            &mut |v, t| ordinary(&program, v, t),
        )
        .unwrap();
        let result = result.adopt_applied(
            &program,
            destination,
            function,
            &[if wrong { *unit } else { *text }],
            ty,
            &control,
        );
        if wrong {
            assert_eq!(result.unwrap_err().code, "normalized_parallel_transfer");
        } else {
            let NormalizedValue::ByteBuffer(buffer) = result.unwrap() else {
                panic!("buffer");
            };
            assert_eq!(buffer.allocation_identity(), identity);
            buffer.validate(destination, true).unwrap();
            assert!(buffer.validate(source, true).is_err());
            assert_eq!(&*buffer.freeze().unwrap(), &[197]);
        }
        assert_eq!(buffers.created(), 1);
        assert_eq!(buffers.live(), (0, 0));
    }
    let mut reservations = 0;
    let error = TaskApplication::bind(
        &program,
        function,
        Arc::from([*text]),
        &control,
        &mut |_| {
            reservations += 1;
            Err(ExecutionError::new(
                ExecutionFailureClass::Resource,
                "test_metadata_budget",
                "closed signature reservation denied",
            ))
        },
    )
    .err()
    .expect("quota refusal precedes signature allocation");
    assert_eq!(reservations, 1);
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert_eq!(error.code, "test_metadata_budget");
}
