use super::*;
use crate::platform::storage::object::ObjectDomain;
use std::time::{Duration, Instant};

fn open_gc_after_reader_drop(root: &Path) -> PackDirectoryStore {
    // Other tests spawn children while our reader is open. A fork duplicates
    // its file description, retaining flock until the child closes it at exec.
    // These callers hold no local reader or publication lock. Retry only the
    // explicit reader-contention result; production GC must remain nonblocking.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match PackDirectoryStore::open_for_gc(root) {
            Ok(store) => return store,
            Err(error) if error.code == "gc_readers_active" && Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(error) => panic!("GC admission after local readers dropped: {error:?}"),
        }
    }
}

fn fixture(path: &Path) -> BTreeMap<ObjectKey, Vec<u8>> {
    let mut store = PackDirectoryStore::initialize(path).unwrap();
    let mut keep = BTreeMap::new();
    for index in 0..3 {
        let value = vec![index; 32_768];
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, &value);
        store.stage(key, &value, &mut StoreWork::default()).unwrap();
        store
            .seal_staged(65_536, &mut StoreWork::default())
            .unwrap();
        if index != 2 {
            keep.insert(key, value);
        }
    }
    keep
}

fn read_all(store: &PackDirectoryStore, keep: &BTreeMap<ObjectKey, Vec<u8>>) {
    for (key, bytes) in keep {
        assert_eq!(
            store
                .read(*key, bytes.len(), &mut StoreWork::default())
                .unwrap()
                .as_ref(),
            Some(bytes)
        );
    }
}

#[test]
fn retained_reader_excludes_gc_without_invalidating_its_catalog() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let keep = fixture(root);
    let old = PackDirectoryStore::open(root).unwrap();
    let catalog = fs::read(root.join("catalog/current.lkjc")).unwrap();
    assert_eq!(
        PackDirectoryStore::open_for_gc(root).unwrap_err().code,
        "gc_readers_active"
    );
    assert_eq!(
        catalog,
        fs::read(root.join("catalog/current.lkjc")).unwrap()
    );
    read_all(&old, &keep);
    let inherited = old.root_directory.try_clone().unwrap();
    drop(old);
    // Duplicate descriptors share the reader lease even after the store drops.
    assert_eq!(
        PackDirectoryStore::open_for_gc(root).unwrap_err().code,
        "gc_readers_active"
    );
    drop(inherited);
    let mut gc = open_gc_after_reader_drop(root);
    let receipt = gc.compact_reachable(keep.clone()).unwrap();
    assert_eq!((receipt.packs_before, receipt.packs_after), (3, 1));
    assert!(receipt.pack_bytes_after < receipt.pack_bytes_before / 20);
    read_all(&gc, &keep);
    let deep = gc.deep_verify().unwrap();
    assert!(deep.catalog_equal);
    assert_eq!(deep.objects, keep.len());
    drop(gc);
    let reopened = PackDirectoryStore::open(root).unwrap();
    read_all(&reopened, &keep);
    let gone = ObjectKey::for_bytes(ObjectDomain::Blob, &vec![2; 32_768]);
    assert!(
        reopened
            .read(gone, 32_768, &mut StoreWork::default())
            .unwrap()
            .is_none()
    );
}

#[test]
fn every_compaction_durability_checkpoint_preserves_reachable_objects_and_retry() {
    for point in SealCheckpoint::ALL {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let keep = fixture(root);
        let mut gc = open_gc_after_reader_drop(root);
        let mut fired = false;
        let error = gc
            .compact_inner(keep.clone(), &mut |observed| {
                if observed == point && !fired {
                    fired = true;
                    return Err(injected_interruption(observed));
                }
                Ok(())
            })
            .unwrap_err();
        assert!(fired, "{point:?}");
        assert_eq!(error.code, INJECTED_INTERRUPTION_CODE);
        drop(gc);
        let reopened = PackDirectoryStore::open(root).unwrap();
        read_all(&reopened, &keep);
        drop(reopened);
        let mut retry = open_gc_after_reader_drop(root);
        retry.compact_reachable(keep.clone()).unwrap();
        read_all(&retry, &keep);
        assert_eq!(retry.deep_verify().unwrap().packs, 1);
    }
}

#[test]
fn compaction_preserves_foreign_files_and_rejects_unleased_calls() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let keep = fixture(root);
    fs::write(root.join("packs/notes.txt"), b"unrelated").unwrap();
    fs::write(root.join("application-data"), b"data").unwrap();
    let mut shared = PackDirectoryStore::open(root).unwrap();
    assert_eq!(
        shared.compact_reachable(keep.clone()).unwrap_err().code,
        "gc_lease"
    );
    drop(shared);
    let mut gc = open_gc_after_reader_drop(root);
    gc.compact_reachable(keep).unwrap();
    assert_eq!(
        fs::read(root.join("packs/notes.txt")).unwrap(),
        b"unrelated"
    );
    assert_eq!(fs::read(root.join("application-data")).unwrap(), b"data");
}
