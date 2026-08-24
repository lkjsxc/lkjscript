//! Generic immutable store and pack conformance tests.

use super::catalog::ObjectCatalog;
use super::directory::{CatalogState, PackDirectoryStore};
use super::memory::MemoryPackedStore;
use super::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, ObjectStage, StageOutcome, StoreWork,
};
use super::pack::{PackBuilder, PackId, PackMetadata};
use super::page_store::ObjectPageStore;
use crate::platform::persistent_map::{MapWork, PersistentMap};
use std::io::{Cursor, Seek, SeekFrom, Write};

fn object(domain: ObjectDomain, value: &[u8]) -> (ObjectKey, Vec<u8>) {
    (ObjectKey::for_bytes(domain, value), value.to_vec())
}

fn pack_footer_offset(bytes: &[u8]) -> usize {
    let trailer = bytes.len() - super::pack::TRAILER_BYTES;
    usize::try_from(u64::from_be_bytes(
        bytes[trailer..trailer + 8]
            .try_into()
            .expect("footer offset bytes"),
    ))
    .expect("footer offset")
}

fn pack_index_bounds(bytes: &[u8]) -> (usize, usize) {
    let footer = pack_footer_offset(bytes);
    let length = usize::try_from(u64::from_be_bytes(
        bytes[footer + 28..footer + 36]
            .try_into()
            .expect("index length bytes"),
    ))
    .expect("index length");
    (footer - length, footer)
}

fn rewrite_pack_index_checksum(bytes: &mut [u8]) {
    let footer = pack_footer_offset(bytes);
    let (start, end) = pack_index_bounds(bytes);
    let mut hasher = blake3::Hasher::new_derive_key(super::contract::PACK_INDEX_CHECKSUM_DOMAIN);
    hasher.update(&((end - start) as u64).to_be_bytes());
    hasher.update(&bytes[start..end]);
    bytes[footer + 36..footer + 68].copy_from_slice(hasher.finalize().as_bytes());
}

fn rewrite_catalog_checksum(bytes: &mut [u8]) {
    let trailer = bytes.len() - 40;
    let mut hasher = blake3::Hasher::new_derive_key(super::contract::CATALOG_CHECKSUM_DOMAIN);
    hasher.update(&(trailer as u64).to_be_bytes());
    hasher.update(&bytes[..trailer]);
    bytes[trailer..trailer + 32].copy_from_slice(hasher.finalize().as_bytes());
}

#[test]
fn multi_domain_pack_is_deterministic_and_exact() {
    let entries = [
        object(ObjectDomain::Owner, b"owner-one"),
        object(ObjectDomain::Type, b"type-one"),
        object(ObjectDomain::Blob, b"blob-one"),
    ];
    let mut forward = PackBuilder::default();
    for (key, bytes) in &entries {
        forward.insert(*key, bytes).expect("object must stage");
    }
    let mut reverse = PackBuilder::default();
    for (key, bytes) in entries.iter().rev() {
        reverse.insert(*key, bytes).expect("object must stage");
    }
    let forward = forward.seal().expect("pack must seal");
    let reverse = reverse.seal().expect("pack must seal");
    assert_eq!(forward.id, reverse.id);
    assert_eq!(forward.bytes, reverse.bytes);
    assert_eq!(forward.metadata.entries.len(), 3);
    forward
        .metadata
        .verify_all(&forward.bytes)
        .expect("pack must verify deeply");
    for (key, expected) in entries {
        assert_eq!(
            forward
                .metadata
                .read(&forward.bytes, key, expected.len())
                .expect("entry must read")
                .expect("entry must exist"),
            expected
        );
    }
}

