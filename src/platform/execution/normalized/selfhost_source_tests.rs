//! Source decoding is tested independently of semantic graph admission.
use super::*;
use crate::platform::kernel::{self, KernelSnapshot, TypeObjectDigest};

fn probes(module: &str) -> &'static (KernelSnapshot, NormalizedProgram) {
    static PROBES: [OnceLock<(KernelSnapshot, NormalizedProgram)>; 7] =
        [const { OnceLock::new() }; 7];
    let (index, proposal) = match module {
        "source-probe" => (
            0,
            include_str!("../../../../tests/fixtures/selfhost-source-probes.lkjc"),
        ),
        "source-record-probe" => (
            1,
            include_str!("../../../../tests/fixtures/selfhost-source-record-probes.lkjc"),
        ),
        "source-map-probe" => (
            2,
            include_str!("../../../../tests/fixtures/selfhost-source-map-probes.lkjc"),
        ),
        "source-state-probe" => (
            3,
            include_str!("../../../../tests/fixtures/selfhost-source-state-probes.lkjc"),
        ),
        "source-history-probe" => (
            4,
            include_str!("../../../../tests/fixtures/selfhost-source-history-probes.lkjc"),
        ),
        "source-object-probe" => (
            5,
            include_str!("../../../../tests/fixtures/selfhost-source-object-probes.lkjc"),
        ),
        "source-graph-probe" => (
            6,
            include_str!("../../../../tests/fixtures/selfhost-source-graph-probes.lkjc"),
        ),
        _ => return compiler(),
    };
    // Each independent probe graph retains the entire compiler source. Unrelated
    // test entrypoints do not need to share one finite-admission transaction history.
    PROBES[index].get_or_init(|| {
        let changes = proposals::updates(proposal).unwrap();
        let memory = crate::platform::MemProjectStore::new(128 * 1_048_576).unwrap();
        let repository = GraphRepository::create_memory(&memory, &compiler().0, None)
            .unwrap()
            .repository;
        let source = publish_proposals(&repository, changes);
        // Compile the public transaction result in its owning repository. A
        // second import needlessly repeats complete admission and storage.
        let program = prepare_from_repository(&repository);
        (source, program)
    })
}

fn function(module: &str, name: &str) -> DeclarationReference {
    let (source, _) = probes(module);
    let module = source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Module(id), OwnerRecord::Module(record))
                if record.name.as_str() == module =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap();
    let declaration = source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(record))
                if record.module == module && record.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap();
    DeclarationReference {
        package: source.root.package_id,
        declaration,
    }
}

fn invoke(module: &str, name: &str, arguments: Vec<NormalizedValue>) -> NormalizedValue {
    let (source, program) = probes(module);
    let declaration = function(module, name);
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::for_test(program, policy)
        .invoke(declaration, arguments.clone(), None, &control)
        .unwrap();
    let reference = NormalizedReferenceInterpreter::new(source, program, policy)
        .invoke(declaration, arguments, None, &control)
        .unwrap();
    assert_eq!(vm.0, reference.0, "{module}::{name}");
    assert_eq!(vm.1.live_call_frames_after, 0);
    assert_eq!(reference.1.live_call_frames_after, 0);
    vm.0
}

fn field<'a>(value: &'a NormalizedValue, name: &str) -> &'a NormalizedValue {
    let NormalizedValue::Record(NormalizedRecord::Structural { fields }) = value else {
        panic!("expected structural probe result");
    };
    &fields
        .iter()
        .find(|(key, _)| key.as_str() == name)
        .unwrap()
        .1
}

