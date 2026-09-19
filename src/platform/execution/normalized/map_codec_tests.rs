//! Fixed map byte contracts, independently constructed envelopes and decoder faults.

use super::super::value::MAXIMUM_ADMISSION_ITEMS;
use super::super::{codec, data_codec, data_codec_reference};
use super::*;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::kernel::{ModuleRecord, ParameterUse};
use crate::platform::semantic_id::ModuleId;
use base64::Engine;

struct Fixture {
    program: NormalizedProgram,
    reference: super::super::reference::BoundReferenceSchema,
    boolean: TypeObjectDigest,
    integer: TypeObjectDigest,
    bytes: TypeObjectDigest,
    text: TypeObjectDigest,
    maps: [TypeObjectDigest; 4],
    list: TypeObjectDigest,
    nested: TypeObjectDigest,
    map_chain: Vec<TypeObjectDigest>,
}

fn fixture() -> Fixture {
    fixture_with_map_depth(0)
}

fn fixture_with_map_depth(depth: usize) -> Fixture {
    let seed = b"persistent-map-byte-contract";
    let mut snapshot = empty_normalized_snapshot(seed);
    let module = ModuleId::migrate(seed, 0);
    let function = DeclarationId::migrate(seed, 0);
    let body = ExpressionId::migrate(seed, 0);
    let unit = admit_snapshot_type(&mut snapshot, TypeForm::Unit);
    let boolean = admit_snapshot_type(&mut snapshot, TypeForm::Bool);
    let integer = admit_snapshot_type(&mut snapshot, TypeForm::I64);
    let bytes = admit_snapshot_type(&mut snapshot, TypeForm::Bytes);
    let text = admit_snapshot_type(&mut snapshot, TypeForm::Text);
    let maps = [boolean, integer, bytes, text].map(|key| {
        admit_snapshot_type(
            &mut snapshot,
            TypeForm::Map {
                key,
                value: integer,
            },
        )
    });
    let list = admit_snapshot_type(&mut snapshot, TypeForm::List { item: integer });
    let nested = admit_snapshot_type(
        &mut snapshot,
        TypeForm::Map {
            key: text,
            value: list,
        },
    );
    let mut map_chain = vec![integer];
    for index in 0..depth {
        let ty = admit_snapshot_type(
            &mut snapshot,
            TypeForm::Map {
                key: integer,
                value: map_chain[index],
            },
        );
        map_chain.push(ty);
    }
    snapshot.owners.insert(
        OwnerKey::Module(module),
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: Name::new("maps").unwrap(),
        }),
    );
    snapshot.owners.insert(
        OwnerKey::Expression(body),
        OwnerRecord::Expression(ExpressionRecord::new(body, ExpressionOperation::Unit {}).unwrap()),
    );
    let parameters = maps
        .into_iter()
        .chain([nested])
        .chain(map_chain.last().copied().filter(|_| depth > 0))
        .enumerate()
        .map(|(index, ty)| {
            let parameter = ParameterId::migrate(seed, index as u64);
            snapshot.owners.insert(
                OwnerKey::Parameter(parameter),
                OwnerRecord::Parameter(ParameterRecord {
                    header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                    parent: ParameterParent::Function(function),
                    name: Name::new(format!("input{index}")).unwrap(),
                    ty,
                    use_mode: ParameterUse::Unrestricted,
                    resource_requirement: None,
                }),
            );
            parameter
        })
        .collect();
    snapshot.owners.insert(
        OwnerKey::Declaration(function),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(function), OwnerKind::PureFunction),
            module,
            name: Name::new("consume").unwrap(),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                requirement_parameters: vec![],
                effect_parameters: vec![],
                type_parameters: vec![],
                parameters,
                result: unit,
                effect: FunctionEffect::Pure,
                body,
            }),
        }),
    );
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    let program = prepare_snapshot(&snapshot);
    let reference = super::super::reference::BoundReferenceSchema {
        canonical: Arc::new(
            super::super::NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap(),
        ),
        value_origin: program.value_origin,
    };
    Fixture {
        program,
        reference,
        boolean,
        integer,
        bytes,
        text,
        maps,
        list,
        nested,
        map_chain,
    }
}