#[test]
fn object_stage_reads_through_without_mutating_accepted_storage() {
    let mut base = MemoryPackedStore::default();
    let (accepted_key, accepted_bytes) = object(ObjectDomain::Owner, b"accepted-owner");
    let mut base_work = StoreWork::default();
    base.stage(accepted_key, &accepted_bytes, &mut base_work)
        .expect("accepted object must stage");
    base.seal_staged(16 * 1024, &mut base_work)
        .expect("accepted object must seal");

    let (candidate_key, candidate_bytes) = object(ObjectDomain::Type, b"candidate-type");
    let staged_objects = {
        let mut stage = ObjectStage::new(&base);
        let mut work = StoreWork::default();
        assert_eq!(
            stage
                .stage(accepted_key, &accepted_bytes, &mut work)
                .expect("accepted bytes must deduplicate"),
            StageOutcome::Reused
        );
        assert_eq!(
            stage
                .stage(candidate_key, &candidate_bytes, &mut work)
                .expect("candidate bytes must stage"),
            StageOutcome::Inserted
        );
        assert_eq!(stage.len(), 1);
        assert_eq!(stage.stored_bytes(), candidate_bytes.len());
        assert_eq!(
            stage
                .read(accepted_key, accepted_bytes.len(), &mut work)
                .expect("accepted read"),
            Some(accepted_bytes.clone())
        );
        assert_eq!(
            stage
                .read(candidate_key, candidate_bytes.len(), &mut work)
                .expect("candidate read"),
            Some(candidate_bytes.clone())
        );
        stage.into_objects()
    };
    assert_eq!(staged_objects.get(&candidate_key), Some(&candidate_bytes));
    assert!(
        base.read(
            candidate_key,
            candidate_bytes.len(),
            &mut StoreWork::default()
        )
        .expect("base lookup")
        .is_none()
    );
}

#[test]
fn target_sized_packs_and_catalog_round_trip() {
    let mut builder = PackBuilder::default();
    let mut expected = Vec::new();
    for ordinal in 0..100_u16 {
        let bytes = vec![ordinal as u8; 1024];
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, &bytes);
        builder.insert(key, &bytes).expect("object must stage");
        expected.push((key, bytes));
    }
    let packs = builder
        .seal_targeted(16 * 1024)
        .expect("targeted packs must seal");
    assert!(packs.len() > 1);
    let build = ObjectCatalog::rebuild(packs.iter().map(|pack| (pack.id, &pack.metadata)))
        .expect("catalog must rebuild");
    assert!(build.duplicates.is_empty());
    assert_eq!(build.catalog.len(), expected.len());
    let encoded = build.catalog.encode().expect("catalog must encode");
    let decoded =
        ObjectCatalog::decode(&encoded, build.catalog.generation()).expect("catalog must decode");
    assert_eq!(decoded, build.catalog);
    assert!(ObjectCatalog::decode(&encoded, [0x44; 32]).is_err());
    let mut corrupt = encoded.clone();
    let checksum = corrupt.len() - 40;
    corrupt[checksum] ^= 1;
    assert!(ObjectCatalog::decode(&corrupt, build.catalog.generation()).is_err());
    let mut overflow = encoded.clone();
    let first_location_offset = 52 + 65;
    overflow[first_location_offset..first_location_offset + 8]
        .copy_from_slice(&u64::MAX.to_be_bytes());
    rewrite_catalog_checksum(&mut overflow);
    let error = ObjectCatalog::decode(&overflow, build.catalog.generation())
        .expect_err("catalog coordinates must not overflow");
    assert_eq!(error.code, "catalog_location_overflow");
    for (key, _) in expected {
        assert!(decoded.get(key).is_some());
    }
}

#[test]
fn memory_store_deduplicates_seals_and_verifies_reads() {
    let mut store = MemoryPackedStore::default();
    let (owner, owner_bytes) = object(ObjectDomain::Owner, b"owner");
    let mut work = StoreWork::default();
    assert_eq!(
        store
            .stage(owner, &owner_bytes, &mut work)
            .expect("stage must succeed"),
        StageOutcome::Inserted
    );
    assert_eq!(
        store
            .stage(owner, &owner_bytes, &mut work)
            .expect("repeated stage must succeed"),
        StageOutcome::Reused
    );
    let packs = store
        .seal_staged(16 * 1024, &mut work)
        .expect("stage must seal");
    assert_eq!(packs.len(), 1);
    assert_eq!(store.staged_len(), 0);
    assert_eq!(store.pack_len(), 1);
    assert_eq!(
        store
            .read(owner, owner_bytes.len(), &mut work)
            .expect("read must succeed"),
        Some(owner_bytes)
    );
    assert!(
        store
            .contains(owner, &mut work)
            .expect("contains must work")
    );
}

