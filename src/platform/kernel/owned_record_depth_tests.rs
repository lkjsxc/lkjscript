//! Combined layout-depth counterexamples independent of native authoring.
use super::*;
use crate::platform::semantic_id::{DeclarationId, ModuleId};

const SEED: &[u8] = b"owned-record-combined-depth";

#[derive(Clone, Default)]
struct Read {
    owners: BTreeMap<OwnerKey, OwnerRecord>,
    types: BTreeMap<TypeObjectDigest, TypeObject>,
    imports: BTreeMap<(PackageId, OwnerKey), PackageInterfaceRecord>,
    representations: BTreeMap<(PackageId, OwnerKey), PackageInterfaceRecord>,
}

impl ExpressionRead for Read {
    fn package_id(&self) -> PackageId {
        PackageId::migrate(SEED, 0)
    }
    fn owner(&self, key: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        Ok(self.owners.get(&key).cloned())
    }
    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        Ok(self.types.get(&digest).cloned())
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        Ok(self.imports.get(&(package, owner)).cloned())
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        Ok(self
            .imports
            .keys()
            .any(|(candidate, _)| *candidate == package))
    }
    fn package_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        Ok(self
            .imports
            .get(&(package, owner))
            .or_else(|| self.representations.get(&(package, owner)))
            .cloned())
    }
    fn has_representation_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        Ok(self.has_dependency(package)?
            || self
                .representations
                .keys()
                .any(|(candidate, _)| *candidate == package))
    }
}

#[test]
fn opaque_import_proves_transitive_layout_without_granting_reference_authority() {
    let mut read = Read::default();
    let imported = PackageId::migrate(SEED, 1);
    let hidden = PackageId::migrate(SEED, 2);
    let public_id = DeclarationId::migrate(SEED, 10);
    let private_owner_id = DeclarationId::migrate(SEED, 11);
    let private_data_id = DeclarationId::migrate(SEED, 12);
    let private_owner = DeclarationReference {
        package: hidden,
        declaration: private_owner_id,
    };
    let private_data = DeclarationReference {
        package: hidden,
        declaration: private_data_id,
    };
    let public = DeclarationReference {
        package: imported,
        declaration: public_id,
    };
    let cell = read.ty(TypeForm::OwnedI64Cell);
    let scalar = read.ty(TypeForm::I64);
    let owner_type = read.ty(TypeForm::Named {
        declaration: private_owner,
    });
    let data_type = read.ty(TypeForm::Named {
        declaration: private_data,
    });
    let public_type = read.ty(TypeForm::Named {
        declaration: public,
    });
    for (package, declaration, ordinal, owned, field_types) in [
        (hidden, private_owner_id, 11, true, vec![cell]),
        (hidden, private_data_id, 12, false, vec![scalar]),
        (imported, public_id, 10, true, vec![owner_type, data_type]),
    ] {
        let evidence = if package == imported {
            &mut read.imports
        } else {
            &mut read.representations
        };
        let mut fields = Vec::new();
        for (index, ty) in field_types.into_iter().enumerate() {
            let field = FieldId::migrate(SEED, ordinal * 10 + index as u64);
            fields.push(field);
            evidence.insert(
                (package, OwnerKey::Field(field)),
                PackageInterfaceRecord::Field(FieldRecord {
                    header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
                    declaration,
                    name: Name::new(format!("field{index}")).unwrap(),
                    ty,
                }),
            );
        }
        let record = PackageInterfaceDeclaration {
            header: OwnerHeader::new(
                OwnerKey::Declaration(declaration),
                if owned {
                    OwnerKind::OwnedRecord
                } else {
                    OwnerKind::Record
                },
            ),
            name: Name::new(format!("Record{ordinal}")).unwrap(),
            payload: if owned {
                PackageInterfaceDeclarationPayload::OwnedRecord {
                    type_parameters: Vec::new(),
                    fields,
                }
            } else {
                PackageInterfaceDeclarationPayload::Record {
                    type_parameters: Vec::new(),
                    fields,
                }
            },
        };
        evidence.insert(
            (package, OwnerKey::Declaration(declaration)),
            if package == imported {
                PackageInterfaceRecord::Declaration(record)
            } else {
                PackageInterfaceRecord::RepresentationDeclaration(record)
            },
        );
    }
    assert!(read.has_dependency(imported).unwrap());
    assert!(!read.has_dependency(hidden).unwrap());
    assert!(
        read.package_interface_owner(hidden, OwnerKey::Declaration(private_owner_id))
            .unwrap()
            .is_none()
    );
    assert!(read.has_representation_dependency(hidden).unwrap());
    validate_application(&read, public_type, None).unwrap();
    assert!(super::super::transfer::admit(&read, public_type, None).unwrap());
    assert!(super::super::share::admit(&read, public_type, None).unwrap());
    assert_eq!(layout(&read, public_type).unwrap().fields.len(), 2);
    let caller_scope = Some(DeclarationReference {
        package: read.package_id(),
        declaration: DeclarationId::migrate(SEED, 99),
    });
    assert!(require_access(&read, public_type, caller_scope).is_err());
    assert!(require_access(&read, owner_type, caller_scope).is_err());

    // Trusted layout reads must also reveal invalid hidden custody inside an
    // ordinary dependency; missing public import authority cannot erase it.
    let field = FieldId::migrate(SEED, 120);
    let Some(PackageInterfaceRecord::Field(record)) = read
        .representations
        .get_mut(&(hidden, OwnerKey::Field(field)))
    else {
        unreachable!()
    };
    record.ty = owner_type;
    assert!(super::super::memory::contains(&read, data_type).unwrap());
    assert!(validate_application(&read, public_type, None).is_err());
}