fn digest(domain: &'static str, bytes: &[u8]) -> [u8; 32] {
    let mut hash = blake3::Hasher::new_derive_key(domain);
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
    *hash.finalize().as_bytes()
}

fn checksum(mut payload: Vec<u8>) -> Vec<u8> {
    payload.extend_from_slice(&digest("lkjscript.data.typed-value-envelope.v1", &payload));
    payload
}

/// Literal existing tags and framing, independent of either codec's layout walker.
fn envelope(
    map: TypeObjectDigest,
    key: TypeObjectDigest,
    key_tag: u8,
    integer: TypeObjectDigest,
    list: Option<TypeObjectDigest>,
    payload: &[u8],
) -> Vec<u8> {
    let mut layout = map.bytes().to_vec();
    layout.push(8); // Map.
    layout.extend_from_slice(&key.bytes());
    layout.push(key_tag);
    if let Some(list) = list {
        layout.extend_from_slice(&list.bytes());
        layout.push(7); // List.
    }
    layout.extend_from_slice(&integer.bytes());
    layout.push(2); // I64.
    let mut body = b"LKJDVAL1\x00\x01".to_vec();
    body.extend_from_slice(&digest("lkjscript.data.typed-layout.v1", &layout));
    body.extend_from_slice(payload);
    checksum(body)
}

fn text_payload(entries: &[(&str, i64)]) -> Vec<u8> {
    let mut bytes = (entries.len() as u32).to_be_bytes().to_vec();
    for (key, value) in entries {
        bytes.extend_from_slice(&(key.len() as u32).to_be_bytes());
        bytes.extend_from_slice(key.as_bytes());
        bytes.extend_from_slice(&value.to_be_bytes());
    }
    bytes
}

fn assert_bytes(
    fixture: &Fixture,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
    json: &[u8],
    data: &[u8],
) {
    for schema in [
        &fixture.program as &dyn super::super::value_schema::NormalizedValueSchema,
        &fixture.reference,
    ] {
        assert_eq!(
            codec::encode_typed(schema, value, ty, JsonLimits::default()).unwrap(),
            json
        );
        let decoded = codec::decode_typed(schema, json, ty, JsonLimits::default()).unwrap();
        assert_eq!(&decoded, value);
    }
    assert_eq!(
        data_codec::encode_typed(&fixture.program, value, ty).unwrap(),
        data
    );
    assert_eq!(
        data_codec_reference::encode_typed(&fixture.reference, value, ty).unwrap(),
        data
    );
    assert_eq!(
        &data_codec::decode_typed(&fixture.program, data, ty).unwrap(),
        value
    );
    assert_eq!(
        &data_codec_reference::decode_typed(&fixture.reference, data, ty).unwrap(),
        value
    );
}