#[test]
fn persistent_map_pages_use_the_generic_packed_store() {
    let mut pages = ObjectPageStore::new(MemoryPackedStore::default());
    let mut map_work = MapWork::default();
    let map = PersistentMap::empty(&mut pages, &mut map_work).expect("empty map must build");
    let (map, _) = map
        .insert(&mut pages, b"alpha", b"one", &mut map_work)
        .expect("map insert must work");
    let (map, _) = map
        .insert(&mut pages, b"beta", b"two", &mut map_work)
        .expect("map insert must work");
    assert_eq!(
        map.lookup(&pages, b"alpha", &mut map_work)
            .expect("map lookup must work"),
        Some(b"one".to_vec())
    );
    let mut store_work = pages.work();
    pages
        .objects_mut()
        .seal_staged(16 * 1024, &mut store_work)
        .expect("map pages must seal");
    assert_eq!(
        map.lookup(&pages, b"beta", &mut map_work)
            .expect("packed map lookup must work"),
        Some(b"two".to_vec())
    );
}

#[test]
fn malformed_pack_footer_payload_and_domain_reject() {
    let (key, bytes) = object(ObjectDomain::Owner, b"checked-owner");
    let mut builder = PackBuilder::default();
    builder.insert(key, &bytes).expect("object must stage");
    let pack = builder.seal().expect("pack must seal");

    let mut footer = pack.bytes.clone();
    let footer_byte = footer.len() - 60;
    footer[footer_byte] ^= 1;
    assert!(PackMetadata::decode(&footer, false).is_err());

    let mut payload = pack.bytes.clone();
    let payload_offset = pack.metadata.entries[0].offset as usize;
    payload[payload_offset] ^= 1;
    let metadata = PackMetadata::decode(&payload, false).expect("footer remains valid");
    assert!(metadata.read(&payload, key, bytes.len()).is_err());

    let foreign = ObjectKey::for_bytes(ObjectDomain::Type, &bytes);
    assert_ne!(foreign.digest, key.digest);
    assert!(
        pack.metadata
            .read(&pack.bytes, foreign, bytes.len())
            .unwrap()
            .is_none()
    );
}

#[test]
fn pack_decoder_rejects_truncation_trailing_bounds_order_and_foreign_domain() {
    let mut builder = PackBuilder::default();
    for value in [b"first".as_slice(), b"second".as_slice()] {
        let key = ObjectKey::for_bytes(ObjectDomain::Owner, value);
        builder.insert(key, value).expect("object must stage");
    }
    let pack = builder.seal().expect("pack must seal");

    assert!(PackMetadata::decode(&pack.bytes[..pack.bytes.len() - 1], false).is_err());
    let mut trailing = pack.bytes.clone();
    trailing.push(0);
    assert!(PackMetadata::decode(&trailing, false).is_err());

    let footer = pack_footer_offset(&pack.bytes);
    let mut excessive = pack.bytes.clone();
    excessive[footer + 12..footer + 20]
        .copy_from_slice(&((super::contract::MAXIMUM_PACK_ENTRIES as u64) + 1).to_be_bytes());
    let error = PackMetadata::decode(&excessive, false).expect_err("count must reject");
    assert_eq!(error.code, "pack_entry_count");

    let (index, _) = pack_index_bounds(&pack.bytes);
    let mut overlapping = pack.bytes.clone();
    overlapping[index + 33..index + 41]
        .copy_from_slice(&(super::pack::HEADER_BYTES as u64 + 1).to_be_bytes());
    rewrite_pack_index_checksum(&mut overlapping);
    let error = PackMetadata::decode(&overlapping, false).expect_err("offset must reject");
    assert_eq!(error.code, "pack_entry_layout");

    let mut overflow = pack.bytes.clone();
    overflow[index + 33..index + 41].copy_from_slice(&u64::MAX.to_be_bytes());
    rewrite_pack_index_checksum(&mut overflow);
    assert!(PackMetadata::decode(&overflow, false).is_err());

    let mut duplicate = pack.bytes.clone();
    let second = index + 89;
    let first_key = duplicate[index..index + 33].to_vec();
    duplicate[second..second + 33].copy_from_slice(&first_key);
    rewrite_pack_index_checksum(&mut duplicate);
    let error = PackMetadata::decode(&duplicate, false).expect_err("duplicate must reject");
    assert_eq!(error.code, "pack_index_order");

    let mut foreign = pack.bytes.clone();
    foreign[index] = 0xff;
    rewrite_pack_index_checksum(&mut foreign);
    let error = PackMetadata::decode(&foreign, false).expect_err("domain must reject");
    assert_eq!(error.code, "object_domain_tag");
}