impl Read {
    fn ty(&mut self, form: TypeForm) -> TypeObjectDigest {
        let object = TypeObject::new(form).unwrap();
        let digest = encode_type_object(&object).unwrap().0;
        self.types.insert(digest, object);
        digest
    }

    fn record(&mut self, ordinal: u64, field_type: TypeObjectDigest) -> TypeObjectDigest {
        let declaration = DeclarationId::migrate(SEED, ordinal);
        let field = FieldId::migrate(SEED, ordinal);
        self.owners.insert(
            OwnerKey::Field(field),
            OwnerRecord::Field(FieldRecord {
                header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
                declaration,
                name: Name::new("child").unwrap(),
                ty: field_type,
            }),
        );
        self.owners.insert(
            OwnerKey::Declaration(declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(declaration),
                    OwnerKind::OwnedRecord,
                ),
                module: ModuleId::migrate(SEED, 0),
                name: Name::new(format!("Record{ordinal}")).unwrap(),
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::OwnedRecord {
                    type_parameters: Vec::new(),
                    fields: vec![field],
                },
            }),
        );
        self.ty(TypeForm::Named {
            declaration: DeclarationReference {
                package: self.package_id(),
                declaration,
            },
        })
    }
}

#[test]
fn combined_structural_and_nominal_depth_obeys_one_global_boundary() {
    let mut read = Read::default();
    let mut ty = read.ty(TypeForm::ByteBuffer);
    for edge in 0..contract::MAXIMUM_TYPE_DEPTH {
        ty = if edge % 2 == 0 {
            read.ty(TypeForm::OwnedSequence { item: ty })
        } else {
            read.record(edge as u64, ty)
        };
    }
    validate_type_depth(&read, ty, 0).unwrap();
    let too_deep = read.ty(TypeForm::OwnedSequence { item: ty });
    let error = validate_type_depth(&read, too_deep, 0).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "kernel_owned_record_depth");
    assert!(super::super::owned_product::validate(&read, too_deep, None).is_err());
}

#[test]
fn shared_nominal_child_is_rechecked_on_its_longest_path() {
    let mut read = Read::default();
    let leaf = read.ty(TypeForm::ByteBuffer);
    let shared = read.record(1, leaf);
    let mut deep = shared;
    for _ in 0..contract::MAXIMUM_TYPE_DEPTH - 1 {
        deep = read.ty(TypeForm::OwnedSequence { item: deep });
    }
    // LIFO traversal encounters the shared nominal child first through the short
    // path. That admission cannot erase its hidden field on the longer path.
    let root = read.ty(TypeForm::OwnedProduct {
        fields: vec![
            StructuralTypeField {
                name: Name::new("deep").unwrap(),
                ty: deep,
            },
            StructuralTypeField {
                name: Name::new("short").unwrap(),
                ty: shared,
            },
        ],
    });
    assert_eq!(
        validate_type_depth(&read, root, 0).unwrap_err().code,
        "kernel_owned_record_depth"
    );
}

