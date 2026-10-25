//! Logical source commitments, canonical history, and exact object bindings.
//! Matching digests do not establish graph semantics or execution permission.
use super::*;

fn map_packet(key_bytes: u32, value_bytes: u32, rows: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&key_bytes.to_le_bytes());
    bytes.extend_from_slice(&value_bytes.to_le_bytes());
    for (key, value) in rows {
        bytes.extend_from_slice(key);
        bytes.extend_from_slice(value);
    }
    bytes
}

fn check_map(key_bytes: u32, value_bytes: u32, rows: Vec<(Vec<u8>, Vec<u8>)>) {
    let packet = map_packet(key_bytes, value_bytes, &rows);
    let expected = crate::platform::persistent_map::MapContentRoot::from_sorted(rows).unwrap();
    let (code, digest) = result("read-map", &packet);
    assert_eq!(code, 0);
    assert_eq!(digest, expected.digest().bytes());
}

fn map_rows(key_bytes: usize, value_bytes: usize, count: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
    (0..count)
        .map(|index| {
            let mut key = vec![0; key_bytes];
            let suffix = key_bytes.min(8);
            key[key_bytes - suffix..].copy_from_slice(&(index as u64).to_be_bytes()[8 - suffix..]);
            (key, vec![((index * 37 + 11) % 256) as u8; value_bytes])
        })
        .collect()
}

#[test]
fn selfhost_source_maps_match_independent_logical_commitments_and_current_state() {
    for (key_bytes, value_bytes) in [(1, 1), (16, 32), (17, 32), (17, 33), (17, 65), (32, 128)] {
        for count in [0, 1, 2, 255, 256, 257, 1024] {
            if key_bytes == 1 && count > 256 {
                continue;
            }
            check_map(
                key_bytes as u32,
                value_bytes as u32,
                map_rows(key_bytes, value_bytes, count),
            );
        }
    }
    // Cross both historical logical-map quotas with independent canonical roots.
    check_map(3, 1, map_rows(3, 1, 100_001));
    check_map(16, 112, map_rows(16, 112, 65_537));
    let (source, _) = compiler();
    let owners = source
        .owners
        .iter()
        .map(|(id, record)| {
            let (object, _) = kernel::encode_owner(record).unwrap();
            (
                kernel::owner_map_key(*id).to_vec(),
                kernel::encode_owner_binding(&kernel::OwnerBinding {
                    kind: record.kind(),
                    object,
                }),
            )
        })
        .collect::<Vec<_>>();
    let dependencies = source
        .dependencies
        .iter()
        .map(|(id, record)| {
            let (object, _) = kernel::encode_dependency(record).unwrap();
            (
                kernel::dependency_map_key(*id).to_vec(),
                kernel::encode_dependency_binding(&kernel::DependencyBinding { object }),
            )
        })
        .collect::<Vec<_>>();
    let retirements = source
        .retirements
        .iter()
        .map(|(id, record)| {
            let (object, _) = kernel::encode_retirement(record).unwrap();
            (
                kernel::retirement_map_key(*id).to_vec(),
                kernel::encode_retirement_binding(&kernel::RetirementBinding { object }),
            )
        })
        .collect::<Vec<_>>();
    for (name, key_bytes, value_bytes, rows, expected) in [
        ("owners", 17, 33, owners, source.root.owners),
        (
            "dependencies",
            16,
            32,
            dependencies,
            source.root.dependencies,
        ),
        ("retirements", 17, 32, retirements, source.root.retirements),
    ] {
        let logical =
            crate::platform::persistent_map::MapContentRoot::from_sorted(rows.clone()).unwrap();
        assert_eq!(logical, expected.content_root(), "{name}");
        let packet = map_packet(key_bytes, value_bytes, &rows);
        let (code, digest) = result("read-map", &packet);
        assert_eq!(code, 0, "{name}");
        assert_eq!(digest, logical.digest().bytes(), "{name}");
        eprintln!(
            "selfhost source map kind={name} entries={} input_bytes={} input_sha256={} root={}",
            rows.len(),
            packet.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&packet)),
            logical.digest()
        );
    }
}

