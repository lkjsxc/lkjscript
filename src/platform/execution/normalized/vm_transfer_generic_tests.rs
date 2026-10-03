//! Two valid actual applications can share a target and result but not return custody.
use super::*;

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