#[test]
fn map_codecs_preserve_fixed_bytes_for_all_primitive_key_kinds() {
    let fixture = fixture();
    let cases = [
        (
            fixture.boolean,
            1,
            br#"[[false,-1],[true,1]]"#.as_slice(),
            {
                let mut bytes = 2u32.to_be_bytes().to_vec();
                bytes.push(0);
                bytes.extend_from_slice(&(-1i64).to_be_bytes());
                bytes.push(1);
                bytes.extend_from_slice(&1i64.to_be_bytes());
                bytes
            },
        ),
        (
            fixture.integer,
            2,
            br#"[[-9223372036854775808,-1],[9223372036854775807,1]]"#,
            {
                let mut bytes = 2u32.to_be_bytes().to_vec();
                bytes.extend_from_slice(&i64::MIN.to_be_bytes());
                bytes.extend_from_slice(&(-1i64).to_be_bytes());
                bytes.extend_from_slice(&i64::MAX.to_be_bytes());
                bytes.extend_from_slice(&1i64.to_be_bytes());
                bytes
            },
        ),
        (
            fixture.bytes,
            3,
            br#"[[{"$bytes":""},-1],[{"$bytes":"AP8="},1]]"#,
            {
                let mut bytes = 2u32.to_be_bytes().to_vec();
                bytes.extend_from_slice(&0u32.to_be_bytes());
                bytes.extend_from_slice(&(-1i64).to_be_bytes());
                bytes.extend_from_slice(&2u32.to_be_bytes());
                bytes.extend_from_slice(&[0, 255]);
                bytes.extend_from_slice(&1i64.to_be_bytes());
                bytes
            },
        ),
        (
            fixture.text,
            4,
            br#"[["a",2],["b",7]]"#,
            text_payload(&[("a", 2), ("b", 7)]),
        ),
    ];
    for (map, (key, key_tag, json, payload)) in fixture.maps.into_iter().zip(cases) {
        let data = envelope(map, key, key_tag, fixture.integer, None, &payload);
        let value =
            codec::decode_typed(&fixture.program, json, map, JsonLimits::default()).unwrap();
        assert_bytes(&fixture, &value, map, json, &data);
        let empty = NormalizedValue::map(BTreeMap::new()).unwrap();
        assert_bytes(
            &fixture,
            &empty,
            map,
            b"[]",
            &envelope(map, key, key_tag, fixture.integer, None, &[0, 0, 0, 0]),
        );
    }
}

#[test]
fn map_codecs_read_and_reproduce_genuine_predecessor_bytes() {
    let fixture = fixture();
    let predecessor: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/map-predecessor.json"
    ))
    .unwrap();
    assert_eq!(
        predecessor["source_commit"],
        "d77fb8bafab8a5a92b4d5251e62a9c44a41aea82"
    );
    assert_eq!(
        predecessor["executable_sha256"],
        "a30ace674685bbef5d2b15dc5b6566640b77b041f5c540a66e6a896c62a54b74"
    );
    let json = predecessor["json_value"].as_str().unwrap().as_bytes();
    assert_eq!(json, br#"[["a",2],["b",7]]"#);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(predecessor["typed_value_base64"].as_str().unwrap())
        .unwrap();
    let ty = fixture.maps[3];
    assert_eq!(
        bytes,
        envelope(
            ty,
            fixture.text,
            4,
            fixture.integer,
            None,
            &text_payload(&[("a", 2), ("b", 7)])
        )
    );
    let arguments = &predecessor["arguments"][0];
    let value =
        codec::decode_value(&fixture.program, arguments, ty, JsonLimits::default()).unwrap();
    assert_bytes(&fixture, &value, ty, json, &bytes);
}

#[test]
fn map_codecs_preserve_nested_bytes_across_insertion_removal_and_retained_versions() {
    let fixture = fixture();
    let json = br#"[["a",[1,-1]],["b",[2,-2]],["c",[3,-3]],["d",[4,-4]],["e",[5,-5]],["f",[6,-6]],["g",[7,-7]]]"#;
    let mut payload = 7u32.to_be_bytes().to_vec();
    for (index, key) in (*b"abcdefg").into_iter().enumerate() {
        payload.extend_from_slice(&1u32.to_be_bytes());
        payload.push(key);
        payload.extend_from_slice(&2u32.to_be_bytes());
        payload.extend_from_slice(&(index as i64 + 1).to_be_bytes());
        payload.extend_from_slice(&(-(index as i64) - 1).to_be_bytes());
    }
    let data = envelope(
        fixture.nested,
        fixture.text,
        4,
        fixture.integer,
        Some(fixture.list),
        &payload,
    );
    for order in [
        [0, 1, 2, 3, 4, 5, 6],
        [6, 5, 4, 3, 2, 1, 0],
        [3, 1, 5, 0, 2, 4, 6],
    ] {
        let mut map = super::super::map::Map::default();
        for index in order {
            let key = NormalizedMapKey::Text(char::from(b'a' + index).to_string());
            let values = vec![
                NormalizedValue::I64(i64::from(index) + 1),
                NormalizedValue::I64(-i64::from(index) - 1),
            ];
            map = map
                .insert(
                    key,
                    NormalizedValue::list(values).unwrap(),
                    MAXIMUM_ADMISSION_ITEMS,
                    &mut |_| Ok(()),
                )
                .unwrap();
        }
        let retained = map.clone();
        map = map
            .insert(
                NormalizedMapKey::Text("z".to_owned()),
                NormalizedValue::list(vec![]).unwrap(),
                MAXIMUM_ADMISSION_ITEMS,
                &mut |_| Ok(()),
            )
            .unwrap();
        map = map
            .remove(&NormalizedMapKey::Text("z".to_owned()), &mut |_| Ok(()))
            .unwrap();
        map = map
            .remove(&NormalizedMapKey::Text("missing".to_owned()), &mut |_| {
                Ok(())
            })
            .unwrap();
        for map in [map, retained] {
            assert_bytes(
                &fixture,
                &NormalizedValue::Map(map),
                fixture.nested,
                json,
                &data,
            );
        }
    }
}

