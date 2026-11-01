//! The canonical oracle must admit nominal custody without kernel/compiler proof reuse.
use super::*;
use crate::platform::kernel::{FieldReference, StructuralTypeField};

const SOURCE: &str = r#"declarations.begin
(units (module create nominal-reference
  (owned-record create Packet (visibility private)
    (field create payload (type ByteBuffer))
    (field create tag (type I64)))
  (owned-record create Other (visibility private)
    (field create payload (type ByteBuffer))
    (field create tag (type I64)))
  (owned-record create Empty (visibility private))
  (owned-record create Phantom (visibility private)
    (type-parameter create T (constraint owned transferable shareable)))
  (external create length (visibility private) (implementation core.buffer.length)
    (parameter create value (type ByteBuffer) (use borrow)) (returns I64))
  (function create pack (visibility private) (effect pure)
    (parameter create value (type ByteBuffer) (use consume)) (returns Packet)
    (body (pack-owned (type Packet) (field tag (i64 9)) (field payload (local value)))))
  (function create unpack (visibility private) (effect pure)
    (parameter create value (type Packet) (use consume)) (returns Unit)
    (body (unpack-owned (type Packet) (local value)
      (field tag (binding tag (type I64)))
      (field payload (binding payload (type ByteBuffer))) (in (unit)))))
  (function create borrow (visibility private) (effect pure)
    (parameter create value (type Packet) (use borrow)) (returns I64)
    (body (borrow-owned-field (type Packet) (local value)
      (field payload (binding view (type ByteBuffer))) (in (call length (local view))))))
  (function create metadata (visibility private) (effect pure)
    (parameter create value (type Packet) (use borrow)) (returns I64)
    (body (field (local value) Packet::tag)))
  (function create empty (visibility private) (effect pure) (returns Unit)
    (body (let (binding value (type Empty) (pack-owned (type Empty)))
      (in (unpack-owned (type Empty) (local value) (in (unit))))))))
  (module create foreign
    (function create marker (visibility private) (effect pure) (returns Unit) (body (unit)))))
declarations.end
"#;

fn source() -> KernelSnapshot {
    super::super::tests::byte_buffer_tests::author_only(SOURCE).unwrap()
}

fn declaration(source: &KernelSnapshot, name: &str) -> DeclarationReference {
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner))
                if owner.name.as_str() == name =>
            {
                Some(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *id,
                })
            }
            _ => None,
        })
        .unwrap()
}

fn field(source: &KernelSnapshot, owner: &str, name: &str) -> FieldReference {
    let declaration = declaration(source, owner);
    source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Field(field), OwnerRecord::Field(owner))
                if owner.declaration == declaration.declaration && owner.name.as_str() == name =>
            {
                Some(FieldReference {
                    package: declaration.package,
                    field: *field,
                })
            }
            _ => None,
        })
        .unwrap()
}

fn admit(source: &KernelSnapshot) -> Result<NormalizedReferenceSchema, ExecutionError> {
    NormalizedReferenceSchema::independently_admit_witness_source(source)
}

fn intern(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let (ty, _) = encode_type_object(&object).unwrap();
    source.types.insert(ty, object);
    ty
}

#[test]
fn canonical_owned_record_generic_fields_and_borrowed_results_admit_independently() {
    let mut source = super::super::tests::byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create generic-reference
  (owned-record create Parcel (visibility public)
    (type-parameter create T (constraint owned))
    (field create value (type T)) (field create stamp (type I64)))
  (owned-record create Data (visibility public)
    (type-parameter create T (constraint transferable)) (field create value (type T)))
  (function create selected (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create parcel (type (Parcel T)) (use borrow))
    (returns T (borrow-from parcel))
    (body (borrow-owned-field (type (Parcel T)) (local parcel)
      (field value (binding view (type T))) (in (local view)))))
  (function create data (visibility private) (effect pure)
    (parameter create value (type I64)) (returns I64)
    (body (let (binding owner (type (Data I64))
      (pack-owned (type (Data I64)) (field value (local value))))
      (in (unpack-owned (type (Data I64)) (local owner)
        (field value (binding result (type I64))) (in (local result)))))))))
declarations.end"#,
    )
    .unwrap();
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let parcel_declaration = declaration(&source, "Parcel");
    let parcel = intern(
        &mut source,
        TypeForm::Applied {
            declaration: parcel_declaration,
            arguments: vec![cell],
        },
    );
    let schema = admit(&source).expect("generic nominal fields retain their declaration scopes");
    let scalar = TypeObject::new(TypeForm::I64).unwrap();
    let (scalar, _) = encode_type_object(&scalar).unwrap();
    let data = TypeObject::new(TypeForm::Applied {
        declaration: declaration(&source, "Data"),
        arguments: vec![scalar],
    })
    .unwrap();
    let (data, _) = encode_type_object(&data).unwrap();
    assert!(schema.is_owned_type(data));
    assert_eq!(
        schema.owned_record_layout(data).unwrap().fields[0].ty,
        scalar
    );
    let control = crate::platform::execution::ExecutionControl::uncancelled();
    parallel_transfer_type(
        &schema,
        &[&source],
        parcel,
        None,
        &mut Meter::new(None),
        &mut Meter::new(None),
        &control,
    )
    .expect("actual cell movement does not require a bound on the record's owning formal");
    parallel_share_type(
        &schema,
        &[&source],
        parcel,
        None,
        &mut Meter::new(None),
        &mut Meter::new(None),
        &control,
    )
    .expect("actual cell sharing does not require a bound on the record's owning formal");
}

