use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;
use crate::platform::storage::memory::MemoryPackedStore;

struct LayoutFixture {
    package: PackageId,
    outer: DeclarationId,
    inner: DeclarationId,
    inner_field: FieldId,
    parameter: TypeParameterId,
    owners: BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: BTreeMap<TypeObjectDigest, Vec<u8>>,
    outer_type: TypeObjectDigest,
    inner_type: TypeObjectDigest,
}

fn layout_fixture() -> LayoutFixture {
    let seed = b"owned-record-interface";
    let package = PackageId::migrate(seed, 0);
    let outer = DeclarationId::migrate(seed, 0);
    let inner = DeclarationId::migrate(seed, 1);
    let outer_field = FieldId::migrate(seed, 0);
    let inner_field = FieldId::migrate(seed, 1);
    let parameter = TypeParameterId::migrate(seed, 0);
    let mut types = BTreeMap::new();
    let mut ty = |form| {
        let (digest, bytes) = encode_type_object(&TypeObject::new(form).unwrap()).unwrap();
        types.insert(digest, bytes);
        digest
    };
    let i64_type = ty(TypeForm::I64);
    let inner_type = ty(TypeForm::Named {
        declaration: DeclarationReference {
            package,
            declaration: inner,
        },
    });
    let outer_type = ty(TypeForm::Applied {
        declaration: DeclarationReference {
            package,
            declaration: outer,
        },
        arguments: vec![i64_type],
    });
    let declaration = |id, name, fields, type_parameters| PackageInterfaceDeclaration {
        header: OwnerHeader::new(OwnerKey::Declaration(id), OwnerKind::OwnedRecord),
        name: Name::new(name).unwrap(),
        payload: PackageInterfaceDeclarationPayload::OwnedRecord {
            fields,
            type_parameters,
        },
    };
    let field = |id, parent, name, ty| {
        PackageInterfaceRecord::Field(FieldRecord {
            header: OwnerHeader::new(OwnerKey::Field(id), OwnerKind::Field),
            declaration: parent,
            name: Name::new(name).unwrap(),
            ty,
        })
    };
    let records = [
        PackageInterfaceRecord::Declaration(declaration(
            outer,
            "Outer",
            vec![outer_field],
            vec![parameter],
        )),
        PackageInterfaceRecord::RepresentationDeclaration(declaration(
            inner,
            "Inner",
            vec![inner_field],
            Vec::new(),
        )),
        field(outer_field, outer, "inner", inner_type),
        field(inner_field, inner, "value", i64_type),
        PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter),
            declaration: outer,
            name: Name::new("T").unwrap(),
            constraints: TypeParameterConstraints::Transferable,
        }),
    ];
    let owners = records
        .into_iter()
        .map(|record| {
            (
                record.header().owner,
                PackageInterfaceOwner {
                    contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record,
                },
            )
        })
        .collect();
    LayoutFixture {
        package,
        outer,
        inner,
        inner_field,
        parameter,
        owners,
        types,
        outer_type,
        inner_type,
    }
}

fn validate_layout(fixture: &LayoutFixture) -> Result<PackageInterfaceValidation, Diagnostic> {
    let built = build_package_interface(&fixture.owners, &fixture.types)?;
    let mut store = MemoryPackedStore::default();
    let mut work = StoreWork::default();
    for (key, bytes) in &built.objects {
        store.stage(*key, bytes, &mut work).unwrap();
    }
    validate_package_interface(fixture.package, built.root, &store, &mut work)
}

#[test]
fn interface20_preserves_private_owned_layout_without_public_reference_access() {
    let mut fixture = layout_fixture();
    let admitted = validate_layout(&fixture).unwrap();
    assert!(matches!(
        admitted.owners[&OwnerKey::Declaration(fixture.inner)].record,
        PackageInterfaceRecord::RepresentationDeclaration(_)
    ));
    assert!(interface_owned_type(
        fixture.package,
        fixture.inner_type,
        &admitted.type_objects,
        &admitted.owners
    ));
    let id = DeclarationId::migrate(b"private-layout-signature", 0);
    fixture.owners.insert(
        OwnerKey::Declaration(id),
        PackageInterfaceOwner {
            contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
            record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                header: OwnerHeader::new(OwnerKey::Declaration(id), OwnerKind::Constant),
                name: Name::new("leak").unwrap(),
                payload: PackageInterfaceDeclarationPayload::Constant {
                    ty: fixture.inner_type,
                },
            }),
        },
    );
    assert_eq!(
        validate_layout(&fixture).unwrap_err().code,
        "package_interface_private_signature"
    );
}