#[test]
fn footer_scan_is_bounded_and_deep_file_verification_is_exact() {
    let payload = vec![0x5a; 1024 * 1024];
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, &payload);
    let mut builder = PackBuilder::default();
    builder.insert(key, &payload).expect("blob must stage");
    let pack = builder.seal().expect("pack must seal");
    let mut reader = Cursor::new(&pack.bytes);
    let footer = PackMetadata::read_footer(&mut reader, pack.bytes.len() as u64)
        .expect("footer must decode");
    assert_eq!(footer.metadata, pack.metadata);
    assert!(footer.bytes_read < pack.bytes.len() as u64 / 100);
    let verified = footer
        .metadata
        .verify_file(&mut reader, pack.id)
        .expect("file must verify");
    assert!(verified.bytes_read >= pack.bytes.len() as u64);
}

#[test]
fn pack_names_are_canonical_and_empty_packs_reject() {
    assert!(PackBuilder::default().seal().is_err());
    let id = PackId::from_bytes([0xab; 32]);
    assert_eq!(
        PackId::parse_file_name(&id.file_name()).expect("pack name must parse"),
        id
    );
    assert!(PackId::parse_file_name(&id.file_name().to_uppercase()).is_err());
    assert!(PackId::parse_file_name("pack_short.lkjp").is_err());
}

#[test]
fn directory_store_seals_reopens_rebuilds_and_deep_verifies() {
    let temporary = tempfile::TempDir::new().expect("temporary store parent");
    let root = temporary.path().join("objects");
    let mut store = PackDirectoryStore::initialize(&root).expect("store must initialize");
    assert_eq!(store.catalog_state(), CatalogState::RebuiltPersisted);
    assert!(store.catalog_rebuild_note().is_some());
    let entries = [
        object(ObjectDomain::Owner, b"owner-on-disk"),
        object(ObjectDomain::Type, b"type-on-disk"),
        object(ObjectDomain::Blob, b"blob-on-disk"),
    ];
    let mut work = StoreWork::default();
    for (key, bytes) in &entries {
        assert_eq!(
            store
                .stage(*key, bytes, &mut work)
                .expect("object must stage"),
            StageOutcome::Inserted
        );
    }
    let receipt = store
        .seal_staged(16 * 1024, &mut work)
        .expect("objects must seal");
    assert_eq!(receipt.packs.len(), 1);
    assert_eq!(receipt.objects, entries.len());
    assert_eq!(receipt.catalog_state, CatalogState::RebuiltPersisted);
    assert!(receipt.catalog_persist_error.is_none());
    assert_eq!(store.staged_len(), 0);
    let deep = store.deep_verify().expect("store must verify deeply");
    assert_eq!(deep.packs, 1);
    assert_eq!(deep.objects, entries.len());
    assert!(deep.duplicate_objects.is_empty());
    drop(store);

    let reopened = PackDirectoryStore::open(&root).expect("store must reopen");
    assert_eq!(reopened.catalog_state(), CatalogState::Loaded);
    assert_eq!(reopened.root(), root);
    for (key, bytes) in &entries {
        assert_eq!(
            reopened
                .read(*key, bytes.len(), &mut work)
                .expect("object must read"),
            Some(bytes.clone())
        );
    }

    std::fs::write(root.join("catalog/current.lkjc"), b"corrupt catalog")
        .expect("catalog corruption fixture");
    let rebuilt = PackDirectoryStore::open(&root).expect("corrupt catalog must rebuild");
    assert_eq!(rebuilt.catalog_state(), CatalogState::RebuiltPersisted);
    assert!(rebuilt.catalog_rebuild_note().is_some());
    assert!(rebuilt.catalog_persist_error().is_none());
    assert_eq!(rebuilt.catalog().len(), entries.len());
}

