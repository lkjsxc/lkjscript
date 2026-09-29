//! Key sharing must not turn physical aliases into admission exemptions.
use super::*;

fn payload(value: &NormalizedValue) -> &[u8] {
    match value {
        NormalizedValue::Text(value) => value.as_bytes(),
        NormalizedValue::Bytes(value) => value,
        _ => panic!("expected text or bytes"),
    }
}

fn projected_key(value: &NormalizedValue) -> &NormalizedValue {
    let NormalizedValue::List(entries) = value else {
        panic!("expected entries")
    };
    assert_eq!(entries.len(), 1);
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) =
        entries.iter().next().unwrap()
    else {
        panic!("expected entry record")
    };
    assert_eq!(fields.len(), 2);
    &fields
        .iter()
        .find(|(name, _)| name.as_str() == "key")
        .unwrap()
        .1
}

#[test]
fn checked_map_keys_preserve_payload_identity_in_both_evaluators() {
    let (mut program, mut snapshot) = fixture();
    let mut schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let integer = internal_type(&mut program, &mut schema, TypeForm::I64);
    let text = internal_type(&mut program, &mut schema, TypeForm::Text);
    let bytes = internal_type(&mut program, &mut schema, TypeForm::Bytes);
    snapshot.types = schema.types.clone();
    let reader = boundary_reader(&snapshot, &schema);
    for (ty, input) in [
        (text, NormalizedValue::text("shared-日本語".repeat(512))),
        (bytes, NormalizedValue::bytes([0, 1, 128, 255].repeat(1024))),
    ] {
        let address = payload(&input).as_ptr();
        let key = NormalizedMapKey::from_value(input.clone()).unwrap();
        let retained = map([(key.clone(), NormalizedValue::I64(17))]);
        for reference in [false, true] {
            let control = ExecutionControl::uncancelled();
            let projected = invoke(
                &program,
                &reader,
                reference,
                "core.map.entries",
                &[ty, integer],
                vec![retained.clone()],
                Default::default(),
                &control,
                None,
            );
            let value = projected.value.unwrap();
            assert_eq!(payload(projected_key(&value)), payload(&input));
            assert_eq!(payload(projected_key(&value)).as_ptr(), address);
            assert_eq!(projected.work.maps.key_bytes_copied, 0);
            let replaced = invoke(
                &program,
                &reader,
                reference,
                "core.map.insert",
                &[ty, integer],
                vec![retained.clone(), input.clone(), NormalizedValue::I64(99)],
                Default::default(),
                &control,
                None,
            );
            let NormalizedValue::Map(updated) = replaced.value.unwrap() else {
                panic!("expected updated map")
            };
            let (stored, value) = updated.iter().next().unwrap();
            assert_eq!(payload(&stored.to_value()).as_ptr(), address);
            assert_eq!(value, &NormalizedValue::I64(99));
            assert_eq!(replaced.work.maps.key_bytes_copied, 0);
            let NormalizedValue::Map(original) = &retained else {
                unreachable!()
            };
            assert_eq!(original.get(&key), Some(&NormalizedValue::I64(17)));
        }
    }
}

#[test]
fn checked_map_key_aliases_keep_logical_admission_but_not_copy_charges() {
    let (mut program, mut snapshot) = fixture();
    let mut schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let integer = internal_type(&mut program, &mut schema, TypeForm::I64);
    let text = internal_type(&mut program, &mut schema, TypeForm::Text);
    let bytes = internal_type(&mut program, &mut schema, TypeForm::Bytes);
    snapshot.types = schema.types.clone();
    let reader = boundary_reader(&snapshot, &schema);
    for reference in [false, true] {
        for ty in [text, bytes] {
            let mut sizes = Vec::new();
            for length in [1, 8193] {
                let input = if ty == text {
                    NormalizedValue::text("x".repeat(length))
                } else {
                    NormalizedValue::bytes(vec![255; length])
                };
                let key = NormalizedMapKey::from_value(input.clone()).unwrap();
                let retained = map([(key, NormalizedValue::I64(17))]);
                let control = ExecutionControl::uncancelled();
                let lookup = invoke(
                    &program,
                    &reader,
                    reference,
                    "core.map.get-or",
                    &[ty, integer],
                    vec![retained.clone(), input.clone(), NormalizedValue::I64(0)],
                    Default::default(),
                    &control,
                    None,
                );
                assert_eq!(lookup.value.unwrap(), NormalizedValue::I64(17));
                assert_eq!(lookup.work.maps.key_bytes_copied, 0);
                let distinct = if ty == text {
                    NormalizedValue::text(String::from_utf8(payload(&input).to_vec()).unwrap())
                } else {
                    NormalizedValue::bytes(payload(&input).to_vec())
                };
                assert_ne!(payload(&distinct).as_ptr(), payload(&input).as_ptr());
                let unshared = invoke(
                    &program,
                    &reader,
                    reference,
                    "core.map.get-or",
                    &[ty, integer],
                    vec![retained.clone(), distinct, NormalizedValue::I64(0)],
                    Default::default(),
                    &control,
                    None,
                );
                assert_eq!(unshared.value.unwrap(), NormalizedValue::I64(17));
                assert_eq!(
                    (lookup.bytes, lookup.items),
                    (unshared.bytes, unshared.items)
                );
                let projected = invoke(
                    &program,
                    &reader,
                    reference,
                    "core.map.entries",
                    &[ty, integer],
                    vec![retained.clone()],
                    Default::default(),
                    &control,
                    None,
                );
                assert!(projected.value.is_ok());
                for (limit, accepted) in [(lookup.bytes, true), (lookup.bytes - 1, false)] {
                    let bounded = invoke(
                        &program,
                        &reader,
                        reference,
                        "core.map.get-or",
                        &[ty, integer],
                        vec![retained.clone(), input.clone(), NormalizedValue::I64(0)],
                        NormalizedRunPolicy {
                            maximum_allocated_bytes: Some(limit),
                            ..Default::default()
                        },
                        &control,
                        None,
                    );
                    assert_eq!(bounded.value.is_ok(), accepted);
                }
                sizes.push((lookup.bytes, projected.bytes));
            }
            // The shared key occurs once in the map and once as a lookup argument.
            // Projection has one input occurrence and shares, rather than copies, its output key.
            assert_eq!(sizes[1].0 - sizes[0].0, 2 * 8192);
            assert_eq!(sizes[1].1 - sizes[0].1, 8192);
        }
    }
}