fn result(name: &str, bytes: &[u8]) -> (i64, Vec<u8>) {
    let module = match name {
        "read-owner" | "read-records" => "source-record-probe",
        "read-carrier" | "read-graph" => "source-graph-probe",
        "read-map" => "source-map-probe",
        "read-state" => "source-state-probe",
        "read-revision" | "read-dependency" | "read-retirement" => "source-history-probe",
        "bind-type" | "bind-owner" | "bind-dependency" | "bind-retirement" | "read-bindings" => {
            "source-object-probe"
        }
        _ => "source-probe",
    };
    let value = invoke(module, name, vec![NormalizedValue::bytes(bytes)]);
    let NormalizedValue::I64(code) = field(&value, "diagnostic") else {
        panic!("code")
    };
    let NormalizedValue::Bytes(image) = field(&value, "image") else {
        panic!("image")
    };
    static IMAGES: OnceLock<std::sync::Mutex<BTreeMap<String, Vec<u8>>>> = OnceLock::new();
    let native = IMAGES
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .entry(name.to_owned())
        .or_insert_with(|| {
            let (_, program) = probes(module);
            let image = crate::platform::compiler::elf_x86_64::compile_function(
                program,
                program.function(function(module, name)).unwrap().0 as usize,
            )
            .unwrap()
            .0;
            eprintln!(
                "selfhost source native producer entry={name} bytes={} sha256={}",
                image.len(),
                crate::platform::semantic_id::encode_hex(&Sha256::digest(&image))
            );
            image
        })
        .clone();
    let output = native_filter_tests::execute_filter(&native, bytes, &[], 127);
    assert_eq!(
        output.status.code(),
        Some(if *code == 0 { 0 } else { 65 }),
        "{name}: {:?}",
        output.stderr
    );
    assert_eq!(output.stdout, image.as_ref(), "{name}");
    if *code == 0 {
        assert!(output.stderr.is_empty());
    } else {
        assert!(image.is_empty());
        assert_eq!(
            output.stderr,
            format!("compiler_rejected:{code}\n").as_bytes()
        );
    }
    (*code, image.to_vec())
}

fn encode<T: bincode::Encode>(value: T) -> Vec<u8> {
    bincode::encode_to_vec(value, bincode::config::standard()).unwrap()
}

#[test]
fn selfhost_source_integers_decode_values_canonically_on_three_engines() {
    for value in [
        0,
        1,
        -1,
        125,
        -125,
        126,
        -126,
        32767,
        -32768,
        32768,
        -32769,
        2147483647,
        -2147483648,
        2147483648,
        i64::MIN,
        i64::MAX,
    ] {
        let bytes = encode(value);
        let decoded = invoke(
            "source-read",
            "signed",
            vec![
                NormalizedValue::bytes(bytes.as_slice()),
                NormalizedValue::I64(0),
                NormalizedValue::I64(bytes.len() as i64),
            ],
        );
        assert_eq!(field(&decoded, "value"), &NormalizedValue::I64(value));
        assert_eq!(field(&decoded, "code"), &NormalizedValue::I64(0));
        assert_eq!(
            field(&decoded, "next"),
            &NormalizedValue::I64(bytes.len() as i64)
        );
        let mut expected = (bytes.len() as u32).to_le_bytes().to_vec();
        expected.extend_from_slice(&bytes);
        assert_eq!(result("read-signed", &bytes), (0, expected.clone()));
        for end in 0..bytes.len() {
            assert_eq!(result("read-signed", &bytes[..end]), (12, vec![]));
        }
        assert_eq!(
            result("read-signed", &[bytes, vec![254, 255]].concat()),
            (0, expected)
        );
    }
    for value in [
        0_u64,
        1,
        250,
        251,
        65535,
        65536,
        u32::MAX as u64,
        u32::MAX as u64 + 1,
        i64::MAX as u64,
    ] {
        let bytes = encode(value);
        let decoded = invoke(
            "source-read",
            "unsigned",
            vec![
                NormalizedValue::bytes(bytes.as_slice()),
                NormalizedValue::I64(0),
                NormalizedValue::I64(bytes.len() as i64),
            ],
        );
        assert_eq!(
            field(&decoded, "value"),
            &NormalizedValue::I64(value as i64)
        );
        let expected = [(bytes.len() as u32).to_le_bytes().to_vec(), bytes.clone()].concat();
        assert_eq!(result("read-unsigned", &bytes), (0, expected));
        for end in 0..bytes.len() {
            assert_eq!(result("read-unsigned", &bytes[..end]), (12, vec![]));
        }
    }
    for malformed in [
        vec![251, 0, 0],
        vec![251, 250, 0],
        vec![252, 255, 255, 0, 0],
        vec![253, 255, 255, 255, 255, 0, 0, 0, 0],
        vec![254],
        vec![255],
    ] {
        for name in ["read-signed", "read-unsigned"] {
            assert_eq!(result(name, &malformed), (12, vec![]));
        }
    }
    for value in [i64::MAX as u64 + 1, u64::MAX] {
        assert_eq!(result("read-unsigned", &encode(value)), (12, vec![]));
    }
}

