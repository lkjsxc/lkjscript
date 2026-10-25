use super::*;

fn fixture(count: u32) -> (MemoryPageStore, PersistentMap) {
    let mut store = MemoryPageStore::default();
    let map = PersistentMap::from_sorted(
        &mut store,
        (0..count).map(|number| (number.to_be_bytes().to_vec(), vec![number as u8; 64])),
        &mut MapWork::default(),
    )
    .unwrap();
    (store, map)
}

#[test]
fn warm_pages_recheck_current_bytes_and_charge_every_read_dimension() {
    let (mut store, map) = fixture(32);
    let cache = Mutex::new(Cache::new(2, SHARD_BYTES));
    let digest = map.root.page;
    let mut cold_work = MapWork::default();
    let cold = load_with_cache(&store, digest, &mut cold_work, &cache).unwrap();
    let mut warm_work = MapWork::default();
    let warm = load_with_cache(&store, digest, &mut warm_work, &cache).unwrap();
    assert!(Arc::ptr_eq(&cold, &warm));
    assert_eq!(cold_work.pages_decoded, 1);
    assert_eq!(warm_work.pages_decoded, 0);
    assert_eq!(cold_work.pages_read, warm_work.pages_read);
    assert_eq!(cold_work.bytes_read, warm_work.bytes_read);
    assert_eq!(cold_work.entries_visited, warm_work.entries_visited);
    let exact = MapAdmission {
        maximum_pages_read: Some(cold_work.pages_read),
        maximum_bytes_read: Some(cold_work.bytes_read),
        maximum_entries_visited: Some(cold_work.entries_visited),
        ..MapAdmission::unbounded()
    };
    load_with_cache(&store, digest, &mut MapWork::with_admission(exact), &cache).unwrap();
    for dimension in 0..3 {
        let mut limit = exact;
        let code = match dimension {
            0 => {
                limit.maximum_pages_read = limit.maximum_pages_read.map(|value| value - 1);
                "persistent_map_admission_pages_read"
            }
            1 => {
                limit.maximum_bytes_read = limit.maximum_bytes_read.map(|value| value - 1);
                "persistent_map_admission_bytes_read"
            }
            _ => {
                limit.maximum_entries_visited = limit.maximum_entries_visited.map(|value| value - 1);
                "persistent_map_admission_entries_visited"
            }
        };
        let error = load_with_cache(&store, digest, &mut MapWork::with_admission(limit), &cache)
            .unwrap_err();
        assert_eq!(error.code, code);
    }
    let bytes = store.pages.get_mut(&digest).unwrap();
    bytes[0] ^= 1;
    assert_eq!(
        load_with_cache(&store, digest, &mut MapWork::default(), &cache)
            .unwrap_err()
            .code,
        "persistent_map_page_digest"
    );
    store.pages.remove(&digest);
    assert_eq!(
        load_with_cache(&store, digest, &mut MapWork::default(), &cache)
            .unwrap_err()
            .code,
        "persistent_map_page_missing"
    );
}

