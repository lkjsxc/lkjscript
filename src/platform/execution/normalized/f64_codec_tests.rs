//! Fixed numeric and byte expectations for the ordinary typed JSON/data boundaries.

use super::super::{codec, data_codec, data_codec_reference};
use super::*;
use crate::platform::binary64::Binary64;
use crate::platform::kernel::{FieldRecord, ModuleRecord, ParameterUse};
use crate::platform::semantic_id::{FieldId, ModuleId};

struct Fixture {
    program: NormalizedProgram,
    reference: super::super::reference::BoundReferenceSchema,
    float: TypeObjectDigest,
    integer: TypeObjectDigest,
    list: TypeObjectDigest,
    packet: TypeObjectDigest,
}

fn fixture() -> Fixture {
    let seed = b"binary64-ordinary-typed-codecs";
    let mut snapshot = empty_normalized_snapshot(seed);
    let module = ModuleId::migrate(seed, 0);
    let record = DeclarationId::migrate(seed, 0);
    let function = DeclarationId::migrate(seed, 1);
    let parameter = ParameterId::migrate(seed, 0);
    let body = ExpressionId::migrate(seed, 0);
    let unit = admit_snapshot_type(&mut snapshot, TypeForm::Unit);
    let integer = admit_snapshot_type(&mut snapshot, TypeForm::I64);
    let float = admit_snapshot_type(&mut snapshot, TypeForm::F64);
    let list = admit_snapshot_type(&mut snapshot, TypeForm::List { item: float });
    let reference = DeclarationReference {
        package: snapshot.root.package_id,
        declaration: record,
    };
    let packet = admit_snapshot_type(
        &mut snapshot,
        TypeForm::Named {
            declaration: reference,
        },
    );
    snapshot.owners.insert(
        OwnerKey::Module(module),
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: Name::new("numeric").unwrap(),
        }),
    );
    let mut fields = [FieldId::migrate(seed, 0), FieldId::migrate(seed, 1)];
    fields.sort();
    for (field, name, ty) in [(fields[0], "count", integer), (fields[1], "samples", list)] {
        snapshot.owners.insert(
            OwnerKey::Field(field),
            OwnerRecord::Field(FieldRecord {
                header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
                declaration: record,
                name: Name::new(name).unwrap(),
                ty,
            }),
        );
    }
    snapshot.owners.insert(
        OwnerKey::Declaration(record),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(record), OwnerKind::Record),
            module,
            name: Name::new("Packet").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Record {
                type_parameters: vec![],
                fields: fields.to_vec(),
            },
        }),
    );
    snapshot.owners.insert(
        OwnerKey::Expression(body),
        OwnerRecord::Expression(ExpressionRecord::new(body, ExpressionOperation::Unit {}).unwrap()),
    );
    snapshot.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(function),
            name: Name::new("input").unwrap(),
            ty: packet,
            use_mode: ParameterUse::Unrestricted,
            resource_requirement: None,
        }),
    );
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
                parameters: vec![parameter],
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
        float,
        integer,
        list,
        packet,
    }
}

fn scalar(bits: u64) -> NormalizedValue {
    NormalizedValue::F64(Binary64::from_bits(bits).unwrap())
}

fn scalar_bits(value: NormalizedValue) -> u64 {
    let NormalizedValue::F64(value) = value else {
        panic!("expected F64")
    };
    value.bits()
}

fn checksum(mut body: Vec<u8>) -> Vec<u8> {
    let mut hash = blake3::Hasher::new_derive_key("lkjscript.data.typed-value-envelope.v1");
    hash.update(&(body.len() as u64).to_be_bytes());
    hash.update(&body);
    body.extend_from_slice(hash.finalize().as_bytes());
    body
}

/// An independent expression of the scalar layout and envelope contract, with literal tags.
fn expected_scalar_envelope(ty: TypeObjectDigest, tag: u8, payload: &[u8]) -> Vec<u8> {
    let mut description = ty.bytes().to_vec();
    description.push(tag);
    let mut layout = blake3::Hasher::new_derive_key("lkjscript.data.typed-layout.v1");
    layout.update(&33u64.to_be_bytes());
    layout.update(&description);
    let mut body = b"LKJDVAL1\x00\x01".to_vec();
    body.extend_from_slice(layout.finalize().as_bytes());
    body.extend_from_slice(payload);
    checksum(body)
}

