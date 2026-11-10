//! Typed-data byte contracts and collection lifecycle across serialization boundaries.
use super::super::{data_codec, data_codec_reference};
use super::*;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::kernel::KernelSnapshot;

const BYTE_LIMIT: usize = 4 * 1_048_576;
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
    sequence: TypeObjectDigest,
    pair: TypeObjectDigest,
    list_identity: DeclarationReference,
    map_identity: DeclarationReference,
    snapshot: KernelSnapshot,
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
    (parameter create sequence (type (owned-sequence I64)) (use consume))
    (parameter create pair (type (owned-product (field rest (owned-sequence I64)) (field value I64))) (use consume))
    (returns Unit) (body (unit)))
  (function create list-identity (visibility private) (effect pure)
    (parameter create values (type (list Unit))) (returns (list Unit))
    (body (local values)))
  (function create map-identity (visibility private) (effect pure)
    (parameter create values (type (map I64 Unit))) (returns (map I64 Unit))
    (body (local values)))))
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
    let declaration = |name: &str| {
        source
            .owners
            .iter()
            .find_map(|(key, owner)| match (key, owner) {
                (OwnerKey::Declaration(id), OwnerRecord::Declaration(d))
                    if d.name.as_str() == name =>
                {
                    Some(DeclarationReference {
                        package: source.root.package_id,
                        declaration: *id,
                    })
                }
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
        sequence: declared("sequence"),
        pair: declared("pair"),
        list_identity: declaration("list-identity"),
        map_identity: declaration("map-identity"),
        snapshot: source,
    }
}

fn round_trip(f: &Fixture, value: &NormalizedValue, ty: TypeObjectDigest) -> Vec<u8> {
    let encoded = data_codec::encode_typed(&f.program, value, ty).unwrap();
    let independent = data_codec_reference::encode_typed(&f.reference, value, ty).unwrap();
    assert!(encoded == independent, "independent encoders disagree");
    assert_eq!(
        crate::platform::execution::normalized::structural_equality::assertions::equal(
            &data_codec::decode_typed(&f.program, &encoded, ty).unwrap(),
            value
        ),
        Ok(true)
    );
    assert_eq!(
        crate::platform::execution::normalized::structural_equality::assertions::equal(
            &data_codec_reference::decode_typed(&f.reference, &encoded, ty).unwrap(),
            value
        ),
        Ok(true)
    );
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
        for length in [0, 1024, BYTE_LIMIT - ENVELOPE_BYTES] {
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
fn typed_data_collections_preserve_counts_and_reject_truncation() {
    let f = fixture();
    for count in [0, 1, 32, 1024] {
        let value = NormalizedValue::list(vec![NormalizedValue::Unit; count]).unwrap();
        let encoded = round_trip(&f, &value, f.list);
        assert_eq!(encoded.len(), ENVELOPE_BYTES);
        assert_eq!(&encoded[42..46], &(count as u32).to_be_bytes());
    }
    for (outer, inner) in [(3, 5), (32, 33)] {
        let child = NormalizedValue::list(vec![NormalizedValue::Unit; inner]).unwrap();
        let value = NormalizedValue::list(vec![child; outer]).unwrap();
        let encoded = round_trip(&f, &value, f.nested);
        assert_eq!(encoded.len(), ENVELOPE_BYTES + outer * 4);
        assert_eq!(&encoded[42..46], &(outer as u32).to_be_bytes());
        for count in encoded[46..encoded.len() - 32].as_chunks::<4>().0 {
            assert_eq!(count, &(inner as u32).to_be_bytes());
        }
    }
    let entries = (0..1024)
        .map(|n| (NormalizedMapKey::I64(n), NormalizedValue::Unit))
        .collect();
    let flat = NormalizedValue::map(entries).unwrap();
    let encoded = round_trip(&f, &flat, f.map);
    assert_eq!(encoded.len(), ENVELOPE_BYTES + 1024 * 8);
    let missing_keys = counted_envelope(&encoded, &1024_u32.to_be_bytes());
    for error in [
        data_codec::decode_typed(&f.program, &missing_keys, f.map).unwrap_err(),
        data_codec_reference::decode_typed(&f.reference, &missing_keys, f.map).unwrap_err(),
    ] {
        assert_eq!(error.class, DiagnosticClass::Corrupt);
        assert_eq!(error.code, "normalized_data_i64");
    }
    let child = NormalizedValue::list(vec![NormalizedValue::Unit; 33]).unwrap();
    let entries = (0..32)
        .map(|n| (NormalizedMapKey::I64(n), child.clone()))
        .collect();
    let nested = NormalizedValue::map(entries).unwrap();
    assert_eq!(
        round_trip(&f, &nested, f.nested_map).len(),
        ENVELOPE_BYTES + 32 * 12
    );
}

fn identity_engines(f: &Fixture, declaration: DeclarationReference, value: &NormalizedValue) {
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(&f.program, NormalizedRunPolicy::default());
    let reference = NormalizedReferenceInterpreter::new(
        &f.snapshot,
        &f.program,
        NormalizedRunPolicy::default(),
    );
    for output in [
        vm.invoke(declaration, vec![value.clone()], None, &control)
            .unwrap()
            .0,
        reference
            .invoke(declaration, vec![value.clone()], None, &control)
            .unwrap()
            .0,
    ] {
        assert_eq!(
            super::super::structural_equality::assertions::equal(&output, value),
            Ok(true)
        );
    }
    for error in [
        vm.invoke(
            declaration,
            vec![value.clone()],
            None,
            &ExecutionControl::cancel_after_checks(32),
        )
        .unwrap_err(),
        reference
            .invoke(
                declaration,
                vec![value.clone()],
                None,
                &ExecutionControl::cancel_after_checks(32),
            )
            .unwrap_err(),
    ] {
        assert_eq!(
            error.class,
            crate::platform::execution::ExecutionFailureClass::Cancelled
        );
    }
}

fn collection_lifecycle(f: &Fixture, count: usize) {
    let control = ExecutionControl::uncancelled();
    let list = NormalizedValue::list(vec![NormalizedValue::Unit; count]).unwrap();
    identity_engines(f, f.list_identity, &list);
    // Unit has no payload bytes: this isolates cardinality from the separate byte contract.
    let encoded = round_trip(f, &list, f.list);
    assert_eq!(encoded.len(), ENVELOPE_BYTES);
    assert_eq!(&encoded[42..46], &(count as u32).to_be_bytes());
    let cancelled = ExecutionControl::cancel_after_checks(64);
    assert!(
        data_codec::decode_typed_with_control(&f.program, &encoded, f.list, &cancelled).is_err()
    );
    let cancelled = ExecutionControl::cancel_after_checks(64);
    assert!(
        data_codec_reference::decode_typed_with_control(&f.reference, &encoded, f.list, &cancelled)
            .is_err()
    );
    drop(list);

    let entries = (0..count)
        .map(|n| (NormalizedMapKey::I64(n as i64), NormalizedValue::Unit))
        .collect();
    let value = NormalizedValue::map_controlled(entries, &control).unwrap();
    identity_engines(f, f.map_identity, &value);
    let NormalizedValue::Map(map) = &value else {
        panic!("map carrier")
    };
    let changed = map
        .insert(
            NormalizedMapKey::I64(count as i64),
            NormalizedValue::Unit,
            &mut |_| control.check(),
        )
        .unwrap();
    assert_eq!(map.len(), count);
    assert_eq!(changed.len(), count + 1);
    let removed = changed
        .remove(&NormalizedMapKey::I64(count as i64), &mut |_| {
            control.check()
        })
        .unwrap();
    assert_eq!(
        super::super::structural_equality::assertions::equal(&removed, map),
        Ok(true)
    );
    drop((changed, removed));
    // Select the existing outer byte control to contain this complete JSON representation.
    let limits = JsonLimits {
        maximum_bytes: count.checked_mul(32).unwrap() + 1024,
        maximum_string_bytes: 32,
    };
    let json = super::super::codec::encode_typed(&f.program, &value, f.map, limits).unwrap();
    let tree = crate::platform::json::decode_application(&json, limits).unwrap();
    assert_eq!(tree.as_array().unwrap().len(), count);
    let cloned_tree = tree.clone();
    assert_eq!(cloned_tree, tree);
    assert_eq!(
        blake3::hash(&serde_json::to_vec(&cloned_tree).unwrap()),
        blake3::hash(&json)
    );
    drop((tree, cloned_tree));
    let decoded = super::super::codec::decode_typed(&f.program, &json, f.map, limits).unwrap();
    assert_eq!(
        super::super::structural_equality::assertions::equal(&decoded, &value),
        Ok(true)
    );
    let mut malformed = json.clone();
    malformed.pop();
    assert!(crate::platform::json::decode_application(&malformed, limits).is_err());
    drop((malformed, decoded, json, value));
    let entries = (0..count)
        .map(|n| (NormalizedMapKey::I64(n as i64), NormalizedValue::Unit))
        .collect();
    assert_eq!(
        NormalizedValue::map_controlled(entries, &ExecutionControl::cancel_after_checks(64))
            .unwrap_err()
            .class,
        crate::platform::execution::ExecutionFailureClass::Cancelled
    );

    let origin = f.program.value_origin;
    let mut sequence = super::super::owned_sequence::OwnedSequence::create(
        origin,
        f.sequence,
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    for n in 0..count {
        sequence = sequence
            .push(
                origin,
                NormalizedValue::I64(n as i64),
                &control,
                &mut |_| Ok(()),
            )
            .unwrap();
    }
    sequence.establish_admission(origin).unwrap();
    assert_eq!(sequence.len(origin, &control).unwrap(), count);
    let reader = sequence.borrow().unwrap();
    assert!(
        matches!(reader.get(origin, count as i64 - 1, &control, &mut |_| Ok(())).unwrap(), NormalizedValue::I64(n) if n == count as i64 - 1)
    );
    drop(reader);
    let replaced = sequence
        .replace(
            origin,
            count as i64 - 1,
            NormalizedValue::I64(-1),
            f.pair,
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
    let NormalizedValue::OwnedProduct(pair) = replaced else {
        panic!("replacement pair")
    };
    let mut fields = pair.unpack(origin, f.pair, &control).unwrap().into_iter();
    let NormalizedValue::OwnedSequence(sequence) = fields.next().unwrap() else {
        panic!("sequence owner")
    };
    assert!(matches!(fields.next().unwrap(), NormalizedValue::I64(n) if n == count as i64 - 1));
    assert!(fields.next().is_none());
    let reader = sequence.borrow().unwrap();
    assert!(matches!(
        reader
            .get(origin, count as i64 - 1, &control, &mut |_| Ok(()))
            .unwrap(),
        NormalizedValue::I64(-1)
    ));
    drop(reader);
    // Cancellation owns cleanup of the complete consumed sequence.
    assert_eq!(
        sequence
            .push(
                origin,
                NormalizedValue::I64(0),
                &ExecutionControl::cancel_after_checks(0),
                &mut |_| Ok(())
            )
            .unwrap_err()
            .class,
        crate::platform::execution::ExecutionFailureClass::Cancelled
    );
}

#[test]
fn collection_cardinality_compact_lifecycle() {
    collection_lifecycle(&fixture(), 1024);
}

#[test]
#[ignore = "slow: geometric collection construction, VM/reference admission, codecs and cleanup beyond one million items"]
fn collection_cardinality_geometric_lifecycle() {
    let f = fixture();
    for count in [1 << 18, 1 << 20, 1 << 21] {
        let started = std::time::Instant::now();
        collection_lifecycle(&f, count);
        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        let rss = status
            .lines()
            .find(|line| line.starts_with("VmRSS:"))
            .unwrap_or("unavailable");
        let high_water = status
            .lines()
            .find(|line| line.starts_with("VmHWM:"))
            .unwrap_or("unavailable");
        eprintln!(
            "COLLECTION_CARDINALITY count={count} seconds={:.6} rss={rss} high_water={high_water}",
            started.elapsed().as_secs_f64()
        );
    }
}
