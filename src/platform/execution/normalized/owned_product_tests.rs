//! Literal native product programs and independent expected results.
use super::*;

fn composed_source() -> crate::platform::kernel::KernelSnapshot {
    byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-products-independent.lkjc"),
            include_str!("../../../../tests/fixtures/owned-products-transformer.lkjc"),
            include_str!("../../../../tests/fixtures/owned-products-consumer.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap()
}

#[test]
fn owned_products_type_encoding_is_disjoint_and_fields_are_canonical() {
    use crate::platform::kernel::{
        StructuralTypeField, TypeObject, contract, decode_type_object, encode_type_object,
    };
    let child = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    let fields = vec![
        StructuralTypeField {
            name: Name::new("a").unwrap(),
            ty: child,
        },
        StructuralTypeField {
            name: Name::new("b").unwrap(),
            ty: child,
        },
    ];
    let product = TypeObject::new(TypeForm::OwnedProduct {
        fields: fields.clone(),
    })
    .unwrap();
    let (digest, bytes) = encode_type_object(&product).unwrap();
    assert_eq!(&bytes[..8], b"LKJPRD01");
    assert_eq!(decode_type_object(&bytes, digest).unwrap(), product);
    // Ordinary record encoding is still distinct, even before semantic admission
    // rejects the record's affine children.
    let record = TypeObject::new(TypeForm::StructuralRecord {
        fields: fields.clone(),
    })
    .unwrap();
    let (ordinary, bytes) = encode_type_object(&record).unwrap();
    assert_ne!(digest, ordinary);
    assert_eq!(&bytes[..8], b"LKJTYP10");
    let mut disguised = product;
    disguised.contract_version = 10;
    let bytes = crate::platform::packed::encode(
        contract::TYPE_OBJECT_MAGIC,
        contract::TYPE_OBJECT_ENVELOPE_DOMAIN,
        &disguised,
        contract::MAXIMUM_TYPE_OBJECT_BYTES,
    )
    .unwrap();
    assert!(decode_type_object(&bytes, TypeObjectDigest::of(&bytes)).is_err());
    for fields in [
        vec![],
        vec![fields[0].clone(), fields[0].clone()],
        vec![fields[1].clone(), fields[0].clone()],
        vec![fields[0].clone(); contract::MAXIMUM_CHILDREN + 1],
    ] {
        assert!(TypeObject::new(TypeForm::OwnedProduct { fields }).is_err());
    }
}

#[test]
fn owned_products_supported_depth_drops_iteratively_with_surviving_markers() {
    use super::super::{
        byte_buffer::ByteBuffer,
        owned_product::{OwnedProduct, StorageObservation},
        value::ValueOrigin,
    };
    use crate::platform::kernel::{StructuralTypeField, TypeObject, contract, encode_type_object};
    let products = StorageObservation::start();
    let buffers = super::super::byte_buffer::StorageObservation::start();
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let mut ty = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    let child = ByteBuffer::create(origin, &control, &mut |_| Ok(())).unwrap();
    let child_marker = child.clone();
    let mut value = NormalizedValue::ByteBuffer(child);
    let mut markers = Vec::new();
    for _ in 0..contract::MAXIMUM_TYPE_DEPTH {
        ty = encode_type_object(
            &TypeObject::new(TypeForm::OwnedProduct {
                fields: vec![StructuralTypeField {
                    name: Name::new("child").unwrap(),
                    ty,
                }],
            })
            .unwrap(),
        )
        .unwrap()
        .0;
        let product =
            OwnedProduct::create(origin, ty, vec![value], &control, &mut |_| Ok(())).unwrap();
        markers.push(product.clone());
        value = NormalizedValue::OwnedProduct(product);
    }
    assert_eq!(products.live(), (contract::MAXIMUM_TYPE_DEPTH, 0));
    drop(value);
    assert_eq!(products.live(), (0, 0));
    assert_eq!(buffers.live(), (0, 0));
    assert!(child_marker.validate(origin, false).is_err());
    assert!(
        markers
            .iter()
            .all(|marker| marker.validate(origin, false).is_err())
    );
}

// Adversarial type objects, not an authoring generator: the native workflows use
// the retained literal requests. These shapes exercise shared-path admission.
fn product_type_in(
    types: &mut BTreeMap<TypeObjectDigest, crate::platform::kernel::TypeObject>,
    fields: &[(&str, TypeObjectDigest)],
) -> TypeObjectDigest {
    use crate::platform::kernel::{StructuralTypeField, TypeObject, encode_type_object};
    let object = TypeObject::new(TypeForm::OwnedProduct {
        fields: fields
            .iter()
            .map(|(name, ty)| StructuralTypeField {
                name: Name::new(*name).unwrap(),
                ty: *ty,
            })
            .collect(),
    })
    .unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    types.insert(digest, object);
    digest
}

#[test]
fn owned_products_shared_type_paths_cannot_hide_excessive_depth() {
    let mut source = byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create depth (function create relay (visibility private) (effect pure)
  (parameter create p (type (owned-product (field child ByteBuffer))) (use consume))
  (returns (owned-product (field child ByteBuffer))) (body (local p)))))
