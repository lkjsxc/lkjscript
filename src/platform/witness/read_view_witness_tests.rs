use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

const SEED: &[u8] = b"read-view-witness-controls";

fn binding(id: BindingId, kind: BindingKind) -> BindingRecord {
    BindingRecord {
        header: OwnerHeader::new(OwnerKey::Binding(id), OwnerKind::Binding),
        name: Name::new("read").unwrap(),
        kind,
        value: None,
        declared_type: Some(
            encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
                .unwrap()
                .0,
        ),
    }
}

#[test]
fn view_borrow_roles_follow_exact_canonical_binding_identity_and_kind() {
    let id = BindingId::migrate(SEED, 0);
    let declaration = DeclarationReference {
        package: PackageId::migrate(SEED, 0),
        declaration: DeclarationId::migrate(SEED, 0),
    };
    let field_borrow = OwnerRecord::Expression(
        ExpressionRecord::new(
            ExpressionId::migrate(SEED, 0),
            ExpressionOperation::BorrowReadViewField {
                value: LocalValueReference::FunctionParameter(ParameterId::migrate(SEED, 0)),
                field: ReadViewFieldReference {
                    declaration,
                    field: FieldId::migrate(SEED, 0),
                },
                binding: id,
                body: ExpressionId::migrate(SEED, 1),
            },
        )
        .unwrap(),
    );
    assert!(ownership_contributions(&field_borrow).is_err());
    assert!(aggregation_children(&field_borrow).is_err());
    for (kind, role) in [
        (BindingKind::OwnedBorrow, BindingContainerRole::OwnedBorrow),
        (
            BindingKind::ReadViewBorrow,
            BindingContainerRole::ReadViewBorrow,
        ),
    ] {
        let record = binding(id, kind);
        let facts =
            ownership_contributions_with_bindings(&field_borrow, |_| Ok(Some(record.clone())))
                .unwrap();
        assert_eq!(
            facts[&OwnerKey::Binding(id)].role,
            OwnershipRole::ExpressionBinding { role, ordinal: 0 }
        );
        assert!(
            aggregation_children_with_bindings(&field_borrow, |_| Ok(Some(record.clone())))
                .unwrap()
                .contains(&(
                    OwnershipRole::ExpressionBinding { role, ordinal: 0 },
                    OwnerKey::Binding(id)
                ))
        );
    }
    let wrong = binding(BindingId::migrate(SEED, 2), BindingKind::OwnedBorrow);
    assert!(
        ownership_contributions_with_bindings(&field_borrow, |_| Ok(Some(wrong.clone()))).is_err()
    );
    let owning_view = binding(id, BindingKind::ReadView);
    assert!(
        ownership_contributions_with_bindings(&field_borrow, |_| Ok(Some(owning_view.clone())))
            .is_err()
    );
}

#[test]
fn borrowed_call_adoption_distinguishes_affine_view_and_owned_read() {
    let id = BindingId::migrate(SEED, 0);
    let expression = ExpressionRecord::new(
        ExpressionId::migrate(SEED, 0),
        ExpressionOperation::BorrowCall {
            call: ExpressionId::migrate(SEED, 1),
            binding: id,
            body: ExpressionId::migrate(SEED, 2),
        },
    )
    .unwrap();
    for (kind, role) in [
        (BindingKind::OwnedBorrow, BindingContainerRole::OwnedBorrow),
        (BindingKind::ReadView, BindingContainerRole::ReadView),
    ] {
        let facts = ownership_contributions_with_bindings(
            &OwnerRecord::Expression(expression.clone()),
            |_| Ok(Some(binding(id, kind))),
        )
        .unwrap();
        assert_eq!(
            facts[&OwnerKey::Binding(id)].role,
            OwnershipRole::ExpressionBinding { role, ordinal: 0 }
        );
    }
    assert!(ownership_contributions(&OwnerRecord::Expression(expression.clone())).is_err());
}

#[test]
fn foreign_validator_cannot_be_resigned_as_current_admission() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let witness = rebuild_full_witness(&snapshot).unwrap();
    let mut foreign = witness.manifest;
    foreign.validator_contract = ValidatorContractDigest::from_bytes([0x5a; 32]);
    foreign.certificate = super::codec::certificate_digest(&foreign.core()).unwrap();
    let bytes = crate::platform::packed::encode_canonical(
        super::contract::WITNESS_MAGIC,
        super::contract::WITNESS_ENVELOPE_DOMAIN,
        &foreign,
        super::contract::MAXIMUM_WITNESS_MANIFEST_BYTES,
    )
    .unwrap();
    assert!(!foreign.contract_is_current());
    assert_eq!(
        decode_witness_manifest(&bytes, ValidationWitnessDigest::of(&bytes))
            .unwrap_err()
            .code,
        "witness_validator_contract"
    );
    assert!(encode_witness_manifest(&foreign).is_err());
}

#[test]
fn complete_witness_admission_rejects_an_unused_hidden_view_type() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let missing = DeclarationReference {
        package: snapshot.root.package_id,
        declaration: DeclarationId::migrate(SEED, 100),
    };
    let object = TypeObject::new(TypeForm::ReadViewApplied {
        declaration: missing,
        arguments: Vec::new(),
        read_scope: ReadScopeArgument::Lexical {
            callable: missing,
            binding: BindingId::migrate(SEED, 100),
        },
    })
    .unwrap();
    snapshot
        .types
        .insert(encode_type_object(&object).unwrap().0, object);
    assert!(
        rebuild_full_witness(&snapshot).is_err(),
        "unused canonical type roots must be admitted"
    );
}