#[test]
fn selfhost_source_maps_reject_invalid_widths_extents_duplicates_and_order() {
    for length in 0..8 {
        assert_eq!(result("read-map", &vec![0; length]), (12, vec![]));
    }
    for (key_bytes, value_bytes) in [
        (0, 32),
        (33, 32),
        (16, 0),
        (16, 129),
        (u32::MAX, 32),
        (16, u32::MAX),
    ] {
        assert_eq!(
            result("read-map", &map_packet(key_bytes, value_bytes, &[])),
            (12, vec![])
        );
    }
    let valid = map_packet(17, 33, &map_rows(17, 33, 2));
    for length in [9, 57, 59, valid.len() - 1] {
        assert_eq!(result("read-map", &valid[..length]), (12, vec![]));
    }
    let mut duplicated = map_rows(17, 33, 2);
    duplicated[1].0 = duplicated[0].0.clone();
    assert_eq!(
        result("read-map", &map_packet(17, 33, &duplicated)),
        (12, vec![])
    );
    let mut reversed = map_rows(17, 33, 2);
    reversed.reverse();
    assert_eq!(
        result("read-map", &map_packet(17, 33, &reversed)),
        (12, vec![])
    );
    let mut duplicate_rows = map_packet(1, 1, &[]);
    duplicate_rows.resize(8 + 2 * 100_001, 0);
    assert_eq!(result("read-map", &duplicate_rows), (12, vec![]));
    let mut unaligned_bytes = map_packet(16, 112, &[]);
    unaligned_bytes.resize(8 + 8_388_609, 0);
    assert_eq!(result("read-map", &unaligned_bytes), (12, vec![]));
    for (key_bytes, value_bytes) in [
        (i64::MIN, 32),
        (-1, 32),
        (i64::MAX, 32),
        (16, i64::MIN),
        (16, -1),
        (16, i64::MAX),
    ] {
        let observed = invoke(
            "source-map",
            "root",
            vec![
                NormalizedValue::bytes(&[][..]),
                NormalizedValue::I64(key_bytes),
                NormalizedValue::I64(value_bytes),
            ],
        );
        assert_eq!(field(&observed, "diagnostic"), &NormalizedValue::I64(12));
        assert_eq!(field(&observed, "image"), &NormalizedValue::bytes(&[][..]));
    }
}

// not source semantic admission or authority granted by a matching digest.
fn state_packet(source: &KernelSnapshot) -> Vec<u8> {
    let mut owners = Vec::new();
    for (id, record) in &source.owners {
        let (object, _) = kernel::encode_owner(record).unwrap();
        owners.extend_from_slice(&kernel::owner_map_key(*id));
        owners.extend_from_slice(&kernel::encode_owner_binding(&kernel::OwnerBinding {
            kind: record.kind(),
            object,
        }));
    }
    let mut dependencies = Vec::new();
    for (id, record) in &source.dependencies {
        let (object, _) = kernel::encode_dependency(record).unwrap();
        dependencies.extend_from_slice(&kernel::dependency_map_key(*id));
        dependencies.extend_from_slice(&kernel::encode_dependency_binding(
            &kernel::DependencyBinding { object },
        ));
    }
    let mut retirements = Vec::new();
    for (id, record) in &source.retirements {
        let (object, _) = kernel::encode_retirement(record).unwrap();
        retirements.extend_from_slice(&kernel::retirement_map_key(*id));
        retirements.extend_from_slice(&kernel::encode_retirement_binding(
            &kernel::RetirementBinding { object },
        ));
    }
    let name = source.root.package_name.as_str().as_bytes();
    let parts = [
        name,
        owners.as_slice(),
        dependencies.as_slice(),
        retirements.as_slice(),
    ];
    let mut packet = source.root.package_id.bytes().to_vec();
    for part in &parts {
        packet.extend_from_slice(&(part.len() as u32).to_le_bytes());
    }
    for part in parts {
        packet.extend_from_slice(part);
    }
    packet
}

