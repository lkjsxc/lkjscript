use super::*;
use crate::platform::kernel::{StructuralTypeField, encode_type_object};

fn insert(types: &mut BTreeMap<TypeObjectDigest, TypeObject>, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let (id, _) = encode_type_object(&object).unwrap();
    types.insert(id, object);
    id
}

fn field(name: &str, ty: TypeObjectDigest) -> StructuralTypeField {
    StructuralTypeField {
        name: Name::new(name).unwrap(),
        ty,
    }
}

#[test]
fn region_pointer_layout_fingerprint_delimits_nested_record_fields() {
    let mut types = BTreeMap::new();
    let region = insert(&mut types, TypeForm::Unit);
    let i64_type = insert(&mut types, TypeForm::I64);
    let inner_one = insert(
        &mut types,
        TypeForm::StructuralRecord {
            fields: vec![field("b", i64_type)],
        },
    );
    let inner_two = insert(
        &mut types,
        TypeForm::StructuralRecord {
            fields: vec![field("b", i64_type), field("c", i64_type)],
        },
    );
    let left = insert(
        &mut types,
        TypeForm::StructuralRecord {
            fields: vec![field("a", inner_one), field("c", i64_type)],
        },
    );
    let right = insert(
        &mut types,
        TypeForm::StructuralRecord {
            fields: vec![field("a", inner_two)],
        },
    );
    let fingerprint_of = |ty| {
        let mut hasher = blake3::Hasher::new();
        fingerprint(
            &types,
            ty,
            region,
            &mut hasher,
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
        )
        .unwrap();
        hasher.finalize()
    };
    assert_ne!(fingerprint_of(left), fingerprint_of(right));
}

#[test]
fn region_deep_layout_codec_and_failure_teardown_use_no_recursive_host_stack() {
    let depth = 16_384;
    let control = ExecutionControl::uncancelled();
    let mut types = BTreeMap::new();
    let region = insert(&mut types, TypeForm::Unit);
    let boolean = insert(&mut types, TypeForm::Bool);
    let mut ty = boolean;
    let mut value = NormalizedValue::Bool(true);
    for _ in 0..depth {
        ty = insert(
            &mut types,
            TypeForm::StructuralRecord {
                fields: vec![field("next", ty)],
            },
        );
        value = NormalizedValue::Record(NormalizedRecord::Structural {
            fields: Arc::new(vec![(Name::new("next").unwrap(), value)]),
        });
    }
    let value = RawValue::new(value);
    check_plain(&types, value.as_raw(), ty, &control).unwrap();
    let layout = Layout::derive(&types, ty, region, &control, &mut |_| Ok(())).unwrap();
    let arena = runtime::Arena::new().unwrap();
    let cap = arena.open(1_u8, 0, &control, |_| Ok(())).unwrap();
    let identity = cap.identity();
    let encoded = layout
        .encode(value.as_raw(), identity, &control, &mut |_| Ok(()))
        .unwrap();
    assert_eq!(encoded, [1]);
    let decoded = RawValue::new(
        layout
            .decode(&encoded, identity, &control, &mut |_| Ok(()))
            .unwrap(),
    );
    let mut current = decoded.as_raw();
    for _ in 0..depth {
        let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = current else {
            panic!("missing nested region field");
        };
        assert_eq!(fields[0].0.as_str(), "next");
        current = &fields[0].1;
    }
    assert!(matches!(current, NormalizedValue::Bool(true)));

    let pointer = insert(&mut types, TypeForm::RelPtr { region, item: ty });
    let pointer_layout =
        Layout::derive(&types, pointer, region, &control, &mut |_| Ok(())).unwrap();
    let address = u64::from(u32::MAX) + 17;
    let pointer_value = RawValue::new(NormalizedValue::RegionPointer {
        ty: pointer,
        pointer: runtime::Pointer::in_region(identity, address),
    });
    assert_eq!(
        pointer_layout
            .encode(pointer_value.as_raw(), identity, &control, &mut |_| Ok(()))
            .unwrap(),
        address.to_le_bytes()
    );

    // One complete deep value is retained when the later Bool cell is rejected.
    let pair = insert(
        &mut types,
        TypeForm::StructuralRecord {
            fields: vec![field("first", ty), field("second", boolean)],
        },
    );
    let pair = Layout::derive(&types, pair, region, &control, &mut |_| Ok(())).unwrap();
    assert_eq!(
        pair.decode(&[1, 2], identity, &control, &mut |_| Ok(()))
            .err()
            .unwrap()
            .code,
        "normalized_region_layout"
    );
    for cancel in [false, true] {
        let interrupted = ExecutionControl::uncancelled();
        let mut charges = 0;
        let error = layout
            .decode(&encoded, identity, &interrupted, &mut |_| {
                charges += 1;
                if charges == 40 {
                    if cancel {
                        interrupted.cancel();
                    } else {
                        return Err(ExecutionError::resource("quota", "test quota"));
                    }
                }
                Ok(())
            })
            .err()
            .unwrap();
        assert_eq!(
            error.class,
            if cancel {
                crate::platform::execution::ExecutionFailureClass::Cancelled
            } else {
                crate::platform::execution::ExecutionFailureClass::Resource
            }
        );
    }
    arena.close(identity).unwrap();
}

#[test]
fn region_layout_and_pointer_fingerprint_reject_cyclic_inline_type_graphs() {
    let control = ExecutionControl::uncancelled();
    let mut types = BTreeMap::new();
    let region = insert(&mut types, TypeForm::Unit);
    let scalar = insert(&mut types, TypeForm::I64);
    let cyclic = insert(
        &mut types,
        TypeForm::StructuralRecord {
            fields: vec![field("next", scalar)],
        },
    );
    types.insert(
        cyclic,
        TypeObject::new(TypeForm::StructuralRecord {
            fields: vec![field("next", cyclic)],
        })
        .unwrap(),
    );
    assert!(Layout::derive(&types, cyclic, region, &control, &mut |_| Ok(())).is_err());
    let mut hasher = blake3::Hasher::new();
    assert!(
        fingerprint(&types, cyclic, region, &mut hasher, &control, &mut |_| Ok(
            ()
        ))
        .is_err()
    );
}

#[test]
fn region_layout_and_pointer_fingerprint_share_completed_type_dag_nodes() {
    let control = ExecutionControl::uncancelled();
    let mut types = BTreeMap::new();
    let unit = insert(&mut types, TypeForm::Unit);
    let region = insert(&mut types, TypeForm::Bool);
    let mut ty = unit;
    let depth = 64;
    for _ in 0..depth {
        ty = insert(
            &mut types,
            TypeForm::StructuralRecord {
                fields: vec![field("left", ty), field("right", ty)],
            },
        );
    }
    let mut requested = 0_u64;
    let mut reserve = |bytes: u64| {
        requested = requested.checked_add(bytes).ok_or_else(storage_error)?;
        if requested > 1_048_576 {
            return Err(ExecutionError::resource(
                "quota",
                "explicit finite test allowance",
            ));
        }
        Ok(())
    };
    let layout = Layout::derive(&types, ty, region, &control, &mut reserve).unwrap();
    assert_eq!(layout.nodes.len(), depth + 1);
    assert_eq!(layout.nodes[layout.root].size, 0);
    let mut left = blake3::Hasher::new();
    fingerprint(&types, ty, region, &mut left, &control, &mut reserve).unwrap();
    let mut right = blake3::Hasher::new();
    fingerprint(&types, ty, region, &mut right, &control, &mut |_| Ok(())).unwrap();
    assert_eq!(left.finalize(), right.finalize());
}