declarations.end"#,
    )
    .unwrap();
    let original = source
        .types
        .iter()
        .find_map(|(ty, t)| matches!(t.form, TypeForm::OwnedProduct { .. }).then_some(*ty))
        .unwrap();
    let shared_depth = 4;
    let mut shared = original;
    for _ in 1..shared_depth {
        shared = product_type_in(&mut source.types, &[("child", shared)]);
    }
    let mut deep = shared;
    for _ in shared_depth..crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH - 1 {
        deep = product_type_in(&mut source.types, &[("child", deep)]);
    }
    deep = product_type_in(
        &mut source.types,
        &[("a", shared), ("middle", deep), ("z", shared)],
    );
    let set = |source: &mut crate::platform::kernel::KernelSnapshot, ty| {
        for record in source.owners.values_mut() {
            match record {
                OwnerRecord::Parameter(p) => p.ty = ty,
                OwnerRecord::Declaration(d) => {
                    if let crate::platform::kernel::DeclarationPayload::Function(f) = &mut d.payload
                    {
                        f.result = ty;
                    }
                }
                _ => {}
            }
        }
    };
    set(&mut source, deep);
    crate::platform::kernel::validate_full(&source).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let excessive = product_type_in(
        &mut source.types,
        &[("a", shared), ("middle", deep), ("z", shared)],
    );
    set(&mut source, excessive);
    let errors = crate::platform::kernel::validate_full(&source).unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == "kernel_owned_product"),
        "{errors:?}"
    );
    assert!(!crate::platform::kernel::memory_reference::accepts(&source));
}

#[test]
fn owned_products_closed_generic_substitutions_enforce_full_depth_in_both_derivations() {
    use crate::platform::kernel::{TypeObject, encode_type_object};
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-products.lkjc"
    ))
    .unwrap();
    let (parameter_type, parameter) = source
        .types
        .iter()
        .find_map(|(ty, t)| match t.form {
            TypeForm::TypeParameter { parameter } => Some((*ty, parameter)),
            _ => None,
        })
        .unwrap();
    let mut types = source.types.clone();
    let symbolic = product_type_in(&mut types, &[("payload", parameter_type)]);
    let buffer = TypeObject::new(TypeForm::ByteBuffer).unwrap();
    let mut closed = encode_type_object(&buffer).unwrap().0;
    types.insert(closed, buffer);
    for _ in 0..crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH - 1 {
        closed = product_type_in(&mut types, &[("payload", closed)]);
    }
    let bindings = BTreeMap::from([(parameter, closed)]);
    super::super::prepared_types::check_product_substitution(
        types.clone(),
        symbolic,
        bindings.clone(),
    )
    .unwrap();
    super::super::reference_types::check_product_substitution(types.clone(), symbolic, bindings)
        .unwrap();
    closed = product_type_in(&mut types, &[("payload", closed)]);
    let bindings = BTreeMap::from([(parameter, closed)]);
    let error = super::super::prepared_types::check_product_substitution(
        types.clone(),
        symbolic,
        bindings.clone(),
    )
    .unwrap_err();
    assert_eq!(error.code, "normalized_product_depth");
    assert_eq!(error.class, crate::platform::DiagnosticClass::Semantic);
    let error =
        super::super::reference_types::check_product_substitution(types, symbolic, bindings)
            .unwrap_err();
    assert_eq!(error.code, "reference_product_depth");
    assert_ne!(error.class, ExecutionFailureClass::Resource);
}

#[test]
fn owned_products_composition_nested_cleanup_and_tail_transfer() {
    let source = composed_source();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        let products = super::super::owned_product::StorageObservation::start();
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        let buffers = super::super::byte_buffer::StorageObservation::start();
        for n in [i64::MIN, 128, i64::MAX] {
            for name in ["main", "nested", "abandoned"] {
                let entry = declaration_named(&source, name);
                let args = if name == "abandoned" {
                    vec![]
                } else {
                    vec![NormalizedValue::I64(n)]
                };
                let actual = if reference {
                    NormalizedReferenceInterpreter::new(
                        &source,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .invoke(entry, args, None, &control)
                    .unwrap()
                    .0
                } else {
                    NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                        .invoke(entry, args, None, &control)
                        .unwrap()
                        .0
                };
                match name {
                    "nested" => assert_eq!(actual, NormalizedValue::bytes(vec![0, 255, 128])),
                    "abandoned" => assert_eq!(actual, NormalizedValue::Unit),
                    _ => {
                        let NormalizedValue::Record(
                            super::super::value::NormalizedRecord::Structural { fields },
                        ) = actual
                        else {
                            panic!("ordinary result");
                        };
                        assert_eq!(
                            fields.as_ref(),
                            &[
                                (
                                    Name::new("bytes").unwrap(),
                                    NormalizedValue::bytes(vec![0, 255, 128])
                                ),
                                (Name::new("opaque").unwrap(), NormalizedValue::I64(99)),
                                (Name::new("scalar").unwrap(), NormalizedValue::I64(n)),
                                (Name::new("tag").unwrap(), NormalizedValue::I64(128)),
                            ]
                        );
                    }
                }
                assert_eq!(products.live(), (0, 0));
                assert_eq!(cells.live(), (0, 0));
                assert_eq!(buffers.live(), (0, 0));
                products.assert_transfers_preserve_allocations();
            }
        }
    }
}

#[test]
fn owned_products_signature_only_requires_graph_19() {
    use crate::platform::kernel::{encode_owner, validate_full};
    let source = byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create signatures
  (function create relay (visibility public) (effect pure)
    (parameter create p (type (owned-product (field data ByteBuffer))) (use consume))
    (returns (owned-product (field data ByteBuffer))) (body (local p)))))
declarations.end"#,
    )
    .unwrap();
    validate_full(&source).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    prepare_snapshot(&source);
    let mut old = source.clone();
    old.root.graph_contract_version = 18;
    for record in old.owners.values_mut() {
        record.set_encoding_for_edit(18);
        // Product types have their own additive envelope; all owner encodings
        // are otherwise valid Graph 18 and rehash under the frozen domain.
        encode_owner(record).unwrap();
    }
    let errors = validate_full(&old).unwrap_err();
    assert!(
        errors.iter().any(|e| e.code == "kernel_product_generation"),
        "{errors:?}"
    );
    assert_ne!(
        crate::platform::kernel::semantic_state_digest(&old).unwrap(),
        crate::platform::kernel::semantic_state_digest(&source).unwrap()
    );
    assert!(!crate::platform::kernel::memory_reference::accepts(&old));
}