#[test]
fn directory_store_detects_duplicate_physical_objects() {
    let temporary = tempfile::TempDir::new().expect("temporary store parent");
    let root = temporary.path().join("objects");
    drop(PackDirectoryStore::initialize(&root).expect("store must initialize"));
    let shared = object(ObjectDomain::Owner, b"shared-owner");
    let left = object(ObjectDomain::Blob, b"left-only");
    let right = object(ObjectDomain::Blob, b"right-only");
    let mut first = PackBuilder::default();
    first
        .insert(shared.0, &shared.1)
        .expect("shared must stage");
    first.insert(left.0, &left.1).expect("left must stage");
    let first = first.seal().expect("first pack must seal");
    let mut second = PackBuilder::default();
    second
        .insert(shared.0, &shared.1)
        .expect("shared must stage");
    second.insert(right.0, &right.1).expect("right must stage");
    let second = second.seal().expect("second pack must seal");
    std::fs::write(root.join("packs").join(first.id.file_name()), &first.bytes)
        .expect("first pack fixture");
    std::fs::write(
        root.join("packs").join(second.id.file_name()),
        &second.bytes,
    )
    .expect("second pack fixture");
    let store = PackDirectoryStore::open(&root).expect("store must reopen");
    assert_eq!(store.duplicate_objects().len(), 1);
    assert_eq!(store.duplicate_objects()[0].key, shared.0);
    assert_eq!(store.deep_verify().expect("packs must verify").packs, 2);
}

#[test]
fn directory_point_read_detects_payload_corruption() {
    let temporary = tempfile::TempDir::new().expect("temporary store parent");
    let root = temporary.path().join("objects");
    let mut store = PackDirectoryStore::initialize(&root).expect("store must initialize");
    let (key, bytes) = object(ObjectDomain::Owner, b"payload-to-corrupt");
    let mut work = StoreWork::default();
    store
        .stage(key, &bytes, &mut work)
        .expect("object must stage");
    let receipt = store
        .seal_staged(16 * 1024, &mut work)
        .expect("object must seal");
    let path = root.join("packs").join(receipt.packs[0].file_name());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .expect("pack fixture must open");
    file.seek(SeekFrom::Start(super::pack::HEADER_BYTES as u64))
        .expect("payload seek");
    file.write_all(b"X").expect("payload corruption");
    file.sync_all().expect("payload corruption sync");
    assert!(store.read(key, bytes.len(), &mut work).is_err());
}

#[cfg(unix)]
#[test]
fn directory_store_rejects_symlinked_pack_and_predecessor_bytes() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::TempDir::new().expect("temporary store parent");
    let root = temporary.path().join("objects");
    drop(PackDirectoryStore::initialize(&root).expect("store must initialize"));
    let external = temporary.path().join("external");
    std::fs::write(&external, b"LKJGRPH4 predecessor").expect("external fixture");
    let symlink_name = PackId::from_bytes([0x44; 32]).file_name();
    symlink(&external, root.join("packs").join(symlink_name)).expect("symlink fixture");
    assert!(PackDirectoryStore::open(&root).is_err());

    std::fs::remove_dir_all(&root).expect("replace fixture store");
    drop(PackDirectoryStore::initialize(&root).expect("store must initialize"));
    let predecessor_name = PackId::from_bytes([0x55; 32]).file_name();
    std::fs::write(
        root.join("packs").join(predecessor_name),
        b"LKJGRPH4 predecessor",
    )
    .expect("predecessor fixture");
    let error = PackDirectoryStore::open(&root).expect_err("predecessor bytes must reject");
    assert!(matches!(
        error.class,
        super::object::StoreErrorClass::Corrupt | super::object::StoreErrorClass::Resource
    ));
}

#[test]
fn staging_leftovers_are_classified_without_becoming_authority() {
    let temporary = tempfile::TempDir::new().expect("temporary store parent");
    let root = temporary.path().join("objects");
    drop(PackDirectoryStore::initialize(&root).expect("store must initialize"));
    std::fs::write(root.join("staging/.pack-stage-interrupted"), b"partial")
        .expect("staging fixture");
    let store = PackDirectoryStore::open(&root).expect("store must reopen");
    assert_eq!(
        store.staging_leftovers(),
        &[".pack-stage-interrupted".to_owned()]
    );
    assert!(store.catalog().is_empty());
}
