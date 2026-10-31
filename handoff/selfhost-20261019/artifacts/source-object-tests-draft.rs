fn bound_objects(source: &KernelSnapshot) -> Vec<(u8, Vec<u8>, Vec<u8>)> {
    let mut objects = vec![];
    for object in source.types.values().chain(source.dependency_types.values()) {
        let (digest, bytes) = kernel::encode_type_object(object).unwrap();
        objects.push((0, digest.bytes().to_vec(), bytes));
    }
    for (key, record) in &source.owners {
        let (digest, bytes) = kernel::encode_owner(record).unwrap();
        let mut entry = kernel::owner_map_key(*key).to_vec();
        entry.extend_from_slice(&kernel::encode_owner_binding(&kernel::OwnerBinding { kind: record.kind(), object: digest }));
        objects.push((1, entry, bytes));
    }
    for (key, record) in &source.dependencies {
        let (digest, bytes) = kernel::encode_dependency(record).unwrap();
        let mut entry = kernel::dependency_map_key(*key).to_vec(); entry.extend_from_slice(&digest.bytes());
        objects.push((2, entry, bytes));
    }
    for (key, record) in &source.retirements {
        let (digest, bytes) = kernel::encode_retirement(record).unwrap();
        let mut entry = kernel::retirement_map_key(*key).to_vec(); entry.extend_from_slice(&digest.bytes());
        objects.push((3, entry, bytes));
    }
    objects
}

#[test]
fn selfhost_source_objects_bind_every_record_to_logical_directory_identity() {
    for source in [&compiler().0, &byte_buffer_tests::author_only(include_str!("../../../../tests/fixtures/selfhost-source-nominal.lkjc")).unwrap()] {
        let objects = bound_objects(source);
        let mut packet = vec![];
        for (kind, entry, bytes) in &objects {
            packet.push(*kind); packet.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            packet.extend_from_slice(entry); packet.extend_from_slice(bytes);
        }
        assert_eq!(result("read-bindings", &packet), (0, vec![0; objects.len()]));
        eprintln!("selfhost bound source objects={} owners={} retirements={} input_bytes={} input_sha256={}", objects.len(), source.owners.len(), source.retirements.len(), packet.len(), crate::platform::semantic_id::encode_hex(&Sha256::digest(&packet)));
    }
    let source = &compiler().0;
    let dependency = kernel::DependencyRecord { graph_contract_version: 1, package: source.root.package_id, semantic_revision: crate::platform::semantic_id::RevisionId::from_digest([17;32]), package_revision: kernel::PackageRevisionDigest::from_bytes([31;32]) };
    let (digest, bytes) = kernel::encode_dependency(&dependency).unwrap();
    let mut packet = dependency.package.bytes().to_vec(); packet.extend_from_slice(&digest.bytes()); packet.extend_from_slice(&bytes);
    assert_eq!(result("bind-dependency", &packet), (0, vec![]));
    for offset in [0, 16, 47, packet.len()-1] {
        let mut changed = packet.clone(); changed[offset] ^= 1;
        assert_eq!(result("bind-dependency", &changed), (12, vec![]));
    }
}

#[test]
fn selfhost_source_object_bindings_reject_wrong_keys_kinds_hashes_and_lengths() {
    let source = &compiler().0;
    let mut selected = BTreeSet::new();
    for (kind, entry, bytes) in bound_objects(source) {
        let discriminator = if kind == 1 { entry[17] } else { 0 };
        if !selected.insert((kind, discriminator)) { continue; }
        let name = ["bind-type", "bind-owner", "bind-dependency", "bind-retirement"][kind as usize];
        let packet = [entry.clone(), bytes.clone()].concat();
        assert_eq!(result(name, &packet), (0, vec![]));
        for length in [0, entry.len()-1, entry.len(), packet.len()-1] {
            assert_eq!(result(name, &packet[..length]), (12, vec![]));
        }
        for offset in [0, entry.len()-1, packet.len()-1] {
            let mut changed = packet.clone(); changed[offset] ^= 1;
            assert_eq!(result(name, &changed), (12, vec![]));
        }
        if kind == 1 {
            for offset in [1, 17] {
                let mut changed = packet.clone(); changed[offset] ^= 1;
                assert_eq!(result(name, &changed), (12, vec![]));
            }
        }
    }
    for packet in [vec![4,0,0,0,0], vec![0,255,255,255,255], vec![0], vec![0,0,0,0,0]] {
        assert_eq!(result("read-bindings", &packet), (12, vec![]));
    }
}