#[test]
fn owned_products_canonical_unpack_binding_scope_is_exact() {
    use crate::platform::kernel::{
        ExpressionOperation, LocalValueReference, OwnerKey, OwnerRecord, validate_full,
    };
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-products.lkjc"
    ))
    .unwrap();
    let (unpack, bindings, body) = source
        .owners
        .iter()
        .find_map(|(key, record)| {
            if let OwnerRecord::Expression(e) = record
                && let ExpressionOperation::UnpackOwned { fields, body, .. } = &e.operation
            {
                Some((*key, fields.clone(), *body))
            } else {
                None
            }
        })
        .unwrap();
    let outer = source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Binding(id), OwnerRecord::Binding(b)) if b.name.as_str() == "a" => Some(*id),
            _ => None,
        })
        .unwrap();
    for fault in [
        "duplicate",
        "outer",
        "type",
        "foreign-parameter",
        "binding-origin",
    ] {
        let mut bad = source.clone();
        match fault {
            "duplicate" | "outer" => {
                let OwnerRecord::Expression(e) = bad.owners.get_mut(&unpack).unwrap() else {
                    unreachable!()
                };
                let ExpressionOperation::UnpackOwned { fields, .. } = &mut e.operation else {
                    unreachable!()
                };
                fields[1].binding = if fault == "duplicate" {
                    fields[0].binding
                } else {
                    outer
                };
            }
            "type" | "binding-origin" => {
                let ordinary = match &source.owners[&OwnerKey::Binding(bindings[1].binding)] {
                    OwnerRecord::Binding(b) => b.declared_type,
                    _ => unreachable!(),
                };
                let OwnerRecord::Binding(b) = bad
                    .owners
                    .get_mut(&OwnerKey::Binding(bindings[0].binding))
                    .unwrap()
                else {
                    unreachable!()
                };
                if fault == "type" {
                    b.declared_type = ordinary;
                } else {
                    b.kind = crate::platform::kernel::BindingKind::Let;
                }
            }
            "foreign-parameter" => {
                let foreign = source
                    .owners
                    .iter()
                    .find_map(|(key, record)| match (key, record) {
                        (OwnerKey::Parameter(id), OwnerRecord::Parameter(p))
                            if p.name.as_str() == "payload" =>
                        {
                            Some(*id)
                        }
                        _ => None,
                    })
                    .unwrap();
                let OwnerRecord::Expression(e) =
                    bad.owners.get_mut(&OwnerKey::Expression(body)).unwrap()
                else {
                    unreachable!()
                };
                e.operation = ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(foreign),
                };
            }
            _ => unreachable!(),
        }
        assert!(validate_full(&bad).is_err(), "{fault}");
        assert!(
            !crate::platform::kernel::memory_reference::accepts(&bad),
            "{fault}"
        );
    }
}

#[test]
fn owned_products_unpack_cannot_reuse_a_foreign_unpack_identity() {
    let mut source = byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create scopes
  (function create a (visibility private) (effect pure)
    (parameter create p (type (owned-product (field cell OwnedI64Cell))) (use consume))
    (returns Unit) (body (unpack-owned (type (owned-product (field cell OwnedI64Cell)))
      (local p) (field cell (binding child (type OwnedI64Cell))) (in (unit)))))
  (function create b (visibility private) (effect pure)
    (parameter create p (type (owned-product (field cell OwnedI64Cell))) (use consume))
    (returns Unit) (body (unpack-owned (type (owned-product (field cell OwnedI64Cell)))
      (local p) (field cell (binding child (type OwnedI64Cell))) (in (unit)))))))
declarations.end"#,
    )
    .unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let mut first = None;
    for record in source.owners.values_mut() {
        if let OwnerRecord::Expression(e) = record
            && let crate::platform::kernel::ExpressionOperation::UnpackOwned { fields, .. } =
                &mut e.operation
        {
            if let Some(binding) = first {
                fields[0].binding = binding;
            } else {
                first = Some(fields[0].binding);
            }
        }
    }
    assert!(crate::platform::kernel::validate_full(&source).is_err());
    assert!(!crate::platform::kernel::memory_reference::accepts(&source));
}

#[test]
fn owned_products_recursive_type_growth_is_rejected_before_execution() {
    let source = r#"declarations.begin
(units (module create invalid
  (function create grow (visibility private) (effect pure)
    (type-parameter create T (constraint owned)) (parameter create p (type T) (use consume))
    (returns Unit)
    (body (let
      (binding wrapper (type (owned-product (field p T)))
        (pack-owned (type (owned-product (field p T))) (field p (local p))))
      (in (call grow (types (owned-product (field p T))) (local wrapper))))))))
declarations.end"#;
    let error = byte_buffer_tests::author_only(source).unwrap_err();
    assert!(error.contains("kernel_callable"), "{error}");
}

#[test]
fn owned_products_metadata_read_preserves_owner_across_reborrow() {
    use super::super::value::NormalizedRecord;
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-products-read.lkjc"
    ))
    .unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let main = declaration_named(&source, "main");
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        for n in [i64::MIN, -257, 0, 128, i64::MAX] {
            let cells = super::super::owned_i64_cell::StorageObservation::start();
            let products = super::super::owned_product::StorageObservation::start();
            let result = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(main, vec![NormalizedValue::I64(n)], None, &control)
                .unwrap()
                .0
            } else {
                NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .invoke(main, vec![NormalizedValue::I64(n)], None, &control)
                    .unwrap()
                    .0
            };
            let expected = NormalizedValue::Record(NormalizedRecord::Structural {
                fields: Arc::new(
                    ["direct", "forwarded", "payload"]
                        .into_iter()
                        .map(|name| (Name::new(name).unwrap(), NormalizedValue::I64(n)))
                        .collect(),
                ),
            });
            assert_eq!(result, expected, "reference={reference}");
            assert_eq!(products.created(), 1);
            products.assert_transfers_preserve_allocations();
            assert_eq!(products.live(), (0, 0));
            assert_eq!(cells.live(), (0, 0));
        }
    }
}

