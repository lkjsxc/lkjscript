use super::*;
use crate::platform::storage::directory::PackDirectoryStore;
use crate::platform::storage::memory::MemoryPackedStore;
use crate::platform::storage::object::{
    ImmutableObjectStore, StoreReadAdmission, StoreReadLimits, StoreWork,
};
use std::io::{Cursor, Read, Seek, SeekFrom};

fn pack(values: &[Vec<u8>]) -> SealedPack {
    let mut builder = PackBuilder::default();
    for bytes in values {
        builder
            .insert(ObjectKey::for_bytes(ObjectDomain::Blob, bytes), bytes)
            .unwrap();
    }
    builder.seal_compressed().unwrap()
}

fn entry(sealed: &SealedPack, key: ObjectKey) -> &PackIndexEntry {
    sealed
        .metadata
        .entries
        .iter()
        .find(|entry| entry.key == key)
        .unwrap()
}

#[test]
fn compressed_packs_are_deterministic_and_both_footer_readers_admit_exact_objects() {
    let values = vec![
        vec![0; 32_768],
        (0..65_536).map(|n| (n % 251) as u8).collect(),
    ];
    let sealed = pack(&values);
    let mut reversed = values.clone();
    reversed.reverse();
    assert_eq!(sealed.bytes, pack(&reversed).bytes);
    assert_eq!(sealed.metadata.encoding, PackEncoding::Deflate);
    assert!(sealed.bytes.len() < 4096);
    assert_eq!(
        PackMetadata::decode(&sealed.bytes, true).unwrap(),
        sealed.metadata
    );
    let footer =
        PackMetadata::read_footer(&mut Cursor::new(&sealed.bytes), sealed.bytes.len() as u64)
            .unwrap();
    assert_eq!(footer.metadata, sealed.metadata);
    sealed.metadata.verify_all(&sealed.bytes).unwrap();
    sealed
        .metadata
        .verify_file(&mut Cursor::new(&sealed.bytes), sealed.id)
        .unwrap();
    for value in values {
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, &value);
        assert_eq!(
            sealed
                .metadata
                .read(&sealed.bytes, key, value.len())
                .unwrap(),
            Some(value.clone())
        );
        assert_eq!(
            sealed
                .metadata
                .read_from(&mut Cursor::new(&sealed.bytes), key, value.len())
                .unwrap(),
            Some(value)
        );
    }
}

#[test]
fn disk_and_memory_catalogs_charge_decoded_bytes_for_compressed_and_raw_entries() {
    for value in [vec![7; 32_768], b"short".to_vec(), Vec::new()] {
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, &value);
        let sealed = pack(std::slice::from_ref(&value));
        let directory = tempfile::tempdir().unwrap();
        drop(PackDirectoryStore::initialize(directory.path()).unwrap());
        std::fs::write(
            directory.path().join("packs").join(sealed.id.file_name()),
            &sealed.bytes,
        )
        .unwrap();
        let disk =
            PackDirectoryStore::recover_catalog(directory.path(), "compressed fixture").unwrap();
        let mut memory = MemoryPackedStore::default();
        memory.install(sealed).unwrap();
        memory.rebuild_catalog().unwrap();
        for store in [&disk as &dyn ImmutableObjectStore, &memory] {
            let mut work = StoreWork::default();
            let mut admission = StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: 1,
                maximum_objects: 1,
                maximum_bytes: value.len() as u64,
            });
            assert_eq!(
                store
                    .read_admitted(key, value.len(), &mut admission, &mut work)
                    .unwrap(),
                Some(value.clone())
            );
            assert_eq!(admission.remaining().maximum_bytes, 0);
            assert_eq!(work.bytes_read, value.len() as u64);
            if !value.is_empty() {
                let mut admission = StoreReadAdmission::new(StoreReadLimits {
                    maximum_catalog_lookups: 1,
                    maximum_objects: 1,
                    maximum_bytes: value.len() as u64 - 1,
                });
                let mut work = StoreWork::default();
                assert_eq!(
                    store
                        .read_admitted(key, value.len(), &mut admission, &mut work)
                        .unwrap_err()
                        .class,
                    StoreErrorClass::Resource
                );
                assert_eq!(work.packs_opened, 0);
                assert_eq!(work.objects_read, 0);
            }
        }
    }
}

#[test]
fn mixed_small_and_incompressible_entries_never_expand_compacted_payloads() {
    let mut state = 0x1234_5678_9abc_def0_u64;
    let noise = (0..4096)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect::<Vec<_>>();
    let values = vec![Vec::new(), b"tiny".to_vec(), noise, vec![0; 16_384]];
    let sealed = pack(&values);
    let mut raw = PackBuilder::default();
    for value in &values {
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, value);
        raw.insert(key, value).unwrap();
        let entry = entry(&sealed, key);
        assert!(entry.encoded_length <= entry.uncompressed_length);
        assert_eq!(
            sealed
                .metadata
                .read(&sealed.bytes, key, value.len())
                .unwrap()
                .as_ref(),
            Some(value)
        );
    }
    assert_eq!(
        entry(
            &sealed,
            ObjectKey::for_bytes(ObjectDomain::Blob, &values[1])
        )
        .encoded_length,
        4
    );
    assert_eq!(
        entry(
            &sealed,
            ObjectKey::for_bytes(ObjectDomain::Blob, &values[2])
        )
        .encoded_length,
        4096
    );
    assert!(sealed.bytes.len() < raw.seal().unwrap().bytes.len());
}