fn inspect(op: i64, bytes: &[u8], at: i64, end: i64, a: i64, b: i64, expected: [i64; 4]) {
    let mut packet = [op, at, end, a, b]
        .into_iter()
        .flat_map(encode)
        .collect::<Vec<_>>();
    packet.extend_from_slice(bytes);
    assert_eq!(
        result("inspect-fields", &packet),
        (0, expected.into_iter().flat_map(encode).collect()),
        "op={op}, at={at}, end={end}, a={a}, b={b}, input={bytes:?}"
    );
}

#[test]
fn selfhost_source_fields_reject_hostile_ranges_tags_names_and_counts() {
    for (at, count, end, yes) in [
        (0, 0, 0, 1),
        (0, 3, 3, 1),
        (3, 0, 3, 1),
        (0, 4, 3, 0),
        (1, 3, 3, 0),
        (-1, 0, 3, 0),
        (0, -1, 3, 0),
        (0, 0, -1, 0),
        (0, 0, 4, 0),
        (i64::MAX, i64::MAX, 3, 0),
        (i64::MIN, 1, 3, 0),
        (0, i64::MAX, 3, 0),
        (0, 0, i64::MAX, 0),
        (0, 0, i64::MIN, 0),
    ] {
        inspect(0, b"abc", at, end, count, 0, [0, 0, yes, 0]);
    }
    let id = [vec![9], vec![1; 16]].concat();
    inspect(1, &id, 0, 17, 9, 17, [0, 17, 0, 0]);
    for (bytes, at, end, tag, width) in [
        (id.clone(), 0, 16, 9, 17),
        (id.clone(), 0, 17, 8, 17),
        ([vec![9], vec![0; 16]].concat(), 0, 17, 9, 17),
        (id.clone(), i64::MAX, 17, 9, 17),
        (id.clone(), 0, 17, 9, i64::MAX),
    ] {
        inspect(1, &bytes, at, end, tag, width, [12, -1, 0, 0]);
    }
    let digest = [vec![2], vec![0; 32]].concat();
    inspect(1, &digest, 0, 33, 2, 33, [0, 33, 0, 0]);
    let array = [vec![2], id.clone(), id.clone()].concat();
    inspect(2, &array, 0, 35, 9, 17, [0, 35, 1, 2]);
    inspect(2, &[0], 0, 1, 9, 17, [0, 1, 1, 0]);
    for bytes in [
        array[..34].to_vec(),
        vec![1, 8],
        encode(100001_u64),
        encode(u64::MAX),
    ] {
        inspect(2, &bytes, 0, bytes.len() as i64, 9, 17, [12, 0, 0, 0]);
    }
    inspect(2, &[0], 0, 1, 9, i64::MAX, [12, 0, 0, 0]);
    for value in ["", "abc", "é水🦀"] {
        let bytes = encode(value);
        inspect(
            3,
            &bytes,
            0,
            bytes.len() as i64,
            0,
            128,
            [0, bytes.len() as i64, 1, value.len() as i64],
        );
    }
    for bytes in [
        vec![1, 0xff],
        vec![2, 0xc0, 0xaf],
        vec![3, 0xed, 0xa0, 0x80],
        vec![4, 0xf4, 0x90, 0x80, 0x80],
        vec![2, 0xc2],
        vec![251, 1, 0, b'a'],
    ] {
        inspect(3, &bytes, 0, bytes.len() as i64, 0, 128, [12, 0, 0, 0]);
    }
    for value in ["alpha", "_", "A0-_", &"a".repeat(128)] {
        let bytes = encode(value);
        inspect(
            4,
            &bytes,
            0,
            bytes.len() as i64,
            0,
            0,
            [0, bytes.len() as i64, 1, value.len() as i64],
        );
    }
    for value in ["", "1a", "-a", "a.b", "é", &"a".repeat(129)] {
        let bytes = encode(value);
        inspect(4, &bytes, 0, bytes.len() as i64, 0, 0, [12, 0, 0, 0]);
    }
    let reference = [vec![18], vec![1; 16], id.clone()].concat();
    inspect(5, &reference, 0, 34, 9, 0, [0, 34, 0, 0]);
    inspect(5, &reference, 0, 33, 9, 0, [12, -1, 0, 0]);
    inspect(6, &[42, 255], 0, 2, 42, 0, [0, 1, 0, 0]);
    inspect(6, &[41], 0, 1, 42, 0, [12, -1, 0, 0]);
    inspect(7, &[0], 0, 1, 9, 17, [0, 1, 0, 0]);
    inspect(7, &[vec![1], id].concat(), 0, 18, 9, 17, [0, 18, 0, 0]);
    inspect(7, &[2], 0, 1, 9, 17, [12, -1, 0, 0]);
    for op in [8, 9] {
        for (at, end) in [(i64::MIN, 1), (i64::MAX, 1), (0, i64::MAX), (0, -1)] {
            inspect(op, &[0], at, end, 0, 0, [12, at, 0, 0]);
        }
    }
    inspect(8, &[255, 83, 255], 1, 2, 0, 0, [0, 2, -42, 0]);
    inspect(9, &[255, 42, 255], 1, 2, 0, 0, [0, 2, 42, 0]);
}