#[test]
fn selfhost_source_state_binds_complete_logical_maps_and_package_identity() {
    let independent = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-nominal.lkjc"
    ))
    .unwrap();
    for source in [&compiler().0, &independent] {
        let expected = kernel::semantic_state_digest(source).unwrap();
        assert_eq!(
            expected,
            kernel::semantic_state_digest_from_root(&source.root).unwrap()
        );
        let packet = state_packet(source);
        let (code, digest) = result("read-state", &packet);
        assert_eq!(code, 0);
        assert_eq!(digest, expected.bytes());
        eprintln!(
            "selfhost logical source state owners={} dependencies={} retirements={} packet_bytes={} packet_sha256={} semantic_state={expected}",
            source.owners.len(),
            source.dependencies.len(),
            source.retirements.len(),
            packet.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&packet))
        );
    }
    let mut renamed = independent.clone();
    renamed.root.package_name = Name::new("different_package_name").unwrap();
    let expected = kernel::semantic_state_digest(&renamed).unwrap();
    assert_ne!(
        expected,
        kernel::semantic_state_digest(&independent).unwrap()
    );
    assert_eq!(
        result("read-state", &state_packet(&renamed)),
        (0, expected.bytes().to_vec())
    );
    // Repository identity belongs to revision provenance, not logical semantic state.
    renamed.root.repository_id =
        crate::platform::semantic_id::RepositoryId::migrate(b"independent-state-repository", 0);
    assert_eq!(kernel::semantic_state_digest(&renamed).unwrap(), expected);
}

#[test]
fn selfhost_source_state_rejects_invalid_package_names_maps_and_framing() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/selfhost-source-nominal.lkjc"
    ))
    .unwrap();
    let valid = state_packet(&source);
    for length in [0, 15, 31, valid.len() - 1] {
        assert_eq!(result("read-state", &valid[..length]), (12, vec![]));
    }
    let mut changed = valid.clone();
    changed[..16].fill(0);
    assert_eq!(result("read-state", &changed), (12, vec![]));
    changed = valid.clone();
    changed.push(0);
    assert_eq!(result("read-state", &changed), (12, vec![]));
    for offset in [16, 20, 24, 28] {
        changed = valid.clone();
        changed[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(result("read-state", &changed), (12, vec![]));
    }
    let name_len = u32::from_le_bytes(valid[16..20].try_into().unwrap()) as usize;
    for bad_name in [
        b"".as_slice(),
        b"9bad",
        b"has.dot",
        b"\xc3\xa9",
        &[b'a'; 129],
    ] {
        changed = valid[..32].to_vec();
        changed[16..20].copy_from_slice(&(bad_name.len() as u32).to_le_bytes());
        changed.extend_from_slice(bad_name);
        changed.extend_from_slice(&valid[32 + name_len..]);
        assert_eq!(result("read-state", &changed), (12, vec![]));
    }
    let owner_start = 32 + name_len;
    changed = valid.clone();
    let duplicate = changed[owner_start..owner_start + 17].to_vec();
    changed[owner_start + 50..owner_start + 67].copy_from_slice(&duplicate);
    assert_eq!(result("read-state", &changed), (12, vec![]));
}

#[test]
fn selfhost_source_revision_identity_matches_independent_current_core() {
    use crate::platform::publication::RevisionCore;
    use crate::platform::semantic_id::{RepositoryId, RevisionId};
    let source = &compiler().0;
    let mut core = RevisionCore {
        contract_version: 1,
        graph_contract_version: 1,
        repository_id: source.root.repository_id,
        parents: vec![],
        semantic_state: kernel::semantic_state_digest(source).unwrap(),
    };
    let mut previous = None;
    for parents in [
        vec![],
        vec![RevisionId::from_digest([0; 32])],
        vec![
            RevisionId::from_digest([0; 32]),
            RevisionId::from_digest([255; 32]),
        ],
    ] {
        core.parents = parents;
        let expected = core.revision_id().unwrap();
        let bytes = encode(&core);
        assert_eq!(
            result("read-revision", &bytes),
            (0, expected.bytes().to_vec())
        );
        assert_ne!(previous, Some(expected));
        previous = Some(expected);
    }
    let valid = encode(&core);
    for length in 0..valid.len() {
        assert_eq!(result("read-revision", &valid[..length]), (12, vec![]));
    }
    let mut changed = valid.clone();
    changed.push(0);
    assert_eq!(result("read-revision", &changed), (12, vec![]));
    for (offset, value) in [
        (0, 2),
        (1, 2),
        (2, 2),
        (19, 3),
        (20, 254),
        (53, 254),
        (86, 9),
    ] {
        changed = valid.clone();
        changed[offset] = value;
        assert_eq!(result("read-revision", &changed), (12, vec![]));
    }
    changed = valid.clone();
    changed[3..19].fill(0);
    assert_eq!(result("read-revision", &changed), (12, vec![]));
    changed = valid.clone();
    changed.splice(0..1, [251, 1, 0]);
    assert_eq!(result("read-revision", &changed), (12, vec![]));
    core.parents.reverse();
    assert!(core.revision_id().is_err());
    assert_eq!(result("read-revision", &encode(&core)), (12, vec![]));
    core.parents[0] = core.parents[1];
    assert!(core.revision_id().is_err());
    assert_eq!(result("read-revision", &encode(&core)), (12, vec![]));
    core.parents.clear();
    let initial = core.revision_id().unwrap();
    core.repository_id = RepositoryId::migrate(b"source-revision-other-repository", 0);
    let expected = core.revision_id().unwrap();
    assert_ne!(initial, expected);
    assert_eq!(
        result("read-revision", &encode(&core)),
        (0, expected.bytes().to_vec())
    );
    core.semantic_state = kernel::SemanticStateDigest::from_bytes([91; 32]);
    let changed_state = core.revision_id().unwrap();
    assert_ne!(expected, changed_state);
    assert_eq!(
        result("read-revision", &encode(&core)),
        (0, changed_state.bytes().to_vec())
    );
}