#[test]
fn cached_children_do_not_certify_a_rehashed_false_parent_summary() {
    let (mut store, map) = fixture(512);
    for number in 0_u32..512 {
        assert_eq!(
            map.lookup(&store, &number.to_be_bytes(), &mut MapWork::default())
                .unwrap(),
            Some(vec![number as u8; 64])
        );
    }
    let mut traversal = MapWork::default();
    let mut observed = Vec::new();
    map.for_each(&store, &mut traversal, |key, value| {
        observed.push((key.to_vec(), value.to_vec()));
        Ok(())
    })
    .unwrap();
    assert_eq!(
        observed,
        (0_u32..512)
            .map(|n| (n.to_be_bytes().to_vec(), vec![n as u8; 64]))
            .collect::<Vec<_>>()
    );
    assert_eq!(traversal.pages_read, traversal.pages_decoded);

    let mut parent = load_page(&store, map.root.page, &mut MapWork::default()).unwrap();
    let Page::Branch {
        count, children, ..
    } = &mut parent
    else {
        panic!("multi-page fixture")
    };
    *count += 1;
    children[0].count += 1;
    let bytes = encode_page(&parent).unwrap();
    let digest = PageDigest::of(&bytes);
    store.write_page(digest, &bytes).unwrap();
    let forged = PersistentMap::from_root(MapRoot {
        page: digest,
        entries: parent.count(),
        content: parent.content().unwrap(),
    });
    // Cache the internally canonical parent too. Its child relationship must still fail.
    load(&store, digest, &mut MapWork::default()).unwrap();
    assert_eq!(
        forged
            .lookup(&store, &0_u32.to_be_bytes(), &mut MapWork::default())
            .unwrap_err()
            .code,
        "persistent_map_child_summary"
    );
    let wrong_root = PersistentMap::from_root(MapRoot {
        entries: 0,
        ..map.root
    });
    assert_eq!(
        wrong_root
            .lookup(&store, &0_u32.to_be_bytes(), &mut MapWork::default())
            .unwrap_err()
            .code,
        "persistent_map_root_summary"
    );
}

#[test]
fn count_and_byte_eviction_release_only_cache_retention() {
    let (store, map) = fixture(512);
    let cache = Mutex::new(Cache::new(1, SHARD_BYTES));
    let root = load_with_cache(&store, map.root.page, &mut MapWork::default(), &cache).unwrap();
    let Page::Branch { children, .. } = &root.page else {
        panic!("multi-page fixture")
    };
    let child_digest = children[0].digest;
    let weak_root = Arc::downgrade(&root);
    let child = load_with_cache(&store, child_digest, &mut MapWork::default(), &cache).unwrap();
    assert!(
        weak_root.upgrade().is_some(),
        "caller retains its in-flight immutable read"
    );
    drop(root);
    assert!(weak_root.upgrade().is_none());
    assert_eq!(cache.lock().unwrap().entries.len(), 1);

    for (maximum, retained) in [(child.retained - 1, 0), (child.retained, 1)] {
        let cache = Mutex::new(Cache::new(2, maximum));
        load_with_cache(&store, child_digest, &mut MapWork::default(), &cache).unwrap();
        let cache = cache.lock().unwrap();
        assert_eq!(cache.entries.len(), retained);
        assert!(cache.retained <= maximum);
    }
}

#[test]
fn saturated_cold_observation_keeps_exact_warm_cache_admission() {
    let (store, map) = fixture(32);
    let cache = Mutex::new(Cache::new(2, SHARD_BYTES));
    let mut cold_work = MapWork::default();
    cold_work.entries_visited = u64::MAX - 1;
    cold_work.pages_decoded = u64::MAX;
    let cold = load_with_cache(&store, map.root.page, &mut cold_work, &cache).unwrap();
    assert_eq!(cold.visited, 32);
    assert_eq!(cold_work.entries_visited, u64::MAX);
    assert_eq!(cold_work.pages_decoded, u64::MAX);
    assert!(cold_work.saturated);
    assert!(cold_work.counter_saturated(MapWorkCounter::EntriesVisited));
    assert!(cold_work.counter_saturated(MapWorkCounter::PagesDecoded));

    for maximum in [Some(0), Some(31), Some(32), Some(u64::MAX), None] {
        let mut work = MapWork::with_admission(MapAdmission {
            maximum_entries_visited: maximum,
            ..MapAdmission::unbounded()
        });
        let read = load_with_cache(&store, map.root.page, &mut work, &cache);
        if maximum.is_some_and(|maximum| maximum < 32) {
            assert_eq!(read.unwrap_err().code, "persistent_map_admission_entries_visited");
            assert_eq!(work.entries_visited, 0);
        } else {
            let warm = read.unwrap();
            assert!(Arc::ptr_eq(&cold, &warm));
            assert_eq!(work.entries_visited, 32);
            assert_eq!(work.pages_decoded, 0);
            assert!(!work.saturated);
        }
    }
}
