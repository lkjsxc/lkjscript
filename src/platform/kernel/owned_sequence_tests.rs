//! Sequence type identity, complete generation admission and finite nesting.
use super::*;
use crate::platform::packed;
use crate::platform::semantic_id::{BindingId, ExpressionId};

fn intern(snapshot: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    snapshot.types.insert(digest, object);
    digest
}

#[test]
fn owned_sequence_has_an_exact_disjoint_canonical_type_envelope() {
    let mut types = TypeObjectInterner::default();
    let cell = types.intern(TypeForm::OwnedI64Cell).unwrap();
    let sequence = types
        .intern(TypeForm::OwnedSequence { item: cell })
        .unwrap();
    let object = types.get(sequence).unwrap().clone();
    let (digest, bytes) = encode_type_object(&object).unwrap();
    assert_eq!(&bytes[..8], b"LKJSEQ01");
    assert_eq!(decode_type_object(&bytes, digest).unwrap(), object);
    assert_eq!(object.child_types(), vec![cell]);
    assert_eq!(object.child_type_count(), 1);
    assert_ne!(
        sequence,
        types.intern(TypeForm::List { item: cell }).unwrap()
    );

    // Rehashed bytes still cannot select an unknown constructor or generation.
    for (generation, tag, expected) in [
        (1_u16, 2_u8, "kernel_sequence_type_tag"),
        (2_u16, 1_u8, "kernel_type_contract"),
    ] {
        let forged = packed::encode(
            contract::OWNED_SEQUENCE_TYPE_MAGIC,
            contract::OWNED_SEQUENCE_TYPE_ENVELOPE_DOMAIN,
            &(generation, tag, cell),
            contract::MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert_eq!(
            decode_type_object(&forged, TypeObjectDigest::of(&forged))
                .unwrap_err()
                .code,
            expected
        );
    }

    let disguised = packed::encode(
        contract::TYPE_OBJECT_MAGIC,
        contract::TYPE_OBJECT_ENVELOPE_DOMAIN,
        &object,
        contract::MAXIMUM_TYPE_OBJECT_BYTES,
    )
    .unwrap();
    assert!(decode_type_object(&disguised, TypeObjectDigest::of(&disguised)).is_err());
}

#[test]
fn every_sequence_operation_requires_graph_25_and_retains_all_type_roots() {
    let sequence_type = TypeObjectDigest::from_bytes([3; 32]);
    let result_type = TypeObjectDigest::from_bytes([4; 32]);
    let id = ExpressionId::migrate(b"sequence-operation-generation", 0);
    let source = ExpressionId::migrate(b"sequence-operation-generation", 1);
    let value = ExpressionId::migrate(b"sequence-operation-generation", 2);
    let index = ExpressionId::migrate(b"sequence-operation-generation", 3);
    let body = ExpressionId::migrate(b"sequence-operation-generation", 4);
    let operations = [
        ExpressionOperation::SequenceEmpty { sequence_type },
        ExpressionOperation::SequenceLength {
            sequence_type,
            source,
        },
        ExpressionOperation::SequencePush {
            sequence_type,
            value,
            source,
        },
        ExpressionOperation::SequencePop {
            sequence_type,
            result_type,
            source,
        },
        ExpressionOperation::BorrowOwnedItem {
            sequence_type,
            source,
            index,
            binding: BindingId::migrate(b"sequence-operation-generation", 0),
            body,
        },
    ];
    for operation in operations {
        let expression = ExpressionRecord::new(id, operation).unwrap();
        assert!(expression.type_roots().contains(&sequence_type));
        if matches!(
            expression.operation,
            ExpressionOperation::SequencePop { .. }
        ) {
            assert_eq!(expression.type_roots(), vec![sequence_type, result_type]);
        }
        let owner = OwnerRecord::Expression(expression.clone());
        let (digest, bytes) = encode_owner(&owner).unwrap();
        assert_eq!(&bytes[..8], contract::OWNER_MAGIC);
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
        // Freeze the original owner ordinal, expression ordinals and field
        // order independently of the current enum and predecessor conversions.
        for generation in [25_u16, 26, 27] {
            let magic = format!("LKJOWN{generation}").as_bytes().try_into().unwrap();
            let domain = format!("lkjscript.kernel.owner-envelope.v{generation}");
            let literal = match &expression.operation {
                ExpressionOperation::SequenceEmpty { sequence_type } => packed::encode(
                    magic,
                    &domain,
                    &(9_u32, generation, id, 33_u32, sequence_type),
                    contract::MAXIMUM_OWNER_OBJECT_BYTES,
                ),
                ExpressionOperation::SequenceLength {
                    sequence_type,
                    source,
                } => packed::encode(
                    magic,
                    &domain,
                    &(9_u32, generation, id, 34_u32, sequence_type, source),
                    contract::MAXIMUM_OWNER_OBJECT_BYTES,
                ),
                ExpressionOperation::SequencePush {
                    sequence_type,
                    value,
                    source,
                } => packed::encode(
                    magic,
                    &domain,
                    &(9_u32, generation, id, 35_u32, sequence_type, value, source),
                    contract::MAXIMUM_OWNER_OBJECT_BYTES,
                ),
                ExpressionOperation::SequencePop {
                    sequence_type,
                    result_type,
                    source,
                } => packed::encode(
                    magic,
                    &domain,
                    &(
                        9_u32,
                        generation,
                        id,
                        36_u32,
                        sequence_type,
                        result_type,
                        source,
                    ),
                    contract::MAXIMUM_OWNER_OBJECT_BYTES,
                ),
                ExpressionOperation::BorrowOwnedItem {
                    sequence_type,
                    source,
                    index,
                    binding,
                    body,
                } => packed::encode(
                    magic,
                    &domain,
                    &(
                        9_u32,
                        generation,
                        id,
                        37_u32,
                        sequence_type,
                        source,
                        index,
                        binding,
                        body,
                    ),
                    contract::MAXIMUM_OWNER_OBJECT_BYTES,
                ),
                _ => unreachable!(),
            }
            .unwrap();
            let literal_digest = OwnerObjectDigest::of(&literal);
            let mut expected = owner.clone();
            expected.set_encoding_for_edit(generation);
            assert_eq!(
                encode_owner(&expected).unwrap(),
                (literal_digest, literal.clone())
            );
            assert_eq!(
                decode_owner(&literal, expected.owner(), expected.kind(), literal_digest).unwrap(),
                expected
            );
        }
        let mut predecessor = expression;
        predecessor.contract_version = 24;
        assert_eq!(
            predecessor.validate_local().unwrap_err().code,
            "kernel_sequence_generation"
        );
    }
}

#[test]
fn sequence_child_order_preserves_push_and_index_evaluation() {
    let source = ExpressionId::migrate(b"sequence-child-order", 0);
    let value = ExpressionId::migrate(b"sequence-child-order", 1);
    let index = ExpressionId::migrate(b"sequence-child-order", 2);
    let body = ExpressionId::migrate(b"sequence-child-order", 3);
    let sequence_type = TypeObjectDigest::from_bytes([3; 32]);
    let push = ExpressionRecord::new(
        source,
        ExpressionOperation::SequencePush {
            sequence_type,
            value,
            source,
        },
    )
    .unwrap();
    assert_eq!(
        push.children()
            .iter()
            .map(|child| child.expression)
            .collect::<Vec<_>>(),
        vec![value, source]
    );
    let borrow = ExpressionRecord::new(
        source,
        ExpressionOperation::BorrowOwnedItem {
            sequence_type,
            source,
            index,
            binding: BindingId::migrate(b"sequence-child-order", 0),
            body,
        },
    )
    .unwrap();
    assert_eq!(
        borrow
            .children()
            .iter()
            .map(|child| child.expression)
            .collect::<Vec<_>>(),
        vec![index, source, body]
    );
}

#[test]
fn sequence_element_eligibility_and_nesting_are_checked_even_when_empty() {
    let mut snapshot = tests::witness_snapshot();
    let unit = intern(&mut snapshot, TypeForm::Unit);
    let cell = intern(&mut snapshot, TypeForm::OwnedI64Cell);
    let ordinary = intern(&mut snapshot, TypeForm::OwnedSequence { item: unit });
    owned_product::validate(&snapshot, ordinary, None).unwrap();
    assert!(memory::direct(&snapshot, ordinary).unwrap());
    assert!(!owned_contract::ordinary_closed(&snapshot, ordinary).unwrap());
    assert!(transfer::admit(&snapshot, ordinary, None).unwrap());
    assert!(share::admit(&snapshot, ordinary, None).unwrap());
    let secret = intern(&mut snapshot, TypeForm::Secret);
    let bad = intern(&mut snapshot, TypeForm::OwnedSequence { item: secret });
    assert_eq!(
        owned_product::validate(&snapshot, bad, None)
            .unwrap_err()
            .code,
        "kernel_owned_sequence"
    );
    let good = intern(&mut snapshot, TypeForm::OwnedSequence { item: cell });
    owned_product::validate(&snapshot, good, None).unwrap();
    assert!(memory::direct(&snapshot, good).unwrap());
    assert!(!owned_contract::ordinary_closed(&snapshot, good).unwrap());

    let hidden_owner = intern(&mut snapshot, TypeForm::List { item: cell });
    let hidden_secret = intern(&mut snapshot, TypeForm::Option { item: secret });
    let callable = intern(
        &mut snapshot,
        TypeForm::Function {
            parameters: vec![],
            result: unit,
        },
    );
    for item in [hidden_owner, hidden_secret, callable] {
        let sequence = intern(&mut snapshot, TypeForm::OwnedSequence { item });
        assert!(owned_product::validate(&snapshot, sequence, None).is_err());
        assert!(transfer::admit(&snapshot, sequence, None).is_err());
        assert!(share::admit(&snapshot, sequence, None).is_err());
    }

    let mut nested = cell;
    for _ in 0..contract::MAXIMUM_TYPE_DEPTH {
        nested = intern(&mut snapshot, TypeForm::OwnedSequence { item: nested });
    }
    owned_product::validate(&snapshot, nested, None).unwrap();
    let excessive = intern(&mut snapshot, TypeForm::OwnedSequence { item: nested });
    assert_eq!(
        owned_product::validate(&snapshot, excessive, None)
            .unwrap_err()
            .code,
        "kernel_owned_sequence"
    );
}

#[test]
fn generalized_sequence_operations_bind_exact_roots_and_authored_child_order() {
    let source = ExpressionId::migrate(b"generalized-sequence-child-order", 0);
    let value = ExpressionId::migrate(b"generalized-sequence-child-order", 1);
    let index = ExpressionId::migrate(b"generalized-sequence-child-order", 2);
    let sequence_type = TypeObjectDigest::from_bytes([3; 32]);
    let result_type = TypeObjectDigest::from_bytes([4; 32]);
    for (operation, roots, children) in [
        (
            ExpressionOperation::SequenceGet {
                sequence_type,
                source,
                index,
            },
            vec![sequence_type],
            vec![index, source],
        ),
        (
            ExpressionOperation::SequenceReplace {
                sequence_type,
                result_type,
                index,
                value,
                source,
            },
            vec![sequence_type, result_type],
            vec![index, value, source],
        ),
    ] {
        let expression = ExpressionRecord::new(source, operation).unwrap();
        assert_eq!(expression.type_roots(), roots);
        assert_eq!(
            expression
                .children()
                .iter()
                .map(|child| child.expression)
                .collect::<Vec<_>>(),
            children
        );
        for generation in [25, 30] {
            let mut predecessor = expression.clone();
            predecessor.contract_version = generation;
            assert_eq!(
                predecessor.validate_local().unwrap_err().code,
                "kernel_sequence_generation"
            );
        }
        let owner = OwnerRecord::Expression(expression);
        let (digest, bytes) = encode_owner(&owner).unwrap();
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
    }
}

#[test]
fn ordinary_sequence_closures_require_graph_31_even_without_operations() {
    let source = r#"declarations.begin
(units (module create generalized-sequence-generation
  (function create relay (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create values (type (owned-sequence T)) (use consume))
    (returns (owned-sequence T)) (body (local values)))
  (function create dispose (visibility public) (effect pure)
    (parameter create values (type (owned-sequence I64)) (use consume))
    (returns Unit) (body (unit)))))
declarations.end"#;
    let snapshot =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
            .unwrap();
    validate_full(&snapshot).unwrap();
    assert!(memory_reference::accepts(&snapshot));
    let mut older = snapshot.clone();
    older.root.graph_contract_version = 30;
    assert!(!memory_reference::accepts(&older));
    assert!(
        validate_full(&older)
            .unwrap_err()
            .iter()
            .any(|d| d.code == "kernel_sequence_generation")
    );
    for (key, owner) in &snapshot.owners {
        if !matches!(owner, OwnerRecord::Declaration(declaration) if matches!(declaration.payload, DeclarationPayload::Function(_)))
        {
            continue;
        }
        let mut older = snapshot.clone();
        older.owners.get_mut(key).unwrap().set_encoding_for_edit(30);
        assert!(memory::validate_owner(&older, *key, &older.owners[key]).is_err());
        assert!(
            !memory_reference::accepts(&older),
            "older declaration cannot inherit a newer parameter's ordinary sequence authority"
        );
    }
    let mut unused = tests::witness_snapshot();
    unused.root.graph_contract_version = 30;
    let i64_type = intern(&mut unused, TypeForm::I64);
    intern(&mut unused, TypeForm::OwnedSequence { item: i64_type });
    assert!(!memory_reference::accepts(&unused));
    assert!(
        validate_full(&unused)
            .unwrap_err()
            .iter()
            .any(|d| d.code == "kernel_sequence_generation")
    );
}

#[test]
fn signature_only_sequence_cannot_gain_predecessor_graph_authority() {
    let source = r#"declarations.begin
(units (module create sequence-generation
  (function create relay (visibility public) (effect pure)
    (parameter create values (type (owned-sequence OwnedI64Cell)) (use consume))
    (returns (owned-sequence OwnedI64Cell)) (body (local values)))))
declarations.end
"#;
    let mut snapshot =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
            .unwrap();
    assert!(snapshot.owners.values().all(|owner| !matches!(owner, OwnerRecord::Expression(e) if !matches!(e.operation, ExpressionOperation::Local { .. }))));
    assert!(memory_reference::accepts(&snapshot));
    snapshot.root.graph_contract_version = 24;
    for owner in snapshot.owners.values_mut() {
        owner.set_encoding_for_edit(24);
    }
    assert!(!memory_reference::accepts(&snapshot));
    let diagnostics = validate_full(&snapshot).unwrap_err();
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "kernel_sequence_generation"),
        "{diagnostics:?}"
    );
}