#[test]
fn ownership_closure_rejects_equal_cardinality_with_missing_and_invented_owners() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let existing = snapshot
        .owners
        .keys()
        .find(|owner| matches!(owner, OwnerKey::Expression(_)))
        .copied()
        .unwrap();
    let record = snapshot.owners.remove(&existing).unwrap();
    snapshot.owners.insert(
        OwnerKey::Expression(ExpressionId::migrate(SEED, 100)),
        record,
    );
    assert!(super::ownership::derive_ownership(&snapshot).is_err());
}

#[test]
fn complete_witness_admission_rejects_missing_field_type_in_unused_view_owner() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let module = snapshot
        .owners
        .keys()
        .find_map(|owner| match owner {
            OwnerKey::Module(id) => Some(*id),
            _ => None,
        })
        .unwrap();
    let owner = OwnerKey::Declaration(DeclarationId::migrate(SEED, 101));
    snapshot.owners.insert(
        owner,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(owner, OwnerKind::ReadView),
            module,
            name: Name::new("UnusedView").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::ReadView(ReadViewDeclaration {
                type_parameters: Vec::new(),
                scope_parameter: ReadScopeParameter {
                    id: ReadScopeParameterId::migrate(SEED, 101),
                    name: Name::new("source").unwrap(),
                },
                fields: vec![ReadViewField {
                    id: FieldId::migrate(SEED, 101),
                    name: Name::new("hidden").unwrap(),
                    ty: TypeObjectDigest::from_bytes([201; 32]),
                    kind: ReadViewFieldKind::Ordinary,
                }],
            }),
        }),
    );
    assert!(
        rebuild_full_witness(&snapshot).is_err(),
        "every unused owner field type must be admitted"
    );
}

#[test]
fn constructor_local_view_binding_under_let_keeps_exact_role_and_initializer_owner() {
    let ordinary_id = BindingId::migrate(SEED, 200);
    let view_id = BindingId::migrate(SEED, 201);
    let ordinary_value = ExpressionId::migrate(SEED, 200);
    let view_value = ExpressionId::migrate(SEED, 201);
    let mut ordinary = binding(ordinary_id, BindingKind::Let);
    ordinary.value = Some(ordinary_value);
    let mut view = binding(view_id, BindingKind::ReadView);
    view.value = Some(view_value);
    let mut expression = ExpressionRecord::new(
        ExpressionId::migrate(SEED, 202),
        ExpressionOperation::Let {
            bindings: vec![ordinary_id, view_id],
            body: ExpressionId::migrate(SEED, 203),
        },
    )
    .unwrap();
    let record = OwnerRecord::Expression(expression.clone());
    let resolve = |id| {
        Ok(if id == ordinary_id {
            Some(ordinary.clone())
        } else if id == view_id {
            Some(view.clone())
        } else {
            None
        })
    };
    let facts = ownership_contributions_with_bindings(&record, resolve).unwrap();
    for (id, role, ordinal) in [
        (ordinary_id, BindingContainerRole::Let, 0),
        (view_id, BindingContainerRole::ReadView, 1),
    ] {
        let expected = OwnershipRole::ExpressionBinding { role, ordinal };
        assert_eq!(facts[&OwnerKey::Binding(id)].role, expected);
        assert!(
            aggregation_children_with_bindings(&record, resolve)
                .unwrap()
                .contains(&(expected, OwnerKey::Binding(id)))
        );
    }
    assert!(
        ownership_contributions(&record).is_err(),
        "current Let kind requires canonical binding reads"
    );
    let view_record = OwnerRecord::Binding(view.clone());
    assert_eq!(
        ownership_contributions(&view_record).unwrap()[&OwnerKey::Expression(view_value)],
        OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Binding(view_id)),
            OwnershipRole::ExpressionRoot(ExpressionRootRole::BindingValue)
        )
    );
    assert_eq!(
        aggregation_children(&view_record).unwrap(),
        vec![(
            OwnershipRole::ExpressionRoot(ExpressionRootRole::BindingValue),
            OwnerKey::Expression(view_value)
        )]
    );

    let mut missing_initializer = view.clone();
    missing_initializer.value = None;
    assert!(
        ownership_contributions_with_bindings(&record, |id| Ok(if id == ordinary_id {
            Some(ordinary.clone())
        } else {
            Some(missing_initializer.clone())
        }))
        .is_err()
    );
    expression.operation = ExpressionOperation::Let {
        bindings: vec![ordinary_id],
        body: ExpressionId::migrate(SEED, 203),
    };
    assert_eq!(
        ownership_contributions_with_bindings(&OwnerRecord::Expression(expression), resolve)
            .unwrap()[&OwnerKey::Binding(ordinary_id)]
            .role,
        OwnershipRole::ExpressionBinding {
            role: BindingContainerRole::Let,
            ordinal: 0
        }
    );

    let adopted = OwnerRecord::Expression(
        ExpressionRecord::new(
            ExpressionId::migrate(SEED, 204),
            ExpressionOperation::BorrowCall {
                call: ExpressionId::migrate(SEED, 205),
                binding: view_id,
                body: ExpressionId::migrate(SEED, 206),
            },
        )
        .unwrap(),
    );
    assert!(
        ownership_contributions_with_bindings(&adopted, |_| Ok(Some(view.clone()))).is_err(),
        "BorrowCall cannot also own a separate initializer expression"
    );
}