#[test]
fn selfhost_source_history_records_reject_rehashed_noncanonical_fields() {
    use crate::platform::semantic_id::RevisionId;
    let source = &compiler().0;
    let dependency = kernel::DependencyRecord {
        graph_contract_version: 1,
        package: source.root.package_id,
        semantic_revision: RevisionId::from_digest([31; 32]),
        package_revision: kernel::PackageRevisionDigest::from_bytes([47; 32]),
    };
    let (_, dependency_bytes) = kernel::encode_dependency(&dependency).unwrap();
    assert_eq!(result("read-dependency", &dependency_bytes), (0, vec![]));
    let retirement = source.retirements.values().next().unwrap().clone();
    for last_name in [None, Some(Name::new("retired_name").unwrap())] {
        for last_parent in [None, Some(retirement.owner)] {
            let mut record = retirement.clone();
            record.last_name = last_name.clone();
            record.last_parent = last_parent;
            let (_, bytes) = kernel::encode_retirement(&record).unwrap();
            assert_eq!(result("read-retirement", &bytes), (0, vec![]));
        }
    }
    let mut record = retirement.clone();
    record.last_name = None;
    record.last_parent = None;
    let (_, retirement_bytes) = kernel::encode_retirement(&record).unwrap();
    for (name, magic, context, bytes, edits) in [
        (
            "read-dependency",
            b"LKJDEP14",
            "lkjscript.kernel.dependency-envelope",
            dependency_bytes,
            vec![(0, 2), (1, 17), (18, 254), (51, 10)],
        ),
        (
            "read-retirement",
            b"LKJRET14",
            "lkjscript.kernel.retirement-envelope",
            retirement_bytes,
            vec![
                (0, 2),
                (1, 17),
                (2, 255),
                (19, 29),
                (20, 2),
                (21, 2),
                (22, 254),
                (55, 8),
            ],
        ),
    ] {
        for length in [0, 17, bytes.len() - 1] {
            assert_eq!(result(name, &bytes[..length]), (12, vec![]));
        }
        let payload = &bytes[18..bytes.len() - 32];
        for (offset, value) in edits {
            let mut changed = payload.to_vec();
            changed[offset] = value;
            let bytes = object_envelope(magic, context, &changed);
            if name == "read-dependency" {
                assert!(
                    kernel::decode_dependency(
                        &bytes,
                        &dependency.package,
                        kernel::DependencyObjectDigest::of(&bytes)
                    )
                    .is_err()
                );
            } else {
                assert!(
                    kernel::decode_retirement(
                        &bytes,
                        record.owner,
                        kernel::RetirementObjectDigest::of(&bytes)
                    )
                    .is_err()
                );
            }
            assert_eq!(result(name, &bytes), (12, vec![]));
        }
        let mut changed = payload.to_vec();
        changed.push(0);
        assert_eq!(
            result(name, &object_envelope(magic, context, &changed)),
            (12, vec![])
        );
        let mut changed = bytes;
        changed[8] = 2;
        assert_eq!(result(name, &changed), (12, vec![]));
    }
}