struct NoPayload;
impl Read for NoPayload {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        panic!("rejected decoded length read a payload")
    }
}
impl Seek for NoPayload {
    fn seek(&mut self, _: SeekFrom) -> std::io::Result<u64> {
        panic!("rejected decoded length sought a payload")
    }
}

#[test]
fn decoded_caller_limit_is_checked_before_payload_io() {
    let value = vec![0; 32_768];
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, &value);
    let sealed = pack(std::slice::from_ref(&value));
    assert!(sealed.metadata.entries[0].encoded_length < 100);
    let error = sealed
        .metadata
        .read_from(&mut NoPayload, key, value.len() - 1)
        .unwrap_err();
    assert_eq!(error.code, "pack_read_limit");
}

// Deliberately rebind every physical checksum so malformed streams reach the decoder.
fn replace_payload(original: &SealedPack, encoded: &[u8], declared: u64) -> Vec<u8> {
    assert_eq!(original.metadata.entries.len(), 1);
    let entry = PackIndexEntry {
        encoded_length: encoded.len() as u64,
        uncompressed_length: declared,
        checksum: digest(contract::PACK_ENTRY_CHECKSUM_DOMAIN, encoded),
        ..original.metadata.entries[0].clone()
    };
    let nonce = nonce_for_entries(std::slice::from_ref(&entry));
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&contract::PACK_MAGIC);
    bytes.extend_from_slice(&contract::PACK_CONTRACT_VERSION.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&nonce);
    bytes.extend_from_slice(encoded);
    let index_start = bytes.len();
    bytes.push(entry.key.domain.tag());
    bytes.extend_from_slice(&entry.key.digest.bytes());
    bytes.extend_from_slice(&entry.offset.to_be_bytes());
    bytes.extend_from_slice(&entry.encoded_length.to_be_bytes());
    bytes.extend_from_slice(&entry.uncompressed_length.to_be_bytes());
    bytes.extend_from_slice(&entry.checksum);
    let checksum = digest(contract::PACK_INDEX_CHECKSUM_DOMAIN, &bytes[index_start..]);
    let footer = bytes.len() as u64;
    bytes.extend_from_slice(&contract::PACK_INDEX_MAGIC);
    bytes.extend_from_slice(&contract::PACK_CONTRACT_VERSION.to_be_bytes());
    bytes.extend_from_slice(&1_u16.to_be_bytes());
    bytes.extend_from_slice(&1_u64.to_be_bytes());
    bytes.extend_from_slice(&(encoded.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&(INDEX_ENTRY_BYTES as u64).to_be_bytes());
    bytes.extend_from_slice(&checksum);
    let checksum = digest(contract::PACK_CHECKSUM_DOMAIN, &bytes);
    bytes.extend_from_slice(&footer.to_be_bytes());
    bytes.extend_from_slice(&checksum);
    bytes.extend_from_slice(&contract::PACK_END_MAGIC);
    bytes
}

#[test]
fn coherently_hashed_truncation_trailing_streams_and_expansion_are_rejected() {
    let value = vec![b'x'; 32_768];
    let sealed = pack(std::slice::from_ref(&value));
    let entry = &sealed.metadata.entries[0];
    let encoded =
        &sealed.bytes[entry.offset as usize..(entry.offset + entry.encoded_length) as usize];
    let mut trailing = encoded.to_vec();
    trailing.push(0);
    let mut concatenated = encoded.to_vec();
    concatenated.extend_from_slice(encoded);
    let cases = [
        (encoded[..encoded.len() - 1].to_vec(), value.len() as u64),
        (trailing, value.len() as u64),
        (concatenated, value.len() as u64),
        (encoded.to_vec(), value.len() as u64 - 1),
        (encoded.to_vec(), value.len() as u64 + 1),
    ];
    for (encoded, length) in cases {
        let bytes = replace_payload(&sealed, &encoded, length);
        let metadata = PackMetadata::decode(&bytes, true).unwrap();
        assert_eq!(
            metadata,
            PackMetadata::read_footer(&mut Cursor::new(&bytes), bytes.len() as u64)
                .unwrap()
                .metadata
        );
        let error = metadata
            .read(&bytes, entry.key, length as usize)
            .unwrap_err();
        assert!(error.code.starts_with("pack_decompression"), "{error:?}");
        assert!(metadata.verify_all(&bytes).is_err());
    }
    let excessive = replace_payload(
        &sealed,
        encoded,
        contract::MAXIMUM_PACK_ENTRY_BYTES as u64 + 1,
    );
    assert_eq!(
        PackMetadata::decode(&excessive, true).unwrap_err().class,
        StoreErrorClass::Resource
    );
    assert_eq!(
        PackMetadata::read_footer(&mut Cursor::new(&excessive), excessive.len() as u64)
            .unwrap_err()
            .class,
        StoreErrorClass::Resource
    );
}

#[test]
fn unknown_or_mismatched_codecs_reject_before_object_decoding() {
    let sealed = pack(&[b"codec".to_vec()]);
    let footer_start = sealed.bytes.len() - TRAILER_BYTES - FOOTER_BYTES;
    for (offset, flag) in [(10, 2_u16), (footer_start + 10, 0)] {
        let mut bytes = sealed.bytes.clone();
        bytes[offset..offset + 2].copy_from_slice(&flag.to_be_bytes());
        assert!(PackMetadata::decode(&bytes, false).is_err());
        assert!(PackMetadata::read_footer(&mut Cursor::new(&bytes), bytes.len() as u64).is_err());
    }
}