#[test]
fn owned_products_metadata_rejects_owned_fields_and_consumed_parents() {
    use crate::platform::kernel::{ExpressionOperation, FieldSelector, OwnerRecord};
    let literal = include_str!("../../../../tests/fixtures/owned-products-read.lkjc");
    let mut source = byte_buffer_tests::author_only(literal).unwrap();
    let mut changed = 0;
    for owner in source.owners.values_mut() {
        if let OwnerRecord::Expression(expression) = owner
            && let ExpressionOperation::Field { selector, .. } = &mut expression.operation
        {
            *selector = FieldSelector::Structural(Name::new("payload").unwrap());
            changed += 1;
        }
    }
    assert_eq!(changed, 2);
    assert!(!crate::platform::kernel::memory_reference::accepts(&source));
    for (original, replacement) in [
        (
            "(field (local packet) (name tag))",
            "(field (local packet) (name payload))",
        ),
        (
            "(field (local packet) (name tag))",
            "(field (local packet) (name absent))",
        ),
        (
            "(field direct (local direct))",
            "(field direct (field (local p) (name tag)))",
        ),
        (
            "(field (local p) (name tag))",
            "(field (if (bool true) (local p) (local p)) (name tag))",
        ),
    ] {
        assert!(literal.contains(original));
        let error =
            byte_buffer_tests::author_only(&literal.replace(original, replacement)).unwrap_err();
        assert!(
            error.contains("kernel_buffer_ownership")
                || error.contains("kernel_type_structural_field_missing"),
            "{error}"
        );
    }
}