#[test]
fn f64_json_decimals_integer_boundaries_and_negative_zero_are_exact() {
    let fixture = fixture();
    let limits = JsonLimits::default();
    for (token, expected) in [
        ("-0", 0x8000_0000_0000_0000u64),
        ("-0.0", 0x8000_0000_0000_0000),
        ("0", 0),
        ("1.25", 0x3ff4_0000_0000_0000),
        ("125e-2", 0x3ff4_0000_0000_0000),
        ("9007199254740993", 0x4340_0000_0000_0000),
        ("9223372036854775807", 0x43e0_0000_0000_0000),
        ("-9223372036854775808", 0xc3e0_0000_0000_0000),
        ("18446744073709553664", 0x43f0_0000_0000_0000),
        ("18446744073709553665", 0x43f0_0000_0000_0001),
        ("1.7976931348623157e308", 0x7fef_ffff_ffff_ffff),
        ("2.2250738585072014e-308", 0x0010_0000_0000_0000),
        ("5e-324", 1),
        ("-1e-324", 0x8000_0000_0000_0000),
    ] {
        for schema in [
            &fixture.program as &dyn super::super::value_schema::NormalizedValueSchema,
            &fixture.reference,
        ] {
            let value =
                codec::decode_typed(schema, token.as_bytes(), fixture.float, limits).unwrap();
            assert_eq!(scalar_bits(value.clone()), expected, "{token}");
            let encoded = codec::encode_typed(schema, &value, fixture.float, limits).unwrap();
            assert_ne!(encoded.first(), Some(&b'"'));
            assert_eq!(
                scalar_bits(codec::decode_typed(schema, &encoded, fixture.float, limits).unwrap()),
                expected,
                "{token}",
            );
        }
    }
    let midpoint = "1.00000000000000011102230246251565404236316680908203125";
    let long = format!("{}{}e-853", midpoint.replace('.', ""), "0".repeat(800));
    assert_eq!(
        scalar_bits(
            codec::decode_typed(&fixture.program, long.as_bytes(), fixture.float, limits).unwrap()
        ),
        0x3ff0_0000_0000_0000
    );
    for (token, expected) in [
        ("-9223372036854775808", i64::MIN),
        ("9223372036854775807", i64::MAX),
        ("9007199254740993", 9_007_199_254_740_993),
    ] {
        assert_eq!(
            codec::decode_typed(&fixture.program, token.as_bytes(), fixture.integer, limits)
                .unwrap(),
            NormalizedValue::I64(expected)
        );
        assert_eq!(
            codec::encode_typed(
                &fixture.program,
                &NormalizedValue::I64(expected),
                fixture.integer,
                limits
            )
            .unwrap(),
            token.as_bytes()
        );
    }
    for token in [
        "1.0",
        "1e0",
        "9223372036854775808",
        "-9223372036854775809",
        "-0",
    ] {
        assert!(
            codec::decode_typed(&fixture.program, token.as_bytes(), fixture.integer, limits)
                .is_err(),
            "{token}"
        );
    }
    for token in [
        "1e400",
        "nan",
        "inf",
        "-inf",
        "01",
        "1.0 false",
        "\"1.25\"",
        "null",
        "true",
    ] {
        assert!(
            codec::decode_typed(&fixture.program, token.as_bytes(), fixture.float, limits).is_err(),
            "{token}"
        );
    }
    assert_eq!(
        codec::encode_typed(
            &fixture.program,
            &scalar(0x8000_0000_0000_0000),
            fixture.float,
            limits
        )
        .unwrap(),
        b"-0.0"
    );
    let packet = codec::decode_typed(
        &fixture.program,
        br#"{"count":9007199254740993,"samples":[0.5,-0,1.25e1]}"#,
        fixture.packet,
        limits,
    )
    .unwrap();
    assert_eq!(
        codec::encode_typed(&fixture.program, &packet, fixture.packet, limits).unwrap(),
        br#"{"count":9007199254740993,"samples":[0.5,-0.0,12.5]}"#
    );
}

