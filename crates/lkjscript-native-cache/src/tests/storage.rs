use std::os::unix::fs::symlink;

use super::*;

#[test]
fn publishes_hits_replaces_corruption_and_rejects_symlink() {
    let root = Root::new();
    let cache = NativeArtifactCache::open(&root.0, CacheLimits::default()).expect("cache");
    let key = key(&root);
    assert!(matches!(
        cache.lookup(&key).expect("miss"),
        Lookup::Miss(MissReason::NotFound)
    ));
    assert!(matches!(
        cache.publish(&key, &image()).expect("publish"),
        Publication::Published { .. }
    ));
    assert!(matches!(
        cache.lookup(&key).expect("hit"),
        Lookup::Hit { .. }
    ));
    assert!(matches!(
        cache.publish(&key, &image()).expect("duplicate"),
        Publication::Duplicate { .. }
    ));

    let object = cache.objects.join(format!("{}.image", key.hex()));
    std::fs::write(&object, b"corrupt").expect("corrupt object");
    assert!(matches!(
        cache.lookup(&key).expect("corrupt"),
        Lookup::Miss(MissReason::Corrupt)
    ));
    assert!(matches!(
        cache.publish(&key, &image()).expect("replace"),
        Publication::Published { .. }
    ));
    std::fs::remove_file(&object).expect("remove object");
    symlink("/tmp", &object).expect("symlink object");
    assert!(matches!(
        cache.lookup(&key).expect("symlink"),
        Lookup::Miss(MissReason::Corrupt)
    ));
}

#[test]
fn stale_staging_is_removed_before_atomic_publication() {
    let root = Root::new();
    let cache = NativeArtifactCache::open(&root.0, CacheLimits::default()).expect("cache");
    std::fs::write(cache.staging.join("publish.tmp"), b"partial").expect("stale staging");
    assert!(matches!(
        cache.publish(&key(&root), &image()).expect("publish"),
        Publication::Published { .. }
    ));
    assert_eq!(
        std::fs::read_dir(&cache.staging).expect("staging").count(),
        0
    );
}

#[test]
fn concurrent_same_key_publishers_leave_one_valid_object() {
    let root = Root::new();
    let key = key(&root);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let path = root.0.clone();
        let key = key.clone();
        let barrier = barrier.clone();
        workers.push(std::thread::spawn(move || {
            let cache =
                NativeArtifactCache::open(&path, CacheLimits::default()).expect("concurrent cache");
            barrier.wait();
            cache.publish(&key, &image()).expect("concurrent publish")
        }));
    }
    let results: Vec<_> = workers
        .into_iter()
        .map(|worker| worker.join().expect("publisher"))
        .collect();
    assert!(results.iter().any(|result| {
        matches!(
            result,
            Publication::Published { .. } | Publication::Duplicate { .. }
        )
    }));
    let cache = NativeArtifactCache::open(&root.0, CacheLimits::default()).expect("cache");
    assert!(matches!(
        cache.lookup(&key).expect("lookup"),
        Lookup::Hit { .. }
    ));
}

#[test]
fn object_and_aggregate_limits_skip_publication_without_partial_files() {
    let root = Root::new();
    let limits = CacheLimits {
        max_object_bytes: 64,
        max_objects: 1,
        max_total_bytes: 64,
        max_records: 100,
    };
    let cache = NativeArtifactCache::open(&root.0, limits).expect("cache");
    assert!(cache.publish(&key(&root), &image()).is_err());
    assert_eq!(
        std::fs::read_dir(&cache.objects).expect("objects").count(),
        0
    );
    assert_eq!(
        std::fs::read_dir(&cache.staging).expect("staging").count(),
        0
    );
}
