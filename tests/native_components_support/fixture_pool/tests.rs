use super::*;
use std::fs;

#[test]
fn artifact_pool_rechecks_tampered_and_missing_bytes_after_a_hit_and_detachment() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("artifact.lkja");
    let bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard/generated/standard.lkja"),
    )
    .unwrap();
    fs::write(&path, &bytes).unwrap();
    let pool = ArtifactPool::default();
    let first = pool.bind(&path).unwrap();
    let repeated = pool.bind(&path).unwrap();
    assert!(Arc::ptr_eq(&first.tool, &repeated.tool));
    assert_eq!(pool.binding_hits.get(), 1);
    let fresh = pool.fresh_after_detachment(&path).unwrap();
    assert!(!Arc::ptr_eq(&first.tool, &fresh));
    assert!(Arc::ptr_eq(
        &fresh,
        &pool.fresh_after_detachment(&path).unwrap()
    ));
    assert_eq!(pool.fresh_preparations.get(), 1);

    let mut corrupt = bytes.clone();
    corrupt[0] ^= 1;
    fs::write(&path, &corrupt).unwrap();
    let independent = PureTool::load(&corrupt, &ExecutionControl::uncancelled())
        .err()
        .unwrap();
    for failure in [
        pool.bind(&path).err(),
        pool.fresh_after_detachment(&path).err(),
    ] {
        let failure = failure.unwrap();
        assert_eq!(failure.class, independent.class);
        assert_eq!(failure.code, independent.code);
    }
    assert!(pool.current.borrow().is_none());
    fs::write(&path, &bytes).unwrap();
    assert_eq!(pool.bind(&path).unwrap().digest, first.digest);
    assert!(!pool.current.borrow().as_ref().unwrap().fresh_detached);
    fs::remove_file(&path).unwrap();
    assert_eq!(
        pool.bind(&path).err().unwrap().code,
        "fixture_artifact_read"
    );
    assert!(pool.current.borrow().is_none());
}

#[test]
fn content_pool_bounds_entries_and_bytes_and_never_caches_failed_admission() {
    let mut pool = Programs::<usize>::new(2, 6);
    let preparations = Cell::new(0);
    let load = |bytes: &[u8]| {
        preparations.set(preparations.get() + 1);
        if bytes == b"bad" {
            Err("rejected")
        } else {
            Ok(bytes.len())
        }
    };
    let (first, prepared) = pool.admit(b"aaa".to_vec(), load).unwrap();
    assert!(prepared);
    let (same, prepared) = pool.admit(b"aaa".to_vec(), load).unwrap();
    assert!(!prepared && Arc::ptr_eq(&first, &same));
    assert_eq!(preparations.get(), 1);
    assert!(pool.admit(b"bad".to_vec(), load).is_err());
    assert!(pool.admit(b"bad".to_vec(), load).is_err());
    assert_eq!(preparations.get(), 3);
    assert_eq!(pool.entries.len(), 1);
    assert_eq!(pool.artifact_bytes, 3);
    pool.admit(b"bbb".to_vec(), load).unwrap();
    pool.admit(b"ccc".to_vec(), load).unwrap();
    assert_eq!(pool.entries.len(), 2);
    assert_eq!(pool.artifact_bytes, 6);
    assert!(
        pool.entries
            .iter()
            .all(|entry| entry.bytes.as_ref() != b"aaa")
    );
    // Eviction drops only cache ownership; an active invocation remains usable.
    assert_eq!(*first.value, 3);
    let (repeated, prepared) = pool.admit(b"aaa".to_vec(), load).unwrap();
    assert!(prepared && !Arc::ptr_eq(&first, &repeated));
    let (oversize, prepared) = pool.admit(b"1234567".to_vec(), load).unwrap();
    assert!(prepared && *oversize.value == 7);
    assert_eq!(pool.entries.len(), 2);
    assert_eq!(pool.artifact_bytes, 6);
    pool.admit(b"dddd".to_vec(), load).unwrap();
    assert_eq!(pool.entries.len(), 1);
    assert_eq!(pool.artifact_bytes, 4);
}