#[test]
fn owned_products_metadata_token_checks_before_read_or_copy() {
    use super::super::{owned_product::OwnedProduct, value::ValueOrigin};
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let value = NormalizedValue::Option(Some(Box::new(NormalizedValue::Option(Some(Box::new(
        NormalizedValue::I64(73),
    ))))));
    // Raw storage control, intentionally separate from canonical product-type admission.
    let owner = OwnedProduct::create(
        origin,
        TypeObjectDigest::of(b"metadata-storage-control"),
        vec![value.clone()],
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    assert!(
        owner
            .read_metadata(origin, 0, &control, &mut |_| panic!("owner read allocated"))
            .is_err()
    );
    let inert = owner.clone();
    assert!(
        inert
            .read_metadata(origin, 0, &control, &mut |_| panic!("inert read allocated"))
            .is_err()
    );
    let loan = owner.borrow().unwrap();
    assert!(
        loan.read_metadata(ValueOrigin::fresh().unwrap(), 0, &control, &mut |_| panic!(
            "foreign read allocated"
        ))
        .is_err()
    );
    assert!(
        loan.read_metadata(origin, 1, &control, &mut |_| panic!(
            "missing field allocated"
        ))
        .is_err()
    );
    let expected = 2 * std::mem::size_of::<NormalizedValue>() as u64;
    let error = loan
        .read_metadata(origin, 0, &control, &mut |bytes| {
            assert_eq!(bytes, expected);
            Err(ExecutionError::resource(
                "test_metadata_quota",
                "deny before cloning boxes",
            ))
        })
        .unwrap_err();
    assert_eq!(error.code, "test_metadata_quota");
    let mut reserved = 0;
    let retained = loan
        .read_metadata(origin, 0, &control, &mut |bytes| {
            reserved += bytes;
            Ok(())
        })
        .unwrap();
    assert_eq!(reserved, expected);
    assert_eq!(retained, value);
    for checks in 0..5 {
        let cancelled = ExecutionControl::cancel_after_checks(checks);
        let error = loan
            .read_metadata(origin, 0, &cancelled, &mut |bytes| {
                assert_eq!(bytes, expected);
                Ok(())
            })
            .unwrap_err();
        assert_eq!(error.code, "execution_cancelled");
        owner.validate(origin, false).unwrap();
    }
    let cancelled = ExecutionControl::uncancelled();
    let error = loan
        .read_metadata(origin, 0, &cancelled, &mut |_| {
            cancelled.cancel();
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code, "execution_cancelled");
    assert!(owner.validate(origin, true).is_err());
    drop(loan);
    owner.validate(origin, true).unwrap();
    let loan = owner.borrow().unwrap();
    drop(owner);
    assert!(
        loan.read_metadata(origin, 0, &control, &mut |_| panic!("stale read allocated"))
            .is_err()
    );
    drop(loan);
    assert_eq!(retained, value);
}

#[test]
fn owned_products_metadata_outlives_consumed_parent() {
    let source = byte_buffer_tests::author_only(r#"declarations.begin
(units (module create retained
  (record create Meta (visibility private) (field create tag (type I64)))
  (external create cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (function create take-meta (visibility private) (effect pure)
    (parameter create p (type (owned-product (field child OwnedI64Cell) (field meta Meta))) (use consume))
    (returns Meta) (body (field (local p) (name meta))))
  (function create main (visibility public) (effect pure)
    (parameter create n (type I64)) (returns I64)
    (body (let
      (binding c (type OwnedI64Cell) (call cell (i64 0)))
      (binding p (type (owned-product (field child OwnedI64Cell) (field meta Meta)))
        (pack-owned (type (owned-product (field child OwnedI64Cell) (field meta Meta)))
          (field meta (record Meta (field Meta::tag (local n)))) (field child (local c))))
      (binding meta (type Meta) (call take-meta (local p)))
      (in (field (local meta) Meta::tag)))))))
declarations.end"#).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let main = declaration_named(&source, "main");
    for reference in [false, true] {
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        let products = super::super::owned_product::StorageObservation::start();
        let control = ExecutionControl::uncancelled();
        let value = if reference {
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(main, vec![NormalizedValue::I64(-257)], None, &control)
            .unwrap()
            .0
        } else {
            NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                .invoke(main, vec![NormalizedValue::I64(-257)], None, &control)
                .unwrap()
                .0
        };
        assert_eq!(value, NormalizedValue::I64(-257));
        assert_eq!(products.created(), 1);
        assert_eq!(products.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }
}

#[test]
fn owned_products_native_generic_cell_roundtrip() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-products.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    let main = declaration_named(&source, "main");
    let control = ExecutionControl::uncancelled();
    for n in [i64::MIN, -257, 0, 128, 255, i64::MAX] {
        let value = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
            .invoke(main, vec![NormalizedValue::I64(n)], None, &control)
            .unwrap()
            .0;
        assert_eq!(value, NormalizedValue::I64(n));
        let value = NormalizedReferenceInterpreter::new(
            &source,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(main, vec![NormalizedValue::I64(n)], None, &control)
        .unwrap()
        .0;
        assert_eq!(value, NormalizedValue::I64(n));
    }
}

#[test]
fn owned_products_closed_nominal_and_structural_metadata_roundtrip() {
    let source = byte_buffer_tests::author_only(r#"declarations.begin
(units (module create metadata
  (record create Meta (visibility private) (field create tag (type I64)))
  (external create cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (function create main (visibility public) (effect pure) (parameter create n (type I64)) (returns I64)
    (body (let
      (binding c (type OwnedI64Cell) (call cell (i64 0)))
      (binding p (type (owned-product (field child OwnedI64Cell) (field meta Meta) (field info (record (version I64)))))
        (pack-owned (type (owned-product (field child OwnedI64Cell) (field meta Meta) (field info (record (version I64)))))
          (field meta (record Meta (field Meta::tag (local n))))
          (field info (record structural (field version (i64 19))))
          (field child (local c))))
      (in (unpack-owned (type (owned-product (field child OwnedI64Cell) (field meta Meta) (field info (record (version I64)))))
        (local p) (field child (binding unused (type OwnedI64Cell)))
        (field info (binding info (type (record (version I64)))))
        (field meta (binding meta (type Meta)))
        (in (field (local meta) Meta::tag)))))))))
declarations.end"#).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        let products = super::super::owned_product::StorageObservation::start();
        for n in [i64::MIN, i64::MAX] {
            let entry = declaration_named(&source, "main");
            let control = ExecutionControl::uncancelled();
            let value = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, vec![NormalizedValue::I64(n)], None, &control)
                .unwrap()
                .0
            } else {
                NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, vec![NormalizedValue::I64(n)], None, &control)
                    .unwrap()
                    .0
            };
            assert_eq!(value, NormalizedValue::I64(n));
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(products.live(), (0, 0));
        }
    }
}

#[test]
fn owned_products_generic_only_and_closed_metadata_rejections() {
    let generic = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-products-independent.lkjc"
    ))
    .unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&generic));
    assert!(
        !generic
            .types
            .values()
            .any(|t| matches!(t.form, TypeForm::ByteBuffer | TypeForm::OwnedI64Cell))
    );
    prepare_snapshot(&generic);
    // Task helpers may now consume complete products, but cannot receive loans.
    // Keep the newly valid predecessor input as a positive admission control.
    let task_product = byte_buffer_tests::author_only(
        "declarations.begin\n(units (module create task-product (function create consume (visibility private) (effect (task)) (parameter create p (type (owned-product (field data ByteBuffer))) (use consume)) (returns Unit) (body (unit)))))\ndeclarations.end",
    )
    .unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(
        &task_product
    ));
    prepare_snapshot(&task_product);
    for declaration in [
        "(function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field tag I64))) (use consume)) (returns Unit) (body (unit)))",
        "(function create bad (visibility private) (effect pure) (type-parameter create T) (parameter create p (type (owned-product (field data ByteBuffer) (field meta T))) (use consume)) (returns Unit) (body (unit)))",
        "(function create bad (visibility private) (effect pure) (type-parameter create T (constraint capture-safe)) (parameter create p (type (owned-product (field data ByteBuffer) (field meta T))) (use consume)) (returns Unit) (body (unit)))",
        "(function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field data ByteBuffer) (field meta Secret))) (use consume)) (returns Unit) (body (unit)))",
        "(function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field data ByteBuffer) (field meta (stream I64)))) (use consume)) (returns Unit) (body (unit)))",
        "(record create Hidden (visibility private) (field create secret (type Secret))) (function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field data ByteBuffer) (field meta Hidden))) (use consume)) (returns Unit) (body (unit)))",
        "(interface create IO (visibility private) (operation create noop (returns Unit) (idempotency idempotent) (external-visibility none))) (function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field data ByteBuffer) (field meta (resource IO)))) (use consume)) (returns Unit) (body (unit)))",
        "(function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field data ByteBuffer) (field meta (function () I64)))) (use consume)) (returns Unit) (body (unit)))",
        "(record create bad (visibility private) (field create p (type (owned-product (field data ByteBuffer)))))",
        "(function create bad (visibility private) (effect pure) (parameter create p (type (list (owned-product (field data ByteBuffer))))) (returns Unit) (body (unit)))",
        "(function create bad (visibility private) (effect (task)) (parameter create p (type (owned-product (field data ByteBuffer))) (use borrow)) (returns Unit) (body (unit)))",
        "(function create ordinary (visibility private) (type-parameter create T) (effect pure) (returns Unit) (body (unit))) (function create bad (visibility private) (effect pure) (returns Unit) (body (call ordinary (types (owned-product (field data ByteBuffer))))))",
    ] {
        let error = byte_buffer_tests::author_only(&format!(
            "declarations.begin\n(units (module create negatives {declaration}))\ndeclarations.end"
        ))
        .unwrap_err();
        assert!(error.contains("kernel_"), "{declaration}\n{error}");
    }
}

