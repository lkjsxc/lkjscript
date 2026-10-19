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

const MULTI_PACK_TARGET: usize = 33 * 1024;

fn selected_packs(store: &PackDirectoryStore) -> BTreeSet<PackId> {
    let selected = owned_pack_names(&store.packs_directory)
        .unwrap()
        .into_iter()
        .filter_map(|(id, _)| store.catalog.pack(id).map(|_| id))
        .collect::<BTreeSet<_>>();
    assert_eq!(selected.len() as u64, store.catalog.manifest().total_packs);
    for id in &selected {
        verify_existing_pack(&store.packs_directory, *id).unwrap();
    }
    selected
}

#[test]
fn every_compaction_durability_checkpoint_preserves_reachable_objects_and_retry() {
    let expected_directory = tempfile::tempdir().unwrap();
    let expected_keep = fixture(expected_directory.path());
    let mut expected = open_gc_after_reader_drop(expected_directory.path());
    expected
        .compact_inner(expected_keep, MULTI_PACK_TARGET, &mut |_| Ok(()))
        .unwrap();
    let replacement = selected_packs(&expected);
    assert_eq!(replacement.len(), 2);
    for point in SealCheckpoint::ALL {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path();
        let keep = fixture(root);
        let mut gc = open_gc_after_reader_drop(root);
        let original = selected_packs(&gc);
        let original_objects = gc.catalog.len();
        let mut fired = false;
        let error = gc
            .compact_inner(keep.clone(), MULTI_PACK_TARGET, &mut |observed| {
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
        let selected = selected_packs(&reopened);
        assert!(
            (selected == original && reopened.catalog.len() == original_objects)
                || (selected == replacement && reopened.catalog.len() == keep.len()),
            "{point:?}: a failed publication must select a complete old or new catalog"
        );
        drop(reopened);
        let mut retry = open_gc_after_reader_drop(root);
        retry
            .compact_inner(keep.clone(), MULTI_PACK_TARGET, &mut |_| Ok(()))
            .unwrap();
        read_all(&retry, &keep);
        assert_eq!(selected_packs(&retry), replacement);
        assert_eq!(retry.deep_verify().unwrap().packs, 2);
        drop(retry);
        let reopened = PackDirectoryStore::open(root).unwrap();
        read_all(&reopened, &keep);
        assert_eq!(selected_packs(&reopened), replacement);
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

#[test]
fn partial_pack_installation_and_repeated_compaction_preserve_selected_copies() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path();
    let mut keep = fixture(root);
    let third = vec![2; 32_768];
    keep.insert(ObjectKey::for_bytes(ObjectDomain::Blob, &third), third);
    let mut gc = open_gc_after_reader_drop(root);
    let original = selected_packs(&gc);
    let mut published = 0;
    let error = gc
        .compact_inner(keep.clone(), MULTI_PACK_TARGET, &mut |point| {
            if point == SealCheckpoint::PackPublished {
                published += 1;
                if published == 2 {
                    return Err(injected_interruption(point));
                }
            }
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code, INJECTED_INTERRUPTION_CODE);
    assert_eq!(published, 2);
    drop(gc);
    let reopened = PackDirectoryStore::open(root).unwrap();
    assert_eq!(selected_packs(&reopened), original);
    read_all(&reopened, &keep);
    drop(reopened);
    let mut retry = open_gc_after_reader_drop(root);
    let receipt = retry
        .compact_inner(keep.clone(), MULTI_PACK_TARGET, &mut |_| Ok(()))
        .unwrap();
    assert_eq!(receipt.packs_after, 3);
    assert_eq!(receipt.objects_retained, keep.len());
    let selected = selected_packs(&retry);
    retry
        .compact_inner(keep.clone(), MULTI_PACK_TARGET, &mut |_| Ok(()))
        .unwrap();
    assert_eq!(selected_packs(&retry), selected);
    read_all(&retry, &keep);
    assert_eq!(retry.deep_verify().unwrap().packs, 3);
    drop(retry);
    let reopened = PackDirectoryStore::open(root).unwrap();
    read_all(&reopened, &keep);
    assert_eq!(selected_packs(&reopened), selected);
}