#[test]
fn f64_json_preflight_accepts_type_and_nonfinite_values_fail_before_payload_output() {
    let fixture = fixture();
    let limits = JsonLimits::default();
    let control = ExecutionControl::uncancelled();
    codec::require_json_encoding(&fixture.program, fixture.packet, false, limits, &control)
        .unwrap();
    for bits in [
        0x7ff0_0000_0000_0000,
        0xfff0_0000_0000_0000,
        0x7ff8_0000_0000_0000,
    ] {
        for schema in [
            &fixture.program as &dyn super::super::value_schema::NormalizedValueSchema,
            &fixture.reference,
        ] {
            let error =
                codec::encode_typed(schema, &scalar(bits), fixture.float, limits).unwrap_err();
            assert_eq!(error.code, "normalized_json_nonfinite");
            assert_eq!(error.class, crate::platform::DiagnosticClass::Semantic);
            let nested = NormalizedValue::Record(super::super::value::NormalizedRecord::Nominal {
                layout: fixture.program.record_instances[&fixture.packet],
                fields: Arc::new(vec![
                    NormalizedValue::I64(2),
                    NormalizedValue::list(vec![scalar(0x3ff0_0000_0000_0000), scalar(bits)])
                        .unwrap(),
                ]),
            });
            let error = codec::encode_typed(schema, &nested, fixture.packet, limits).unwrap_err();
            assert_eq!(error.code, "normalized_json_nonfinite");
            assert!(error.message.contains("$.samples[1]"));
        }
    }
}

#[test]
fn f64_data_has_fixed_little_endian_scalar_bytes_and_predecessor_integer_bytes() {
    let fixture = fixture();
    for (bits, bytes) in [
        (0u64, [0, 0, 0, 0, 0, 0, 0, 0]),
        (0x8000_0000_0000_0000, [0, 0, 0, 0, 0, 0, 0, 0x80]),
        (1, [1, 0, 0, 0, 0, 0, 0, 0]),
        (0x3ff4_0000_0000_0000, [0, 0, 0, 0, 0, 0, 0xf4, 0x3f]),
        (
            0x7fef_ffff_ffff_ffff,
            [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xef, 0x7f],
        ),
        (0x7ff0_0000_0000_0000, [0, 0, 0, 0, 0, 0, 0xf0, 0x7f]),
        (0xfff0_0000_0000_0000, [0, 0, 0, 0, 0, 0, 0xf0, 0xff]),
        (0x7ff8_0000_0000_0000, [0, 0, 0, 0, 0, 0, 0xf8, 0x7f]),
    ] {
        let value = scalar(bits);
        let expected = expected_scalar_envelope(fixture.float, 10, &bytes);
        assert_eq!(
            data_codec::encode_typed(&fixture.program, &value, fixture.float).unwrap(),
            expected
        );
        assert_eq!(
            data_codec_reference::encode_typed(&fixture.reference, &value, fixture.float).unwrap(),
            expected
        );
        assert_eq!(
            scalar_bits(
                data_codec::decode_typed(&fixture.program, &expected, fixture.float).unwrap()
            ),
            bits
        );
        assert_eq!(
            scalar_bits(
                data_codec_reference::decode_typed(&fixture.reference, &expected, fixture.float)
                    .unwrap()
            ),
            bits
        );
    }
    for (integer, bytes) in [
        (i64::MIN, [0x80, 0, 0, 0, 0, 0, 0, 0]),
        (i64::MAX, [0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]),
        (9_007_199_254_740_993, [0, 0x20, 0, 0, 0, 0, 0, 1]),
    ] {
        let expected = expected_scalar_envelope(fixture.integer, 2, &bytes);
        let value = NormalizedValue::I64(integer);
        assert_eq!(
            data_codec::encode_typed(&fixture.program, &value, fixture.integer).unwrap(),
            expected
        );
        assert_eq!(
            data_codec_reference::encode_typed(&fixture.reference, &value, fixture.integer)
                .unwrap(),
            expected
        );
        assert_eq!(
            data_codec::decode_typed(&fixture.program, &expected, fixture.integer).unwrap(),
            value
        );
        assert_eq!(
            data_codec_reference::decode_typed(&fixture.reference, &expected, fixture.integer)
                .unwrap(),
            value
        );
    }
}

