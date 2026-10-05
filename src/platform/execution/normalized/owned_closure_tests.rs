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

#[test]
fn owned_closure_reference_checks_unused_ordered_contract_arguments_after_admission() {
    use crate::platform::kernel::*;
    use crate::platform::semantic_id::TypeParameterId;

    let mut source = byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell.lkjc"),
            include_str!("../../../../tests/fixtures/owned-witness-cell-consumer.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap();
    let contract = declaration_named(&source, "Storage");
    let selected = declaration_named(&source, "Alternate");
    let producer = declaration_named(&source, "produce");
    let entry = declaration_named(&source, "alternate");
    let OwnerRecord::Declaration(owner) =
        &source.owners[&OwnerKey::Declaration(contract.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(record) = &owner.payload else {
        unreachable!()
    };
    let OwnerRecord::TypeParameter(template) =
        &source.owners[&OwnerKey::TypeParameter(record.self_parameter)]
    else {
        unreachable!()
    };
    let template = template.clone();
    let parameters = [
        TypeParameterId::migrate(b"reference-contract-phantom", 0),
        TypeParameterId::migrate(b"reference-contract-phantom", 1),
    ];
    for (parameter, name) in parameters.into_iter().zip(["UnusedCell", "UnusedBuffer"]) {
        let mut owner = template.clone();
        owner.header.owner = OwnerKey::TypeParameter(parameter);
        owner.name = Name::new(name).unwrap();
        source.owners.insert(
            OwnerKey::TypeParameter(parameter),
            OwnerRecord::TypeParameter(owner),
        );
    }
    let mut arguments = Vec::new();
    for form in [TypeForm::OwnedI64Cell, TypeForm::ByteBuffer] {
        let object = TypeObject::new(form).unwrap();
        let (digest, _) = encode_type_object(&object).unwrap();
        source.types.insert(digest, object);
        arguments.push(digest);
    }
    for owner in source.owners.values_mut() {
        if let OwnerRecord::Declaration(owner) = owner {
            match &mut owner.payload {
                DeclarationPayload::OwnedContract(contract) => {
                    contract.type_parameters = parameters.to_vec();
                }
                DeclarationPayload::OwnedImplementation(implementation) => {
                    implementation.type_arguments = arguments.clone();
                }
                DeclarationPayload::Function(function) => {
                    for parameter in &mut function.implementation_parameters {
                        parameter.type_arguments = arguments.clone();
                    }
                }
                _ => {}
            }
        }
    }
    source.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.owners.page(),
        source.owners.len() as u64,
        source.root.owners.content(),
    );
    let program = prepare_snapshot(&source);
    let schema = Arc::new(
        super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&source]).unwrap(),
    );
    let control = ExecutionControl::uncancelled();
    assert_eq!(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(entry, vec![], None, &control)
            .unwrap()
            .0,
        NormalizedValue::I64(99)
    );
    for fault in [
        "missing",
        "extra",
        "reordered",
        "ordinary",
        "formal-reordered",
        "foreign-owner",
        "stronger-constraint",
        "stale-generation",
        "duplicate-name",
        "duplicate-parameter",
    ] {
        let mut invalid = source.clone();
        if ["missing", "extra", "reordered", "ordinary"].contains(&fault) {
            let OwnerRecord::Declaration(owner) = invalid
                .owners
                .get_mut(&OwnerKey::Declaration(selected.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            let DeclarationPayload::OwnedImplementation(implementation) = &mut owner.payload else {
                unreachable!()
            };
            match fault {
                "missing" => {
                    implementation.type_arguments.pop();
                }
                "extra" => implementation.type_arguments.push(arguments[0]),
                "reordered" => implementation.type_arguments.swap(0, 1),
                "ordinary" => {
                    implementation.type_arguments[1] =
                        encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
                            .unwrap()
                            .0;
                }
                _ => unreachable!(),
            }
        } else if fault == "formal-reordered" {
            let OwnerRecord::Declaration(owner) = invalid
                .owners
                .get_mut(&OwnerKey::Declaration(producer.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            let DeclarationPayload::Function(function) = &mut owner.payload else {
                unreachable!()
            };
            function.implementation_parameters[0]
                .type_arguments
                .swap(0, 1);
        } else if fault == "duplicate-parameter" {
            let OwnerRecord::Declaration(owner) = invalid
                .owners
                .get_mut(&OwnerKey::Declaration(contract.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            let DeclarationPayload::OwnedContract(contract) = &mut owner.payload else {
                unreachable!()
            };
            contract.type_parameters[1] = contract.type_parameters[0];
        } else {
            let OwnerRecord::TypeParameter(owner) = invalid
                .owners
                .get_mut(&OwnerKey::TypeParameter(parameters[1]))
                .unwrap()
            else {
                unreachable!()
            };
            if fault == "foreign-owner" {
                owner.declaration = producer.declaration;
            } else if fault == "stale-generation" {
                owner.header.contract_version = 25;
            } else if fault == "duplicate-name" {
                owner.name = Name::new("UnusedCell").unwrap();
            } else {
                owner.constraints = TypeParameterConstraints::OwnedTransferable;
            }
        }
        let reader = FaultedReferenceRead {
            source: &invalid,
            schema: Arc::clone(&schema),
        };
        assert!(
            NormalizedReferenceInterpreter::from_reader(
                &reader,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(entry, vec![], None, &control)
            .is_err(),
            "independent reference dispatch: {fault}",
        );
    }
}

#[test]
fn owned_closure_reference_derives_method_results_without_explicit_caller_aggregate_types() {
    use crate::platform::kernel::*;
    let source = byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create method-result-closure
  (external create new (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (owned-contract create Pair (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint owned))
    (method method_a4900000000000000000000000000001 pair
      (parameters (Self consume) (Item consume))
      (returns (owned-product (field left Self) (field right Item)))))
  (function create combine (visibility private) (effect pure)
    (parameter create left (type OwnedI64Cell) (use consume))
    (parameter create right (type OwnedI64Cell) (use consume))
    (returns (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
    (body (pack-owned
      (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
      (field left (local left)) (field right (local right)))))
  (owned-implementation create CellPair (visibility private) (contract Pair)
    (self OwnedI64Cell) (types OwnedI64Cell)
    (method method_a4900000000000000000000000000001 combine))
  (function create discard (visibility private) (effect pure)
    (type-parameter create Q (constraint owned))
    (type-parameter create I (constraint owned))
    (implementation-parameter implparam_a4900000000000000000000000000001 ops Pair Q (types I))
    (parameter create left (type Q) (use consume))
    (parameter create right (type I) (use consume)) (returns I64)
    (body (sequence
      (method-call parameter@discard@implparam_a4900000000000000000000000000001
        Pair method_a4900000000000000000000000000001 (local left) (local right))
      (i64 11))))
  (function create main (visibility public) (effect pure) (returns I64)
    (body (let
      (binding left (type OwnedI64Cell) (call new (i64 1)))
      (binding right (type OwnedI64Cell) (call new (i64 2)))
      (in (implementation-call discard (types OwnedI64Cell OwnedI64Cell)
        (implementations concrete@CellPair) (local left) (local right))))))))
declarations.end"#,
    )
    .unwrap();
    let discard = declaration_named(&source, "discard");
    let OwnerRecord::Declaration(owner) =
        &source.owners[&OwnerKey::Declaration(discard.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &owner.payload else {
        unreachable!()
    };
    let symbolic_members = function
        .type_parameters
        .iter()
        .map(|parameter| {
            encode_type_object(
                &TypeObject::new(TypeForm::TypeParameter {
                    parameter: *parameter,
                })
                .unwrap(),
            )
            .unwrap()
            .0
        })
        .collect::<Vec<_>>();
    let symbolic_result = encode_type_object(
        &TypeObject::new(TypeForm::OwnedProduct {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("left").unwrap(),
                    ty: symbolic_members[0],
                },
                StructuralTypeField {
                    name: Name::new("right").unwrap(),
                    ty: symbolic_members[1],
                },
            ],
        })
        .unwrap(),
    )
    .unwrap()
    .0;
    assert!(!source.types.contains_key(&symbolic_result));
    let schema =
        super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&source]).unwrap();
    assert!(schema.types.contains_key(&symbolic_result));
    let program = prepare_snapshot(&source);
    let cells = super::super::owned_i64_cell::StorageObservation::start();
    let products = super::super::owned_storage::StorageObservation::start();
    assert_eq!(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(
                declaration_named(&source, "main"),
                vec![],
                None,
                &ExecutionControl::uncancelled()
            )
            .unwrap()
            .0,
        NormalizedValue::I64(11),
    );
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(products.live(), (0, 0));
}