#[test]
fn canonical_owned_record_hidden_fields_share_the_structural_depth_boundary() {
    let mut source = super::super::tests::byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create depth-reference
  (owned-record create Leaf (visibility private) (field create value (type ByteBuffer)))
  (owned-record create Wrapper (visibility private) (field create value (type Leaf)))))
declarations.end"#,
    )
    .unwrap();
    let wrapper = declaration(&source, "Wrapper");
    let wrapper = intern(
        &mut source,
        TypeForm::Named {
            declaration: wrapper,
        },
    );
    let mut deep = wrapper;
    for _ in 0..crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH - 3 {
        deep = intern(&mut source, TypeForm::OwnedSequence { item: deep });
    }
    let product = |deep| TypeForm::OwnedProduct {
        fields: vec![
            StructuralTypeField {
                name: crate::platform::kernel::Name::new("deep").unwrap(),
                ty: deep,
            },
            StructuralTypeField {
                name: crate::platform::kernel::Name::new("short").unwrap(),
                ty: wrapper,
            },
        ],
    };
    intern(&mut source, product(deep));
    admit(&source).expect("combined sequence/product/nominal path at the global boundary");
    let deep = intern(&mut source, TypeForm::OwnedSequence { item: deep });
    intern(&mut source, product(deep));
    let error = admit(&source).unwrap_err();
    assert_eq!(error.code, "reference_product_depth");
    assert_ne!(
        error.class,
        crate::platform::execution::ExecutionFailureClass::Resource
    );
}

#[test]
fn canonical_owned_record_recursive_hidden_layout_rejects_independently() {
    let mut source = super::super::tests::byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create recursive-reference
  (owned-record create Owner (visibility private) (field create value (type ByteBuffer)))))
declarations.end"#,
    )
    .unwrap();
    admit(&source).unwrap();
    let owner = declaration(&source, "Owner");
    let owner_type = intern(&mut source, TypeForm::Named { declaration: owner });
    let recursive = intern(&mut source, TypeForm::OwnedSequence { item: owner_type });
    let value = field(&source, "Owner", "value");
    let OwnerRecord::Field(value) = source
        .owners
        .get_mut(&OwnerKey::Field(value.field))
        .unwrap()
    else {
        unreachable!();
    };
    value.ty = recursive;
    let error = admit(&source).unwrap_err();
    assert_ne!(
        error.class,
        crate::platform::execution::ExecutionFailureClass::Resource
    );
}

#[test]
fn canonical_owned_record_empty_and_scalar_custody_are_affine_and_complete() {
    let source = source();
    let schema = admit(&source).expect("canonical nominal control admits independently");
    for name in ["Packet", "Other", "Empty"] {
        let object = TypeObject::new(TypeForm::Named {
            declaration: declaration(&source, name),
        })
        .unwrap();
        let (ty, _) = encode_type_object(&object).unwrap();
        assert!(schema.is_owned_type(ty));
        assert!(schema.owned_record_layout(ty).is_some());
        assert!(schema.transferable_types.contains(&ty));
        assert!(schema.shareable_types.contains(&ty));
        assert!(!schema.ordinary_types.contains(&ty));
        assert!(!schema.buffer_free_types.contains(&ty));
        assert!(!schema.capture_safe_types.contains(&ty));
        assert!(!schema.comparable_types.contains(&ty));
    }
}

#[test]
fn canonical_owned_record_empty_layout_cannot_cross_typed_data_boundaries() {
    use super::super::value::{NormalizedRecord, NormalizedValue, RecordLayoutIndex};
    use super::super::value_schema::NormalizedValueSchema;
    let source = source();
    let schema = admit(&source).unwrap();
    let object = TypeObject::new(TypeForm::Named {
        declaration: declaration(&source, "Empty"),
    })
    .unwrap();
    let (ty, _) = encode_type_object(&object).unwrap();
    let index = schema.record_instances[&ty];
    let value = NormalizedValue::Record(NormalizedRecord::Nominal {
        layout: RecordLayoutIndex(index as u32, schema.value_origin()),
        fields: Arc::new(Vec::new()),
    });
    assert!(super::super::data_codec_reference::encode_typed(&schema, &value, ty).is_err());
    let mut falsely_ordinary = schema.clone();
    falsely_ordinary.records[index].owned = false;
    let bytes =
        super::super::data_codec_reference::encode_typed(&falsely_ordinary, &value, ty).unwrap();
    assert!(super::super::data_codec_reference::decode_typed(&schema, &bytes, ty).is_err());
}