#[test]
fn f64_data_nested_nominal_list_roundtrips_and_strict_readers_reject_rehashed_faults() {
    let fixture = fixture();
    let value = NormalizedValue::Record(super::super::value::NormalizedRecord::Nominal {
        layout: fixture.program.record_instances[&fixture.packet],
        fields: Arc::new(vec![
            NormalizedValue::I64(5),
            NormalizedValue::list(vec![
                scalar(0x3ff4_0000_0000_0000),
                scalar(0x8000_0000_0000_0000),
                scalar(0x7ff0_0000_0000_0000),
                scalar(0xfff0_0000_0000_0000),
                scalar(0x7ff8_0000_0000_0000),
            ])
            .unwrap(),
        ]),
    });
    let encoded = data_codec::encode_typed(&fixture.program, &value, fixture.packet).unwrap();
    let payload = [
        0, 0, 0, 0, 0, 0, 0, 5, // predecessor I64 count, big endian
        0, 0, 0, 5, // predecessor list count, big endian
        0, 0, 0, 0, 0, 0, 0xf4, 0x3f, 0, 0, 0, 0, 0, 0, 0, 0x80, 0, 0, 0, 0, 0, 0, 0xf0, 0x7f, 0,
        0, 0, 0, 0, 0, 0xf0, 0xff, 0, 0, 0, 0, 0, 0, 0xf8, 0x7f,
    ];
    assert_eq!(&encoded[42..encoded.len() - 32], payload);
    assert_eq!(
        data_codec_reference::encode_typed(&fixture.reference, &value, fixture.packet).unwrap(),
        encoded
    );
    assert_eq!(
        data_codec::decode_typed(&fixture.program, &encoded, fixture.packet).unwrap(),
        value
    );
    assert_eq!(
        data_codec_reference::decode_typed(&fixture.reference, &encoded, fixture.packet).unwrap(),
        value
    );
    for bits in [
        0x7ff0_0000_0000_0001u64,
        0x7ff8_0000_0000_0001,
        0xfff8_0000_0000_0000,
    ] {
        let mut body = encoded[..encoded.len() - 32].to_vec();
        body[86..94].copy_from_slice(&bits.to_le_bytes());
        let fault = checksum(body);
        assert_eq!(
            data_codec::decode_typed(&fixture.program, &fault, fixture.packet)
                .unwrap_err()
                .code,
            "normalized_data_f64"
        );
        assert_eq!(
            data_codec_reference::decode_typed(&fixture.reference, &fault, fixture.packet)
                .unwrap_err()
                .code,
            "normalized_data_f64"
        );
    }
    let body = encoded[..encoded.len() - 32].to_vec();
    let mut trailing = body.clone();
    trailing.push(0);
    let mut truncated = body.clone();
    truncated.pop();
    let mut wrong_layout = body.clone();
    wrong_layout[10] ^= 1;
    let mut foreign_version = body;
    foreign_version[9] = 0;
    for (fault, code) in [
        (checksum(trailing), "normalized_data_value_trailing"),
        (checksum(truncated), "normalized_data_f64"),
        (checksum(wrong_layout), "normalized_data_value_layout"),
        (checksum(foreign_version), "normalized_data_value_version"),
    ] {
        assert_eq!(
            data_codec::decode_typed(&fixture.program, &fault, fixture.packet)
                .unwrap_err()
                .code,
            code
        );
        assert_eq!(
            data_codec_reference::decode_typed(&fixture.reference, &fault, fixture.packet)
                .unwrap_err()
                .code,
            code
        );
    }
    for ty in [fixture.float, fixture.integer, fixture.list] {
        assert!(data_codec::decode_typed(&fixture.program, &encoded, ty).is_err());
        assert!(data_codec_reference::decode_typed(&fixture.reference, &encoded, ty).is_err());
    }
}

#[test]
fn f64_cannot_be_a_json_or_data_map_key_even_in_an_empty_map() {
    let mut fixture = fixture();
    let map = admit_runtime_type(
        &mut fixture.program,
        TypeForm::Map {
            key: fixture.float,
            value: fixture.integer,
        },
    );
    let mut schema = (*fixture.reference.canonical).clone();
    schema
        .types
        .insert(map, fixture.program.types[&map].clone());
    let reference = super::super::reference::BoundReferenceSchema {
        canonical: Arc::new(schema),
        value_origin: fixture.program.value_origin,
    };
    let value = NormalizedValue::Map(Arc::new(BTreeMap::new()));
    assert!(NormalizedMapKey::from_value(scalar(0)).is_none());
    for schema in [
        &fixture.program as &dyn super::super::value_schema::NormalizedValueSchema,
        &reference,
    ] {
        assert!(codec::encode_typed(schema, &value, map, JsonLimits::default()).is_err());
        assert!(codec::decode_typed(schema, b"[]", map, JsonLimits::default()).is_err());
    }
    assert!(data_codec::encode_typed(&fixture.program, &value, map).is_err());
    assert!(data_codec_reference::encode_typed(&reference, &value, map).is_err());
}