fn bound_objects(source: &KernelSnapshot) -> Vec<(u8, Vec<u8>, Vec<u8>)> {
    let mut objects = vec![];
    for object in source
        .types
        .values()
        .chain(source.dependency_types.values())
    {
        let (digest, bytes) = kernel::encode_type_object(object).unwrap();
        objects.push((0, digest.bytes().to_vec(), bytes));
    }
    for (key, record) in &source.owners {
        let (digest, bytes) = kernel::encode_owner(record).unwrap();
        let mut entry = kernel::owner_map_key(*key).to_vec();
        entry.extend_from_slice(&kernel::encode_owner_binding(&kernel::OwnerBinding {
            kind: record.kind(),
            object: digest,
        }));
        objects.push((1, entry, bytes));
    }
    for (key, record) in &source.dependencies {
        let (digest, bytes) = kernel::encode_dependency(record).unwrap();
        let mut entry = kernel::dependency_map_key(*key).to_vec();
        entry.extend_from_slice(&digest.bytes());
        objects.push((2, entry, bytes));
    }
    for (key, record) in &source.retirements {
        let (digest, bytes) = kernel::encode_retirement(record).unwrap();
        let mut entry = kernel::retirement_map_key(*key).to_vec();
        entry.extend_from_slice(&digest.bytes());
        objects.push((3, entry, bytes));
    }
    objects
}

#[test]
fn selfhost_source_objects_bind_every_record_to_logical_directory_identity() {
    for source in [
        &compiler().0,
        &byte_buffer_tests::author_only(include_str!(
            "../../../../tests/fixtures/selfhost-source-nominal.lkjc"
        ))
        .unwrap(),
    ] {
        let objects = bound_objects(source);
        let mut packet = vec![];
        for (kind, entry, bytes) in &objects {
            packet.push(*kind);
            packet.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            packet.extend_from_slice(entry);
            packet.extend_from_slice(bytes);
        }
        assert_eq!(
            result("read-bindings", &packet),
            (0, vec![0; objects.len()])
        );
        eprintln!(
            "selfhost bound source objects={} owners={} retirements={} input_bytes={} input_sha256={}",
            objects.len(),
            source.owners.len(),
            source.retirements.len(),
            packet.len(),
            crate::platform::semantic_id::encode_hex(&Sha256::digest(&packet))
        );
    }
    let source = &compiler().0;
    let dependency = kernel::DependencyRecord {
        graph_contract_version: 1,
        package: source.root.package_id,
        semantic_revision: crate::platform::semantic_id::RevisionId::from_digest([17; 32]),
        package_revision: kernel::PackageRevisionDigest::from_bytes([31; 32]),
    };
    let (digest, bytes) = kernel::encode_dependency(&dependency).unwrap();
    let mut packet = dependency.package.bytes().to_vec();
    packet.extend_from_slice(&digest.bytes());
    packet.extend_from_slice(&bytes);
    assert_eq!(result("bind-dependency", &packet), (0, vec![]));
    for offset in [0, 16, 47, packet.len() - 1] {
        let mut changed = packet.clone();
        changed[offset] ^= 1;
        assert_eq!(result("bind-dependency", &changed), (12, vec![]));
    }
}

#[test]
fn selfhost_source_object_bindings_reject_wrong_keys_kinds_hashes_and_lengths() {
    let source = &compiler().0;
    let mut selected = BTreeSet::new();
    for (kind, entry, bytes) in bound_objects(source) {
        let discriminator = if kind == 1 { entry[17] } else { 0 };
        if !selected.insert((kind, discriminator)) {
            continue;
        }
        let name = [
            "bind-type",
            "bind-owner",
            "bind-dependency",
            "bind-retirement",
        ][kind as usize];
        let packet = [entry.clone(), bytes.clone()].concat();
        assert_eq!(result(name, &packet), (0, vec![]));
        for length in [0, entry.len() - 1, entry.len(), packet.len() - 1] {
            assert_eq!(result(name, &packet[..length]), (12, vec![]));
        }
        for offset in [0, entry.len() - 1, packet.len() - 1] {
            let mut changed = packet.clone();
            changed[offset] ^= 1;
            assert_eq!(result(name, &changed), (12, vec![]));
        }
        if kind == 1 {
            for offset in [1, 17] {
                let mut changed = packet.clone();
                changed[offset] ^= 1;
                assert_eq!(result(name, &changed), (12, vec![]));
            }
        }
    }
    for packet in [
        vec![4, 0, 0, 0, 0],
        vec![0, 255, 255, 255, 255],
        vec![0],
        vec![0, 0, 0, 0, 0],
    ] {
        assert_eq!(result("read-bindings", &packet), (12, vec![]));
    }
}