#[test]
fn owned_products_token_seals_identity_and_releases_children_with_inert_markers() {
    use super::super::{
        byte_buffer::ByteBuffer,
        owned_product::{OwnedProduct, StorageObservation},
        value::ValueOrigin,
    };
    let observation = StorageObservation::start();
    let domain = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let ty = TypeObjectDigest::from_bytes([42; 32]);
    let buffer = ByteBuffer::empty(domain)
        .push(255, &control, &mut |_| Ok(()))
        .unwrap();
    let pointer = buffer.allocation_identity();
    let marker = buffer.clone();
    let product = OwnedProduct::create(
        domain,
        ty,
        vec![NormalizedValue::ByteBuffer(buffer)],
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let inert = product.clone();
    assert!(inert.validate(domain, false).is_err());
    assert!(
        product
            .validate(ValueOrigin::fresh().unwrap(), true)
            .is_err()
    );
    let loan = product.borrow().unwrap();
    let reborrow = loan.borrow().unwrap();
    assert!(loan.validate(domain, true).is_err());
    assert!(product.validate(domain, true).is_err());
    drop(reborrow);
    drop(loan);
    let transferred = product; // Whole-owner transfer has no child cloning operation.
    let mut fields = transferred.unpack(domain, ty, &control).unwrap();
    let NormalizedValue::ByteBuffer(buffer) = fields.remove(0) else {
        panic!("buffer field");
    };
    assert_eq!(pointer, buffer.allocation_identity());
    assert_eq!(&*buffer.freeze().unwrap(), &[255]);
    assert!(marker.validate(domain, false).is_err());
    assert!(inert.validate(domain, false).is_err());
    assert_eq!(observation.created(), 1);
    assert_eq!(observation.live(), (0, 0));
    let child = ByteBuffer::empty(domain);
    let marker = child.clone();
    let product = OwnedProduct::create(
        domain,
        ty,
        vec![NormalizedValue::ByteBuffer(child)],
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let inert = product.clone();
    drop(product);
    assert!(marker.validate(domain, false).is_err());
    assert!(inert.validate(domain, false).is_err());
    assert_eq!(observation.live(), (0, 0));
    let child = ByteBuffer::empty(domain);
    let marker = child.clone();
    let cancelled = ExecutionControl::uncancelled();
    let before = observation.created();
    let error = OwnedProduct::create(
        domain,
        ty,
        vec![NormalizedValue::ByteBuffer(child)],
        &cancelled,
        &mut |_| {
            cancelled.cancel();
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
    assert_eq!(
        observation.created(),
        before,
        "reservation cancellation precedes allocation"
    );
    assert!(marker.validate(domain, false).is_err());
    assert_eq!(observation.live(), (0, 0));
}

#[test]
fn owned_products_authored_trap_order_and_failure_cleanup() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-products-failures.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let products = super::super::owned_product::StorageObservation::start();
        let buffers = super::super::byte_buffer::StorageObservation::start();
        for (name, expected) in [
            ("divide-first", "normalized_integer_division"),
            ("index-first", "normalized_buffer_index"),
            ("field-trap", "normalized_integer_division"),
            ("unpack-trap", "normalized_buffer_index"),
        ] {
            let control = ExecutionControl::uncancelled();
            let entry = declaration_named(&source, name);
            let failure = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, vec![], None, &control)
                .unwrap_err()
            } else {
                NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, vec![], None, &control)
                    .unwrap_err()
            };
            let expected = if reference && expected == "normalized_integer_division" {
                "reference_integer_division"
            } else {
                expected
            };
            assert_eq!(failure.code, expected, "{reference} {name}");
            assert_eq!(products.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
        }
    }
}

#[test]
fn owned_products_complete_local_consumption_and_scope_rejections() {
    for declaration in [
        "(function create bad (visibility private) (effect pure) (parameter create b (type ByteBuffer) (use consume)) (returns (owned-product (field a ByteBuffer) (field b ByteBuffer))) (body (pack-owned (type (owned-product (field a ByteBuffer) (field b ByteBuffer))) (field a (local b)) (field b (local b)))))",
        "(function create bad (visibility private) (effect pure) (parameter create b (type ByteBuffer) (use borrow)) (returns (owned-product (field b ByteBuffer))) (body (pack-owned (type (owned-product (field b ByteBuffer))) (field b (local b)))))",
        "(function create bad (visibility private) (effect pure) (parameter create b (type ByteBuffer) (use consume)) (returns (owned-product (field b ByteBuffer))) (body (pack-owned (type (owned-product (field b ByteBuffer))) (field b (if (bool true) (local b) (local b))))))",
        "(function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field b ByteBuffer))) (use borrow)) (returns Unit) (body (unpack-owned (type (owned-product (field b ByteBuffer))) (local p) (field b (binding b (type ByteBuffer))) (in (unit)))))",
        "(function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field b ByteBuffer))) (use consume)) (returns (owned-product (field b ByteBuffer))) (body (sequence (unpack-owned (type (owned-product (field b ByteBuffer))) (local p) (field b (binding b (type ByteBuffer))) (in (unit))) (local p))))",
        "(function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field b ByteBuffer) (field tag I64))) (use consume)) (returns Unit) (body (unpack-owned (type (owned-product (field b ByteBuffer) (field tag I64))) (local p) (field b (binding b (type ByteBuffer))) (in (unit)))))",
        "(function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field b ByteBuffer))) (use consume)) (returns Unit) (body (unpack-owned (type (owned-product (field b ByteBuffer))) (local p) (field b (binding b (type I64))) (in (unit)))))",
        "(function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field b ByteBuffer))) (use consume)) (returns Unit) (body (unpack-owned (type (owned-product (field b ByteBuffer))) (local p) (field wrong (binding b (type ByteBuffer))) (in (unit)))))",
        "(function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field b ByteBuffer))) (use consume)) (returns ByteBuffer) (body (field (local p) (name b))))",
        "(record create Phantom (visibility public) (type-parameter create T) (field create tag (type I64))) (function create bad (visibility private) (effect pure) (parameter create p (type (owned-product (field b ByteBuffer) (field metadata (Phantom Secret)))) (use consume)) (returns Unit) (body (unit)))",
    ] {
        let error = byte_buffer_tests::author_only(&format!(
            "declarations.begin\n(units (module create invalid {declaration}))\ndeclarations.end"
        ))
        .unwrap_err();
        assert!(error.contains("kernel_"), "{declaration}\n{error}");
    }
    let leaked = r#"declarations.begin
(units (module create invalid
  (function create bad (visibility private) (effect pure)
    (parameter create p (type (owned-product (field b ByteBuffer))) (use consume))
    (returns ByteBuffer) (body (sequence
      (unpack-owned (type (owned-product (field b ByteBuffer))) (local p)
        (field b (binding leaked (type ByteBuffer))) (in (unit))) (local leaked))))))
declarations.end"#;
    assert!(
        byte_buffer_tests::author_only(leaked)
            .unwrap_err()
            .contains("change_")
    );
}