fn envelope(payload: &[u8]) -> Vec<u8> {
    object_envelope(b"LKJTYPES", "lkjscript.kernel.type-envelope", payload)
}

fn object_envelope(magic: &[u8; 8], context: &str, payload: &[u8]) -> Vec<u8> {
    let mut bytes = magic.to_vec();
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&(payload.len() as u64).to_le_bytes());
    bytes.extend_from_slice(payload);
    let mut hash = blake3::Hasher::new_derive_key(context);
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(&bytes);
    bytes.extend_from_slice(hash.finalize().as_bytes());
    bytes
}

fn checksum(bytes: &mut Vec<u8>) {
    let mut hash = blake3::Hasher::new_derive_key("lkjscript.kernel.type-envelope");
    hash.update(&(bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
    bytes.extend_from_slice(hash.finalize().as_bytes());
}

#[test]
fn selfhost_source_type_shapes_match_current_codec_and_complete_source_types() {
    let (source, _) = compiler();
    assert!(!source.types.is_empty());
    for (digest, object) in source.types.iter().chain(&source.dependency_types) {
        let (actual, bytes) = kernel::encode_type_object(object).unwrap();
        assert_eq!(*digest, actual);
        assert_eq!(
            result("read-type", &bytes),
            (0, vec![]),
            "{:?}",
            object.form
        );
        assert_eq!(
            result("read-envelope", &bytes),
            (0, bytes[18..bytes.len() - 32].to_vec())
        );
    }
    let digest = [vec![2], vec![0; 32]].concat();
    let reference = [vec![18], vec![1; 16], vec![3], vec![2; 16]].concat();
    let mut payloads = [0, 1, 2, 3, 4, 5, 20].map(|form| vec![1, form]).to_vec();
    payloads.extend([
        [vec![1, 7, 17], vec![1; 16]].concat(),
        [vec![1, 8], reference.clone()].concat(),
        [vec![1, 11], digest.clone()].concat(),
        [vec![1, 16, 0], digest.clone()].concat(),
        [vec![1, 16, 2], digest.repeat(3)].concat(),
        [vec![1, 17], reference, vec![1], digest.clone()].concat(),
        [
            vec![1, 10, 2],
            encode("a"),
            digest.clone(),
            encode("b"),
            digest,
        ]
        .concat(),
    ]);
    for payload in payloads {
        let bytes = envelope(&payload);
        kernel::decode_type_object(&bytes, TypeObjectDigest::of(&bytes)).unwrap();
        assert_eq!(result("read-type", &bytes), (0, vec![]), "{payload:?}");
    }
    eprintln!(
        "selfhost source type decoding types={} dependency_types={}",
        source.types.len(),
        source.dependency_types.len()
    );
}

#[test]
fn selfhost_source_type_shapes_reject_rehashed_malformed_payloads_and_foreign_envelopes() {
    let valid = envelope(&[1, 2]);
    for end in 0..valid.len() {
        assert_eq!(result("read-type", &valid[..end]), (12, vec![]));
    }
    for at in [0, 8, 9, 10, 14, valid.len() - 1] {
        let mut changed = valid.clone();
        changed[at] ^= 1;
        if at < 18 {
            changed.truncate(changed.len() - 32);
            checksum(&mut changed);
        }
        assert_eq!(result("read-type", &changed), (12, vec![]));
    }
    assert_eq!(
        result("read-type", &[valid, vec![0]].concat()),
        (12, vec![])
    );
    let digest = [vec![2], vec![0; 32]].concat();
    let reference = [vec![18], vec![1; 16], vec![3], vec![2; 16]].concat();
    let mut malformed = vec![
        vec![],
        vec![1],
        vec![2, 2],
        vec![1, 2, 0],
        vec![1, 35],
        vec![251, 1, 0, 2],
        vec![1, 251, 2, 0],
        vec![1, 10, 0],
        vec![1, 16, 251, 0, 0],
    ];
    malformed.extend([
        [vec![1, 7, 17], vec![0; 16]].concat(),
        [vec![1, 7, 9], vec![1; 16]].concat(),
        [vec![1, 11, 1], vec![0; 32]].concat(),
        [vec![1, 17], reference, vec![0]].concat(),
        [
            vec![1, 10, 2],
            encode("b"),
            digest.clone(),
            encode("a"),
            digest.clone(),
        ]
        .concat(),
        [
            vec![1, 10, 2],
            encode("a"),
            digest.clone(),
            encode("a"),
            digest.clone(),
        ]
        .concat(),
        [vec![1, 10, 1], encode("1x"), digest].concat(),
    ]);
    for payload in malformed {
        let bytes = envelope(&payload);
        assert!(
            kernel::decode_type_object(&bytes, TypeObjectDigest::of(&bytes)).is_err(),
            "{payload:?}"
        );
        assert_eq!(result("read-type", &bytes), (12, vec![]), "{payload:?}");
    }
    // A known out-of-profile form is rejected explicitly before any output.
    // This is not a semantic admission claim for that unsupported form.
    for form in [
        6, 9, 12, 13, 14, 15, 18, 19, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34,
    ] {
        assert_eq!(result("read-type", &envelope(&[1, form])), (13, vec![]));
    }
}

fn batch(source: &KernelSnapshot) {
    let mut packet = vec![];
    let mut records = vec![];
    for object in source
        .types
        .values()
        .chain(source.dependency_types.values())
    {
        records.push((0, kernel::encode_type_object(object).unwrap().1));
    }
    for owner in source.owners.values() {
        records.push((1, kernel::encode_owner(owner).unwrap().1));
    }
    for (kind, bytes) in &records {
        packet.push(*kind);
        packet.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        packet.extend_from_slice(bytes);
    }
    let (code, statuses) = result("read-records", &packet);
    assert_eq!(code, 0);
    assert_eq!(statuses.len(), records.len());
    if let Some((index, code)) = statuses.iter().enumerate().find(|(_, code)| **code != 0) {
        let (kind, bytes) = &records[index];
        panic!(
            "source record {index} kind={kind} code={code} payload={:?}",
            &bytes[18..(bytes.len() - 32).min(218)]
        );
    }
    eprintln!(
        "selfhost source record shapes types={} dependency_types={} owners={} packet_bytes={} packet_sha256={}",
        source.types.len(),
        source.dependency_types.len(),
        source.owners.len(),
        packet.len(),
        crate::platform::semantic_id::encode_hex(&Sha256::digest(&packet))
    );
}

#[test]
fn selfhost_source_decodes_all_authoritative_records_and_independent_programs() {
    batch(&compiler().0);
    for proposal in [
        include_str!("../../../../tests/fixtures/selfhost-aggregates.lkjc"),
        include_str!("../../../../tests/fixtures/selfhost-generic-list.lkjc"),
        include_str!("../../../../tests/fixtures/selfhost-source-nominal.lkjc"),
    ] {
        batch(&byte_buffer_tests::author_only(proposal).unwrap());
    }
    for malformed in [
        vec![0],
        vec![0, 0, 0, 0],
        vec![2, 0, 0, 0, 0],
        vec![1, 255, 255, 255, 255],
        vec![0, 1, 0, 0, 0],
    ] {
        assert_eq!(result("read-records", &malformed), (12, vec![]));
    }
}

fn reject_owner(owner: &OwnerRecord, payload: &[u8]) {
    let bytes = object_envelope(b"LKJOWNER", "lkjscript.kernel.owner-envelope", payload);
    assert!(
        kernel::decode_owner(
            &bytes,
            owner.owner(),
            owner.kind(),
            kernel::OwnerObjectDigest::of(&bytes)
        )
        .is_err(),
        "{payload:?}"
    );
    assert_eq!(result("read-owner", &bytes), (12, vec![]), "{payload:?}");
}

#[test]
fn selfhost_source_owners_reject_mismatched_headers_and_noncanonical_expressions() {
    let (source, _) = compiler();
    let mut kinds = BTreeSet::new();
    for owner in source.owners.values() {
        if !kinds.insert(owner.kind()) {
            continue;
        }
        let bytes = kernel::encode_owner(owner).unwrap().1;
        let payload = bytes[18..bytes.len() - 32].to_vec();
        assert_eq!(result("read-owner", &bytes), (0, vec![]));
        let mut changed = payload.clone();
        changed[1] = 2;
        reject_owner(owner, &changed);
        let mut changed = payload.clone();
        if matches!(owner, OwnerRecord::Expression(_)) {
            changed[2] = 0;
            reject_owner(owner, &changed);
            changed = payload.clone();
            changed[3..19].fill(0);
        } else {
            changed[2] = 255;
            reject_owner(owner, &changed);
            changed = payload.clone();
            changed[3] = 0;
            reject_owner(owner, &changed);
            changed = payload.clone();
            changed[4..20].fill(0);
        }
        reject_owner(owner, &changed);
        let mut changed = payload.clone();
        changed.push(0);
        reject_owner(owner, &changed);
    }
    let owner = source
        .owners
        .values()
        .find(|owner| matches!(owner, OwnerRecord::Expression(_)))
        .unwrap();
    let original = kernel::encode_owner(owner).unwrap().1;
    let head = original[18..37].to_vec();
    for operation in [
        vec![1, 2],
        vec![2, 251, 0, 0],
        vec![251, 0, 0],
        vec![52],
        vec![255],
        vec![3, 0, 1, 255],
        vec![3, 2],
        vec![5, 5],
        vec![9, 0],
        vec![13, 0, 0, 0],
    ] {
        reject_owner(owner, &[head.clone(), operation].concat());
    }
    for operation in [17, 18, 19, 21, 23, 24, 38, 51] {
        let bytes = object_envelope(
            b"LKJOWNER",
            "lkjscript.kernel.owner-envelope",
            &[head.clone(), vec![operation]].concat(),
        );
        assert_eq!(result("read-owner", &bytes), (13, vec![]));
    }
    let unit = object_envelope(
        b"LKJOWNER",
        "lkjscript.kernel.owner-envelope",
        &[head, vec![0]].concat(),
    );
    for end in 0..unit.len() {
        assert_eq!(result("read-owner", &unit[..end]), (12, vec![]));
    }
    let mut changed = unit;
    changed[8] = 2;
    assert_eq!(result("read-owner", &changed), (12, vec![]));
}

#[path = "selfhost_source_binding_tests.rs"]
mod binding;

#[path = "selfhost_source_graph_tests.rs"]
mod graph;
