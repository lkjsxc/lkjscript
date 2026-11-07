// Flat container/collector capacity controls. Full source and graph authority are
// exercised by the public Sequence crossing and the independent admission tests.

const FORMER_CONTAINER_BYTE_CEILING: usize = 256 * 1024 * 1024;

fn capacity_container(objects: BTreeMap<ObjectKey, Vec<u8>>) -> PackageContainer {
    let root = PackageTransportBinding {
        package_revision: PackageRevisionDigest::from_bytes([1; 32]),
        transport: PackageTransportDigest::from_bytes([2; 32]),
    };
    PackageContainer {
        root,
        selections: vec![root],
        objects,
    }
}

fn small_capacity_container() -> PackageContainer {
    let bytes = b"framed source capacity".to_vec();
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
    capacity_container(BTreeMap::from([(key, bytes)]))
}

#[test]
fn source_container_decoder_preserves_exact_optional_limits() {
    let container = small_capacity_container();
    let bytes = container.encode().unwrap();
    let counts = [
        u64::try_from(bytes.len()).unwrap(),
        u64::try_from(container.selections.len()).unwrap(),
        u64::try_from(container.objects.len()).unwrap(),
    ];
    for (dimension, required) in counts.into_iter().enumerate() {
        for maximum in [
            None,
            Some(0),
            Some(required - 1),
            Some(required),
            Some(required + 1),
            Some(u64::MAX),
        ] {
            let mut limits = PackageContainerDecodeLimits::default();
            match dimension {
                0 => limits.maximum_bytes = maximum,
                1 => limits.maximum_packages = maximum,
                2 => limits.maximum_objects = maximum,
                _ => unreachable!(),
            }
            let result =
                PackageContainer::decode_with_limits(&bytes, container.root.transport, limits);
            if maximum.is_none_or(|maximum| maximum >= required) {
                let decoded = result.unwrap();
                assert_eq!(decoded.root, container.root);
                assert_eq!(decoded.selections, container.selections);
                assert_eq!(decoded.objects, container.objects);
                assert_eq!(decoded.encode().unwrap(), bytes);
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.class, DiagnosticClass::Resource);
                assert_eq!(error.code, "package_source_budget");
            }
        }
    }
    let all_maximum = PackageContainerDecodeLimits {
        maximum_bytes: Some(u64::MAX),
        maximum_packages: Some(u64::MAX),
        maximum_objects: Some(u64::MAX),
    };
    assert_eq!(
        PackageContainer::decode_with_limits(&bytes, container.root.transport, all_maximum)
            .unwrap()
            .encode()
            .unwrap(),
        bytes
    );
    let empty = capacity_container(BTreeMap::new());
    let empty_bytes = empty.encode().unwrap();
    assert!(
        PackageContainer::decode_with_limits(
            &empty_bytes,
            empty.root.transport,
            PackageContainerDecodeLimits {
                maximum_objects: Some(0),
                ..PackageContainerDecodeLimits::default()
            },
        )
        .unwrap()
        .objects
        .is_empty()
    );
}

#[test]
fn source_container_large_declared_counts_require_actual_framing() {
    let container = small_capacity_container();
    let original = container.encode().unwrap();
    // A count beyond the removed default is not evidence of an actual large
    // container. Its minimum records must be present before any inventory grows.
    let mut bytes = original.clone();
    bytes[76..84].copy_from_slice(&1_000_001_u64.to_be_bytes());
    let error = PackageContainer::decode(&bytes, container.root.transport).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "package_container_truncated");
    bytes[76..84].copy_from_slice(&u64::MAX.to_be_bytes());
    assert!(PackageContainer::decode(&bytes, container.root.transport).is_err());

    let mut bytes = original.clone();
    let record = 84 + 64;
    let length = u64::try_from(bytes.len()).unwrap() + 1;
    bytes[record + 33..record + 41].copy_from_slice(&length.to_be_bytes());
    assert_eq!(
        PackageContainer::decode(&bytes, container.root.transport)
            .unwrap_err()
            .code,
        "package_container_truncated"
    );
    let mut bytes = original;
    bytes.push(0);
    assert_eq!(
        PackageContainer::decode(&bytes, container.root.transport)
            .unwrap_err()
            .code,
        "package_container_trailing"
    );
}

#[test]
#[ignore = "round-trips and retains more than 256 MiB of actual source-container data"]
fn source_container_and_collector_cross_former_byte_default() {
    // This is a wire/collector component witness, not an admitted semantic
    // package: the public million-owner Sequence supplies that separate proof.
    // Each blob remains within its unchanged physical per-object boundary.
    let mut objects = BTreeMap::new();
    for value in 0..4_u8 {
        let bytes = vec![value; 64 * 1024 * 1024];
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
        objects.insert(key, bytes);
    }
    let final_bytes = vec![0xff];
    objects.insert(
        ObjectKey::for_bytes(ObjectDomain::Blob, &final_bytes),
        final_bytes,
    );
    let payload_bytes = FORMER_CONTAINER_BYTE_CEILING + 1;
    assert_eq!(objects.values().map(Vec::len).sum::<usize>(), payload_bytes);
    let container = capacity_container(objects);
    let encoded = container.encode().unwrap();
    assert!(encoded.len() > FORMER_CONTAINER_BYTE_CEILING);
    assert_eq!(encoded.len(), container.encoded_size().unwrap());
    let finite = PackageContainerDecodeLimits {
        maximum_bytes: Some(u64::try_from(encoded.len()).unwrap() - 1),
        ..PackageContainerDecodeLimits::default()
    };
    assert_eq!(
        PackageContainer::decode_with_limits(&encoded, container.root.transport, finite)
            .unwrap_err()
            .code,
        "package_source_budget"
    );
    let decoded = PackageContainer::decode(&encoded, container.root.transport).unwrap();
    assert_eq!(decoded.root, container.root);
    assert_eq!(decoded.selections, container.selections);
    assert_eq!(decoded.objects, container.objects);
    drop(container);
    let store = CollectingStore::new(&decoded.objects);
    for (key, expected) in &decoded.objects {
        let actual = store.read(*key, None, &mut StoreWork::default()).unwrap();
        assert_eq!(actual.as_deref(), Some(expected.as_slice()));
    }
    assert_eq!(store.remaining_visits(), None);
    assert_eq!(store.remaining_read_bytes(), None);
    assert_eq!(store.visits(), 5);
    assert_eq!(store.read_bytes(), u64::try_from(payload_bytes).unwrap());
    assert!(!store.visits_saturated());
    assert!(!store.read_bytes_saturated());
    assert_eq!(*store.objects.borrow(), decoded.objects);
    eprintln!(
        "PACKAGE_SOURCE_CAPACITY container_bytes={} payload_bytes={} objects={} default_decode=true default_collect=true",
        encoded.len(),
        payload_bytes,
        decoded.objects.len(),
    );
}