#[test]
fn owned_products_exact_monomorphic_witness_selection() {
    let literal = include_str!("../../../../tests/fixtures/owned-products-witness.lkjc");
    let source = byte_buffer_tests::author_only(literal).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let control = ExecutionControl::uncancelled();
    let entry = declaration_named(&source, "selected");
    for n in [i64::MIN, i64::MAX] {
        for reference in [false, true] {
            let products = super::super::owned_product::StorageObservation::start();
            let cells = super::super::owned_i64_cell::StorageObservation::start();
            let actual = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, vec![NormalizedValue::I64(n)], None, &control)
                .unwrap()
                .0
            } else {
                NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, vec![NormalizedValue::I64(n)], None, &control)
                    .unwrap()
                    .0
            };
            let NormalizedValue::Record(super::super::value::NormalizedRecord::Structural {
                fields,
            }) = actual
            else {
                panic!("ordinary result");
            };
            assert_eq!(
                fields.as_ref(),
                &[
                    (Name::new("alternate").unwrap(), NormalizedValue::I64(99)),
                    (Name::new("scalar").unwrap(), NormalizedValue::I64(n))
                ]
            );
            assert_eq!(products.live(), (0, 0));
            assert_eq!(cells.live(), (0, 0));
        }
    }
    let wrong_kind = literal.replace(
        "create alternate (visibility public) (effect pure)",
        "create alternate (visibility public) (effect (task))",
    );
    assert!(
        byte_buffer_tests::author_only(&wrong_kind)
            .unwrap_err()
            .contains("kernel_")
    );
}

struct ProductProbe<'a> {
    products: &'a super::super::owned_product::StorageObservation,
    buffers: &'a super::super::byte_buffer::StorageObservation,
    cancel: bool,
    forge: bool,
}
impl ProductProbe<'_> {
    fn answer(&self, control: &ExecutionControl) -> Result<NormalizedValue, ExecutionError> {
        assert_eq!(self.products.live(), (2, 1));
        assert_eq!(self.buffers.live(), (2, 0));
        if self.cancel {
            control.cancel();
        }
        if self.forge {
            let origin = super::super::value::ValueOrigin::fresh().unwrap();
            let raw = super::super::owned_product::OwnedProduct::create(
                origin,
                TypeObjectDigest::from_bytes([77; 32]),
                vec![],
                control,
                &mut |_| Ok(()),
            )?;
            return Ok(NormalizedValue::OwnedProduct(raw));
        }
        Ok(NormalizedValue::I64(0))
    }
}
impl super::super::vm::NormalizedHost for ProductProbe<'_> {
    fn call(
        &self,
        _: &NormalizedProgram,
        _: &super::super::prepare::NormalizedFunction,
        _: &crate::platform::kernel::ImplementationName,
        _: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.answer(control)
    }
}
impl super::super::reference::NormalizedReferenceHost for ProductProbe<'_> {
    fn call(
        &self,
        _: &dyn super::super::value_schema::NormalizedValueSchema,
        _: &super::super::reference::ReferenceSignature,
        _: &crate::platform::kernel::ImplementationName,
        _: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.answer(control)
    }
}