#[test]
fn interface20_rejects_missing_foreign_and_unreachable_representation_evidence() {
    for fault in ["missing", "foreign", "unreachable"] {
        let mut fixture = layout_fixture();
        match fault {
            "missing" => {
                fixture.owners.remove(&OwnerKey::Field(fixture.inner_field));
            }
            "foreign" => {
                let PackageInterfaceRecord::Field(field) = &mut fixture
                    .owners
                    .get_mut(&OwnerKey::Field(fixture.inner_field))
                    .unwrap()
                    .record
                else {
                    unreachable!()
                };
                field.declaration = fixture.outer;
            }
            "unreachable" => {
                let id = DeclarationId::migrate(b"unreachable-layout", 0);
                fixture.owners.insert(
                    OwnerKey::Declaration(id),
                    PackageInterfaceOwner {
                        contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                        record: PackageInterfaceRecord::RepresentationDeclaration(
                            PackageInterfaceDeclaration {
                                header: OwnerHeader::new(
                                    OwnerKey::Declaration(id),
                                    OwnerKind::OwnedRecord,
                                ),
                                name: Name::new("Unreachable").unwrap(),
                                payload: PackageInterfaceDeclarationPayload::OwnedRecord {
                                    fields: Vec::new(),
                                    type_parameters: Vec::new(),
                                },
                            },
                        ),
                    },
                );
            }
            _ => unreachable!(),
        }
        assert!(validate_layout(&fixture).is_err(), "{fault}");
    }
}

#[test]
fn interface20_checks_unused_parameter_bounds_and_recursive_owned_layouts() {
    let mut fixture = layout_fixture();
    let PackageInterfaceRecord::TypeParameter(parameter) = &mut fixture
        .owners
        .get_mut(&OwnerKey::TypeParameter(fixture.parameter))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    parameter.constraints = TypeParameterConstraints::CaptureSafe;
    assert_eq!(
        validate_layout(&fixture).unwrap_err().code,
        "package_interface_owned_record_parameter"
    );
    let mut fixture = layout_fixture();
    let PackageInterfaceRecord::Field(field) = &mut fixture
        .owners
        .get_mut(&OwnerKey::Field(fixture.inner_field))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    field.ty = fixture.outer_type;
    assert_eq!(
        validate_layout(&fixture).unwrap_err().code,
        "package_interface_owned_record_cycle"
    );
}

#[test]
fn interface20_rejects_owned_layouts_disguised_as_interface19() {
    let fixture = layout_fixture();
    for value in fixture.owners.values() {
        let mut hostile = value.clone();
        hostile.contract_version = 19;
        assert!(hostile.encode().is_err());
        let bytes = crate::platform::packed::encode(
            *b"LKJPIF19",
            "lkjscript.package-interface-owner-envelope.v19",
            &hostile,
            MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
        )
        .unwrap();
        assert!(
            PackageInterfaceOwner::decode(
                &bytes,
                hostile.owner(),
                PackageInterfaceOwnerDigest::of(&bytes)
            )
            .is_err()
        );
    }
}

#[test]
fn interface20_retains_historical_private_data_layout_generation() {
    let mut fixture = layout_fixture();
    let PackageInterfaceRecord::RepresentationDeclaration(record) = &mut fixture
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.inner))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    record.header.contract_version = 31;
    record.header.kind = OwnerKind::Record;
    record.payload = PackageInterfaceDeclarationPayload::Record {
        type_parameters: Vec::new(),
        fields: vec![fixture.inner_field],
    };
    let field = fixture
        .owners
        .get_mut(&OwnerKey::Field(fixture.inner_field))
        .unwrap();
    field.contract_version = 19;
    let PackageInterfaceRecord::Field(field) = &mut field.record else {
        unreachable!()
    };
    field.header.contract_version = 31;
    assert!(validate_layout(&fixture).is_ok());
    let mut marker = fixture.owners[&OwnerKey::Declaration(fixture.inner)].clone();
    marker.contract_version = 19;
    assert!(marker.encode().is_err());
}

type OwnedLayoutDag = (
    PackageId,
    BTreeMap<OwnerKey, PackageInterfaceOwner>,
    BTreeMap<TypeObjectDigest, TypeObject>,
    BTreeMap<OwnerKey, OwnerRecord>,
);