#[test]
fn nested_generic_record_instances_are_finite_but_representation_cycles_reject() {
    let mut read = Read::default();
    let declaration = DeclarationId::migrate(SEED, 2);
    let reference = DeclarationReference {
        package: read.package_id(),
        declaration,
    };
    let parameter = TypeParameterId::migrate(SEED, 2);
    let parameter_type = read.ty(TypeForm::TypeParameter { parameter });
    let _ = read.record(2, parameter_type);
    let OwnerRecord::Declaration(record) = read
        .owners
        .get_mut(&OwnerKey::Declaration(declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedRecord {
        type_parameters, ..
    } = &mut record.payload
    else {
        unreachable!()
    };
    type_parameters.push(parameter);
    read.owners.insert(
        OwnerKey::TypeParameter(parameter),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter),
            declaration,
            name: Name::new("T").unwrap(),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    let cell = read.ty(TypeForm::OwnedI64Cell);
    let inner = read.ty(TypeForm::Applied {
        declaration: reference,
        arguments: vec![cell],
    });
    let outer = read.ty(TypeForm::Applied {
        declaration: reference,
        arguments: vec![inner],
    });
    validate_application(&read, outer, None).unwrap();
    let field = FieldId::migrate(SEED, 2);
    let recursive = read.ty(TypeForm::Applied {
        declaration: reference,
        arguments: vec![parameter_type],
    });
    let recursive_sequence = read.ty(TypeForm::OwnedSequence { item: recursive });
    let OwnerRecord::Field(record) = read.owners.get_mut(&OwnerKey::Field(field)).unwrap() else {
        unreachable!()
    };
    record.ty = recursive_sequence;
    let error = validate_definition(&read, reference).unwrap_err();
    assert!(error.message.contains("recursive owned record"));
}

#[test]
fn shared_nominal_dag_cycle_proof_fits_linear_metadata_and_rejects_back_edges() {
    let mut read = Read::default();
    let leaf = read.ty(TypeForm::OwnedI64Cell);
    let mut records = vec![read.record(1000, leaf), read.record(1001, leaf)];
    for ordinal in 2..32 {
        let children = read.ty(TypeForm::OwnedProduct {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("left").unwrap(),
                    ty: records[ordinal - 1],
                },
                StructuralTypeField {
                    name: Name::new("right").unwrap(),
                    ty: records[ordinal - 2],
                },
            ],
        });
        records.push(read.record(1000 + ordinal as u64, children));
    }
    let root = nominal(&read, *records.last().unwrap()).unwrap().0;
    // These 32 declarations have millions of distinct root-to-leaf paths. A
    // cycle proof that copies or enumerates paths exhausts this small admission.
    let mut budget = LayoutBudget::with_metadata_limit(65_536);
    validate_representation_cycles(&read, root, &mut budget).unwrap();

    let field = FieldId::migrate(SEED, 1000);
    let Some(OwnerRecord::Field(record)) = read.owners.get_mut(&OwnerKey::Field(field)) else {
        unreachable!()
    };
    record.ty = *records.last().unwrap();
    let mut budget = LayoutBudget::with_metadata_limit(65_536);
    let error = validate_representation_cycles(&read, root, &mut budget).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert!(error.message.contains("recursive owned record"));

    let error =
        validate_representation_cycles(&read, root, &mut LayoutBudget::with_metadata_limit(0))
            .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "kernel_owned_record_storage");
}

#[test]
fn nominal_parameters_keep_existing_owned_bound_envelope_generations() {
    for (constraints, generation) in [
        (TypeParameterConstraints::Owned, 18),
        (TypeParameterConstraints::OwnedTransferable, 22),
        (TypeParameterConstraints::OwnedShareable, 30),
        (TypeParameterConstraints::OwnedTransferableShareable, 30),
    ] {
        let mut read = Read::default();
        let declaration = DeclarationId::migrate(SEED, 2000);
        let parameter = TypeParameterId::migrate(SEED, 2000);
        let parameter_type = read.ty(TypeForm::TypeParameter { parameter });
        read.record(2000, parameter_type);
        let Some(OwnerRecord::Declaration(record)) =
            read.owners.get_mut(&OwnerKey::Declaration(declaration))
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedRecord {
            type_parameters, ..
        } = &mut record.payload
        else {
            unreachable!()
        };
        type_parameters.push(parameter);
        let mut header =
            OwnerHeader::new(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter);
        header.contract_version = generation;
        let record = OwnerRecord::TypeParameter(TypeParameterRecord {
            header,
            declaration,
            name: Name::new("T").unwrap(),
            constraints,
        });
        read.owners
            .insert(OwnerKey::TypeParameter(parameter), record.clone());
        record.validate_local().unwrap();
        super::super::memory::validate_owner(&read, OwnerKey::TypeParameter(parameter), &record)
            .unwrap();
        let cell = read.ty(TypeForm::OwnedI64Cell);
        let applied = read.ty(TypeForm::Applied {
            declaration: DeclarationReference {
                package: read.package_id(),
                declaration,
            },
            arguments: vec![cell],
        });
        validate_application(&read, applied, None).unwrap();
    }
}
