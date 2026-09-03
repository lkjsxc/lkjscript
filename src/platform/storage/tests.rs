//! Generic immutable store and pack conformance tests.

use super::catalog::{
    CatalogEntry, CatalogHistory, CatalogIndex, CatalogLocation, CatalogManifest, ObjectCatalog,
    PackDescriptor, SegmentId, read_segment_metadata, write_segment,
};
use super::directory::{CatalogState, PackDirectoryStore, SealCheckpoint};
use super::memory::MemoryPackedStore;
use super::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, ObjectStage, ObjectStageLimits, StageOutcome,
    StoreReadAdmission, StoreReadLimits, StoreWork,
};
use super::pack::{PackBuilder, PackId, PackMetadata};
use super::page_store::ObjectPageStore;
use crate::platform::persistent_map::{MapWork, PersistentMap};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Cursor, Seek, SeekFrom, Write};

fn object(domain: ObjectDomain, value: &[u8]) -> (ObjectKey, Vec<u8>) {
    (ObjectKey::for_bytes(domain, value), value.to_vec())
}

fn read_admission(catalog_lookups: u64, objects: u64, bytes: u64) -> StoreReadAdmission {
    StoreReadAdmission::new(StoreReadLimits {
        maximum_catalog_lookups: catalog_lookups,
        maximum_objects: objects,
        maximum_bytes: bytes,
    })
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
fn admitted_memory_reads_reject_before_payload_consumption() {
    let mut store = MemoryPackedStore::default();
    let (key, bytes) = object(ObjectDomain::Owner, b"admitted-owner");
    let byte_count = bytes.len() as u64;
    let mut setup_work = StoreWork::default();
    store
        .stage(key, &bytes, &mut setup_work)
        .expect("fixture object must stage");
    store
        .seal_staged(16 * 1024, &mut setup_work)
        .expect("fixture object must seal");

    let mut admission = read_admission(0, 1, byte_count);
    let mut work = StoreWork::default();
    let error = store
        .read_admitted(key, bytes.len(), &mut admission, &mut work)
        .expect_err("zero catalog allowance must reject");
    assert_eq!(error.code, "object_read_catalog_lookups_exhausted");
    assert_eq!(work, StoreWork::default());
    assert_eq!(
        admission.remaining(),
        read_admission(0, 1, byte_count).remaining()
    );

    let mut admission = read_admission(1, 0, byte_count);
    let mut work = StoreWork::default();
    let error = store
        .read_admitted(key, bytes.len(), &mut admission, &mut work)
        .expect_err("zero object allowance must reject");
    assert_eq!(error.code, "object_read_objects_exhausted");
    assert_eq!(work.catalog_lookups, 1);
    assert_eq!(work.packs_opened, 0);
    assert_eq!(work.objects_read, 0);
    assert_eq!(work.bytes_read, 0);

    let mut admission = read_admission(1, 1, byte_count - 1);
    let mut work = StoreWork::default();
    let error = store
        .read_admitted(key, bytes.len(), &mut admission, &mut work)
        .expect_err("short byte allowance must reject");
    assert_eq!(error.code, "object_read_bytes_exhausted");
    assert_eq!(work.catalog_lookups, 1);
    assert_eq!(work.packs_opened, 0);
    assert_eq!(work.objects_read, 0);
    assert_eq!(work.bytes_read, 0);
    assert_eq!(admission.remaining().maximum_objects, 1);
    assert_eq!(admission.remaining().maximum_bytes, byte_count - 1);

    let mut admission = read_admission(1, 1, byte_count);
    let mut work = StoreWork::default();
    assert_eq!(
        store
            .read_admitted(key, bytes.len(), &mut admission, &mut work)
            .expect("exact allowance must read"),
        Some(bytes.clone())
    );
    assert_eq!(admission.remaining(), read_admission(0, 0, 0).remaining());
    assert_eq!(work.catalog_lookups, 1);
    assert_eq!(work.packs_opened, 1);
    assert_eq!(work.objects_read, 1);
    assert_eq!(work.bytes_read, byte_count);

    let missing = ObjectKey::for_bytes(ObjectDomain::Owner, b"missing-owner");
    let mut admission = read_admission(1, 0, 0);
    let mut work = StoreWork::default();
    assert_eq!(
        store
            .read_admitted(missing, 0, &mut admission, &mut work)
            .expect("missing lookup needs no payload allowance"),
        None
    );
    assert_eq!(work.catalog_lookups, 1);
    assert_eq!(work.objects_read, 0);
}

#[test]
fn admitted_object_stage_reads_local_objects_without_catalog_work() {
    let base = MemoryPackedStore::default();
    let (key, bytes) = object(ObjectDomain::Type, b"locally-staged-type");
    let byte_count = bytes.len() as u64;
    let mut stage = ObjectStage::new(&base);
    stage
        .stage(key, &bytes, &mut StoreWork::default())
        .expect("local object must stage");

    let mut admission = read_admission(0, 0, byte_count);
    let mut work = StoreWork::default();
    let error = stage
        .read_admitted(key, bytes.len(), &mut admission, &mut work)
        .expect_err("zero object allowance must reject the staged payload");
    assert_eq!(error.code, "object_read_objects_exhausted");
    assert_eq!(work, StoreWork::default());

    let mut admission = read_admission(0, 1, byte_count - 1);
    let mut work = StoreWork::default();
    let error = stage
        .read_admitted(key, bytes.len(), &mut admission, &mut work)
        .expect_err("short byte allowance must reject the staged payload");
    assert_eq!(error.code, "object_read_bytes_exhausted");
    assert_eq!(work, StoreWork::default());
    assert_eq!(admission.remaining().maximum_objects, 1);

    let mut admission = read_admission(0, 1, byte_count);
    let mut work = StoreWork::default();
    assert_eq!(
        stage
            .read_admitted(key, bytes.len(), &mut admission, &mut work)
            .expect("exact staged allowance must read"),
        Some(bytes.clone())
    );
    assert_eq!(work.catalog_lookups, 0);
    assert_eq!(work.objects_read, 1);
    assert_eq!(work.bytes_read, byte_count);
    assert_eq!(admission.remaining(), read_admission(0, 0, 0).remaining());

    let mut admission = read_admission(0, 0, 0);
    let mut work = StoreWork::default();
    assert!(
        stage
            .contains_admitted(key, &mut admission, &mut work)
            .expect("local presence needs no catalog lookup")
    );
    assert_eq!(work, StoreWork::default());
}

#[test]
fn admitted_object_stage_deduplication_is_fail_closed() {
    let mut base = MemoryPackedStore::default();
    let (key, bytes) = object(ObjectDomain::Owner, b"accepted-deduplication-object");
    let byte_count = bytes.len() as u64;
    let mut setup_work = StoreWork::default();
    base.stage(key, &bytes, &mut setup_work)
        .expect("base object must stage");
    base.seal_staged(16 * 1024, &mut setup_work)
        .expect("base object must seal");

    let mut stage = ObjectStage::new(&base);
    let mut admission = read_admission(0, 1, byte_count);
    let mut work = StoreWork::default();
    let error = stage
        .stage_admitted(key, &bytes, &mut admission, &mut work)
        .expect_err("deduplication may not read before catalog admission");
    assert_eq!(error.code, "object_read_catalog_lookups_exhausted");
    assert!(stage.is_empty());
    assert_eq!(work, StoreWork::default());

    let mut admission = read_admission(1, 1, byte_count);
    let mut work = StoreWork::default();
    assert_eq!(
        stage
            .stage_admitted(key, &bytes, &mut admission, &mut work)
            .expect("admitted deduplication must reuse the base object"),
        StageOutcome::Reused
    );
    assert!(stage.is_empty());
    assert_eq!(work.catalog_lookups, 1);
    assert_eq!(work.objects_read, 1);
    assert_eq!(work.bytes_read, byte_count);
    assert_eq!(work.objects_reused, 1);
}

#[test]
fn request_local_object_stage_shares_one_base_read_admission() {
    let mut base = MemoryPackedStore::default();
    let first = object(ObjectDomain::Owner, b"first-accepted-owner");
    let second = object(ObjectDomain::Owner, b"second-accepted-owner");
    let third = object(ObjectDomain::Owner, b"third-accepted-owner");
    let mut setup_work = StoreWork::default();
    for (key, bytes) in [&first, &second, &third] {
        base.stage(*key, bytes, &mut setup_work)
            .expect("base object must stage");
    }
    base.seal_staged(16 * 1024, &mut setup_work)
        .expect("base objects must seal");

    let admitted_bytes = (first.1.len() + second.1.len()) as u64;
    let mut stage = ObjectStage::with_limits_and_read_admission(
        &base,
        ObjectStageLimits {
            maximum_objects: 4,
            maximum_bytes: 4 * 1024,
            maximum_pages: 0,
        },
        read_admission(4, 2, admitted_bytes),
    );
    let mut work = StoreWork::default();
    assert_eq!(
        stage
            .stage(first.0, &first.1, &mut work)
            .expect("ordinary stage must use shared admission for base deduplication"),
        StageOutcome::Reused
    );
    assert_eq!(
        stage
            .read(second.0, second.1.len(), &mut work)
            .expect("ordinary read must use the same shared admission"),
        Some(second.1.clone())
    );
    let missing = ObjectKey::for_bytes(ObjectDomain::Owner, b"absent-owner");
    assert!(
        !stage
            .contains(missing, &mut work)
            .expect("ordinary contains must use the same shared admission")
    );

    let error = stage
        .read(third.0, third.1.len(), &mut work)
        .expect_err("next base payload must reject after cumulative object exhaustion");
    assert_eq!(error.code, "object_read_objects_exhausted");
    assert_eq!(work.catalog_lookups, 4);
    assert_eq!(work.packs_opened, 2);
    assert_eq!(work.objects_read, 2);
    assert_eq!(work.bytes_read, admitted_bytes);
    assert_eq!(work.objects_reused, 1);
    assert_eq!(
        stage.remaining_read_admission(),
        Some(read_admission(0, 0, 0).remaining())
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
    let descriptors = packs
        .iter()
        .map(|pack| {
            Ok((
                pack.id,
                PackDescriptor::from_metadata(pack.id, &pack.metadata)?,
            ))
        })
        .collect::<Result<BTreeMap<_, _>, super::object::StoreError>>()
        .expect("pack descriptors");
    let mut segment_bytes = Vec::new();
    let written = write_segment(
        &mut segment_bytes,
        0,
        1,
        &descriptors,
        build.catalog.entries().map(Ok),
    )
    .expect("segment must encode");
    let mut reader = Cursor::new(&segment_bytes);
    let metadata =
        read_segment_metadata(&mut reader, segment_bytes.len() as u64, written.metadata.id)
            .expect("segment metadata must decode");
    assert_eq!(metadata, written.metadata);
    let mut decoded = BTreeMap::new();
    for ordinal in 0..metadata.blocks.len() {
        for entry in metadata
            .read_block(&mut reader, ordinal)
            .expect("segment block must decode")
        {
            decoded.insert(entry.key, entry.location);
        }
    }
    assert_eq!(decoded.len(), build.catalog.len());
    let manifest = CatalogManifest::from_segments(
        1,
        CatalogHistory::default(),
        std::slice::from_ref(&metadata),
    )
    .expect("manifest must construct");
    let manifest_bytes = manifest.encode().expect("manifest must encode");
    let decoded_manifest = CatalogManifest::decode(&manifest_bytes).expect("manifest must decode");
    assert_eq!(decoded_manifest, manifest);
    let index = CatalogIndex::new(decoded_manifest, vec![metadata]).expect("index must bind");
    assert_eq!(index.len(), expected.len());
    assert_eq!(
        index.manifest().logical_commitment,
        build.catalog.logical_commitment()
    );
    let mut corrupt = manifest_bytes;
    let checksum = corrupt.len() - 40;
    corrupt[checksum] ^= 1;
    assert!(CatalogManifest::decode(&corrupt).is_err());
    let invalid = CatalogEntry {
        key: expected[0].0,
        location: CatalogLocation {
            pack: packs[0].id,
            offset: u64::MAX,
            length: 1,
            checksum: [0_u8; 32],
        },
    };
    let error = write_segment(&mut Vec::new(), 0, 1, &descriptors, [Ok(invalid)])
        .expect_err("catalog coordinates must not overflow");
    assert_eq!(error.code, "catalog_location_overflow");
    for (key, _) in expected {
        assert!(decoded.contains_key(&key));
    }
}

#[test]
fn catalog_segment_decoder_rejects_hostile_identity_layout_and_payload() {
    let (key, bytes) = object(ObjectDomain::Owner, b"strict-catalog-segment");
    let mut builder = PackBuilder::default();
    builder.insert(key, &bytes).expect("object must stage");
    let pack = builder.seal().expect("pack must seal");
    let descriptor = PackDescriptor::from_metadata(pack.id, &pack.metadata)
        .expect("pack descriptor must construct");
    let descriptors = BTreeMap::from([(pack.id, descriptor)]);
    let location = CatalogLocation {
        pack: pack.id,
        offset: pack.metadata.entries[0].offset,
        length: pack.metadata.entries[0].encoded_length,
        checksum: pack.metadata.entries[0].checksum,
    };
    let mut encoded = Vec::new();
    let written = write_segment(
        &mut encoded,
        0,
        1,
        &descriptors,
        [Ok(CatalogEntry { key, location })],
    )
    .expect("segment must encode");

    assert_eq!(
        SegmentId::parse_file_name(&written.metadata.id.file_name())
            .expect("canonical segment name"),
        written.metadata.id
    );
    assert!(SegmentId::parse_file_name(&written.metadata.id.file_name().to_uppercase()).is_err());
    assert!(SegmentId::parse_file_name("../segment_escape.lkjs").is_err());

    let truncated_length = encoded.len() as u64 - 1;
    assert!(
        read_segment_metadata(
            &mut Cursor::new(&encoded[..encoded.len() - 1]),
            truncated_length,
            written.metadata.id,
        )
        .is_err()
    );
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(
        read_segment_metadata(
            &mut Cursor::new(&trailing),
            trailing.len() as u64,
            written.metadata.id,
        )
        .is_err()
    );
    assert!(
        read_segment_metadata(
            &mut Cursor::new(&encoded),
            encoded.len() as u64,
            SegmentId::from_bytes([0x55; 32]),
        )
        .is_err()
    );

    let mut corrupted_payload = encoded;
    let first_entry = usize::try_from(written.metadata.blocks[0].offset)
        .expect("block offset must fit test platform");
    corrupted_payload[first_entry] ^= 1;
    let error = written
        .metadata
        .read_block(&mut Cursor::new(corrupted_payload), 0)
        .expect_err("authenticated block corruption must reject");
    assert_eq!(error.code, "catalog_block_checksum");

    let error = CatalogManifest::from_segments(
        1,
        CatalogHistory::default(),
        &[written.metadata.clone(), written.metadata],
    )
    .expect_err("duplicate segment level and identity must reject");
    assert_eq!(error.code, "catalog_manifest_order");
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
    assert_eq!(store.catalog_state(), CatalogState::Loaded);
    assert!(store.catalog_rebuild_note().is_none());
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
    assert_eq!(receipt.catalog_state, CatalogState::IncrementalPersisted);
    assert_eq!(receipt.catalog_work.full_rebuilds, 0);
    assert_eq!(receipt.catalog_work.full_footer_scan_runs, 0);
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
    assert!(PackDirectoryStore::open(&root).is_err());
    let rebuilt = PackDirectoryStore::recover_catalog(&root, "discarded corrupt fixture catalog")
        .expect("corrupt catalog must rebuild under exclusive recovery");
    assert_eq!(rebuilt.catalog_state(), CatalogState::RebuiltPersisted);
    assert!(rebuilt.catalog_rebuild_note().is_some());
    assert_eq!(
        rebuilt.catalog_observation().entries as usize,
        entries.len()
    );
    assert_eq!(rebuilt.catalog_work().full_footer_scan_runs, 1);
}

#[test]
fn incremental_catalog_merges_are_bounded_and_order_independent() {
    let temporary = tempfile::TempDir::new().expect("temporary catalog parent");
    let forward_root = temporary.path().join("forward");
    let reverse_root = temporary.path().join("reverse");
    let entries = (0..8_u8)
        .map(|ordinal| object(ObjectDomain::Owner, &[b'o', ordinal]))
        .collect::<Vec<_>>();

    for (root, reverse) in [(&forward_root, false), (&reverse_root, true)] {
        let mut store = PackDirectoryStore::initialize(root).expect("initialize catalog fixture");
        for ordinal in 0..entries.len() {
            let index = if reverse {
                entries.len() - ordinal - 1
            } else {
                ordinal
            };
            let (key, bytes) = &entries[index];
            let mut work = StoreWork::default();
            assert_eq!(
                store.stage(*key, bytes, &mut work).expect("stage delta"),
                StageOutcome::Inserted
            );
            let receipt = store
                .seal_staged(16 * 1024, &mut work)
                .expect("seal incremental delta");
            assert_eq!(receipt.catalog_work.full_rebuilds, 0);
            assert_eq!(receipt.catalog_work.full_footer_scan_runs, 0);
            drop(store);
            store = PackDirectoryStore::open(root).expect("healthy incremental reopen");
            assert_eq!(store.catalog_work().full_rebuilds, 0);
            assert_eq!(store.catalog_work().full_footer_scan_runs, 0);
        }
        let observation = store.catalog_observation();
        assert_eq!(observation.entries, entries.len() as u64);
        assert_eq!(observation.segments, 1);
        assert_eq!(observation.maximum_level, Some(3));
        assert_eq!(observation.history.delta_segments, entries.len() as u64);
        assert_eq!(observation.history.merge_operations, 7);
        assert_eq!(observation.history.full_rebuilds, 0);
        assert_eq!(observation.history.full_footer_scan_runs, 0);
        assert!(observation.leftovers.is_empty());
        let deep = store.deep_verify().expect("independent footer oracle");
        assert!(deep.catalog_equal);
        assert_eq!(deep.oracle_commitment, observation.commitment);
    }

    let forward = PackDirectoryStore::open(&forward_root)
        .expect("forward catalog")
        .catalog_observation();
    let reverse = PackDirectoryStore::open(&reverse_root)
        .expect("reverse catalog")
        .catalog_observation();
    assert_eq!(forward.commitment, reverse.commitment);
    assert_eq!(forward.entries, reverse.entries);
}

#[test]
fn admitted_directory_reads_reject_before_opening_pack_payload() {
    let temporary = tempfile::TempDir::new().expect("temporary store parent");
    let root = temporary.path().join("objects");
    let mut store = PackDirectoryStore::initialize(&root).expect("store must initialize");
    let (key, bytes) = object(ObjectDomain::Owner, b"directory-admission-owner");
    let byte_count = bytes.len() as u64;
    let mut setup_work = StoreWork::default();
    store
        .stage(key, &bytes, &mut setup_work)
        .expect("fixture object must stage");
    let receipt = store
        .seal_staged(16 * 1024, &mut setup_work)
        .expect("fixture object must seal");

    let mut admission = read_admission(1, 1, byte_count);
    let mut work = StoreWork::default();
    assert_eq!(
        store
            .read_admitted(key, bytes.len(), &mut admission, &mut work)
            .expect("exact directory allowance must read"),
        Some(bytes.clone())
    );
    assert_eq!(admission.remaining(), read_admission(0, 0, 0).remaining());
    assert_eq!(work.catalog_lookups, 1);
    assert_eq!(work.packs_opened, 1);
    assert_eq!(work.objects_read, 1);
    assert_eq!(work.bytes_read, byte_count);

    let pack_path = root.join("packs").join(receipt.packs[0].file_name());
    std::fs::rename(&pack_path, root.join("unavailable-pack-tripwire"))
        .expect("tripwire pack must move out of the store");

    let mut admission = read_admission(0, 1, byte_count);
    let mut work = StoreWork::default();
    let error = store
        .read_admitted(key, bytes.len(), &mut admission, &mut work)
        .expect_err("zero catalog allowance must reject before opening the missing pack");
    assert_eq!(error.code, "object_read_catalog_lookups_exhausted");
    assert_eq!(work, StoreWork::default());

    let mut admission = read_admission(1, 0, byte_count);
    let mut work = StoreWork::default();
    let error = store
        .read_admitted(key, bytes.len(), &mut admission, &mut work)
        .expect_err("zero object allowance must reject before opening the missing pack");
    assert_eq!(error.code, "object_read_objects_exhausted");
    assert_eq!(work.catalog_lookups, 1);
    assert_eq!(work.packs_opened, 0);
    assert_eq!(work.objects_read, 0);
    assert_eq!(work.bytes_read, 0);

    let mut admission = read_admission(1, 1, byte_count - 1);
    let mut work = StoreWork::default();
    let error = store
        .read_admitted(key, bytes.len(), &mut admission, &mut work)
        .expect_err("short byte allowance must reject before opening the missing pack");
    assert_eq!(error.code, "object_read_bytes_exhausted");
    assert_eq!(work.catalog_lookups, 1);
    assert_eq!(work.packs_opened, 0);
    assert_eq!(work.objects_read, 0);
    assert_eq!(work.bytes_read, 0);
    assert_eq!(admission.remaining().maximum_objects, 1);
}

#[test]
fn every_pack_seal_checkpoint_reopens_and_retries_safely() {
    let names = SealCheckpoint::ALL
        .into_iter()
        .map(SealCheckpoint::name)
        .collect::<BTreeSet<_>>();
    assert_eq!(names.len(), SealCheckpoint::ALL.len());
    for checkpoint in SealCheckpoint::ALL {
        let temporary = tempfile::TempDir::new().expect("temporary failpoint parent");
        let root = temporary.path().join(checkpoint.name());
        let mut store = PackDirectoryStore::initialize(&root).expect("initialize failpoint store");
        let (key, bytes) = object(ObjectDomain::Owner, checkpoint.name().as_bytes());
        let mut work = StoreWork::default();
        assert_eq!(
            store.stage(key, &bytes, &mut work).expect("stage fixture"),
            StageOutcome::Inserted
        );
        let error = store
            .seal_staged_with_fault(16 * 1024, &mut work, checkpoint)
            .expect_err("named checkpoint must interrupt sealing");
        assert_eq!(error.code, "pack_store_injected_interruption");
        assert!(error.message.contains(checkpoint.name()));
        drop(store);

        let catalog_visible = matches!(
            checkpoint,
            SealCheckpoint::ManifestPublished
                | SealCheckpoint::CatalogDirectorySynced
                | SealCheckpoint::ObsoleteSegmentsRemoved
                | SealCheckpoint::DerivedCleanupSynced
        );
        let leaves_pack_stage = matches!(
            checkpoint,
            SealCheckpoint::PackStageCreated
                | SealCheckpoint::PackPayloadWritten
                | SealCheckpoint::PackFooterWritten
                | SealCheckpoint::PackFileSynced
                | SealCheckpoint::PackPublished
        );
        let leaves_catalog_stage = matches!(
            checkpoint,
            SealCheckpoint::SegmentStageCreated
                | SealCheckpoint::SegmentBytesWritten
                | SealCheckpoint::SegmentFileSynced
                | SealCheckpoint::SegmentPublished
                | SealCheckpoint::SegmentStageRemoved
                | SealCheckpoint::SegmentDirectorySynced
                | SealCheckpoint::ManifestStageCreated
                | SealCheckpoint::ManifestBytesWritten
                | SealCheckpoint::ManifestFileSynced
        );
        let mut reopened = PackDirectoryStore::open(&root).expect("interrupted store must reopen");
        assert_eq!(
            !reopened.staging_leftovers().is_empty(),
            leaves_pack_stage,
            "wrong pack-stage classification at {checkpoint:?}"
        );
        assert_eq!(
            !reopened.catalog_leftovers().is_empty(),
            leaves_catalog_stage,
            "wrong catalog-stage classification at {checkpoint:?}"
        );
        assert_eq!(
            reopened
                .read(key, bytes.len(), &mut StoreWork::default())
                .expect("interrupted lookup"),
            catalog_visible.then_some(bytes.clone()),
            "wrong visible immutable object state at {checkpoint:?}"
        );

        let retry_outcome = reopened
            .stage(key, &bytes, &mut StoreWork::default())
            .expect("retry staging must be safe");
        assert_eq!(
            retry_outcome,
            if catalog_visible {
                StageOutcome::Reused
            } else {
                StageOutcome::Inserted
            }
        );
        reopened
            .seal_staged(16 * 1024, &mut StoreWork::default())
            .expect("retry seal must complete");
        assert_eq!(
            reopened
                .read(key, bytes.len(), &mut StoreWork::default())
                .expect("retried lookup"),
            Some(bytes)
        );
        let deep = reopened.deep_verify().expect("retried store must verify");
        assert_eq!(deep.packs, 1);
        assert_eq!(deep.objects, 1);
    }
}

#[test]
fn every_catalog_recovery_checkpoint_reopens_or_repairs_exactly() {
    const RECOVERY_CHECKPOINTS: [SealCheckpoint; 13] = [
        SealCheckpoint::SegmentStageCreated,
        SealCheckpoint::SegmentBytesWritten,
        SealCheckpoint::SegmentFileSynced,
        SealCheckpoint::SegmentPublished,
        SealCheckpoint::SegmentStageRemoved,
        SealCheckpoint::SegmentDirectorySynced,
        SealCheckpoint::ManifestStageCreated,
        SealCheckpoint::ManifestBytesWritten,
        SealCheckpoint::ManifestFileSynced,
        SealCheckpoint::ManifestPublished,
        SealCheckpoint::CatalogDirectorySynced,
        SealCheckpoint::ObsoleteSegmentsRemoved,
        SealCheckpoint::DerivedCleanupSynced,
    ];

    for checkpoint in RECOVERY_CHECKPOINTS {
        let temporary = tempfile::TempDir::new().expect("temporary recovery parent");
        let root = temporary.path().join(checkpoint.name());
        let mut store = PackDirectoryStore::initialize(&root).expect("initialize recovery store");
        let (key, bytes) = object(ObjectDomain::Owner, checkpoint.name().as_bytes());
        store
            .stage(key, &bytes, &mut StoreWork::default())
            .expect("stage recovery fixture");
        let sealed = store
            .seal_staged(16 * 1024, &mut StoreWork::default())
            .expect("seal recovery fixture");
        let expected_commitment = store.catalog_observation().commitment;
        let pack_path = root.join("packs").join(sealed.packs[0].file_name());
        let pack_before = std::fs::read(&pack_path).expect("read immutable pack before recovery");
        drop(store);

        std::fs::write(
            root.join("catalog/current.lkjc"),
            b"malformed recovery fixture",
        )
        .expect("replace disposable manifest");
        let error = PackDirectoryStore::recover_catalog_with_fault(
            &root,
            "fault-injected catalog repair",
            checkpoint,
        )
        .expect_err("recovery checkpoint must interrupt repair");
        assert_eq!(error.code, "pack_store_injected_interruption");

        let manifest_visible = matches!(
            checkpoint,
            SealCheckpoint::ManifestPublished
                | SealCheckpoint::CatalogDirectorySynced
                | SealCheckpoint::ObsoleteSegmentsRemoved
                | SealCheckpoint::DerivedCleanupSynced
        );
        match PackDirectoryStore::open(&root) {
            Ok(opened) if manifest_visible => {
                assert_eq!(opened.catalog_observation().commitment, expected_commitment);
                drop(opened);
            }
            Err(_) if !manifest_visible => {}
            outcome => panic!("unexpected recovery visibility {outcome:?} at {checkpoint:?}"),
        }

        let repaired = PackDirectoryStore::recover_catalog(&root, "retry interrupted repair")
            .expect("retry must reconstruct from immutable footers");
        assert_eq!(
            repaired.catalog_observation().commitment,
            expected_commitment
        );
        assert!(repaired.catalog_leftovers().is_empty());
        assert!(repaired.staging_leftovers().is_empty());
        assert_eq!(repaired.catalog_work().full_rebuilds, 1);
        assert_eq!(repaired.catalog_work().full_footer_scan_runs, 1);
        assert_eq!(
            repaired
                .read(key, bytes.len(), &mut StoreWork::default())
                .expect("read repaired object"),
            Some(bytes)
        );
        assert!(
            repaired
                .deep_verify()
                .expect("deep verify repair")
                .catalog_equal
        );
        assert_eq!(
            std::fs::read(pack_path).expect("read immutable pack after recovery"),
            pack_before
        );
    }
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
    let store = PackDirectoryStore::recover_catalog(&root, "duplicate fixture pack discovery")
        .expect("store must recover from immutable pack footers");
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
    assert!(PackDirectoryStore::recover_catalog(&root, "symlink fixture").is_err());

    std::fs::remove_dir_all(&root).expect("replace fixture store");
    drop(PackDirectoryStore::initialize(&root).expect("store must initialize"));
    let predecessor_name = PackId::from_bytes([0x55; 32]).file_name();
    std::fs::write(
        root.join("packs").join(predecessor_name),
        b"LKJGRPH4 predecessor",
    )
    .expect("predecessor fixture");
    let error = PackDirectoryStore::recover_catalog(&root, "predecessor fixture")
        .expect_err("predecessor bytes must reject");
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
    assert_eq!(store.catalog_observation().entries, 0);
}
