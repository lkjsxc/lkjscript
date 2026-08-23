//! Compact disposable object-location catalog rebuilt from immutable pack footers.

use super::contract;
use super::object::{ObjectDomain, ObjectKey, StoreError, StoreErrorClass};
use super::pack::{PackId, PackMetadata};
use std::collections::BTreeMap;

const HEADER_BYTES: usize = 8 + 2 + 2 + 32 + 8;
const ENTRY_BYTES: usize = 1 + 32 + 32 + 8 + 8 + 32;
const TRAILER_BYTES: usize = 32 + 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogLocation {
    pub pack: PackId,
    pub offset: u64,
    pub length: u64,
    pub checksum: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DuplicateObject {
    pub key: ObjectKey,
    pub packs: Vec<PackId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectCatalog {
    generation: [u8; 32],
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
            generation: catalog_generation(&[]),
            locations: BTreeMap::new(),
        }
    }

    pub fn rebuild<'a>(
        packs: impl IntoIterator<Item = (PackId, &'a PackMetadata)>,
    ) -> Result<CatalogBuild, StoreError> {
        let mut descriptors = packs.into_iter().collect::<Vec<_>>();
        descriptors.sort_by_key(|(pack, _)| *pack);
        let generation = catalog_generation(&descriptors);
        let mut all_locations = BTreeMap::<ObjectKey, Vec<CatalogLocation>>::new();
        for (pack, metadata) in descriptors {
            for entry in &metadata.entries {
                all_locations
                    .entry(entry.key)
                    .or_default()
                    .push(CatalogLocation {
                        pack,
                        offset: entry.offset,
                        length: entry.encoded_length,
                        checksum: entry.checksum,
                    });
            }
        }
        let mut locations = BTreeMap::new();
        let mut duplicates = Vec::new();
        for (key, mut candidates) in all_locations {
            candidates.sort_by_key(|candidate| candidate.pack);
            let primary = candidates[0];
            locations.insert(key, primary);
            if candidates.len() > 1 {
                duplicates.push(DuplicateObject {
                    key,
                    packs: candidates
                        .into_iter()
                        .map(|candidate| candidate.pack)
                        .collect(),
                });
            }
        }
        Ok(CatalogBuild {
            catalog: Self {
                generation,
                locations,
            },
            duplicates,
        })
    }

    pub const fn generation(&self) -> [u8; 32] {
        self.generation
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

    pub fn entries(&self) -> impl Iterator<Item = (ObjectKey, CatalogLocation)> + '_ {
        self.locations
            .iter()
            .map(|(key, location)| (*key, *location))
    }

    pub fn encode(&self) -> Result<Vec<u8>, StoreError> {
        let entries_bytes = self
            .locations
            .len()
            .checked_mul(ENTRY_BYTES)
            .ok_or_else(|| corrupt("catalog_size_overflow", "catalog byte size overflows"))?;
        let capacity = HEADER_BYTES
            .checked_add(entries_bytes)
            .and_then(|size| size.checked_add(TRAILER_BYTES))
            .ok_or_else(|| corrupt("catalog_size_overflow", "catalog byte size overflows"))?;
        if capacity > contract::MAXIMUM_CATALOG_BYTES {
            return Err(StoreError::new(
                StoreErrorClass::Resource,
                "catalog_size",
                "catalog exceeds the hostile decoder byte bound",
            ));
        }
        let mut bytes = Vec::with_capacity(capacity);
        bytes.extend_from_slice(&contract::CATALOG_MAGIC);
        bytes.extend_from_slice(&contract::CATALOG_CONTRACT_VERSION.to_be_bytes());
        bytes.extend_from_slice(&0_u16.to_be_bytes());
        bytes.extend_from_slice(&self.generation);
        bytes.extend_from_slice(&(self.locations.len() as u64).to_be_bytes());
        for (key, location) in &self.locations {
            bytes.push(key.domain.tag());
            bytes.extend_from_slice(&key.digest.bytes());
            bytes.extend_from_slice(&location.pack.bytes());
            bytes.extend_from_slice(&location.offset.to_be_bytes());
            bytes.extend_from_slice(&location.length.to_be_bytes());
            bytes.extend_from_slice(&location.checksum);
        }
        let checksum = digest(contract::CATALOG_CHECKSUM_DOMAIN, &bytes);
        bytes.extend_from_slice(&checksum);
        bytes.extend_from_slice(&contract::CATALOG_END_MAGIC);
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8], expected_generation: [u8; 32]) -> Result<Self, StoreError> {
        if bytes.len() < HEADER_BYTES + TRAILER_BYTES
            || bytes.len() > contract::MAXIMUM_CATALOG_BYTES
        {
            return Err(StoreError::new(
                StoreErrorClass::Resource,
                "catalog_size",
                "catalog byte length is outside decoder bounds",
            ));
        }
        if bytes[..8] != contract::CATALOG_MAGIC
            || read_u16(bytes, 8)? != contract::CATALOG_CONTRACT_VERSION
            || read_u16(bytes, 10)? != 0
        {
            return Err(corrupt(
                "catalog_contract",
                "catalog magic, version, or reserved flags are invalid",
            ));
        }
        let generation = read_array::<32>(bytes, 12)?;
        if generation != expected_generation {
            return Err(corrupt(
                "catalog_generation",
                "catalog is stale for the discovered immutable pack set",
            ));
        }
        let count = usize::try_from(read_u64(bytes, 44)?).map_err(|_| {
            StoreError::new(
                StoreErrorClass::Resource,
                "catalog_count",
                "catalog count does not fit this platform",
            )
        })?;
        let expected = HEADER_BYTES
            .checked_add(
                count
                    .checked_mul(ENTRY_BYTES)
                    .ok_or_else(|| corrupt("catalog_size_overflow", "catalog size overflows"))?,
            )
            .and_then(|size| size.checked_add(TRAILER_BYTES))
            .ok_or_else(|| corrupt("catalog_size_overflow", "catalog size overflows"))?;
        if expected != bytes.len() {
            return Err(corrupt(
                "catalog_length",
                "catalog count does not describe the exact byte length",
            ));
        }
        let trailer = bytes.len() - TRAILER_BYTES;
        if read_array::<8>(bytes, trailer + 32)? != contract::CATALOG_END_MAGIC {
            return Err(corrupt(
                "catalog_closing_magic",
                "catalog closing magic is not current",
            ));
        }
        if digest(contract::CATALOG_CHECKSUM_DOMAIN, &bytes[..trailer])
            != read_array::<32>(bytes, trailer)?
        {
            return Err(corrupt(
                "catalog_checksum",
                "catalog checksum does not match",
            ));
        }
        let mut locations = BTreeMap::new();
        let mut previous = None;
        for ordinal in 0..count {
            let offset = HEADER_BYTES + ordinal * ENTRY_BYTES;
            let domain = ObjectDomain::from_tag(bytes[offset])?;
            let key = ObjectKey::from_digest(domain, read_array::<32>(bytes, offset + 1)?);
            if previous.is_some_and(|previous| previous >= key) {
                return Err(corrupt(
                    "catalog_order",
                    "catalog keys are not strictly ordered",
                ));
            }
            previous = Some(key);
            let location = CatalogLocation {
                pack: PackId::from_bytes(read_array::<32>(bytes, offset + 33)?),
                offset: read_u64(bytes, offset + 65)?,
                length: read_u64(bytes, offset + 73)?,
                checksum: read_array::<32>(bytes, offset + 81)?,
            };
            if location.length > domain.maximum_bytes() as u64 {
                return Err(StoreError::new(
                    StoreErrorClass::Resource,
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
            locations.insert(key, location);
        }
        Ok(Self {
            generation,
            locations,
        })
    }
}

fn catalog_generation(descriptors: &[(PackId, &PackMetadata)]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(contract::CATALOG_GENERATION_DOMAIN);
    hasher.update(&(descriptors.len() as u64).to_be_bytes());
    for (pack, metadata) in descriptors {
        hasher.update(&pack.bytes());
        hasher.update(&metadata.byte_length.to_be_bytes());
        hasher.update(&metadata.index_checksum);
    }
    *hasher.finalize().as_bytes()
}

fn digest(domain: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
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

fn corrupt(code: &'static str, message: impl Into<String>) -> StoreError {
    StoreError::new(StoreErrorClass::Corrupt, code, message)
}