#[test]
fn owned_products_active_loan_cancellation_adapter_and_quota_cleanup() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-products-failures.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "cancellation");
    for reference in [false, true] {
        for (cancel, forge) in [(false, false), (true, false), (false, true)] {
            let products = super::super::owned_product::StorageObservation::start();
            let buffers = super::super::byte_buffer::StorageObservation::start();
            let host = ProductProbe {
                products: &products,
                buffers: &buffers,
                cancel,
                forge,
            };
            let control = ExecutionControl::uncancelled();
            let result = if reference {
                let sink = Mutex::new(None);
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .observing(&sink, &host)
                .invoke(entry, vec![], None, &control)
                .map(|v| v.0)
            } else {
                let sink = Mutex::new(None);
                NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .observing(&sink, &host)
                    .invoke(entry, vec![], None, &control)
                    .map(|v| v.0)
            };
            if cancel {
                assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Cancelled);
            } else if forge {
                assert!(result.is_err());
            } else {
                assert_eq!(result.unwrap(), NormalizedValue::I64(0));
            }
            assert_eq!(products.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
        }
        let run = |limit| {
            let products = super::super::owned_product::StorageObservation::start();
            let buffers = super::super::byte_buffer::StorageObservation::start();
            let control = ExecutionControl::uncancelled();
            let policy = NormalizedRunPolicy {
                maximum_allocated_bytes: limit,
                ..NormalizedRunPolicy::foreground()
            };
            let (result, allocated) = if reference {
                let sink = Mutex::new(None);
                let result = NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .observing_checked(&sink)
                    .invoke(entry, vec![], None, &control)
                    .map(|v| v.0);
                (result, sink.into_inner().unwrap().unwrap().allocated_bytes)
            } else {
                let sink = Mutex::new(None);
                let result = NormalizedVm::new(&program, policy)
                    .observing_checked(&sink)
                    .invoke(entry, vec![], None, &control)
                    .map(|v| v.0);
                (result, sink.into_inner().unwrap().unwrap().allocated_bytes)
            };
            assert_eq!(products.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
            (result, allocated, products.created())
        };
        let (result, total, count) = run(None);
        assert_eq!(result.unwrap(), NormalizedValue::I64(0));
        assert_eq!(count, 2);
        let (mut low, mut high) = (1, total);
        while low < high {
            let middle = low + (high - low) / 2;
            if run(Some(middle)).2 == 0 {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        let (result, before, count) = run(Some(low - 1));
        assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Resource);
        assert_eq!(count, 0);
        let expected = (std::mem::size_of::<super::super::owned_product::OwnedProduct>()
            + std::mem::size_of::<Mutex<(Option<Vec<NormalizedValue>>, usize)>>()
            + 2 * std::mem::size_of::<usize>()) as u64;
        assert_eq!(low - before, expected);
        // Exercise quotas after the first product has taken its nested child too.
        for maximum in [low, total / 2, total - 1, total] {
            let (result, _, _) = run(Some(maximum));
            if maximum == total {
                assert!(result.is_ok());
            } else if let Err(error) = result {
                assert_eq!(error.class, ExecutionFailureClass::Resource);
            }
        }
    }
}

#[test]
fn owned_products_raw_capture_and_persistence_boundaries() {
    use super::super::{owned_product::OwnedProduct, value::ValueOrigin};
    let source = composed_source();
    let program = prepare_snapshot(&source);
    let factory = declaration_named(&source, "factory");
    let OwnerRecord::Declaration(d) = &source.owners[&OwnerKey::Declaration(factory.declaration)]
    else {
        panic!("factory");
    };
    let DeclarationPayload::Function(f) = &d.payload else {
        panic!("function");
    };
    let ty = f.result;
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        let products = super::super::owned_product::StorageObservation::start();
        let buffers = super::super::byte_buffer::StorageObservation::start();
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        let result = if reference {
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(factory, vec![NormalizedValue::I64(128)], None, &control)
            .map(|v| v.0)
        } else {
            NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                .invoke(factory, vec![NormalizedValue::I64(128)], None, &control)
                .map(|v| v.0)
        };
        assert!(
            result.is_err(),
            "raw result cannot export product ownership"
        );
        for mode in ["live", "inert", "borrow", "stale"] {
            let origin = ValueOrigin::fresh().unwrap();
            let owner = OwnedProduct::create(
                origin,
                ty,
                vec![
                    NormalizedValue::ByteBuffer(super::super::byte_buffer::ByteBuffer::empty(
                        origin,
                    )),
                    NormalizedValue::OwnedI64Cell(super::super::owned_i64_cell::OwnedI64Cell::new(
                        origin,
                        i64::MAX,
                    )),
                    NormalizedValue::I64(7),
                ],
                &control,
                &mut |_| Ok(()),
            )
            .unwrap();
            let (token, retained) = match mode {
                "inert" => (owner.clone(), Some(owner)),
                "borrow" => (owner.borrow().unwrap(), Some(owner)),
                "stale" => {
                    let marker = owner.clone();
                    drop(owner);
                    (marker, None)
                }
                _ => (owner, None),
            };
            let raw = NormalizedValue::OwnedProduct(token);
            assert!(
                super::super::codec::encode_typed(
                    &program,
                    &raw,
                    ty,
                    crate::platform::json::JsonLimits::default()
                )
                .is_err()
            );
            assert!(super::super::data_codec::encode_typed(&program, &raw, ty).is_err());
            assert!(super::super::data_codec_reference::encode_typed(&program, &raw, ty).is_err());
            let result = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(factory, vec![raw], None, &control)
                .map(|v| v.0)
            } else {
                NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .invoke(factory, vec![raw], None, &control)
                    .map(|v| v.0)
            };
            assert!(result.is_err());
            drop(retained);
        }
        for input in [
            b"null".as_slice(),
            b"{}",
            b"{\"left\":[],\"right\":128,\"tag\":7}",
        ] {
            assert!(
                super::super::codec::decode_typed(
                    &program,
                    input,
                    ty,
                    crate::platform::json::JsonLimits::default()
                )
                .is_err()
            );
            assert!(super::super::data_codec::decode_typed(&program, input, ty).is_err());
            assert!(super::super::data_codec_reference::decode_typed(&program, input, ty).is_err());
        }
        assert_eq!(products.live(), (0, 0));
        assert_eq!(buffers.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }
    let capture = r#"declarations.begin
(units (module create invalid
  (function create ignore (visibility private) (effect pure)
    (parameter create p (type (owned-product (field b ByteBuffer))) (use consume)) (returns Unit) (body (unit)))
  (function create bad (visibility private) (effect pure) (returns (function () Unit))
    (body (function-value ignore)))))
declarations.end"#;
    assert!(
        byte_buffer_tests::author_only(capture)
            .unwrap_err()
            .contains("kernel_")
    );
}
