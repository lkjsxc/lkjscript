// Proposed additions after current decoder acceptance; this checks commitments,
// not source semantic admission or authority granted by a matching digest.
fn state_packet(source: &KernelSnapshot) -> Vec<u8> {
    let mut owners = Vec::new();
    for (id, record) in &source.owners {
        let (object, _) = kernel::encode_owner(record).unwrap();
        owners.extend_from_slice(&kernel::owner_map_key(*id));
        owners.extend_from_slice(&kernel::encode_owner_binding(&kernel::OwnerBinding { kind: record.kind(), object }));
    }
    let mut dependencies = Vec::new();
    for (id, record) in &source.dependencies {
        let (object, _) = kernel::encode_dependency(record).unwrap();
        dependencies.extend_from_slice(&kernel::dependency_map_key(*id));
        dependencies.extend_from_slice(&kernel::encode_dependency_binding(&kernel::DependencyBinding { object }));
    }
    let mut retirements = Vec::new();
    for (id, record) in &source.retirements {
        let (object, _) = kernel::encode_retirement(record).unwrap();
        retirements.extend_from_slice(&kernel::retirement_map_key(*id));
        retirements.extend_from_slice(&kernel::encode_retirement_binding(&kernel::RetirementBinding { object }));
    }
    let name = source.root.package_name.as_str().as_bytes();
    let parts = [name, owners.as_slice(), dependencies.as_slice(), retirements.as_slice()];
    let mut packet = source.root.package_id.bytes().to_vec();
    for part in &parts { packet.extend_from_slice(&(part.len() as u32).to_le_bytes()); }
    for part in parts { packet.extend_from_slice(part); }
    packet
}

#[test]
fn selfhost_source_state_binds_complete_logical_maps_and_package_identity() {
    let independent = byte_buffer_tests::author_only(include_str!("../../../../tests/fixtures/selfhost-source-nominal.lkjc")).unwrap();
    for source in [&compiler().0, &independent] {
        let expected = kernel::semantic_state_digest(source).unwrap();
        assert_eq!(expected, kernel::semantic_state_digest_from_root(&source.root).unwrap());
        let packet = state_packet(source);
        let (code, digest) = result("read-state", &packet);
        assert_eq!(code, 0);
        assert_eq!(digest, expected.bytes());
        eprintln!("selfhost logical source state owners={} dependencies={} retirements={} packet_bytes={} packet_sha256={} semantic_state={expected}", source.owners.len(), source.dependencies.len(), source.retirements.len(), packet.len(), crate::platform::semantic_id::encode_hex(&Sha256::digest(&packet)));
    }
    let mut renamed = independent.clone();
    renamed.root.package_name = Name::new("different_package_name").unwrap();
    let expected = kernel::semantic_state_digest(&renamed).unwrap();
    assert_ne!(expected, kernel::semantic_state_digest(&independent).unwrap());
    assert_eq!(result("read-state", &state_packet(&renamed)), (0, expected.bytes().to_vec()));
    // Repository identity belongs to revision provenance, not logical semantic state.
    renamed.root.repository_id = crate::platform::semantic_id::RepositoryId::migrate(b"independent-state-repository", 0);
    assert_eq!(kernel::semantic_state_digest(&renamed).unwrap(), expected);
}

#[test]
fn selfhost_source_state_rejects_invalid_package_names_maps_and_framing() {
    let source = byte_buffer_tests::author_only(include_str!("../../../../tests/fixtures/selfhost-source-nominal.lkjc")).unwrap();
    let valid = state_packet(&source);
    for length in [0, 15, 31, valid.len() - 1] {
        assert_eq!(result("read-state", &valid[..length]), (12, vec![]));
    }
    let mut changed = valid.clone();
    changed[..16].fill(0);
    assert_eq!(result("read-state", &changed), (12, vec![]));
    changed = valid.clone(); changed.push(0);
    assert_eq!(result("read-state", &changed), (12, vec![]));
    for offset in [16, 20, 24, 28] {
        changed = valid.clone(); changed[offset..offset+4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(result("read-state", &changed), (12, vec![]));
    }
    let name_len = u32::from_le_bytes(valid[16..20].try_into().unwrap()) as usize;
    for bad_name in [b"".as_slice(), b"9bad", b"has.dot", b"\xc3\xa9", &[b'a'; 129]] {
        changed = valid[..32].to_vec(); changed[16..20].copy_from_slice(&(bad_name.len() as u32).to_le_bytes());
        changed.extend_from_slice(bad_name); changed.extend_from_slice(&valid[32+name_len..]);
        assert_eq!(result("read-state", &changed), (12, vec![]));
    }
    let owner_start = 32 + name_len;
    changed = valid.clone();
    let duplicate = changed[owner_start..owner_start+17].to_vec();
    changed[owner_start+50..owner_start+67].copy_from_slice(&duplicate);
    assert_eq!(result("read-state", &changed), (12, vec![]));
}
