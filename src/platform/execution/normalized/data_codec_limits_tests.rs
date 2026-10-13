//! Producer/consumer limits at the public byte and semantic-node boundaries.
use super::super::{data_codec, data_codec_reference};
use super::*;
use crate::platform::diagnostic::DiagnosticClass;

const BYTE_LIMIT: usize = 4 * 1_048_576;
const ITEM_LIMIT: usize = 1_000_000;
const ENVELOPE_BYTES: usize = 8 + 2 + 32 + 4 + 32;

struct Fixture {
    program: NormalizedProgram,
    reference: super::super::reference::BoundReferenceSchema,
    bytes: TypeObjectDigest,
    text: TypeObjectDigest,
    list: TypeObjectDigest,
    map: TypeObjectDigest,
    nested: TypeObjectDigest,
    nested_map: TypeObjectDigest,
}

fn fixture() -> Fixture {
    let source = byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create limits
  (function create accept (visibility private) (effect pure)
    (parameter create bytes (type Bytes)) (parameter create text (type Text))
    (parameter create list (type (list Unit)))
    (parameter create map (type (map I64 Unit)))
    (parameter create nested (type (list (list Unit))))
    (parameter create nested-map (type (map I64 (list Unit))))
    (returns Unit) (body (unit)))))
declarations.end"#,
    )
    .unwrap();
    let declared = |name: &str| {
        source
            .owners
            .values()
            .find_map(|owner| match owner {
                OwnerRecord::Parameter(p) if p.name.as_str() == name => Some(p.ty),
                _ => None,
            })
            .unwrap()
    };
    let program = prepare_snapshot(&source);
    let reference = super::super::reference::BoundReferenceSchema {
        canonical: Arc::new(
            super::super::NormalizedReferenceSchema::reconstruct([&source]).unwrap(),
        ),
        value_origin: program.value_origin,
    };
    Fixture {
        program,
        reference,
        bytes: declared("bytes"),
        text: declared("text"),
        list: declared("list"),
        map: declared("map"),
        nested: declared("nested"),
        nested_map: declared("nested-map"),
    }
}

fn round_trip(f: &Fixture, value: &NormalizedValue, ty: TypeObjectDigest) -> Vec<u8> {
    let encoded = data_codec::encode_typed(&f.program, value, ty).unwrap();
    let independent = data_codec_reference::encode_typed(&f.reference, value, ty).unwrap();
    assert!(encoded == independent, "independent encoders disagree");
    assert!(data_codec::decode_typed(&f.program, &encoded, ty).unwrap() == *value);
    assert!(data_codec_reference::decode_typed(&f.reference, &encoded, ty).unwrap() == *value);
    encoded
}

// Reuse only the admitted layout identity, then independently write counted
// payloads/checksums. No production encoder must accept the hostile value.
fn counted_envelope(header: &[u8], payload: &[u8]) -> Vec<u8> {
    let mut bytes = header[..42].to_vec();
    bytes.extend_from_slice(payload);
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.data.typed-value-envelope.v1");
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    bytes.extend_from_slice(hasher.finalize().as_bytes());
    bytes
}

fn rejects(f: &Fixture, encoded: &[u8], ty: TypeObjectDigest, code: &str) {
    for error in [
        data_codec::decode_typed(&f.program, encoded, ty).unwrap_err(),
        data_codec_reference::decode_typed(&f.reference, encoded, ty).unwrap_err(),
    ] {
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert_eq!(error.code, code);
    }
}

#[test]
fn typed_data_blob_lengths_use_byte_limits_not_collection_limits() {
    let f = fixture();
    for (ty, text) in [(f.bytes, false), (f.text, true)] {
        for length in [ITEM_LIMIT, ITEM_LIMIT + 1, BYTE_LIMIT - ENVELOPE_BYTES] {
            let value = if text {
                NormalizedValue::text("x".repeat(length))
            } else {
                NormalizedValue::bytes(vec![b'x'; length])
            };
            let encoded = round_trip(&f, &value, ty);
            assert_eq!(encoded.len(), ENVELOPE_BYTES + length);
        }
        let length = BYTE_LIMIT - ENVELOPE_BYTES + 1;
        let value = if text {
            NormalizedValue::text("x".repeat(length))
        } else {
            NormalizedValue::bytes(vec![b'x'; length])
        };
        for error in [
            data_codec::encode_typed(&f.program, &value, ty).unwrap_err(),
            data_codec_reference::encode_typed(&f.reference, &value, ty).unwrap_err(),
        ] {
            assert_eq!(error.code, "normalized_data_value_bytes");
        }
        let empty = if text {
            NormalizedValue::text("")
        } else {
            NormalizedValue::bytes(vec![])
        };
        let header = data_codec::encode_typed(&f.program, &empty, ty).unwrap();
        let mut payload = (length as u32).to_be_bytes().to_vec();
        payload.extend(std::iter::repeat_n(b'x', length));
        rejects(
            &f,
            &counted_envelope(&header, &payload),
            ty,
            "normalized_data_value_bytes",
        );
    }
}

