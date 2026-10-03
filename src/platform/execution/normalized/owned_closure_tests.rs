//! Independent closure regressions from the 2026-09-30 source audit.
use super::*;

#[test]
fn owned_closure_reference_derives_composites_inside_witness_generic_bodies() {
    let source = byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-composite.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap();
    let list_i64 = crate::platform::kernel::encode_type_object(
        &TypeObject::new(TypeForm::List {
            item: crate::platform::kernel::encode_type_object(
                &TypeObject::new(TypeForm::I64).unwrap(),
            )
            .unwrap()
            .0,
        })
        .unwrap(),
    )
    .unwrap()
    .0;
    assert!(!source.types.contains_key(&list_i64));
    let program = prepare_snapshot(&source);
    assert!(program.types.contains_key(&list_i64));
    let entry = declaration_named(&source, "composite-main");
    let control = ExecutionControl::uncancelled();
    assert_eq!(
        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke(entry, vec![], None, &control)
            .unwrap()
            .0,
        NormalizedValue::I64(7)
    );
    assert_eq!(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(entry, vec![], None, &control)
            .unwrap()
            .0,
        NormalizedValue::I64(7)
    );
}

#[test]
fn owned_closure_recursive_nominal_method_types_are_closed_first_order_data() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-witness-recursive-data.lkjc"
    ))
    .unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "recursive-data-main");
    let control = ExecutionControl::uncancelled();
    assert_eq!(
        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke(entry, vec![], None, &control)
            .unwrap()
            .0,
        NormalizedValue::I64(128)
    );
    assert_eq!(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(entry, vec![], None, &control)
            .unwrap()
            .0,
        NormalizedValue::I64(128)
    );
}

#[test]
fn owned_closure_recursive_nominal_properties_reject_nested_callable_fields() {
    use crate::platform::kernel::*;
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-witness-recursive-data.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    let mut schema =
        super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&source]).unwrap();
    let i64_type = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
        .unwrap()
        .0;
    let callable = TypeObject::new(TypeForm::Function {
        parameters: vec![i64_type],
        result: i64_type,
    })
    .unwrap();
    let callable_type = encode_type_object(&callable).unwrap().0;
    schema.types.insert(callable_type, callable.clone());
    let schema = Arc::new(schema);
    for (declaration, field) in [("Node", "value"), ("Right", "left")] {
        let mut invalid = source.clone();
        invalid.types.insert(callable_type, callable.clone());
        let d = declaration_named(&source, declaration);
        let f = invalid
            .owners
            .values_mut()
            .find_map(|v| match v {
                OwnerRecord::Field(f)
                    if f.declaration == d.declaration && f.name.as_str() == field =>
                {
                    Some(f)
                }
                _ => None,
            })
            .unwrap();
        f.ty = callable_type;
        assert!(
            !memory_reference::accepts(&invalid),
            "oracle: {declaration}"
        );
        assert!(validate_full(&invalid).is_err(), "kernel: {declaration}");
        let reader = FaultedReferenceRead {
            source: &invalid,
            schema: Arc::clone(&schema),
        };
        let result = NormalizedReferenceInterpreter::from_reader(
            &reader,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(
            declaration_named(&source, "recursive-data-main"),
            vec![],
            None,
            &ExecutionControl::uncancelled(),
        );
        assert!(result.is_err(), "independent dispatch: {declaration}");
    }
}