fn owned_layout_dag(declaration_count: usize, fanout: usize) -> OwnedLayoutDag {
    let seed = b"owned-record-dense-layout";
    let package = PackageId::migrate(seed, 0);
    let declarations = (0..declaration_count)
        .map(|index| DeclarationId::migrate(seed, index as u64))
        .collect::<Vec<_>>();
    let mut types = BTreeMap::new();
    let nominal_types = declarations
        .iter()
        .map(|declaration| {
            let object = TypeObject::new(TypeForm::Named {
                declaration: DeclarationReference {
                    package,
                    declaration: *declaration,
                },
            })
            .unwrap();
            let (digest, _) = encode_type_object(&object).unwrap();
            types.insert(digest, object);
            digest
        })
        .collect::<Vec<_>>();
    let mut owners = BTreeMap::new();
    let mut canonical = BTreeMap::new();
    let module = ModuleId::migrate(seed, 0);
    for (index, declaration) in declarations.iter().copied().enumerate() {
        let mut fields = Vec::new();
        for (target, ty) in nominal_types
            .iter()
            .copied()
            .enumerate()
            .skip(index + 1)
            .take(fanout)
        {
            let field = FieldId::migrate(seed, (index * fanout + target - index - 1) as u64);
            let record = FieldRecord {
                header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
                declaration,
                name: Name::new(format!("value_{target}")).unwrap(),
                ty,
            };
            fields.push(field);
            canonical.insert(record.header.owner, OwnerRecord::Field(record.clone()));
            owners.insert(
                record.header.owner,
                PackageInterfaceOwner {
                    contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record: PackageInterfaceRecord::Field(record),
                },
            );
        }
        fields.sort();
        let header = OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::OwnedRecord);
        let name = Name::new(format!("Node_{index}")).unwrap();
        let record = PackageInterfaceDeclaration {
            header,
            name: name.clone(),
            payload: PackageInterfaceDeclarationPayload::OwnedRecord {
                type_parameters: Vec::new(),
                fields: fields.clone(),
            },
        };
        canonical.insert(
            header.owner,
            OwnerRecord::Declaration(DeclarationRecord {
                header,
                module,
                name,
                visibility: if index == 0 {
                    DeclarationVisibility::Public
                } else {
                    DeclarationVisibility::Private
                },
                payload: DeclarationPayload::OwnedRecord {
                    type_parameters: Vec::new(),
                    fields,
                },
            }),
        );
        owners.insert(
            record.header.owner,
            PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: if index == 0 {
                    PackageInterfaceRecord::Declaration(record)
                } else {
                    PackageInterfaceRecord::RepresentationDeclaration(record)
                },
            },
        );
    }
    (package, owners, types, canonical)
}

#[test]
fn interface20_shared_layout_dag_is_bounded_without_enumerating_paths() {
    // Each node uses its next two siblings. Thirty-two nodes describe millions of
    // root-to-leaf paths while requiring only sixty-one distinct layout edges.
    let (package, owners, types, canonical) = owned_layout_dag(32, 2);
    let mut work = 0;
    let reached = representation_declarations(package, &owners, &types, &mut |count| {
        work += count;
        Ok(())
    })
    .unwrap();
    assert_eq!(reached.len(), 31);
    assert!(work < 1_000, "shared tails must be proved once: {work}");
    let mut budget = RepresentationBudget {
        bytes: 0,
        maximum: 4_096,
    };
    let error = representation_declarations_admitted(
        package,
        &owners,
        &types,
        &mut |_| Ok(()),
        &mut budget,
    )
    .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "package_interface_representation_storage");
    assert!(budget.bytes <= budget.maximum);

    let mut selection = PackageInterfaceSelection::from_records(package, &canonical).unwrap();
    let mut budget = RepresentationBudget {
        bytes: 0,
        maximum: 4_096,
    };
    let error = selection
        .observe_owned_representations_admitted(&canonical, &types, &BTreeMap::new(), &mut budget)
        .unwrap_err();
    assert_eq!(error.code, "package_interface_representation_storage");
    assert!(budget.bytes <= budget.maximum);
    assert!(selection.declarations.len() < 32);
}

#[test]
fn interface20_owned_layout_depth_counts_hidden_edges_with_an_empty_leaf() {
    let maximum = crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH;
    for edge_count in [maximum, maximum + 1] {
        // An empty terminal nominal record contributes zero child edges. A chain
        // of 257 such declarations has exactly 256 hidden representation edges.
        let (package, owners, types, _) = owned_layout_dag(edge_count + 1, 1);
        let types = types
            .into_iter()
            .map(|(digest, object)| {
                let (encoded_digest, bytes) = encode_type_object(&object).unwrap();
                assert_eq!(digest, encoded_digest);
                (digest, bytes)
            })
            .collect();
        let built = build_package_interface(&owners, &types).unwrap();
        let mut store = MemoryPackedStore::default();
        let mut work = StoreWork::default();
        for (key, bytes) in &built.objects {
            store.stage(*key, bytes, &mut work).unwrap();
        }
        let result = validate_package_interface(package, built.root, &store, &mut work);
        if edge_count == maximum {
            result.unwrap();
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Resource);
            assert_eq!(error.code, "package_interface_type_work");
        }
    }
}