#[test]
fn typed_data_collections_charge_nodes_once_and_preflight_nested_allocations() {
    let f = fixture();
    // Flat raw storage has its own finite bound. Keep its wire-count checks small;
    // use shared nested carriers to reach the codec's independent logical limit.
    for count in [0, 1, 32, 1024] {
        let value = NormalizedValue::list(vec![NormalizedValue::Unit; count]).unwrap();
        let encoded = round_trip(&f, &value, f.list);
        assert_eq!(encoded.len(), ENVELOPE_BYTES);
        assert_eq!(&encoded[42..46], &(count as u32).to_be_bytes());
    }
    let nested_units = |outer, inner| {
        let child = NormalizedValue::list(vec![NormalizedValue::Unit; inner]).unwrap();
        NormalizedValue::list(vec![child; outer]).unwrap()
    };
    for (outer, inner, nodes) in [(500, 999, 500_001), (999, 1000, ITEM_LIMIT)] {
        // Each alias remains a complete logical occurrence in both codecs.
        assert_eq!(1 + outer * (1 + inner), nodes);
        let value = nested_units(outer, inner);
        let encoded = round_trip(&f, &value, f.nested);
        assert_eq!(encoded.len(), ENVELOPE_BYTES + outer * 4);
        assert_eq!(&encoded[42..46], &(outer as u32).to_be_bytes());
        for count in encoded[46..encoded.len() - 32].as_chunks::<4>().0 {
            assert_eq!(count, &(inner as u32).to_be_bytes());
        }
    }
    assert_eq!(1 + 1000 * (1 + 999), ITEM_LIMIT + 1);
    let excessive = nested_units(1000, 999);
    for error in [
        data_codec::encode_typed(&f.program, &excessive, f.nested).unwrap_err(),
        data_codec_reference::encode_typed(&f.reference, &excessive, f.nested).unwrap_err(),
    ] {
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert_eq!(error.code, "normalized_data_value_items");
    }
    drop(excessive);
    let empty_list = NormalizedValue::list(vec![]).unwrap();
    let list_header = data_codec::encode_typed(&f.program, &empty_list, f.list).unwrap();
    rejects(
        &f,
        &counted_envelope(&list_header, &(ITEM_LIMIT as u32).to_be_bytes()),
        f.list,
        "normalized_data_value_items",
    );
    let nested_header = data_codec::encode_typed(&f.program, &empty_list, f.nested).unwrap();
    let mut nested = 2_u32.to_be_bytes().to_vec();
    nested.extend_from_slice(&((ITEM_LIMIT - 1) as u32).to_be_bytes());
    rejects(
        &f,
        &counted_envelope(&nested_header, &nested),
        f.nested,
        "normalized_data_value_items",
    );

    let flat_entries = (0..1024)
        .map(|n| {
            (
                super::super::value::NormalizedMapKey::I64(n),
                NormalizedValue::Unit,
            )
        })
        .collect();
    let flat =
        NormalizedValue::map_controlled(flat_entries, &ExecutionControl::uncancelled()).unwrap();
    let flat_encoded = round_trip(&f, &flat, f.map);
    assert_eq!(flat_encoded.len(), ENVELOPE_BYTES + 1024 * 8);

    // These 500001 semantic nodes fit. The former decoder charged them twice.
    // Shared list backing bounds raw carrier storage without deduplicating keys,
    // list nodes, or Unit occurrences in the serialized value or either decoder.
    let count = 1000;
    let inner = 498;
    assert_eq!(1 + count * (1 + 1 + inner), 500_001);
    let child = NormalizedValue::list(vec![NormalizedValue::Unit; inner]).unwrap();
    let entries = (0..count)
        .map(|n| {
            (
                super::super::value::NormalizedMapKey::I64(n as i64),
                child.clone(),
            )
        })
        .collect();
    let value = NormalizedValue::map_controlled(entries, &ExecutionControl::uncancelled()).unwrap();
    let encoded = round_trip(&f, &value, f.nested_map);
    assert_eq!(encoded.len(), ENVELOPE_BYTES + count * (8 + 4));
    rejects(
        &f,
        &counted_envelope(&flat_encoded, &500_000_u32.to_be_bytes()),
        f.map,
        "normalized_data_value_items",
    );
}
