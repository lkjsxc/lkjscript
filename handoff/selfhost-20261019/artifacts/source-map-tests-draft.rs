// Proposed additions to selfhost_source_tests.rs after the decoder candidate is accepted.
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
            if key_bytes == 1 && count > 256 { continue; }
            check_map(key_bytes as u32, value_bytes as u32, map_rows(key_bytes, value_bytes, count));
        }
    }
    // Exact entry and byte ceilings are independent, supported positive cases.
    check_map(3, 1, map_rows(3, 1, 100_000));
    check_map(16, 112, map_rows(16, 112, 65_536));
    let (source, _) = compiler();
    let owners = source.owners.iter().map(|(id, record)| {
        let (object, _) = kernel::encode_owner(record).unwrap();
        (kernel::owner_map_key(*id).to_vec(), kernel::encode_owner_binding(&kernel::OwnerBinding { kind: record.kind(), object }))
    }).collect::<Vec<_>>();
    let dependencies = source.dependencies.iter().map(|(id, record)| {
        let (object, _) = kernel::encode_dependency(record).unwrap();
        (kernel::dependency_map_key(*id).to_vec(), kernel::encode_dependency_binding(&kernel::DependencyBinding { object }))
    }).collect::<Vec<_>>();
    let retirements = source.retirements.iter().map(|(id, record)| {
        let (object, _) = kernel::encode_retirement(record).unwrap();
        (kernel::retirement_map_key(*id).to_vec(), kernel::encode_retirement_binding(&kernel::RetirementBinding { object }))
    }).collect::<Vec<_>>();
    for (name, key_bytes, value_bytes, rows, expected) in [
        ("owners", 17, 33, owners, source.root.owners),
        ("dependencies", 16, 32, dependencies, source.root.dependencies),
        ("retirements", 17, 32, retirements, source.root.retirements),
    ] {
        let logical = crate::platform::persistent_map::MapContentRoot::from_sorted(rows.clone()).unwrap();
        assert_eq!(logical, expected.content_root(), "{name}");
        let packet = map_packet(key_bytes, value_bytes, &rows);
        let (code, digest) = result("read-map", &packet);
        assert_eq!(code, 0, "{name}");
        assert_eq!(digest, logical.digest().bytes(), "{name}");
        eprintln!("selfhost source map kind={name} entries={} input_bytes={} input_sha256={} root={}", rows.len(), packet.len(), crate::platform::semantic_id::encode_hex(&Sha256::digest(&packet)), logical.digest());
    }
}

#[test]
fn selfhost_source_maps_reject_invalid_widths_extents_duplicates_and_order() {
    for length in 0..8 {
        assert_eq!(result("read-map", &vec![0; length]), (12, vec![]));
    }
    for (key_bytes, value_bytes) in [(0, 32), (33, 32), (16, 0), (16, 129), (u32::MAX, 32), (16, u32::MAX)] {
        assert_eq!(result("read-map", &map_packet(key_bytes, value_bytes, &[])), (12, vec![]));
    }
    let valid = map_packet(17, 33, &map_rows(17, 33, 2));
    for length in [9, 57, 59, valid.len() - 1] {
        assert_eq!(result("read-map", &valid[..length]), (12, vec![]));
    }
    let mut duplicated = map_rows(17, 33, 2);
    duplicated[1].0 = duplicated[0].0.clone();
    assert_eq!(result("read-map", &map_packet(17, 33, &duplicated)), (12, vec![]));
    let mut reversed = map_rows(17, 33, 2);
    reversed.reverse();
    assert_eq!(result("read-map", &map_packet(17, 33, &reversed)), (12, vec![]));
    let mut too_many = map_packet(1, 1, &[]);
    too_many.resize(8 + 2 * 100_001, 0);
    assert_eq!(result("read-map", &too_many), (12, vec![]));
    let mut too_large = map_packet(16, 112, &[]);
    too_large.resize(8 + 8_388_609, 0);
    assert_eq!(result("read-map", &too_large), (12, vec![]));
    for (key_bytes, value_bytes) in [(i64::MIN, 32), (-1, 32), (i64::MAX, 32), (16, i64::MIN), (16, -1), (16, i64::MAX)] {
        let observed = invoke("source-map", "root", vec![NormalizedValue::bytes(&[][..]), NormalizedValue::I64(key_bytes), NormalizedValue::I64(value_bytes)]);
        assert_eq!(field(&observed, "diagnostic"), &NormalizedValue::I64(12));
        assert_eq!(field(&observed, "image"), &NormalizedValue::bytes(&[][..]));
    }
}
