//! Rebuildable contract-2 object-location manifests and immutable sorted segments.

use super::contract;
use super::object::{ObjectDomain, ObjectKey, StoreError, StoreErrorClass};
use super::pack::{PackId, PackMetadata};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io::{Read, Seek, SeekFrom, Write};

pub(crate) const SEGMENT_HEADER_BYTES: usize = 8 + 2 + 2;
pub(crate) const CATALOG_ENTRY_BYTES: usize = 1 + 32 + 32 + 8 + 8 + 32;
const PACK_DESCRIPTOR_BYTES: usize = 32 + 8 + 16 + 8 + 32 + 32 + 8;
const BLOCK_DESCRIPTOR_BYTES: usize =
    1 + 32 + 1 + 32 + 8 + 8 + 32 + contract::CATALOG_BLOCK_FILTER_BYTES;
const SEGMENT_METADATA_HEADER_BYTES: usize =
    8 + 2 + 2 + 2 + 2 + 8 + 8 + 8 + 8 + 1 + 32 + 1 + 32 + 32;
const SEGMENT_FOOTER_BYTES: usize = 8 + 8 + 32 + 8;
const MANIFEST_HEADER_BYTES: usize = 8 + 2 + 2 + 8 + 8 + 8 + 8 + 32 + 32 + (9 * 8);
const MANIFEST_SEGMENT_BYTES: usize = 2 + 2 + 8 + 8 + 8 + 1 + 32 + 1 + 32 + 32 + 8 + 8 + 32;
const MANIFEST_TRAILER_BYTES: usize = 32 + 8;
const FILTER_HASHES: u64 = 7;
const FILTER_BITS: u64 = (contract::CATALOG_BLOCK_FILTER_BYTES * 8) as u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogLocation {
    pub pack: PackId,
    pub offset: u64,
    pub length: u64,
    pub checksum: [u8; 32],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct CatalogEntry {
    pub key: ObjectKey,
    pub location: CatalogLocation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateObject {
    pub key: ObjectKey,
    pub packs: Vec<PackId>,
}

/// Recovery-only complete in-memory catalog rebuilt from immutable pack footers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectCatalog {
    locations: BTreeMap<ObjectKey, CatalogLocation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogBuild {
    pub catalog: ObjectCatalog,
    pub duplicates: Vec<DuplicateObject>,
}

impl ObjectCatalog {
    pub fn empty() -> Self {
        Self {
            locations: BTreeMap::new(),
        }
    }

    pub fn rebuild<'a>(
        packs: impl IntoIterator<Item = (PackId, &'a PackMetadata)>,
    ) -> Result<CatalogBuild, StoreError> {
        let mut descriptors = packs.into_iter().collect::<Vec<_>>();
        if descriptors.len() > contract::MAXIMUM_CATALOG_PACKS {
            return Err(resource(
                "catalog_pack_count",
                "footer reconstruction exceeds the catalog pack bound",
            ));
        }
        descriptors.sort_by_key(|(pack, _)| *pack);
        let mut locations = BTreeMap::<ObjectKey, CatalogLocation>::new();
        let mut duplicate_packs = BTreeMap::<ObjectKey, Vec<PackId>>::new();
        let mut visited = 0_usize;
        for (pack, metadata) in descriptors {
            for entry in &metadata.entries {
                visited = visited.checked_add(1).ok_or_else(|| {
                    resource(
                        "catalog_entry_count",
                        "footer reconstruction entry count overflows",
                    )
                })?;
                if visited > contract::MAXIMUM_CATALOG_ENTRIES {
                    return Err(resource(
                        "catalog_entry_count",
                        "footer reconstruction exceeds the catalog entry bound",
                    ));
                }
                let location = CatalogLocation {
                    pack,
                    offset: entry.offset,
                    length: entry.encoded_length,
                    checksum: entry.checksum,
                };
                validate_location(entry.key, location)?;
                match locations.get_mut(&entry.key) {
                    None => {
                        locations.insert(entry.key, location);
                    }
                    Some(existing) => {
                        let packs = duplicate_packs
                            .entry(entry.key)
                            .or_insert_with(|| vec![existing.pack]);
                        packs.push(pack);
                        if pack < existing.pack {
                            *existing = location;
                        }
                    }
                }
            }
        }
        let duplicates = duplicate_packs
            .into_iter()
            .map(|(key, mut packs)| {
                packs.sort();
                packs.dedup();
                DuplicateObject { key, packs }
            })
            .collect();
        Ok(CatalogBuild {
            catalog: Self { locations },
            duplicates,
        })
    }

    pub fn len(&self) -> usize {
        self.locations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.locations.is_empty()
    }

    pub fn get(&self, key: ObjectKey) -> Option<CatalogLocation> {
        self.locations.get(&key).copied()
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = CatalogEntry> + '_ {
        self.locations.iter().map(|(key, location)| CatalogEntry {
            key: *key,
            location: *location,
        })
    }

    pub(crate) fn logical_sum(&self) -> [u8; 32] {
        let mut sum = [0_u8; 32];
        for entry in self.entries() {
            add_logical_entry(&mut sum, entry);
        }
        sum
    }

    pub(crate) fn logical_commitment(&self) -> CatalogCommitment {
        logical_commitment(self.len() as u64, self.logical_sum())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CatalogWork {
    pub healthy_opens: u64,
    pub manifests_read: u64,
    pub manifest_bytes_read: u64,
    pub segment_metadata_read: u64,
    pub segment_metadata_bytes_read: u64,
    pub segment_lookups: u64,
    pub segment_blocks_read: u64,
    pub segment_block_bytes_read: u64,
    pub segment_entries_examined: u64,
    pub targeted_pack_footers_read: u64,
    pub targeted_pack_footer_bytes_read: u64,
    pub delta_segments_written: u64,
    pub merge_operations: u64,
    pub merge_entries_read: u64,
    pub merge_bytes_read: u64,
    pub segments_written: u64,
    pub segment_entries_written: u64,
    pub manifests_written: u64,
    pub obsolete_segments_removed: u64,
    pub full_rebuilds: u64,
    pub full_footer_scan_runs: u64,
    pub pack_footers_scanned: u64,
}

impl CatalogWork {
    pub fn add(&mut self, other: Self) {
        self.healthy_opens = self.healthy_opens.saturating_add(other.healthy_opens);
        self.manifests_read = self.manifests_read.saturating_add(other.manifests_read);
        self.manifest_bytes_read = self
            .manifest_bytes_read
            .saturating_add(other.manifest_bytes_read);
        self.segment_metadata_read = self
            .segment_metadata_read
            .saturating_add(other.segment_metadata_read);
        self.segment_metadata_bytes_read = self
            .segment_metadata_bytes_read
            .saturating_add(other.segment_metadata_bytes_read);
        self.segment_lookups = self.segment_lookups.saturating_add(other.segment_lookups);
        self.segment_blocks_read = self
            .segment_blocks_read
            .saturating_add(other.segment_blocks_read);
        self.segment_block_bytes_read = self
            .segment_block_bytes_read
            .saturating_add(other.segment_block_bytes_read);
        self.segment_entries_examined = self
            .segment_entries_examined
            .saturating_add(other.segment_entries_examined);
        self.targeted_pack_footers_read = self
            .targeted_pack_footers_read
            .saturating_add(other.targeted_pack_footers_read);
        self.targeted_pack_footer_bytes_read = self
            .targeted_pack_footer_bytes_read
            .saturating_add(other.targeted_pack_footer_bytes_read);
        self.delta_segments_written = self
            .delta_segments_written
            .saturating_add(other.delta_segments_written);
        self.merge_operations = self.merge_operations.saturating_add(other.merge_operations);
        self.merge_entries_read = self
            .merge_entries_read
            .saturating_add(other.merge_entries_read);
        self.merge_bytes_read = self.merge_bytes_read.saturating_add(other.merge_bytes_read);
        self.segments_written = self.segments_written.saturating_add(other.segments_written);
        self.segment_entries_written = self
            .segment_entries_written
            .saturating_add(other.segment_entries_written);
        self.manifests_written = self
            .manifests_written
            .saturating_add(other.manifests_written);
        self.obsolete_segments_removed = self
            .obsolete_segments_removed
            .saturating_add(other.obsolete_segments_removed);
        self.full_rebuilds = self.full_rebuilds.saturating_add(other.full_rebuilds);
        self.full_footer_scan_runs = self
            .full_footer_scan_runs
            .saturating_add(other.full_footer_scan_runs);
        self.pack_footers_scanned = self
            .pack_footers_scanned
            .saturating_add(other.pack_footers_scanned);
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CatalogHistory {
    pub delta_segments: u64,
    pub merge_operations: u64,
    pub merge_entries_read: u64,
    pub merge_bytes_read: u64,
    pub segments_written: u64,
    pub segment_entries_written: u64,
    pub full_rebuilds: u64,
    pub full_footer_scan_runs: u64,
    pub pack_footers_scanned: u64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SegmentId([u8; 32]);

impl SegmentId {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn file_name(self) -> String {
        format!(
            "segment_{}.lkjs",
            crate::platform::semantic_id::encode_hex(&self.0)
        )
    }

    pub fn parse_file_name(value: &str) -> Result<Self, StoreError> {
        let encoded = value
            .strip_prefix("segment_")
            .and_then(|value| value.strip_suffix(".lkjs"))
            .ok_or_else(|| corrupt("catalog_segment_name", "segment file name is not canonical"))?;
        Ok(Self(parse_lower_hex_32(encoded, "catalog_segment_name")?))
    }
}

impl fmt::Display for SegmentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("catalog_segment_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogCommitment([u8; 32]);

impl CatalogCommitment {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for CatalogCommitment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("catalog_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PackDescriptor {
    pub pack: PackId,
    pub byte_length: u64,
    pub nonce: [u8; 16],
    pub payload_bytes: u64,
    pub index_checksum: [u8; 32],
    pub pack_checksum: [u8; 32],
    pub entries: u64,
}

impl PackDescriptor {
    pub fn from_metadata(pack: PackId, metadata: &PackMetadata) -> Result<Self, StoreError> {
        let entries = u64::try_from(metadata.entries.len()).map_err(|_| {
            resource(
                "catalog_pack_entries",
                "pack entry count does not fit the catalog contract",
            )
        })?;
        let descriptor = Self {
            pack,
            byte_length: metadata.byte_length,
            nonce: metadata.nonce,
            payload_bytes: metadata.payload_bytes,
            index_checksum: metadata.index_checksum,
            pack_checksum: metadata.pack_checksum,
            entries,
        };
        descriptor.validate()?;
        Ok(descriptor)
    }

    pub fn matches(&self, metadata: &PackMetadata) -> bool {
        self.byte_length == metadata.byte_length
            && self.nonce == metadata.nonce
            && self.payload_bytes == metadata.payload_bytes
            && self.index_checksum == metadata.index_checksum
            && self.pack_checksum == metadata.pack_checksum
            && usize::try_from(self.entries).ok() == Some(metadata.entries.len())
    }

    fn validate(&self) -> Result<(), StoreError> {
        if self.byte_length == 0
            || self.byte_length > contract::MAXIMUM_PACK_BYTES as u64
            || self.payload_bytes > self.byte_length
            || self.entries == 0
            || self.entries > contract::MAXIMUM_PACK_ENTRIES as u64
        {
            return Err(resource(
                "catalog_pack_descriptor",
                "catalog pack descriptor is outside typed bounds",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CatalogBlock {
    pub first: ObjectKey,
    pub last: ObjectKey,
    pub offset: u64,
    pub count: u64,
    pub checksum: [u8; 32],
    pub filter: [u8; contract::CATALOG_BLOCK_FILTER_BYTES],
}

impl CatalogBlock {
    pub fn might_contain(&self, key: ObjectKey) -> bool {
        filter_contains(&self.filter, key)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SegmentMetadata {
    pub id: SegmentId,
    pub level: u16,
    pub generation: u64,
    pub entry_count: u64,
    pub first: ObjectKey,
    pub last: ObjectKey,
    pub logical_sum: [u8; 32],
    pub packs: BTreeMap<PackId, PackDescriptor>,
    pub blocks: Vec<CatalogBlock>,
    pub file_bytes: u64,
    pub metadata_bytes: u64,
}

impl SegmentMetadata {
    pub fn descriptor(&self) -> ManifestSegment {
        ManifestSegment {
            id: self.id,
            level: self.level,
            generation: self.generation,
            entry_count: self.entry_count,
            pack_count: self.packs.len() as u64,
            first: self.first,
            last: self.last,
            file_bytes: self.file_bytes,
            metadata_bytes: self.metadata_bytes,
            logical_sum: self.logical_sum,
        }
    }

    pub fn find_block(&self, key: ObjectKey) -> Option<usize> {
        let mut low = 0_usize;
        let mut high = self.blocks.len();
        while low < high {
            let middle = low + (high - low) / 2;
            let block = self.blocks.get(middle)?;
            if key < block.first {
                high = middle;
            } else if key > block.last {
                low = middle + 1;
            } else {
                return Some(middle);
            }
        }
        None
    }

    pub fn read_block<R: Read + Seek>(
        &self,
        reader: &mut R,
        block_index: usize,
    ) -> Result<Vec<CatalogEntry>, StoreError> {
        let block = self.blocks.get(block_index).ok_or_else(|| {
            corrupt(
                "catalog_block_index",
                "catalog block index is outside bounds",
            )
        })?;
        let count = usize_from_u64(block.count, "catalog_block_count")?;
        let byte_count = count
            .checked_mul(CATALOG_ENTRY_BYTES)
            .ok_or_else(|| corrupt("catalog_block_size", "catalog block byte size overflows"))?;
        let mut bytes = vec![0_u8; byte_count];
        read_at(
            reader,
            block.offset,
            &mut bytes,
            "catalog_segment_block_read",
        )?;
        if digest(contract::CATALOG_BLOCK_CHECKSUM_DOMAIN, &bytes) != block.checksum {
            return Err(corrupt(
                "catalog_block_checksum",
                "catalog segment block checksum does not match",
            ));
        }
        let mut entries = Vec::with_capacity(count);
        let mut previous = None;
        for ordinal in 0..count {
            let offset = ordinal * CATALOG_ENTRY_BYTES;
            let entry = decode_entry(&bytes[offset..offset + CATALOG_ENTRY_BYTES])?;
            validate_location(entry.key, entry.location)?;
            if !self.packs.contains_key(&entry.location.pack) {
                return Err(corrupt(
                    "catalog_block_pack",
                    "catalog entry references a pack absent from segment metadata",
                ));
            }
            if previous.is_some_and(|value| value >= entry.key) {
                return Err(corrupt(
                    "catalog_block_order",
                    "catalog block keys are not strictly ordered",
                ));
            }
            previous = Some(entry.key);
            entries.push(entry);
        }
        if entries.first().map(|entry| entry.key) != Some(block.first)
            || entries.last().map(|entry| entry.key) != Some(block.last)
            || block_filter(&entries) != block.filter
        {
            return Err(corrupt(
                "catalog_block_metadata",
                "catalog block contents disagree with their authenticated range or filter",
            ));
        }
        Ok(entries)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ManifestSegment {
    pub id: SegmentId,
    pub level: u16,
    pub generation: u64,
    pub entry_count: u64,
    pub pack_count: u64,
    pub first: ObjectKey,
    pub last: ObjectKey,
    pub file_bytes: u64,
    pub metadata_bytes: u64,
    pub logical_sum: [u8; 32],
}

impl ManifestSegment {
    fn matches(&self, metadata: &SegmentMetadata) -> bool {
        *self == metadata.descriptor()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CatalogManifest {
    pub generation: u64,
    pub segments: Vec<ManifestSegment>,
    pub total_entries: u64,
    pub total_packs: u64,
    pub logical_sum: [u8; 32],
    pub logical_commitment: CatalogCommitment,
    pub history: CatalogHistory,
}

impl CatalogManifest {
    pub fn empty() -> Self {
        Self {
            generation: 0,
            segments: Vec::new(),
            total_entries: 0,
            total_packs: 0,
            logical_sum: [0_u8; 32],
            logical_commitment: logical_commitment(0, [0_u8; 32]),
            history: CatalogHistory::default(),
        }
    }

    pub fn from_segments(
        generation: u64,
        history: CatalogHistory,
        segments: &[SegmentMetadata],
    ) -> Result<Self, StoreError> {
        if segments.len() > contract::MAXIMUM_CATALOG_SEGMENTS {
            return Err(resource(
                "catalog_manifest_segments",
                "catalog manifest exceeds the live segment bound",
            ));
        }
        let mut ordered = segments.iter().collect::<Vec<_>>();
        ordered.sort_by_key(|segment| segment.level);
        let mut summaries = Vec::with_capacity(ordered.len());
        let mut total_entries = 0_u64;
        let mut all_packs = BTreeSet::new();
        let mut logical_sum = [0_u8; 32];
        let mut previous_level = None;
        let mut ids = BTreeSet::new();
        for segment in ordered {
            if segment.level > contract::MAXIMUM_CATALOG_LEVEL
                || segment.generation > generation
                || previous_level.is_some_and(|level| level >= segment.level)
                || !ids.insert(segment.id)
            {
                return Err(corrupt(
                    "catalog_manifest_order",
                    "catalog segment levels, generations, or identities are invalid",
                ));
            }
            previous_level = Some(segment.level);
            total_entries = total_entries
                .checked_add(segment.entry_count)
                .ok_or_else(|| resource("catalog_manifest_entries", "catalog entries overflow"))?;
            if total_entries > contract::MAXIMUM_CATALOG_ENTRIES as u64 {
                return Err(resource(
                    "catalog_manifest_entries",
                    "catalog manifest exceeds the entry bound",
                ));
            }
            for pack in segment.packs.keys() {
                if !all_packs.insert(*pack) {
                    return Err(corrupt(
                        "catalog_manifest_pack_duplicate",
                        "one pack descriptor appears in multiple live segments",
                    ));
                }
            }
            add_sum(&mut logical_sum, segment.logical_sum);
            summaries.push(segment.descriptor());
        }
        if all_packs.len() > contract::MAXIMUM_CATALOG_PACKS {
            return Err(resource(
                "catalog_manifest_packs",
                "catalog manifest exceeds the pack bound",
            ));
        }
        Ok(Self {
            generation,
            segments: summaries,
            total_entries,
            total_packs: all_packs.len() as u64,
            logical_sum,
            logical_commitment: logical_commitment(total_entries, logical_sum),
            history,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, StoreError> {
        self.validate_summary()?;
        let entries_bytes = self
            .segments
            .len()
            .checked_mul(MANIFEST_SEGMENT_BYTES)
            .ok_or_else(|| corrupt("catalog_manifest_size", "manifest byte size overflows"))?;
        let capacity = MANIFEST_HEADER_BYTES
            .checked_add(entries_bytes)
            .and_then(|value| value.checked_add(MANIFEST_TRAILER_BYTES))
            .ok_or_else(|| corrupt("catalog_manifest_size", "manifest byte size overflows"))?;
        if capacity > contract::MAXIMUM_CATALOG_MANIFEST_BYTES {
            return Err(resource(
                "catalog_manifest_size",
                "catalog manifest exceeds its byte bound",
            ));
        }
        let mut bytes = Vec::with_capacity(capacity);
        bytes.extend_from_slice(&contract::CATALOG_MANIFEST_MAGIC);
        bytes.extend_from_slice(&contract::CATALOG_CONTRACT_VERSION.to_be_bytes());
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&self.generation.to_be_bytes());
        bytes.extend_from_slice(&(self.segments.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&self.total_entries.to_be_bytes());
        bytes.extend_from_slice(&self.total_packs.to_be_bytes());
        bytes.extend_from_slice(&self.logical_sum);
        bytes.extend_from_slice(&self.logical_commitment.bytes());
        encode_history(&mut bytes, self.history);
        for segment in &self.segments {
            encode_manifest_segment(&mut bytes, *segment);
        }
        let checksum = digest(contract::CATALOG_MANIFEST_CHECKSUM_DOMAIN, &bytes);
        bytes.extend_from_slice(&checksum);
        bytes.extend_from_slice(&contract::CATALOG_MANIFEST_END_MAGIC);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, StoreError> {
        if bytes.len() < MANIFEST_HEADER_BYTES + MANIFEST_TRAILER_BYTES
            || bytes.len() > contract::MAXIMUM_CATALOG_MANIFEST_BYTES
        {
            return Err(resource(
                "catalog_manifest_size",
                "catalog manifest byte length is outside bounds",
            ));
        }
        if bytes[..8] != contract::CATALOG_MANIFEST_MAGIC
            || read_u16(bytes, 8)? != contract::CATALOG_CONTRACT_VERSION
            || read_u16(bytes, 10)? != 0
        {
            return Err(corrupt(
                "catalog_manifest_contract",
                "catalog manifest magic, version, or flags are not current",
            ));
        }
        let count = usize_from_u64(read_u64(bytes, 20)?, "catalog_manifest_segments")?;
        if count > contract::MAXIMUM_CATALOG_SEGMENTS {
            return Err(resource(
                "catalog_manifest_segments",
                "catalog manifest exceeds the live segment bound",
            ));
        }
        let expected = MANIFEST_HEADER_BYTES
            .checked_add(
                count
                    .checked_mul(MANIFEST_SEGMENT_BYTES)
                    .ok_or_else(|| corrupt("catalog_manifest_size", "manifest size overflows"))?,
            )
            .and_then(|value| value.checked_add(MANIFEST_TRAILER_BYTES))
            .ok_or_else(|| corrupt("catalog_manifest_size", "manifest size overflows"))?;
        if expected != bytes.len() {
            return Err(corrupt(
                "catalog_manifest_length",
                "catalog manifest count does not describe its exact bytes",
            ));
        }
        let trailer = bytes.len() - MANIFEST_TRAILER_BYTES;
        if read_array::<8>(bytes, trailer + 32)? != contract::CATALOG_MANIFEST_END_MAGIC
            || read_array::<32>(bytes, trailer)?
                != digest(
                    contract::CATALOG_MANIFEST_CHECKSUM_DOMAIN,
                    &bytes[..trailer],
                )
        {
            return Err(corrupt(
                "catalog_manifest_checksum",
                "catalog manifest checksum or closing marker is invalid",
            ));
        }
        let generation = read_u64(bytes, 12)?;
        let total_entries = read_u64(bytes, 28)?;
        let total_packs = read_u64(bytes, 36)?;
        let logical_sum = read_array::<32>(bytes, 44)?;
        let logical_commitment = CatalogCommitment::from_bytes(read_array::<32>(bytes, 76)?);
        let history = decode_history(bytes, 108)?;
        let mut segments = Vec::with_capacity(count);
        let mut offset = MANIFEST_HEADER_BYTES;
        for _ in 0..count {
            segments.push(decode_manifest_segment(
                &bytes[offset..offset + MANIFEST_SEGMENT_BYTES],
            )?);
            offset += MANIFEST_SEGMENT_BYTES;
        }
        let manifest = Self {
            generation,
            segments,
            total_entries,
            total_packs,
            logical_sum,
            logical_commitment,
            history,
        };
        manifest.validate_summary()?;
        Ok(manifest)
    }

    fn validate_summary(&self) -> Result<(), StoreError> {
        if self.segments.len() > contract::MAXIMUM_CATALOG_SEGMENTS
            || self.total_entries > contract::MAXIMUM_CATALOG_ENTRIES as u64
            || self.total_packs > contract::MAXIMUM_CATALOG_PACKS as u64
            || self.logical_commitment != logical_commitment(self.total_entries, self.logical_sum)
        {
            return Err(corrupt(
                "catalog_manifest_summary",
                "catalog manifest totals or logical commitment are invalid",
            ));
        }
        let mut prior = None;
        let mut ids = BTreeSet::new();
        for segment in &self.segments {
            if segment.level > contract::MAXIMUM_CATALOG_LEVEL
                || segment.generation > self.generation
                || segment.entry_count == 0
                || segment.entry_count > contract::MAXIMUM_CATALOG_ENTRIES as u64
                || segment.pack_count == 0
                || segment.pack_count > contract::MAXIMUM_CATALOG_PACKS as u64
                || segment.first > segment.last
                || segment.file_bytes > contract::MAXIMUM_CATALOG_SEGMENT_BYTES as u64
                || segment.metadata_bytes > contract::MAXIMUM_CATALOG_SEGMENT_METADATA_BYTES as u64
                || prior.is_some_and(|level| level >= segment.level)
                || !ids.insert(segment.id)
            {
                return Err(corrupt(
                    "catalog_manifest_segment",
                    "catalog manifest segment descriptor is noncanonical",
                ));
            }
            prior = Some(segment.level);
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct CatalogIndex {
    manifest: CatalogManifest,
    segments: Vec<SegmentMetadata>,
    packs: BTreeMap<PackId, PackDescriptor>,
}

impl CatalogIndex {
    pub fn new(
        manifest: CatalogManifest,
        mut segments: Vec<SegmentMetadata>,
    ) -> Result<Self, StoreError> {
        segments.sort_by_key(|segment| segment.level);
        if manifest.segments.len() != segments.len() {
            return Err(corrupt(
                "catalog_manifest_selection",
                "manifest segment selection is incomplete",
            ));
        }
        let mut packs = BTreeMap::new();
        for (summary, metadata) in manifest.segments.iter().zip(&segments) {
            if !summary.matches(metadata) {
                return Err(corrupt(
                    "catalog_manifest_segment_binding",
                    "manifest segment descriptor disagrees with immutable segment metadata",
                ));
            }
            for (pack, descriptor) in &metadata.packs {
                if packs.insert(*pack, descriptor.clone()).is_some() {
                    return Err(corrupt(
                        "catalog_manifest_pack_duplicate",
                        "one pack appears in multiple selected segments",
                    ));
                }
            }
        }
        if packs.len() as u64 != manifest.total_packs {
            return Err(corrupt(
                "catalog_manifest_pack_total",
                "manifest pack total disagrees with selected segments",
            ));
        }
        let rebuilt =
            CatalogManifest::from_segments(manifest.generation, manifest.history, &segments)?;
        if rebuilt != manifest {
            return Err(corrupt(
                "catalog_manifest_aggregate",
                "manifest aggregate disagrees with selected segment metadata",
            ));
        }
        Ok(Self {
            manifest,
            segments,
            packs,
        })
    }

    pub fn empty() -> Self {
        Self {
            manifest: CatalogManifest::empty(),
            segments: Vec::new(),
            packs: BTreeMap::new(),
        }
    }

    pub const fn manifest(&self) -> &CatalogManifest {
        &self.manifest
    }

    pub fn segments(&self) -> &[SegmentMetadata] {
        &self.segments
    }

    pub fn pack(&self, pack: PackId) -> Option<&PackDescriptor> {
        self.packs.get(&pack)
    }

    pub fn len(&self) -> usize {
        match usize::try_from(self.manifest.total_entries) {
            Ok(value) => value,
            Err(_) => usize::MAX,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.manifest.total_entries == 0
    }
}

#[derive(Debug)]
pub(crate) struct WrittenSegment {
    pub metadata: SegmentMetadata,
    pub bytes_written: u64,
}

pub(crate) fn write_segment<W, I>(
    writer: &mut W,
    level: u16,
    generation: u64,
    descriptors: &BTreeMap<PackId, PackDescriptor>,
    entries: I,
) -> Result<WrittenSegment, StoreError>
where
    W: Write,
    I: IntoIterator<Item = Result<CatalogEntry, StoreError>>,
{
    if level > contract::MAXIMUM_CATALOG_LEVEL {
        return Err(resource(
            "catalog_segment_level",
            "catalog segment level exceeds its bound",
        ));
    }
    let mut header = Vec::with_capacity(SEGMENT_HEADER_BYTES);
    header.extend_from_slice(&contract::CATALOG_SEGMENT_MAGIC);
    header.extend_from_slice(&contract::CATALOG_CONTRACT_VERSION.to_be_bytes());
    header.extend_from_slice(&0_u16.to_be_bytes());
    write_bytes(writer, &header, "catalog_segment_write")?;
    let mut bytes_written = SEGMENT_HEADER_BYTES as u64;
    let mut blocks = Vec::new();
    let mut block_entries = Vec::with_capacity(contract::CATALOG_BLOCK_ENTRIES);
    let mut referenced = BTreeSet::new();
    let mut logical_sum = [0_u8; 32];
    let mut entry_count = 0_u64;
    let mut first = None;
    let mut previous = None;
    for item in entries {
        let entry = item?;
        validate_location(entry.key, entry.location)?;
        if !descriptors.contains_key(&entry.location.pack) {
            return Err(corrupt(
                "catalog_segment_pack",
                "catalog segment entry references an undescribed pack",
            ));
        }
        if previous.is_some_and(|key| key >= entry.key) {
            return Err(corrupt(
                "catalog_segment_order",
                "catalog segment entries are not strictly ordered",
            ));
        }
        entry_count = entry_count.checked_add(1).ok_or_else(|| {
            resource(
                "catalog_segment_entries",
                "catalog segment entry count overflows",
            )
        })?;
        if entry_count > contract::MAXIMUM_CATALOG_ENTRIES as u64 {
            return Err(resource(
                "catalog_segment_entries",
                "catalog segment exceeds the entry bound",
            ));
        }
        first.get_or_insert(entry.key);
        previous = Some(entry.key);
        referenced.insert(entry.location.pack);
        add_logical_entry(&mut logical_sum, entry);
        block_entries.push(entry);
        if block_entries.len() == contract::CATALOG_BLOCK_ENTRIES {
            flush_block(writer, &mut bytes_written, &mut blocks, &mut block_entries)?;
        }
    }
    if !block_entries.is_empty() {
        flush_block(writer, &mut bytes_written, &mut blocks, &mut block_entries)?;
    }
    let Some(first) = first else {
        return Err(corrupt(
            "catalog_segment_empty",
            "an empty immutable catalog segment is invalid",
        ));
    };
    let last = previous.ok_or_else(|| {
        corrupt(
            "catalog_segment_empty",
            "catalog segment lost its last entry",
        )
    })?;
    let mut packs = BTreeMap::new();
    for pack in referenced {
        let descriptor = descriptors.get(&pack).cloned().ok_or_else(|| {
            corrupt(
                "catalog_segment_pack",
                "catalog segment lost a referenced pack descriptor",
            )
        })?;
        packs.insert(pack, descriptor);
    }
    let metadata_offset = bytes_written;
    let metadata = encode_segment_metadata(
        level,
        generation,
        entry_count,
        (first, last),
        logical_sum,
        &packs,
        &blocks,
    )?;
    let metadata_bytes = metadata.len() as u64;
    let checksum = digest_parts(
        contract::CATALOG_SEGMENT_CHECKSUM_DOMAIN,
        &[&header, &metadata],
    );
    write_bytes(writer, &metadata, "catalog_segment_write")?;
    let mut footer = Vec::with_capacity(SEGMENT_FOOTER_BYTES);
    footer.extend_from_slice(&metadata_offset.to_be_bytes());
    footer.extend_from_slice(&metadata_bytes.to_be_bytes());
    footer.extend_from_slice(&checksum);
    footer.extend_from_slice(&contract::CATALOG_SEGMENT_END_MAGIC);
    write_bytes(writer, &footer, "catalog_segment_write")?;
    bytes_written = bytes_written
        .checked_add(metadata_bytes)
        .and_then(|value| value.checked_add(SEGMENT_FOOTER_BYTES as u64))
        .ok_or_else(|| resource("catalog_segment_size", "segment byte count overflows"))?;
    if bytes_written > contract::MAXIMUM_CATALOG_SEGMENT_BYTES as u64 {
        return Err(resource(
            "catalog_segment_size",
            "catalog segment exceeds its byte bound",
        ));
    }
    Ok(WrittenSegment {
        metadata: SegmentMetadata {
            id: SegmentId::from_bytes(checksum),
            level,
            generation,
            entry_count,
            first,
            last,
            logical_sum,
            packs,
            blocks,
            file_bytes: bytes_written,
            metadata_bytes,
        },
        bytes_written,
    })
}

pub(crate) fn read_segment_metadata<R: Read + Seek>(
    reader: &mut R,
    file_bytes: u64,
    expected: SegmentId,
) -> Result<SegmentMetadata, StoreError> {
    let length = usize_from_u64(file_bytes, "catalog_segment_size")?;
    let minimum = SEGMENT_HEADER_BYTES
        + CATALOG_ENTRY_BYTES
        + SEGMENT_METADATA_HEADER_BYTES
        + PACK_DESCRIPTOR_BYTES
        + BLOCK_DESCRIPTOR_BYTES
        + SEGMENT_FOOTER_BYTES;
    if length < minimum || length > contract::MAXIMUM_CATALOG_SEGMENT_BYTES {
        return Err(resource(
            "catalog_segment_size",
            "catalog segment byte length is outside bounds",
        ));
    }
    let mut header = [0_u8; SEGMENT_HEADER_BYTES];
    read_at(reader, 0, &mut header, "catalog_segment_header_read")?;
    if header[..8] != contract::CATALOG_SEGMENT_MAGIC
        || u16::from_be_bytes([header[8], header[9]]) != contract::CATALOG_CONTRACT_VERSION
        || u16::from_be_bytes([header[10], header[11]]) != 0
    {
        return Err(corrupt(
            "catalog_segment_contract",
            "catalog segment magic, version, or flags are not current",
        ));
    }
    let footer_offset = length - SEGMENT_FOOTER_BYTES;
    let mut footer = [0_u8; SEGMENT_FOOTER_BYTES];
    read_at(
        reader,
        footer_offset as u64,
        &mut footer,
        "catalog_segment_footer_read",
    )?;
    if footer[48..] != contract::CATALOG_SEGMENT_END_MAGIC {
        return Err(corrupt(
            "catalog_segment_closing_magic",
            "catalog segment closing marker is invalid",
        ));
    }
    let metadata_offset = usize_from_u64(
        u64::from_be_bytes(
            footer[..8]
                .try_into()
                .map_err(|_| corrupt("catalog_segment_footer", "metadata offset is truncated"))?,
        ),
        "catalog_segment_metadata_offset",
    )?;
    let metadata_length = usize_from_u64(
        u64::from_be_bytes(
            footer[8..16]
                .try_into()
                .map_err(|_| corrupt("catalog_segment_footer", "metadata length is truncated"))?,
        ),
        "catalog_segment_metadata_size",
    )?;
    if metadata_length > contract::MAXIMUM_CATALOG_SEGMENT_METADATA_BYTES
        || metadata_offset
            .checked_add(metadata_length)
            .is_none_or(|end| end != footer_offset)
    {
        return Err(corrupt(
            "catalog_segment_metadata_bounds",
            "catalog segment metadata does not occupy its exact bounded tail",
        ));
    }
    let mut metadata = vec![0_u8; metadata_length];
    read_at(
        reader,
        metadata_offset as u64,
        &mut metadata,
        "catalog_segment_metadata_read",
    )?;
    let checksum: [u8; 32] = footer[16..48]
        .try_into()
        .map_err(|_| corrupt("catalog_segment_checksum", "segment checksum is truncated"))?;
    if checksum
        != digest_parts(
            contract::CATALOG_SEGMENT_CHECKSUM_DOMAIN,
            &[&header, &metadata],
        )
        || SegmentId::from_bytes(checksum) != expected
    {
        return Err(corrupt(
            "catalog_segment_checksum",
            "catalog segment content identity does not match",
        ));
    }
    decode_segment_metadata(
        SegmentId::from_bytes(checksum),
        file_bytes,
        metadata_offset as u64,
        metadata,
    )
}

fn decode_segment_metadata(
    id: SegmentId,
    file_bytes: u64,
    metadata_offset: u64,
    bytes: Vec<u8>,
) -> Result<SegmentMetadata, StoreError> {
    if bytes.len() < SEGMENT_METADATA_HEADER_BYTES {
        return Err(corrupt(
            "catalog_segment_metadata_size",
            "catalog segment metadata header is truncated",
        ));
    }
    if bytes[..8] != contract::CATALOG_SEGMENT_METADATA_MAGIC
        || read_u16(&bytes, 8)? != contract::CATALOG_CONTRACT_VERSION
        || read_u16(&bytes, 10)? != 0
        || read_u16(&bytes, 14)? != 0
    {
        return Err(corrupt(
            "catalog_segment_metadata_contract",
            "catalog segment metadata contract or flags are invalid",
        ));
    }
    let level = read_u16(&bytes, 12)?;
    let generation = read_u64(&bytes, 16)?;
    let entry_count = read_u64(&bytes, 24)?;
    let pack_count = usize_from_u64(read_u64(&bytes, 32)?, "catalog_segment_pack_count")?;
    let block_count = usize_from_u64(read_u64(&bytes, 40)?, "catalog_segment_block_count")?;
    if level > contract::MAXIMUM_CATALOG_LEVEL
        || entry_count == 0
        || entry_count > contract::MAXIMUM_CATALOG_ENTRIES as u64
        || pack_count == 0
        || pack_count > contract::MAXIMUM_CATALOG_PACKS
        || block_count == 0
        || block_count > contract::MAXIMUM_CATALOG_BLOCKS
        || block_count
            != usize_from_u64(entry_count, "catalog_segment_entry_count")?
                .div_ceil(contract::CATALOG_BLOCK_ENTRIES)
    {
        return Err(resource(
            "catalog_segment_metadata_counts",
            "catalog segment metadata counts or level are outside bounds",
        ));
    }
    let expected_metadata = SEGMENT_METADATA_HEADER_BYTES
        .checked_add(
            pack_count
                .checked_mul(PACK_DESCRIPTOR_BYTES)
                .ok_or_else(|| corrupt("catalog_segment_metadata_size", "pack bytes overflow"))?,
        )
        .and_then(|value| value.checked_add(block_count.checked_mul(BLOCK_DESCRIPTOR_BYTES)?))
        .ok_or_else(|| corrupt("catalog_segment_metadata_size", "metadata size overflows"))?;
    if expected_metadata != bytes.len() {
        return Err(corrupt(
            "catalog_segment_metadata_length",
            "catalog segment metadata counts do not describe exact bytes",
        ));
    }
    let first = read_key(&bytes, 48)?;
    let last = read_key(&bytes, 81)?;
    let logical_sum = read_array::<32>(&bytes, 114)?;
    if first > last {
        return Err(corrupt(
            "catalog_segment_range",
            "catalog segment key range is reversed",
        ));
    }
    let expected_offset = (SEGMENT_HEADER_BYTES as u64)
        .checked_add(
            entry_count
                .checked_mul(CATALOG_ENTRY_BYTES as u64)
                .ok_or_else(|| corrupt("catalog_segment_size", "entry bytes overflow"))?,
        )
        .ok_or_else(|| corrupt("catalog_segment_size", "segment offset overflows"))?;
    if expected_offset != metadata_offset {
        return Err(corrupt(
            "catalog_segment_entry_region",
            "catalog segment entry region is not exact",
        ));
    }
    let mut offset = SEGMENT_METADATA_HEADER_BYTES;
    let mut packs = BTreeMap::new();
    let mut previous_pack = None;
    for _ in 0..pack_count {
        let descriptor = decode_pack_descriptor(&bytes[offset..offset + PACK_DESCRIPTOR_BYTES])?;
        if previous_pack.is_some_and(|pack| pack >= descriptor.pack)
            || packs.insert(descriptor.pack, descriptor.clone()).is_some()
        {
            return Err(corrupt(
                "catalog_segment_pack_order",
                "catalog segment pack descriptors are not strictly ordered",
            ));
        }
        previous_pack = Some(descriptor.pack);
        offset += PACK_DESCRIPTOR_BYTES;
    }
    let mut blocks = Vec::with_capacity(block_count);
    let mut previous_last = None;
    let mut expected_block_offset = SEGMENT_HEADER_BYTES as u64;
    let mut observed_entries = 0_u64;
    for ordinal in 0..block_count {
        let block = decode_block_descriptor(&bytes[offset..offset + BLOCK_DESCRIPTOR_BYTES])?;
        if block.count == 0
            || block.count > contract::CATALOG_BLOCK_ENTRIES as u64
            || (ordinal + 1 != block_count && block.count != contract::CATALOG_BLOCK_ENTRIES as u64)
            || block.first > block.last
            || previous_last.is_some_and(|key| key >= block.first)
            || block.offset != expected_block_offset
        {
            return Err(corrupt(
                "catalog_segment_block_layout",
                "catalog segment blocks are not canonical and contiguous",
            ));
        }
        let block_bytes = block
            .count
            .checked_mul(CATALOG_ENTRY_BYTES as u64)
            .ok_or_else(|| corrupt("catalog_segment_block_size", "block size overflows"))?;
        expected_block_offset = expected_block_offset
            .checked_add(block_bytes)
            .ok_or_else(|| corrupt("catalog_segment_block_size", "block end overflows"))?;
        observed_entries = observed_entries
            .checked_add(block.count)
            .ok_or_else(|| corrupt("catalog_segment_block_count", "block count overflows"))?;
        previous_last = Some(block.last);
        blocks.push(block);
        offset += BLOCK_DESCRIPTOR_BYTES;
    }
    if observed_entries != entry_count
        || expected_block_offset != metadata_offset
        || blocks.first().map(|block| block.first) != Some(first)
        || blocks.last().map(|block| block.last) != Some(last)
    {
        return Err(corrupt(
            "catalog_segment_block_summary",
            "catalog segment blocks disagree with the segment range or count",
        ));
    }
    Ok(SegmentMetadata {
        id,
        level,
        generation,
        entry_count,
        first,
        last,
        logical_sum,
        packs,
        blocks,
        file_bytes,
        metadata_bytes: bytes.len() as u64,
    })
}

fn encode_segment_metadata(
    level: u16,
    generation: u64,
    entry_count: u64,
    range: (ObjectKey, ObjectKey),
    logical_sum: [u8; 32],
    packs: &BTreeMap<PackId, PackDescriptor>,
    blocks: &[CatalogBlock],
) -> Result<Vec<u8>, StoreError> {
    let (first, last) = range;
    if packs.is_empty()
        || packs.len() > contract::MAXIMUM_CATALOG_PACKS
        || blocks.is_empty()
        || blocks.len() > contract::MAXIMUM_CATALOG_BLOCKS
    {
        return Err(resource(
            "catalog_segment_metadata_counts",
            "catalog segment pack or block count is outside bounds",
        ));
    }
    let capacity = SEGMENT_METADATA_HEADER_BYTES
        .checked_add(
            packs
                .len()
                .checked_mul(PACK_DESCRIPTOR_BYTES)
                .ok_or_else(|| corrupt("catalog_segment_metadata_size", "pack bytes overflow"))?,
        )
        .and_then(|value| value.checked_add(blocks.len().checked_mul(BLOCK_DESCRIPTOR_BYTES)?))
        .ok_or_else(|| corrupt("catalog_segment_metadata_size", "metadata size overflows"))?;
    if capacity > contract::MAXIMUM_CATALOG_SEGMENT_METADATA_BYTES {
        return Err(resource(
            "catalog_segment_metadata_size",
            "catalog segment metadata exceeds its byte bound",
        ));
    }
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(&contract::CATALOG_SEGMENT_METADATA_MAGIC);
    bytes.extend_from_slice(&contract::CATALOG_CONTRACT_VERSION.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&level.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&generation.to_be_bytes());
    bytes.extend_from_slice(&entry_count.to_be_bytes());
    bytes.extend_from_slice(&(packs.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&(blocks.len() as u64).to_be_bytes());
    encode_key(&mut bytes, first);
    encode_key(&mut bytes, last);
    bytes.extend_from_slice(&logical_sum);
    for descriptor in packs.values() {
        encode_pack_descriptor(&mut bytes, descriptor);
    }
    for block in blocks {
        encode_block_descriptor(&mut bytes, block);
    }
    Ok(bytes)
}

fn flush_block<W: Write>(
    writer: &mut W,
    bytes_written: &mut u64,
    blocks: &mut Vec<CatalogBlock>,
    entries: &mut Vec<CatalogEntry>,
) -> Result<(), StoreError> {
    let first = entries
        .first()
        .map(|entry| entry.key)
        .ok_or_else(|| corrupt("catalog_block_empty", "cannot flush an empty catalog block"))?;
    let last = entries
        .last()
        .map(|entry| entry.key)
        .ok_or_else(|| corrupt("catalog_block_empty", "cannot flush an empty catalog block"))?;
    let mut bytes = Vec::with_capacity(entries.len() * CATALOG_ENTRY_BYTES);
    for entry in entries.iter().copied() {
        encode_entry(&mut bytes, entry);
    }
    let checksum = digest(contract::CATALOG_BLOCK_CHECKSUM_DOMAIN, &bytes);
    let filter = block_filter(entries);
    let offset = *bytes_written;
    write_bytes(writer, &bytes, "catalog_segment_block_write")?;
    *bytes_written = bytes_written
        .checked_add(bytes.len() as u64)
        .ok_or_else(|| resource("catalog_segment_size", "catalog segment size overflows"))?;
    blocks.push(CatalogBlock {
        first,
        last,
        offset,
        count: entries.len() as u64,
        checksum,
        filter,
    });
    entries.clear();
    Ok(())
}

fn encode_entry(bytes: &mut Vec<u8>, entry: CatalogEntry) {
    encode_key(bytes, entry.key);
    bytes.extend_from_slice(&entry.location.pack.bytes());
    bytes.extend_from_slice(&entry.location.offset.to_be_bytes());
    bytes.extend_from_slice(&entry.location.length.to_be_bytes());
    bytes.extend_from_slice(&entry.location.checksum);
}

fn decode_entry(bytes: &[u8]) -> Result<CatalogEntry, StoreError> {
    if bytes.len() != CATALOG_ENTRY_BYTES {
        return Err(corrupt(
            "catalog_entry_length",
            "catalog entry has an invalid byte length",
        ));
    }
    Ok(CatalogEntry {
        key: read_key(bytes, 0)?,
        location: CatalogLocation {
            pack: PackId::from_bytes(read_array::<32>(bytes, 33)?),
            offset: read_u64(bytes, 65)?,
            length: read_u64(bytes, 73)?,
            checksum: read_array::<32>(bytes, 81)?,
        },
    })
}

fn encode_pack_descriptor(bytes: &mut Vec<u8>, descriptor: &PackDescriptor) {
    bytes.extend_from_slice(&descriptor.pack.bytes());
    bytes.extend_from_slice(&descriptor.byte_length.to_be_bytes());
    bytes.extend_from_slice(&descriptor.nonce);
    bytes.extend_from_slice(&descriptor.payload_bytes.to_be_bytes());
    bytes.extend_from_slice(&descriptor.index_checksum);
    bytes.extend_from_slice(&descriptor.pack_checksum);
    bytes.extend_from_slice(&descriptor.entries.to_be_bytes());
}

fn decode_pack_descriptor(bytes: &[u8]) -> Result<PackDescriptor, StoreError> {
    if bytes.len() != PACK_DESCRIPTOR_BYTES {
        return Err(corrupt(
            "catalog_pack_descriptor_length",
            "catalog pack descriptor has an invalid byte length",
        ));
    }
    let descriptor = PackDescriptor {
        pack: PackId::from_bytes(read_array::<32>(bytes, 0)?),
        byte_length: read_u64(bytes, 32)?,
        nonce: read_array::<16>(bytes, 40)?,
        payload_bytes: read_u64(bytes, 56)?,
        index_checksum: read_array::<32>(bytes, 64)?,
        pack_checksum: read_array::<32>(bytes, 96)?,
        entries: read_u64(bytes, 128)?,
    };
    descriptor.validate()?;
    Ok(descriptor)
}

fn encode_block_descriptor(bytes: &mut Vec<u8>, block: &CatalogBlock) {
    encode_key(bytes, block.first);
    encode_key(bytes, block.last);
    bytes.extend_from_slice(&block.offset.to_be_bytes());
    bytes.extend_from_slice(&block.count.to_be_bytes());
    bytes.extend_from_slice(&block.checksum);
    bytes.extend_from_slice(&block.filter);
}

fn decode_block_descriptor(bytes: &[u8]) -> Result<CatalogBlock, StoreError> {
    if bytes.len() != BLOCK_DESCRIPTOR_BYTES {
        return Err(corrupt(
            "catalog_block_descriptor_length",
            "catalog block descriptor has an invalid byte length",
        ));
    }
    Ok(CatalogBlock {
        first: read_key(bytes, 0)?,
        last: read_key(bytes, 33)?,
        offset: read_u64(bytes, 66)?,
        count: read_u64(bytes, 74)?,
        checksum: read_array::<32>(bytes, 82)?,
        filter: read_array::<{ contract::CATALOG_BLOCK_FILTER_BYTES }>(bytes, 114)?,
    })
}

fn encode_manifest_segment(bytes: &mut Vec<u8>, segment: ManifestSegment) {
    bytes.extend_from_slice(&segment.level.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&segment.generation.to_be_bytes());
    bytes.extend_from_slice(&segment.entry_count.to_be_bytes());
    bytes.extend_from_slice(&segment.pack_count.to_be_bytes());
    encode_key(bytes, segment.first);
    encode_key(bytes, segment.last);
    bytes.extend_from_slice(&segment.id.bytes());
    bytes.extend_from_slice(&segment.file_bytes.to_be_bytes());
    bytes.extend_from_slice(&segment.metadata_bytes.to_be_bytes());
    bytes.extend_from_slice(&segment.logical_sum);
}

fn decode_manifest_segment(bytes: &[u8]) -> Result<ManifestSegment, StoreError> {
    if bytes.len() != MANIFEST_SEGMENT_BYTES || read_u16(bytes, 2)? != 0 {
        return Err(corrupt(
            "catalog_manifest_segment_length",
            "manifest segment descriptor length or flags are invalid",
        ));
    }
    Ok(ManifestSegment {
        level: read_u16(bytes, 0)?,
        generation: read_u64(bytes, 4)?,
        entry_count: read_u64(bytes, 12)?,
        pack_count: read_u64(bytes, 20)?,
        first: read_key(bytes, 28)?,
        last: read_key(bytes, 61)?,
        id: SegmentId::from_bytes(read_array::<32>(bytes, 94)?),
        file_bytes: read_u64(bytes, 126)?,
        metadata_bytes: read_u64(bytes, 134)?,
        logical_sum: read_array::<32>(bytes, 142)?,
    })
}

fn encode_history(bytes: &mut Vec<u8>, history: CatalogHistory) {
    for value in [
        history.delta_segments,
        history.merge_operations,
        history.merge_entries_read,
        history.merge_bytes_read,
        history.segments_written,
        history.segment_entries_written,
        history.full_rebuilds,
        history.full_footer_scan_runs,
        history.pack_footers_scanned,
    ] {
        bytes.extend_from_slice(&value.to_be_bytes());
    }
}

fn decode_history(bytes: &[u8], offset: usize) -> Result<CatalogHistory, StoreError> {
    Ok(CatalogHistory {
        delta_segments: read_u64(bytes, offset)?,
        merge_operations: read_u64(bytes, offset + 8)?,
        merge_entries_read: read_u64(bytes, offset + 16)?,
        merge_bytes_read: read_u64(bytes, offset + 24)?,
        segments_written: read_u64(bytes, offset + 32)?,
        segment_entries_written: read_u64(bytes, offset + 40)?,
        full_rebuilds: read_u64(bytes, offset + 48)?,
        full_footer_scan_runs: read_u64(bytes, offset + 56)?,
        pack_footers_scanned: read_u64(bytes, offset + 64)?,
    })
}

fn validate_location(key: ObjectKey, location: CatalogLocation) -> Result<(), StoreError> {
    if location.length > key.domain.maximum_bytes() as u64
        || location.length > contract::MAXIMUM_PACK_ENTRY_BYTES as u64
    {
        return Err(resource(
            "catalog_object_size",
            "catalog location exceeds its object-domain bound",
        ));
    }
    if location.offset.checked_add(location.length).is_none() {
        return Err(corrupt(
            "catalog_location_overflow",
            "catalog object coordinates overflow",
        ));
    }
    Ok(())
}

pub(crate) fn logical_commitment(count: u64, sum: [u8; 32]) -> CatalogCommitment {
    let mut bytes = Vec::with_capacity(40);
    bytes.extend_from_slice(&count.to_be_bytes());
    bytes.extend_from_slice(&sum);
    CatalogCommitment::from_bytes(digest(contract::CATALOG_LOGICAL_COMMITMENT_DOMAIN, &bytes))
}

pub(crate) fn add_logical_entry(sum: &mut [u8; 32], entry: CatalogEntry) {
    let mut bytes = Vec::with_capacity(1 + 32 + 8 + 32);
    encode_key(&mut bytes, entry.key);
    bytes.extend_from_slice(&entry.location.length.to_be_bytes());
    bytes.extend_from_slice(&entry.location.checksum);
    let value = digest(contract::CATALOG_LOGICAL_ENTRY_DOMAIN, &bytes);
    add_sum(sum, value);
}

pub(crate) fn add_sum(sum: &mut [u8; 32], value: [u8; 32]) {
    let mut carry = 0_u16;
    for index in (0..32).rev() {
        let total = u16::from(sum[index]) + u16::from(value[index]) + carry;
        sum[index] = total as u8;
        carry = total >> 8;
    }
}

fn block_filter(entries: &[CatalogEntry]) -> [u8; contract::CATALOG_BLOCK_FILTER_BYTES] {
    let mut filter = [0_u8; contract::CATALOG_BLOCK_FILTER_BYTES];
    for entry in entries {
        add_filter_key(&mut filter, entry.key);
    }
    filter
}

fn add_filter_key(filter: &mut [u8; contract::CATALOG_BLOCK_FILTER_BYTES], key: ObjectKey) {
    let digest = key.digest.bytes();
    let first = u64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ]) ^ (u64::from(key.domain.tag()) << 56);
    let second = u64::from_be_bytes([
        digest[8], digest[9], digest[10], digest[11], digest[12], digest[13], digest[14],
        digest[15],
    ]) | 1;
    for ordinal in 0..FILTER_HASHES {
        let bit = first.wrapping_add(ordinal.wrapping_mul(second)) % FILTER_BITS;
        let byte = (bit / 8) as usize;
        filter[byte] |= 1_u8 << (bit % 8);
    }
}

fn filter_contains(filter: &[u8; contract::CATALOG_BLOCK_FILTER_BYTES], key: ObjectKey) -> bool {
    let digest = key.digest.bytes();
    let first = u64::from_be_bytes([
        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5], digest[6], digest[7],
    ]) ^ (u64::from(key.domain.tag()) << 56);
    let second = u64::from_be_bytes([
        digest[8], digest[9], digest[10], digest[11], digest[12], digest[13], digest[14],
        digest[15],
    ]) | 1;
    (0..FILTER_HASHES).all(|ordinal| {
        let bit = first.wrapping_add(ordinal.wrapping_mul(second)) % FILTER_BITS;
        let byte = (bit / 8) as usize;
        filter[byte] & (1_u8 << (bit % 8)) != 0
    })
}

fn encode_key(bytes: &mut Vec<u8>, key: ObjectKey) {
    bytes.push(key.domain.tag());
    bytes.extend_from_slice(&key.digest.bytes());
}

fn read_key(bytes: &[u8], offset: usize) -> Result<ObjectKey, StoreError> {
    let tag = *bytes
        .get(offset)
        .ok_or_else(|| corrupt("catalog_key_truncated", "catalog key tag is truncated"))?;
    let domain = ObjectDomain::from_tag(tag)?;
    Ok(ObjectKey::from_digest(
        domain,
        read_array::<32>(bytes, offset + 1)?,
    ))
}

fn parse_lower_hex_32(value: &str, code: &'static str) -> Result<[u8; 32], StoreError> {
    if value.len() != 64 {
        return Err(corrupt(
            code,
            "catalog identity must contain 64 hexadecimal characters",
        ));
    }
    let mut output = [0_u8; 32];
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(corrupt(
            code,
            "catalog identity contains an incomplete hex pair",
        ));
    }
    for (index, pair) in pairs.iter().enumerate() {
        let high = lower_hex(pair[0])
            .ok_or_else(|| corrupt(code, "catalog identity contains non-lowercase hexadecimal"))?;
        let low = lower_hex(pair[1])
            .ok_or_else(|| corrupt(code, "catalog identity contains non-lowercase hexadecimal"))?;
        output[index] = (high << 4) | low;
    }
    Ok(output)
}

fn lower_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn digest(domain: &str, bytes: &[u8]) -> [u8; 32] {
    digest_parts(domain, &[bytes])
}

fn digest_parts(domain: &str, parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    let total = parts
        .iter()
        .fold(0_u64, |sum, part| sum.saturating_add(part.len() as u64));
    hasher.update(&total.to_be_bytes());
    for part in parts {
        hasher.update(part);
    }
    *hasher.finalize().as_bytes()
}

fn write_bytes(
    writer: &mut impl Write,
    bytes: &[u8],
    code: &'static str,
) -> Result<(), StoreError> {
    writer
        .write_all(bytes)
        .map_err(|error| io_error(code, "failed to write catalog bytes", error))
}

fn read_at(
    reader: &mut (impl Read + Seek),
    offset: u64,
    bytes: &mut [u8],
    code: &'static str,
) -> Result<(), StoreError> {
    reader
        .seek(SeekFrom::Start(offset))
        .map_err(|error| io_error(code, "failed to seek catalog bytes", error))?;
    reader.read_exact(bytes).map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            corrupt(code, "catalog bytes are truncated")
        } else {
            io_error(code, "failed to read catalog bytes", error)
        }
    })
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, StoreError> {
    Ok(u16::from_be_bytes(read_array(bytes, offset)?))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, StoreError> {
    Ok(u64::from_be_bytes(read_array(bytes, offset)?))
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N], StoreError> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| corrupt("catalog_offset_overflow", "catalog field offset overflows"))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| corrupt("catalog_truncated", "catalog field is truncated"))?
        .try_into()
        .map_err(|_| corrupt("catalog_field_length", "catalog field length is invalid"))
}

fn usize_from_u64(value: u64, code: &'static str) -> Result<usize, StoreError> {
    usize::try_from(value)
        .map_err(|_| resource(code, "catalog count or length does not fit this platform"))
}

fn io_error(code: &'static str, message: &'static str, error: std::io::Error) -> StoreError {
    StoreError::new(StoreErrorClass::Io, code, format!("{message}: {error}"))
}

fn corrupt(code: &'static str, message: impl Into<String>) -> StoreError {
    StoreError::new(StoreErrorClass::Corrupt, code, message)
}

fn resource(code: &'static str, message: impl Into<String>) -> StoreError {
    StoreError::new(StoreErrorClass::Resource, code, message)
}