#[test]
fn canonical_owned_record_field_operations_require_exact_module_without_kernel_admission() {
    let source = source();
    admit(&source).unwrap();
    let foreign = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Module(owner) if owner.name.as_str() == "foreign" => {
                match owner.header.owner {
                    OwnerKey::Module(id) => Some(id),
                    _ => None,
                }
            }
            _ => None,
        })
        .unwrap();
    for name in ["pack", "unpack", "borrow", "metadata", "empty"] {
        let mut changed = source.clone();
        let reference = declaration(&source, name);
        let OwnerRecord::Declaration(owner) = changed
            .owners
            .get_mut(&OwnerKey::Declaration(reference.declaration))
            .unwrap()
        else {
            unreachable!();
        };
        owner.module = foreign;
        assert!(
            admit(&changed).is_err(),
            "foreign module {name} must reject independently"
        );
    }
}

#[test]
fn canonical_owned_record_same_shape_foreign_fields_and_duplicate_coverage_reject() {
    let source = source();
    admit(&source).unwrap();
    let payload = field(&source, "Packet", "payload");
    let foreign = field(&source, "Other", "payload");
    let OwnerRecord::Field(payload_record) = &source.owners[&OwnerKey::Field(payload.field)] else {
        unreachable!()
    };
    let OwnerRecord::Field(foreign_record) = &source.owners[&OwnerKey::Field(foreign.field)] else {
        unreachable!()
    };
    assert_eq!(payload_record.ty, foreign_record.ty);
    assert_ne!(payload, foreign);
    assert_ne!(payload_record.declaration, foreign_record.declaration);
    for kind in 0..3 {
        let mut changed = source.clone();
        let operation = changed
            .owners
            .values_mut()
            .find_map(|owner| match owner {
                OwnerRecord::Expression(owner)
                    if match (&owner.operation, kind) {
                        (ExpressionOperation::PackOwnedRecord { fields, .. }, 0) => {
                            fields.iter().any(|field| field.field == payload)
                        }
                        (ExpressionOperation::UnpackOwnedRecord { fields, .. }, 1) => {
                            fields.iter().any(|field| field.field == payload)
                        }
                        (ExpressionOperation::BorrowOwnedRecordField { .. }, 2) => true,
                        _ => false,
                    } =>
                {
                    Some(&mut owner.operation)
                }
                _ => None,
            })
            .unwrap();
        match operation {
            ExpressionOperation::PackOwnedRecord { fields, .. } => {
                fields
                    .iter_mut()
                    .find(|field| field.field == payload)
                    .unwrap()
                    .field = foreign;
            }
            ExpressionOperation::UnpackOwnedRecord { fields, .. } => {
                fields
                    .iter_mut()
                    .find(|field| field.field == payload)
                    .unwrap()
                    .field = foreign;
            }
            ExpressionOperation::BorrowOwnedRecordField { field, .. } => {
                assert_eq!(*field, payload);
                *field = foreign;
            }
            _ => unreachable!(),
        }
        assert!(
            admit(&changed).is_err(),
            "same-shaped foreign field operation {kind}"
        );
    }
    let mut changed = source.clone();
    let fields = changed
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(owner) => match &mut owner.operation {
                ExpressionOperation::PackOwnedRecord { fields, .. } if fields.len() == 2 => {
                    Some(fields)
                }
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    fields[1].field = fields[0].field;
    assert!(admit(&changed).is_err());
}

#[test]
fn canonical_owned_record_phantom_arguments_and_parameter_constraints_are_admitted_completely() {
    let source = source();
    admit(&source).unwrap();
    let mut changed = source.clone();
    let cell = intern(&mut changed, TypeForm::OwnedI64Cell);
    let secret = intern(&mut changed, TypeForm::Secret);
    let poison = intern(
        &mut changed,
        TypeForm::OwnedProduct {
            fields: vec![
                StructuralTypeField {
                    name: crate::platform::kernel::Name::new("cell").unwrap(),
                    ty: cell,
                },
                StructuralTypeField {
                    name: crate::platform::kernel::Name::new("secret").unwrap(),
                    ty: secret,
                },
            ],
        },
    );
    let nominal = declaration(&changed, "Phantom");
    intern(
        &mut changed,
        TypeForm::Applied {
            declaration: nominal,
            arguments: vec![poison],
        },
    );
    assert!(
        admit(&changed).is_err(),
        "unused owned nominal argument must reject poison"
    );

    let mut changed = source;
    let owner = changed
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::TypeParameter(owner) if owner.declaration == nominal.declaration => {
                Some(owner)
            }
            _ => None,
        })
        .unwrap();
    owner.constraints = crate::platform::kernel::TypeParameterConstraints::None;
    assert!(
        admit(&changed).is_err(),
        "unused nominal parameter may not erase its contract"
    );
}