#[test]
fn map_json_accepts_unordered_unique_pairs_while_typed_data_requires_increasing_keys() {
    let fixture = fixture();
    let ty = fixture.maps[3];
    let expected = br#"[["a",2],["b",7]]"#;
    for schema in [
        &fixture.program as &dyn super::super::value_schema::NormalizedValueSchema,
        &fixture.reference,
    ] {
        let value = codec::decode_typed(schema, br#"[["b",7],["a",2]]"#, ty, JsonLimits::default())
            .unwrap();
        assert_eq!(
            codec::encode_typed(schema, &value, ty, JsonLimits::default()).unwrap(),
            expected
        );
        for invalid in [
            br#"[["a",2],["a",7]]"#.as_slice(),
            br#"[["a",2],["b","invalid-final-child"]]"#,
        ] {
            assert_eq!(
                codec::decode_typed(schema, invalid, ty, JsonLimits::default())
                    .unwrap_err()
                    .code,
                "normalized_json_type"
            );
        }
    }
    for entries in [[("b", 7), ("a", 2)], [("a", 2), ("a", 7)]] {
        let data = envelope(
            ty,
            fixture.text,
            4,
            fixture.integer,
            None,
            &text_payload(&entries),
        );
        assert_eq!(
            data_codec::decode_typed(&fixture.program, &data, ty)
                .unwrap_err()
                .code,
            "normalized_data_map_order"
        );
        assert_eq!(
            data_codec_reference::decode_typed(&fixture.reference, &data, ty)
                .unwrap_err()
                .code,
            "normalized_data_map_order"
        );
    }
    let mut truncated = text_payload(&[("a", 2), ("b", 7)]);
    truncated.pop();
    let data = envelope(ty, fixture.text, 4, fixture.integer, None, &truncated);
    assert_eq!(
        data_codec::decode_typed(&fixture.program, &data, ty)
            .unwrap_err()
            .code,
        "normalized_data_i64"
    );
    assert_eq!(
        data_codec_reference::decode_typed(&fixture.reference, &data, ty)
            .unwrap_err()
            .code,
        "normalized_data_i64"
    );
}

#[test]
fn map_key_encoding_checks_exact_types_limits_and_static_text_origin() {
    let mut fixture = fixture();
    for (ty, wrong_key) in fixture.maps.into_iter().zip([
        NormalizedMapKey::I64(0),
        NormalizedMapKey::Text("not-an-integer".to_owned()),
        NormalizedMapKey::Bool(true),
        NormalizedMapKey::Bytes(vec![0]),
    ]) {
        let value =
            NormalizedValue::map(BTreeMap::from([(wrong_key, NormalizedValue::I64(1))])).unwrap();
        for schema in [
            &fixture.program as &dyn super::super::value_schema::NormalizedValueSchema,
            &fixture.reference,
        ] {
            assert_eq!(
                codec::encode_typed(schema, &value, ty, JsonLimits::default())
                    .unwrap_err()
                    .code,
                "normalized_json_type"
            );
        }
        assert_eq!(
            data_codec::encode_typed(&fixture.program, &value, ty)
                .unwrap_err()
                .code,
            "normalized_data_runtime_layout"
        );
        assert_eq!(
            data_codec_reference::encode_typed(&fixture.reference, &value, ty)
                .unwrap_err()
                .code,
            "normalized_data_runtime_layout"
        );
    }
    for (ty, key) in [
        (fixture.maps[3], NormalizedMapKey::Text("long".to_owned())),
        (fixture.maps[2], NormalizedMapKey::Bytes(vec![0, 1, 2])),
    ] {
        let value = NormalizedValue::map(BTreeMap::from([(key, NormalizedValue::I64(1))])).unwrap();
        let limits = JsonLimits {
            maximum_string_bytes: 3,
            ..JsonLimits::default()
        };
        assert_eq!(
            codec::encode_typed(&fixture.program, &value, ty, limits)
                .unwrap_err()
                .code,
            "normalized_json_type"
        );
    }
    let static_text = admit_runtime_type(&mut fixture.program, TypeForm::StaticText);
    let static_map = admit_runtime_type(
        &mut fixture.program,
        TypeForm::Map {
            key: static_text,
            value: fixture.integer,
        },
    );
    let mut schema = (*fixture.reference.canonical).clone();
    for ty in [static_text, static_map] {
        schema.types.insert(ty, fixture.program.types[&ty].clone());
    }
    fixture.reference.canonical = Arc::new(schema);
    let empty = NormalizedValue::map(BTreeMap::new()).unwrap();
    let text_key = NormalizedValue::map(BTreeMap::from([(
        NormalizedMapKey::Text("stored-text".to_owned()),
        NormalizedValue::I64(1),
    )]))
    .unwrap();
    for schema in [
        &fixture.program as &dyn super::super::value_schema::NormalizedValueSchema,
        &fixture.reference,
    ] {
        // Existing boundary treatment: the neutral Text key is no StaticText origin.
        assert_eq!(
            codec::encode_typed(schema, &empty, static_map, JsonLimits::default()).unwrap(),
            b"[]"
        );
        assert!(codec::encode_typed(schema, &text_key, static_map, JsonLimits::default()).is_err());
        assert!(codec::decode_typed(schema, b"[]", static_map, JsonLimits::default()).is_err());
    }
    assert!(data_codec::encode_typed(&fixture.program, &empty, static_map).is_err());
    assert!(data_codec_reference::encode_typed(&fixture.reference, &empty, static_map).is_err());
}

#[test]
fn map_key_encoding_cancellation_preserves_the_retained_value() {
    let fixture = fixture();
    let ty = fixture.maps[3];
    let json = br#"[["a",2],["b",7]]"#;
    let value = codec::decode_typed(&fixture.program, json, ty, JsonLimits::default()).unwrap();
    for boundary in 0..4 {
        let encode = |control: &ExecutionControl| match boundary {
            0 => codec::encode_typed_with_control(
                &fixture.program,
                &value,
                ty,
                JsonLimits::default(),
                control,
            ),
            1 => codec::encode_typed_with_control(
                &fixture.reference,
                &value,
                ty,
                JsonLimits::default(),
                control,
            ),
            2 => data_codec::encode_typed_with_control(&fixture.program, &value, ty, control),
            _ => data_codec_reference::encode_typed_with_control(
                &fixture.reference,
                &value,
                ty,
                control,
            ),
        };
        let expected = encode(&ExecutionControl::uncancelled()).unwrap();
        let mut completed = false;
        for checks in 0..256 {
            match encode(&ExecutionControl::cancel_after_checks(checks)) {
                Ok(bytes) => {
                    assert_eq!(bytes, expected);
                    completed = true;
                    break;
                }
                Err(error) => assert_eq!(error.class, DiagnosticClass::Cancelled),
            }
        }
        assert!(completed);
        assert_eq!(encode(&ExecutionControl::uncancelled()).unwrap(), expected);
    }
}

#[test]
fn map_codec_cancellation_including_bulk_construction_leaves_healthy_subsequent_use() {
    let fixture = fixture();
    let ty = fixture.maps[3];
    let json = br#"[["a",2],["b",7]]"#;
    let data = envelope(
        ty,
        fixture.text,
        4,
        fixture.integer,
        None,
        &text_payload(&[("a", 2), ("b", 7)]),
    );
    for boundary in 0..4 {
        let decode = |control: &ExecutionControl| match boundary {
            0 => codec::decode_typed_with_control(
                &fixture.program,
                json,
                ty,
                JsonLimits::default(),
                control,
            ),
            1 => codec::decode_typed_with_control(
                &fixture.reference,
                json,
                ty,
                JsonLimits::default(),
                control,
            ),
            2 => data_codec::decode_typed_with_control(&fixture.program, &data, ty, control),
            _ => data_codec_reference::decode_typed_with_control(
                &fixture.reference,
                &data,
                ty,
                control,
            ),
        };
        let retained = decode(&ExecutionControl::uncancelled()).unwrap();
        let mut successful = false;
        let mut refused = 0;
        for checks in 0..256 {
            match decode(&ExecutionControl::cancel_after_checks(checks)) {
                Ok(value) => {
                    assert_eq!(value, retained);
                    successful = true;
                    break;
                }
                Err(error) => {
                    assert_eq!(error.class, DiagnosticClass::Cancelled);
                    assert_eq!(error.code, "execution_cancelled");
                    refused += 1;
                }
            }
        }
        assert!(
            successful && refused > 8,
            "boundary={boundary}, refused={refused}"
        );
        assert_eq!(decode(&ExecutionControl::uncancelled()).unwrap(), retained);
        assert_bytes(&fixture, &retained, ty, json, &data);
    }
}

fn singleton_map_chain(mut value: NormalizedValue, depth: usize) -> NormalizedValue {
    for _ in 0..depth {
        value = NormalizedValue::map(BTreeMap::from([(NormalizedMapKey::I64(0), value)])).unwrap();
    }
    value
}

fn shared_map_chain(depth: usize) -> NormalizedValue {
    let child = singleton_map_chain(NormalizedValue::I64(7), depth - 1);
    NormalizedValue::map(BTreeMap::from([
        (NormalizedMapKey::I64(0), child.clone()),
        (NormalizedMapKey::I64(1), child),
    ]))
    .unwrap()
}

// Do not use recursive runtime equality as the oracle for the codec's stack boundary.
fn assert_map_chain(value: &NormalizedValue, depth: usize) {
    let NormalizedValue::Map(root) = value else {
        panic!("expected outer map");
    };
    assert_eq!(root.len(), 2);
    for key in [0, 1] {
        let mut value = root.get(&NormalizedMapKey::I64(key)).unwrap();
        for _ in 1..depth {
            let NormalizedValue::Map(entries) = value else {
                panic!("expected nested map");
            };
            assert_eq!(entries.len(), 1);
            value = entries.get(&NormalizedMapKey::I64(0)).unwrap();
        }
        assert!(matches!(value, NormalizedValue::I64(7)));
    }
}

fn assert_json_map_chain(value: &serde_json::Value, depth: usize) {
    let root = value.as_array().unwrap();
    assert_eq!(root.len(), 2);
    for (key, pair) in root.iter().enumerate() {
        let pair = pair.as_array().unwrap();
        assert_eq!(pair.len(), 2);
        assert_eq!(pair[0].as_i64(), Some(key as i64));
        let mut value = &pair[1];
        for _ in 1..depth {
            let entries = value.as_array().unwrap();
            assert_eq!(entries.len(), 1);
            let pair = entries[0].as_array().unwrap();
            assert_eq!(pair.len(), 2);
            assert_eq!(pair[0].as_i64(), Some(0));
            value = &pair[1];
        }
        assert_eq!(value.as_i64(), Some(7));
    }
}

fn nested_map_envelope(fixture: &Fixture, depth: usize) -> Vec<u8> {
    let mut layout = Vec::new();
    for index in (1..=depth).rev() {
        layout.extend_from_slice(&fixture.map_chain[index].bytes());
        layout.push(8); // Map.
        layout.extend_from_slice(&fixture.integer.bytes());
        layout.push(2); // I64 key.
    }
    layout.extend_from_slice(&fixture.integer.bytes());
    layout.push(2); // I64 leaf.
    let mut body = b"LKJDVAL1\x00\x01".to_vec();
    body.extend_from_slice(&digest("lkjscript.data.typed-layout.v1", &layout));
    body.extend_from_slice(&2u32.to_be_bytes());
    for key in [0i64, 1] {
        body.extend_from_slice(&key.to_be_bytes());
        for _ in 1..depth {
            body.extend_from_slice(&1u32.to_be_bytes());
            body.extend_from_slice(&0i64.to_be_bytes());
        }
        body.extend_from_slice(&7i64.to_be_bytes());
    }
    checksum(body)
}

#[test]
fn map_codec_nested_boundaries_fit_the_existing_stack_and_preserve_shared_values() {
    // Preparation has its separate depth owner. Keep it outside the bounded codec worker.
    let fixture = fixture_with_map_depth(129);
    std::thread::Builder::new()
        .name("nested-map-codec-boundaries".to_owned())
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let value = shared_map_chain(128);
            let retained = value.clone();
            let ty = fixture.map_chain[128];
            let expected = nested_map_envelope(&fixture, 128);
            assert_eq!(
                data_codec::encode_typed(&fixture.program, &value, ty).unwrap(),
                expected
            );
            assert_eq!(
                data_codec_reference::encode_typed(&fixture.reference, &value, ty).unwrap(),
                expected
            );
            let decoded = data_codec::decode_typed(&fixture.program, &expected, ty).unwrap();
            assert_map_chain(&decoded, 128);
            let decoded =
                data_codec_reference::decode_typed(&fixture.reference, &expected, ty).unwrap();
            assert_map_chain(&decoded, 128);
            for schema in [
                &fixture.program as &dyn super::super::value_schema::NormalizedValueSchema,
                &fixture.reference,
            ] {
                let json = codec::encode_value(schema, &value, ty, JsonLimits::default()).unwrap();
                assert_json_map_chain(&json, 128);
                let decoded =
                    codec::decode_value(schema, &json, ty, JsonLimits::default()).unwrap();
                assert_map_chain(&decoded, 128);
                // Pair arrays contribute two JSON container levels per map at the byte
                // parser. Exercise that separate, unchanged limit with a shallower chain.
                let shallow = shared_map_chain(63);
                let shallow_ty = fixture.map_chain[63];
                let bytes =
                    codec::encode_typed(schema, &shallow, shallow_ty, JsonLimits::default())
                        .unwrap();
                let decoded =
                    codec::decode_typed(schema, &bytes, shallow_ty, JsonLimits::default()).unwrap();
                assert_map_chain(&decoded, 63);
            }
            let excessive = shared_map_chain(129);
            let excessive_ty = fixture.map_chain[129];
            for error in [
                data_codec::encode_typed(&fixture.program, &excessive, excessive_ty).unwrap_err(),
                data_codec_reference::encode_typed(&fixture.reference, &excessive, excessive_ty)
                    .unwrap_err(),
            ] {
                assert_eq!(error.class, DiagnosticClass::Resource);
                assert_eq!(error.code, "normalized_data_layout_depth");
            }
            // JSON's maintained depth owner reports a source type error, while typed
            // data's independent layout owners report finite resource exhaustion.
            for schema in [
                &fixture.program as &dyn super::super::value_schema::NormalizedValueSchema,
                &fixture.reference,
            ] {
                let error =
                    codec::encode_value(schema, &excessive, excessive_ty, JsonLimits::default())
                        .unwrap_err();
                assert_eq!(error.class, DiagnosticClass::Source);
                assert_eq!(error.code, "normalized_json_type");
            }
            assert_map_chain(&retained, 128);
            assert_eq!(
                data_codec::encode_typed(&fixture.program, &retained, ty).unwrap(),
                expected
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
