//! Immutable deterministic multi-domain pack with a strict footer index.

use super::contract;
use super::object::{ObjectDomain, ObjectKey, StoreError, StoreErrorClass};
use std::collections::BTreeMap;
use std::fmt;
use std::io::{Read, Seek, SeekFrom};

pub(super) const HEADER_BYTES: usize = 8 + 2 + 2 + 16;
const INDEX_ENTRY_BYTES: usize = 1 + 32 + 8 + 8 + 8 + 32;
pub(super) const FOOTER_BYTES: usize = 8 + 2 + 2 + 8 + 8 + 8 + 32;
pub(super) const TRAILER_BYTES: usize = 8 + 32 + 8;
const MINIMUM_PACK_BYTES: usize = HEADER_BYTES + FOOTER_BYTES + TRAILER_BYTES;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PackId([u8; 32]);

impl PackId {
    pub fn of(bytes: &[u8]) -> Self {
        Self(digest(contract::PACK_ID_DOMAIN, bytes))
    }

    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub fn file_name(self) -> String {
        format!("{self}.lkjp")
    }

    pub fn parse_file_name(value: &str) -> Result<Self, StoreError> {
        let encoded = value
            .strip_prefix("pack_")
            .and_then(|value| value.strip_suffix(".lkjp"))
            .ok_or_else(|| corrupt("pack_file_name", "pack file name is not canonical"))?;
        if encoded.len() != 64 {
            return Err(corrupt(
                "pack_file_name_length",
                "pack file identity must contain 64 hexadecimal characters",
            ));
        }
        let mut bytes = [0_u8; 32];
        let (pairs, remainder) = encoded.as_bytes().as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(corrupt(
                "pack_file_name_length",
                "pack file identity must contain complete hexadecimal pairs",
            ));
        }
        for (index, pair) in pairs.iter().enumerate() {
            let high = hex(pair[0]).ok_or_else(|| {
                corrupt(
                    "pack_file_name_hex",
                    "pack file identity is not lowercase hex",
                )
            })?;
            let low = hex(pair[1]).ok_or_else(|| {
                corrupt(
                    "pack_file_name_hex",
                    "pack file identity is not lowercase hex",
                )
            })?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for PackId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("pack_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackIndexEntry {
    pub key: ObjectKey,
    pub offset: u64,
    pub encoded_length: u64,
    pub uncompressed_length: u64,
    pub checksum: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackMetadata {
    pub byte_length: u64,
    pub nonce: [u8; 16],
    pub payload_bytes: u64,
    pub index_checksum: [u8; 32],
    pub pack_checksum: [u8; 32],
    pub entries: Vec<PackIndexEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackMetadataRead {
    pub metadata: PackMetadata,
    pub bytes_read: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackVerification {
    pub bytes_read: u64,
}

impl PackMetadata {
    pub fn read_footer<R: Read + Seek>(
        reader: &mut R,
        byte_length: u64,
    ) -> Result<PackMetadataRead, StoreError> {
        let length = usize_from_u64(byte_length, "pack_file_length")?;
        if !(MINIMUM_PACK_BYTES..=contract::MAXIMUM_PACK_BYTES).contains(&length) {
            return Err(pack_error(
                StoreErrorClass::Resource,
                "pack_size",
                "pack file byte length is outside decoder bounds",
            ));
        }
        let mut header = [0_u8; HEADER_BYTES];
        read_at(reader, 0, &mut header)?;
        let trailer_start = length - TRAILER_BYTES;
        let mut trailer = [0_u8; TRAILER_BYTES];
        read_at(reader, trailer_start as u64, &mut trailer)?;
        if trailer[40..] != contract::PACK_END_MAGIC {
            return Err(corrupt(
                "pack_closing_magic",
                "pack closing magic is not current",
            ));
        }
        let footer_offset =
            usize_from_u64(
                u64::from_be_bytes(trailer[..8].try_into().map_err(|_| {
                    corrupt("pack_footer_offset", "pack footer offset is truncated")
                })?),
                "pack_footer_offset",
            )?;
        if footer_offset
            .checked_add(FOOTER_BYTES)
            .is_none_or(|end| end != trailer_start)
        {
            return Err(corrupt(
                "pack_footer_bounds",
                "pack footer does not end exactly at the trailer",
            ));
        }
        let mut footer = [0_u8; FOOTER_BYTES];
        read_at(reader, footer_offset as u64, &mut footer)?;
        if footer[..8] != contract::PACK_INDEX_MAGIC {
            return Err(corrupt(
                "pack_index_magic",
                "pack footer index magic is not current",
            ));
        }
        let entry_count = usize_from_u64(
            u64::from_be_bytes(
                footer[12..20]
                    .try_into()
                    .map_err(|_| corrupt("pack_entry_count", "pack entry count is truncated"))?,
            ),
            "pack_entry_count",
        )?;
        let index_bytes =
            usize_from_u64(
                u64::from_be_bytes(footer[28..36].try_into().map_err(|_| {
                    corrupt("pack_index_bytes", "pack index byte count is truncated")
                })?),
                "pack_index_bytes",
            )?;
        if entry_count == 0
            || entry_count > contract::MAXIMUM_PACK_ENTRIES
            || entry_count.checked_mul(INDEX_ENTRY_BYTES) != Some(index_bytes)
            || index_bytes > footer_offset.saturating_sub(HEADER_BYTES)
        {
            return Err(pack_error(
                StoreErrorClass::Resource,
                "pack_index_size",
                "pack footer declares an invalid or excessive index",
            ));
        }
        let index_start = footer_offset - index_bytes;
        let mut index = vec![0_u8; index_bytes];
        read_at(reader, index_start as u64, &mut index)?;
        let metadata = decode_footer_parts(&header, &index, &footer, &trailer, byte_length)?;
        Ok(PackMetadataRead {
            metadata,
            bytes_read: (HEADER_BYTES + index_bytes + FOOTER_BYTES + TRAILER_BYTES) as u64,
        })
    }

    pub fn decode(bytes: &[u8], verify_complete_checksum: bool) -> Result<Self, StoreError> {
        if bytes.len() < MINIMUM_PACK_BYTES || bytes.len() > contract::MAXIMUM_PACK_BYTES {
            return Err(pack_error(
                StoreErrorClass::Resource,
                "pack_size",
                format!("pack byte length {} is outside decoder bounds", bytes.len()),
            ));
        }
        if bytes[..8] != contract::PACK_MAGIC {
            return Err(corrupt("pack_magic", "pack header magic is not current"));
        }
        if read_u16(bytes, 8)? != contract::PACK_CONTRACT_VERSION || read_u16(bytes, 10)? != 0 {
            return Err(corrupt(
                "pack_contract",
                "pack contract version or reserved flags are invalid",
            ));
        }
        let nonce = read_array::<16>(bytes, 12)?;
        let trailer_start = bytes
            .len()
            .checked_sub(TRAILER_BYTES)
            .ok_or_else(|| corrupt("pack_trailer", "pack trailer is truncated"))?;
        if read_array::<8>(bytes, trailer_start + 40)? != contract::PACK_END_MAGIC {
            return Err(corrupt(
                "pack_closing_magic",
                "pack closing magic is not current",
            ));
        }
        let footer_offset = usize_from_u64(read_u64(bytes, trailer_start)?, "pack_footer_offset")?;
        let footer_end = footer_offset
            .checked_add(FOOTER_BYTES)
            .ok_or_else(|| corrupt("pack_footer_overflow", "pack footer offset overflows"))?;
        if footer_offset < HEADER_BYTES || footer_end != trailer_start {
            return Err(corrupt(
                "pack_footer_bounds",
                "pack footer does not end exactly at the trailer",
            ));
        }
        if read_array::<8>(bytes, footer_offset)? != contract::PACK_INDEX_MAGIC {
            return Err(corrupt(
                "pack_index_magic",
                "pack footer index magic is not current",
            ));
        }
        if read_u16(bytes, footer_offset + 8)? != contract::PACK_CONTRACT_VERSION
            || read_u16(bytes, footer_offset + 10)? != 0
        {
            return Err(corrupt(
                "pack_index_contract",
                "pack footer contract or reserved flags are invalid",
            ));
        }
        let entry_count = usize_from_u64(read_u64(bytes, footer_offset + 12)?, "pack_entry_count")?;
        if entry_count == 0 || entry_count > contract::MAXIMUM_PACK_ENTRIES {
            return Err(pack_error(
                StoreErrorClass::Resource,
                "pack_entry_count",
                format!("pack entry count {entry_count} is outside decoder bounds"),
            ));
        }
        let payload_bytes = read_u64(bytes, footer_offset + 20)?;
        let index_bytes = usize_from_u64(read_u64(bytes, footer_offset + 28)?, "pack_index_bytes")?;
        let expected_index_bytes = entry_count
            .checked_mul(INDEX_ENTRY_BYTES)
            .ok_or_else(|| corrupt("pack_index_overflow", "pack index size overflows"))?;
        if index_bytes != expected_index_bytes {
            return Err(corrupt(
                "pack_index_length",
                "pack index byte length disagrees with its entry count",
            ));
        }
        let index_start = footer_offset
            .checked_sub(index_bytes)
            .ok_or_else(|| corrupt("pack_index_bounds", "pack index starts before the header"))?;
        if index_start < HEADER_BYTES || payload_bytes != (index_start - HEADER_BYTES) as u64 {
            return Err(corrupt(
                "pack_payload_length",
                "pack payload byte count disagrees with its exact region",
            ));
        }
        let index = &bytes[index_start..footer_offset];
        let index_checksum = read_array::<32>(bytes, footer_offset + 36)?;
        if digest(contract::PACK_INDEX_CHECKSUM_DOMAIN, index) != index_checksum {
            return Err(corrupt(
                "pack_index_checksum",
                "pack footer index checksum does not match",
            ));
        }
        let mut entries = Vec::with_capacity(entry_count);
        let mut expected_offset = HEADER_BYTES as u64;
        let mut previous = None;
        for ordinal in 0..entry_count {
            let start = index_start + ordinal * INDEX_ENTRY_BYTES;
            let domain = ObjectDomain::from_tag(bytes[start])?;
            let key = ObjectKey::from_digest(domain, read_array::<32>(bytes, start + 1)?);
            if previous.is_some_and(|previous| previous >= key) {
                return Err(corrupt(
                    "pack_index_order",
                    "pack footer keys are not strictly ordered",
                ));
            }
            previous = Some(key);
            let offset = read_u64(bytes, start + 33)?;
            let encoded_length = read_u64(bytes, start + 41)?;
            let uncompressed_length = read_u64(bytes, start + 49)?;
            if encoded_length != uncompressed_length {
                return Err(corrupt(
                    "pack_compression_unsupported",
                    "pack entry declares unsupported physical compression",
                ));
            }
            if encoded_length > domain.maximum_bytes() as u64
                || encoded_length > contract::MAXIMUM_PACK_ENTRY_BYTES as u64
            {
                return Err(pack_error(
                    StoreErrorClass::Resource,
                    "pack_entry_size",
                    "pack entry exceeds its object-domain decoder bound",
                ));
            }
            if offset != expected_offset {
                return Err(corrupt(
                    "pack_entry_layout",
                    "pack entry payloads are overlapping, gapped, or out of order",
                ));
            }
            expected_offset = offset
                .checked_add(encoded_length)
                .ok_or_else(|| corrupt("pack_entry_overflow", "pack entry end overflows"))?;
            if expected_offset > index_start as u64 {
                return Err(corrupt(
                    "pack_entry_bounds",
                    "pack entry extends into the footer index",
                ));
            }
            entries.push(PackIndexEntry {
                key,
                offset,
                encoded_length,
                uncompressed_length,
                checksum: read_array::<32>(bytes, start + 57)?,
            });
        }
        if expected_offset != index_start as u64 {
            return Err(corrupt(
                "pack_payload_trailing",
                "pack payload contains bytes outside indexed entries",
            ));
        }
        if nonce_for_entries(&entries) != nonce {
            return Err(corrupt(
                "pack_nonce",
                "pack nonce does not commit to its exact sorted entry metadata",
            ));
        }
        let pack_checksum = read_array::<32>(bytes, trailer_start + 8)?;
        if verify_complete_checksum
            && digest(contract::PACK_CHECKSUM_DOMAIN, &bytes[..trailer_start]) != pack_checksum
        {
            return Err(corrupt(
                "pack_checksum",
                "complete pack checksum does not match",
            ));
        }
        Ok(Self {
            byte_length: bytes.len() as u64,
            nonce,
            payload_bytes,
            index_checksum,
            pack_checksum,
            entries,
        })
    }

    pub fn find(&self, key: ObjectKey) -> Option<&PackIndexEntry> {
        self.entries
            .binary_search_by_key(&key, |entry| entry.key)
            .ok()
            .map(|index| &self.entries[index])
    }

    /// Resolves and bounds one exact payload using footer metadata only.
    ///
    /// Store admission uses this before opening or copying the corresponding payload.
    pub(crate) fn bounded_read_entry(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
    ) -> Result<Option<(&PackIndexEntry, usize)>, StoreError> {
        let Some(entry) = self.find(key) else {
            return Ok(None);
        };
        let length = usize_from_u64(entry.encoded_length, "pack_read_length")?;
        if length > maximum_bytes || length > key.domain.maximum_bytes() {
            return Err(pack_error(
                StoreErrorClass::Resource,
                "pack_read_limit",
                format!("object has {length} bytes; caller allowed {maximum_bytes}"),
            ));
        }
        Ok(Some((entry, length)))
    }

    pub fn verify_all(&self, bytes: &[u8]) -> Result<(), StoreError> {
        if bytes.len() as u64 != self.byte_length {
            return Err(corrupt(
                "pack_length_changed",
                "pack length changed after its footer was decoded",
            ));
        }
        let trailer_start = bytes.len() - TRAILER_BYTES;
        if digest(contract::PACK_CHECKSUM_DOMAIN, &bytes[..trailer_start]) != self.pack_checksum {
            return Err(corrupt(
                "pack_checksum",
                "complete pack checksum does not match",
            ));
        }
        for entry in &self.entries {
            let _ = self.read(bytes, entry.key, entry.key.domain.maximum_bytes())?;
        }
        Ok(())
    }

    pub fn read(
        &self,
        bytes: &[u8],
        key: ObjectKey,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let Some((entry, length)) = self.bounded_read_entry(key, maximum_bytes)? else {
            return Ok(None);
        };
        let start = usize_from_u64(entry.offset, "pack_read_offset")?;
        let end = start
            .checked_add(length)
            .ok_or_else(|| corrupt("pack_read_overflow", "pack read range overflows"))?;
        let value = bytes
            .get(start..end)
            .ok_or_else(|| corrupt("pack_read_bounds", "pack entry range is truncated"))?;
        if digest(contract::PACK_ENTRY_CHECKSUM_DOMAIN, value) != entry.checksum {
            return Err(corrupt(
                "pack_entry_checksum",
                "pack entry checksum does not match",
            ));
        }
        key.verify(value)?;
        Ok(Some(value.to_vec()))
    }

    pub fn read_from<R: Read + Seek>(
        &self,
        reader: &mut R,
        key: ObjectKey,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let Some((entry, length)) = self.bounded_read_entry(key, maximum_bytes)? else {
            return Ok(None);
        };
        let mut value = vec![0_u8; length];
        read_at(reader, entry.offset, &mut value)?;
        if digest(contract::PACK_ENTRY_CHECKSUM_DOMAIN, &value) != entry.checksum {
            return Err(corrupt(
                "pack_entry_checksum",
                "pack entry checksum does not match",
            ));
        }
        key.verify(&value)?;
        Ok(Some(value))
    }

    pub fn verify_file<R: Read + Seek>(
        &self,
        reader: &mut R,
        expected_id: PackId,
    ) -> Result<PackVerification, StoreError> {
        let observed_length = reader
            .seek(SeekFrom::End(0))
            .map_err(|error| io_error("pack_seek", "failed to inspect immutable pack", error))?;
        if observed_length != self.byte_length {
            return Err(corrupt(
                "pack_length_changed",
                "pack length changed after its footer was decoded",
            ));
        }
        let trailer_start = self
            .byte_length
            .checked_sub(TRAILER_BYTES as u64)
            .ok_or_else(|| corrupt("pack_trailer", "pack trailer is truncated"))?;
        let mut identity = blake3::Hasher::new_derive_key(contract::PACK_ID_DOMAIN);
        identity.update(&self.byte_length.to_be_bytes());
        let mut checksum = blake3::Hasher::new_derive_key(contract::PACK_CHECKSUM_DOMAIN);
        checksum.update(&trailer_start.to_be_bytes());
        reader
            .seek(SeekFrom::Start(0))
            .map_err(|error| io_error("pack_seek", "failed to rewind immutable pack", error))?;
        let mut buffer = [0_u8; 64 * 1024];
        let mut position = 0_u64;
        while position < self.byte_length {
            let remaining = usize_from_u64(self.byte_length - position, "pack_verify_length")?;
            let take = remaining.min(buffer.len());
            reader
                .read_exact(&mut buffer[..take])
                .map_err(|error| read_error("pack_verify_read", error))?;
            identity.update(&buffer[..take]);
            if position < trailer_start {
                let checksum_take = usize_from_u64(
                    (trailer_start - position).min(take as u64),
                    "pack_verify_checksum_length",
                )?;
                checksum.update(&buffer[..checksum_take]);
            }
            position = position.checked_add(take as u64).ok_or_else(|| {
                corrupt("pack_verify_overflow", "pack verification work overflows")
            })?;
        }
        if PackId::from_bytes(*identity.finalize().as_bytes()) != expected_id {
            return Err(corrupt(
                "pack_identity",
                "pack bytes do not match their physical file identity",
            ));
        }
        if *checksum.finalize().as_bytes() != self.pack_checksum {
            return Err(corrupt(
                "pack_checksum",
                "complete pack checksum does not match",
            ));
        }
        let mut bytes_read = self.byte_length;
        for entry in &self.entries {
            let value = self
                .read_from(reader, entry.key, entry.key.domain.maximum_bytes())?
                .ok_or_else(|| corrupt("pack_entry_missing", "indexed pack entry disappeared"))?;
            bytes_read = bytes_read
                .checked_add(value.len() as u64)
                .ok_or_else(|| corrupt("pack_verify_work", "pack verification work overflows"))?;
        }
        Ok(PackVerification { bytes_read })
    }
}

fn decode_footer_parts(
    header: &[u8; HEADER_BYTES],
    index: &[u8],
    footer: &[u8; FOOTER_BYTES],
    trailer: &[u8; TRAILER_BYTES],
    byte_length: u64,
) -> Result<PackMetadata, StoreError> {
    if header[..8] != contract::PACK_MAGIC
        || u16::from_be_bytes(
            header[8..10]
                .try_into()
                .map_err(|_| corrupt("pack_contract", "pack header version is truncated"))?,
        ) != contract::PACK_CONTRACT_VERSION
        || u16::from_be_bytes(
            header[10..12]
                .try_into()
                .map_err(|_| corrupt("pack_contract", "pack header flags are truncated"))?,
        ) != 0
    {
        return Err(corrupt(
            "pack_contract",
            "pack magic, version, or reserved flags are invalid",
        ));
    }
    if footer[..8] != contract::PACK_INDEX_MAGIC
        || u16::from_be_bytes(
            footer[8..10]
                .try_into()
                .map_err(|_| corrupt("pack_index_contract", "pack footer version is truncated"))?,
        ) != contract::PACK_CONTRACT_VERSION
        || u16::from_be_bytes(
            footer[10..12]
                .try_into()
                .map_err(|_| corrupt("pack_index_contract", "pack footer flags are truncated"))?,
        ) != 0
    {
        return Err(corrupt(
            "pack_index_contract",
            "pack footer magic, version, or reserved flags are invalid",
        ));
    }
    if trailer[40..] != contract::PACK_END_MAGIC {
        return Err(corrupt(
            "pack_closing_magic",
            "pack closing magic is not current",
        ));
    }
    let total = usize_from_u64(byte_length, "pack_file_length")?;
    let trailer_start = total
        .checked_sub(TRAILER_BYTES)
        .ok_or_else(|| corrupt("pack_trailer", "pack trailer is truncated"))?;
    let footer_offset = usize_from_u64(
        u64::from_be_bytes(
            trailer[..8]
                .try_into()
                .map_err(|_| corrupt("pack_footer_offset", "pack footer offset is truncated"))?,
        ),
        "pack_footer_offset",
    )?;
    let footer_end = footer_offset
        .checked_add(FOOTER_BYTES)
        .ok_or_else(|| corrupt("pack_footer_overflow", "pack footer offset overflows"))?;
    let index_start = footer_offset
        .checked_sub(index.len())
        .filter(|start| *start >= HEADER_BYTES)
        .ok_or_else(|| corrupt("pack_index_bounds", "pack index starts before the header"))?;
    if footer_end != trailer_start {
        return Err(corrupt(
            "pack_footer_bounds",
            "pack footer or index coordinates are invalid",
        ));
    }
    let entry_count = usize_from_u64(
        u64::from_be_bytes(
            footer[12..20]
                .try_into()
                .map_err(|_| corrupt("pack_entry_count", "pack entry count is truncated"))?,
        ),
        "pack_entry_count",
    )?;
    if entry_count == 0
        || entry_count > contract::MAXIMUM_PACK_ENTRIES
        || entry_count.checked_mul(INDEX_ENTRY_BYTES) != Some(index.len())
    {
        return Err(pack_error(
            StoreErrorClass::Resource,
            "pack_index_size",
            "pack index count or byte length is invalid",
        ));
    }
    let payload_bytes = u64::from_be_bytes(
        footer[20..28]
            .try_into()
            .map_err(|_| corrupt("pack_payload_length", "pack payload length is truncated"))?,
    );
    if payload_bytes != (index_start - HEADER_BYTES) as u64
        || u64::from_be_bytes(
            footer[28..36]
                .try_into()
                .map_err(|_| corrupt("pack_index_length", "pack index length is truncated"))?,
        ) != index.len() as u64
    {
        return Err(corrupt(
            "pack_payload_length",
            "pack payload or index length disagrees with file coordinates",
        ));
    }
    let index_checksum: [u8; 32] = footer[36..68]
        .try_into()
        .map_err(|_| corrupt("pack_index_checksum", "pack index checksum is truncated"))?;
    if digest(contract::PACK_INDEX_CHECKSUM_DOMAIN, index) != index_checksum {
        return Err(corrupt(
            "pack_index_checksum",
            "pack footer index checksum does not match",
        ));
    }
    let mut entries = Vec::with_capacity(entry_count);
    let mut previous = None;
    let mut expected_offset = HEADER_BYTES as u64;
    for ordinal in 0..entry_count {
        let start = ordinal * INDEX_ENTRY_BYTES;
        let domain = ObjectDomain::from_tag(index[start])?;
        let key = ObjectKey::from_digest(
            domain,
            index[start + 1..start + 33]
                .try_into()
                .map_err(|_| corrupt("pack_index_digest", "pack entry digest is truncated"))?,
        );
        if previous.is_some_and(|previous| previous >= key) {
            return Err(corrupt(
                "pack_index_order",
                "pack footer keys are not strictly ordered",
            ));
        }
        previous = Some(key);
        let offset = u64::from_be_bytes(
            index[start + 33..start + 41]
                .try_into()
                .map_err(|_| corrupt("pack_entry_offset", "pack entry offset is truncated"))?,
        );
        let encoded_length = u64::from_be_bytes(
            index[start + 41..start + 49]
                .try_into()
                .map_err(|_| corrupt("pack_entry_length", "pack entry length is truncated"))?,
        );
        let uncompressed_length = u64::from_be_bytes(
            index[start + 49..start + 57]
                .try_into()
                .map_err(|_| corrupt("pack_entry_length", "pack entry length is truncated"))?,
        );
        if offset != expected_offset
            || encoded_length != uncompressed_length
            || encoded_length > domain.maximum_bytes() as u64
        {
            return Err(corrupt(
                "pack_entry_layout",
                "pack entry coordinates, compression, or size are invalid",
            ));
        }
        expected_offset = offset
            .checked_add(encoded_length)
            .ok_or_else(|| corrupt("pack_entry_overflow", "pack entry end overflows"))?;
        entries.push(PackIndexEntry {
            key,
            offset,
            encoded_length,
            uncompressed_length,
            checksum: index[start + 57..start + 89]
                .try_into()
                .map_err(|_| corrupt("pack_entry_checksum", "entry checksum is truncated"))?,
        });
    }
    if expected_offset != HEADER_BYTES as u64 + payload_bytes {
        return Err(corrupt(
            "pack_payload_trailing",
            "pack payload contains gaps, overlap, or trailing bytes",
        ));
    }
    let nonce: [u8; 16] = header[12..28]
        .try_into()
        .map_err(|_| corrupt("pack_nonce", "pack nonce is truncated"))?;
    if nonce_for_entries(&entries) != nonce {
        return Err(corrupt(
            "pack_nonce",
            "pack nonce does not commit to its exact sorted entries",
        ));
    }
    Ok(PackMetadata {
        byte_length,
        nonce,
        payload_bytes,
        index_checksum,
        pack_checksum: trailer[8..40]
            .try_into()
            .map_err(|_| corrupt("pack_checksum", "pack checksum is truncated"))?,
        entries,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealedPack {
    pub id: PackId,
    pub bytes: Vec<u8>,
    pub metadata: PackMetadata,
}

#[derive(Clone, Debug, Default)]
pub struct PackBuilder {
    objects: BTreeMap<ObjectKey, Vec<u8>>,
}

impl PackBuilder {
    pub fn insert(&mut self, key: ObjectKey, bytes: &[u8]) -> Result<(), StoreError> {
        key.verify(bytes)?;
        match self.objects.get(&key) {
            Some(existing) if existing == bytes => Ok(()),
            Some(_) => Err(corrupt(
                "pack_builder_collision",
                "one object identity is bound to different staged bytes",
            )),
            None => {
                self.objects.insert(key, bytes.to_vec());
                Ok(())
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub fn seal(self) -> Result<SealedPack, StoreError> {
        seal_entries(self.objects.into_iter().collect())
    }

    pub fn seal_targeted(self, target_bytes: usize) -> Result<Vec<SealedPack>, StoreError> {
        if target_bytes < MINIMUM_PACK_BYTES + INDEX_ENTRY_BYTES {
            return Err(pack_error(
                StoreErrorClass::Input,
                "pack_target_size",
                "target pack size cannot hold one indexed object",
            ));
        }
        if self.objects.is_empty() {
            return Err(pack_error(
                StoreErrorClass::Input,
                "pack_empty",
                "an empty immutable pack is not valid",
            ));
        }
        let mut packs = Vec::new();
        let mut current = Vec::new();
        let mut payload_bytes = 0_usize;
        for (key, bytes) in self.objects {
            let next_count = current.len() + 1;
            let next_payload = payload_bytes
                .checked_add(bytes.len())
                .ok_or_else(|| corrupt("pack_size_overflow", "pack payload size overflows"))?;
            let next_size = encoded_pack_size(next_count, next_payload)?;
            if !current.is_empty() && next_size > target_bytes {
                packs.push(seal_entries(current)?);
                current = Vec::new();
                payload_bytes = 0;
            }
            payload_bytes = payload_bytes
                .checked_add(bytes.len())
                .ok_or_else(|| corrupt("pack_size_overflow", "pack payload size overflows"))?;
            current.push((key, bytes));
        }
        if !current.is_empty() {
            packs.push(seal_entries(current)?);
        }
        Ok(packs)
    }
}

fn seal_entries(entries: Vec<(ObjectKey, Vec<u8>)>) -> Result<SealedPack, StoreError> {
    if entries.is_empty() {
        return Err(pack_error(
            StoreErrorClass::Input,
            "pack_empty",
            "an empty immutable pack is not valid",
        ));
    }
    if entries.len() > contract::MAXIMUM_PACK_ENTRIES {
        return Err(pack_error(
            StoreErrorClass::Resource,
            "pack_entry_count",
            "pack entry count exceeds the hostile decoder bound",
        ));
    }
    let payload_bytes = entries.iter().try_fold(0_usize, |total, (_, bytes)| {
        total
            .checked_add(bytes.len())
            .ok_or_else(|| corrupt("pack_size_overflow", "pack payload size overflows"))
    })?;
    let capacity = encoded_pack_size(entries.len(), payload_bytes)?;
    if capacity > contract::MAXIMUM_PACK_BYTES {
        return Err(pack_error(
            StoreErrorClass::Resource,
            "pack_size",
            "sealed pack would exceed the hostile decoder bound",
        ));
    }
    let mut index_entries = Vec::with_capacity(entries.len());
    let mut offset = HEADER_BYTES as u64;
    for (key, bytes) in &entries {
        key.verify(bytes)?;
        let length = bytes.len() as u64;
        index_entries.push(PackIndexEntry {
            key: *key,
            offset,
            encoded_length: length,
            uncompressed_length: length,
            checksum: digest(contract::PACK_ENTRY_CHECKSUM_DOMAIN, bytes),
        });
        offset = offset
            .checked_add(length)
            .ok_or_else(|| corrupt("pack_offset_overflow", "pack payload offset overflows"))?;
    }
    let nonce = nonce_for_entries(&index_entries);
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(&contract::PACK_MAGIC);
    bytes.extend_from_slice(&contract::PACK_CONTRACT_VERSION.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&nonce);
    for (_, value) in &entries {
        bytes.extend_from_slice(value);
    }
    let mut index = Vec::with_capacity(entries.len() * INDEX_ENTRY_BYTES);
    for entry in &index_entries {
        index.push(entry.key.domain.tag());
        index.extend_from_slice(&entry.key.digest.bytes());
        index.extend_from_slice(&entry.offset.to_be_bytes());
        index.extend_from_slice(&entry.encoded_length.to_be_bytes());
        index.extend_from_slice(&entry.uncompressed_length.to_be_bytes());
        index.extend_from_slice(&entry.checksum);
    }
    let index_checksum = digest(contract::PACK_INDEX_CHECKSUM_DOMAIN, &index);
    bytes.extend_from_slice(&index);
    let footer_offset = bytes.len() as u64;
    bytes.extend_from_slice(&contract::PACK_INDEX_MAGIC);
    bytes.extend_from_slice(&contract::PACK_CONTRACT_VERSION.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&(entries.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&(payload_bytes as u64).to_be_bytes());
    bytes.extend_from_slice(&(index.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&index_checksum);
    let pack_checksum = digest(contract::PACK_CHECKSUM_DOMAIN, &bytes);
    bytes.extend_from_slice(&footer_offset.to_be_bytes());
    bytes.extend_from_slice(&pack_checksum);
    bytes.extend_from_slice(&contract::PACK_END_MAGIC);
    if bytes.len() != capacity {
        return Err(corrupt(
            "pack_encoded_size",
            "sealed pack size disagrees with the checked prediction",
        ));
    }
    let id = PackId::of(&bytes);
    let metadata = PackMetadata::decode(&bytes, true)?;
    Ok(SealedPack {
        id,
        bytes,
        metadata,
    })
}

fn nonce_for_entries(entries: &[PackIndexEntry]) -> [u8; 16] {
    let mut hasher = blake3::Hasher::new_derive_key(contract::PACK_NONCE_DOMAIN);
    hasher.update(&(entries.len() as u64).to_be_bytes());
    for entry in entries {
        hasher.update(&[entry.key.domain.tag()]);
        hasher.update(&entry.key.digest.bytes());
        hasher.update(&entry.encoded_length.to_be_bytes());
        hasher.update(&entry.checksum);
    }
    let mut nonce = [0_u8; 16];
    nonce.copy_from_slice(&hasher.finalize().as_bytes()[..16]);
    nonce
}

fn encoded_pack_size(entries: usize, payload_bytes: usize) -> Result<usize, StoreError> {
    HEADER_BYTES
        .checked_add(payload_bytes)
        .and_then(|size| size.checked_add(entries.checked_mul(INDEX_ENTRY_BYTES)?))
        .and_then(|size| size.checked_add(FOOTER_BYTES))
        .and_then(|size| size.checked_add(TRAILER_BYTES))
        .ok_or_else(|| corrupt("pack_size_overflow", "pack encoded size overflows"))
}

fn digest(domain: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn read_at<R: Read + Seek>(
    reader: &mut R,
    offset: u64,
    target: &mut [u8],
) -> Result<(), StoreError> {
    reader
        .seek(SeekFrom::Start(offset))
        .map_err(|error| io_error("pack_seek", "failed to seek in immutable pack", error))?;
    reader
        .read_exact(target)
        .map_err(|error| read_error("pack_read", error))
}

fn hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

fn io_error(code: &'static str, message: &'static str, error: std::io::Error) -> StoreError {
    StoreError::new(StoreErrorClass::Io, code, format!("{message}: {error}"))
}

fn read_error(code: &'static str, error: std::io::Error) -> StoreError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        corrupt(code, "immutable pack is truncated")
    } else {
        io_error(code, "failed to read immutable pack bytes", error)
    }
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
        .ok_or_else(|| corrupt("pack_offset_overflow", "pack field offset overflows"))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| corrupt("pack_truncated", "pack field is truncated"))?
        .try_into()
        .map_err(|_| corrupt("pack_field_length", "pack field has the wrong byte length"))
}

fn usize_from_u64(value: u64, code: &'static str) -> Result<usize, StoreError> {
    usize::try_from(value).map_err(|_| {
        pack_error(
            StoreErrorClass::Resource,
            code,
            "pack integer does not fit this platform",
        )
    })
}

fn corrupt(code: &'static str, message: impl Into<String>) -> StoreError {
    pack_error(StoreErrorClass::Corrupt, code, message)
}

fn pack_error(
    class: StoreErrorClass,
    code: &'static str,
    message: impl Into<String>,
) -> StoreError {
    StoreError::new(class, code, message)
}
