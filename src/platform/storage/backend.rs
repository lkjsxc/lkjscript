//! Object access shared by persistent and ephemeral repositories.

use super::catalog::{CatalogCommitment, CatalogHistory, CatalogLocation, CatalogWork};
use super::directory::{
    CatalogFooterVerifyReceipt, CatalogObservation, CatalogState, PackDirectoryStore, SealReceipt,
};
use super::object::{
    ImmutableObjectStore, ObjectKey, StageOutcome, StoreError, StoreErrorClass, StoreReadAdmission,
    StoreWork,
};
use super::project::{MemoryObjectStore, memory_error};

#[derive(Debug)]
pub(crate) enum StorageBackend {
    Disk(Box<PackDirectoryStore>),
    Memory(MemoryObjectStore),
}

impl StorageBackend {
    pub(crate) fn root(&self) -> &std::path::Path {
        match self {
            Self::Disk(store) => store.root(),
            Self::Memory(_) => std::path::Path::new("memory:project"),
        }
    }

    pub(crate) fn staged_len(&self) -> usize {
        match self {
            Self::Disk(store) => store.staged_len(),
            Self::Memory(store) => store.staged,
        }
    }

    pub(crate) fn staging_leftovers(&self) -> &[String] {
        match self {
            Self::Disk(store) => store.staging_leftovers(),
            Self::Memory(_) => &[],
        }
    }

    pub(crate) fn catalog_location(
        &self,
        key: ObjectKey,
    ) -> Result<Option<CatalogLocation>, StoreError> {
        match self {
            Self::Disk(store) => store.catalog_location(key),
            Self::Memory(_) => Err(memory_error(
                StoreErrorClass::Input,
                "memory_pack_location",
                "memory objects have no physical pack coordinates",
            )),
        }
    }

    pub(crate) fn catalog_observation(&self) -> CatalogObservation {
        match self {
            Self::Disk(store) => store.catalog_observation(),
            Self::Memory(store) => {
                let mut hash = blake3::Hasher::new_derive_key("lkjscript.memory-object-inventory");
                for (key, bytes) in store.objects.iter() {
                    hash.update(&[key.domain.tag()]);
                    hash.update(&key.digest.bytes());
                    hash.update(&(bytes.as_slice().len() as u64).to_le_bytes());
                }
                CatalogObservation {
                    identity: "lkjscript-memory-object-index",
                    contract_version: crate::platform::canonical_schema::SCHEMA_EPOCH,
                    state: CatalogState::Loaded,
                    generation: 0,
                    commitment: CatalogCommitment::from_bytes(*hash.finalize().as_bytes()),
                    entries: store.objects.len() as u64,
                    packs: 0,
                    segments: 0,
                    segment_bytes: 0,
                    segment_metadata_bytes: 0,
                    maximum_level: None,
                    maximum_live_segments: 0,
                    maximum_lookup_segments: 0,
                    block_entries: 0,
                    history: CatalogHistory::default(),
                    work: CatalogWork::default(),
                    leftovers: Vec::new(),
                }
            }
        }
    }

    pub(crate) fn verify_catalog_from_footers(
        &self,
    ) -> Result<CatalogFooterVerifyReceipt, StoreError> {
        match self {
            Self::Disk(store) => store.verify_catalog_from_footers(),
            Self::Memory(_) => Err(memory_error(
                StoreErrorClass::Input,
                "memory_pack_footer",
                "memory objects have no filesystem pack footers to verify",
            )),
        }
    }

    pub(crate) fn seal_staged(
        &mut self,
        target_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<SealReceipt, StoreError> {
        match self {
            Self::Disk(store) => store.seal_staged(target_bytes, work),
            Self::Memory(store) => Ok(SealReceipt {
                packs: Vec::new(),
                objects: std::mem::take(&mut store.staged),
                catalog_state: CatalogState::Loaded,
                catalog_work: Box::default(),
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn seal_staged_with_fault(
        &mut self,
        target_bytes: usize,
        work: &mut StoreWork,
        point: super::directory::SealCheckpoint,
    ) -> Result<SealReceipt, StoreError> {
        match self {
            Self::Disk(store) => store.seal_staged_with_fault(target_bytes, work, point),
            Self::Memory(_) => Err(memory_error(
                StoreErrorClass::Input,
                "memory_pack_fault",
                "filesystem durability fault points do not exist in memory stores",
            )),
        }
    }
}

impl ImmutableObjectStore for StorageBackend {
    fn read(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.read_admitted(
            key,
            maximum_bytes,
            &mut StoreReadAdmission::unbounded(),
            work,
        )
    }

    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        match self {
            Self::Disk(store) => store.read_admitted(key, maximum_bytes, admission, work),
            Self::Memory(store) => {
                admission.admit_catalog_lookup()?;
                work.add(crate::platform::storage::object::StoreWork { catalog_lookups: 1, ..crate::platform::storage::object::StoreWork::default() });
                let Some(value) = store.objects.get(&key) else {
                    return Ok(None);
                };
                let bytes = value.as_slice();
                if bytes.len() > maximum_bytes {
                    return Err(memory_error(
                        StoreErrorClass::Resource,
                        "memory_read_limit",
                        "memory object exceeds the caller read bound",
                    ));
                }
                admission.admit_object(bytes.len())?;
                value.verify(key)?;
                work.add(crate::platform::storage::object::StoreWork { objects_read: 1, ..crate::platform::storage::object::StoreWork::default() });
                work.add(crate::platform::storage::object::StoreWork { bytes_read: bytes.len() as u64, ..crate::platform::storage::object::StoreWork::default() });
                Ok(Some(bytes.to_vec()))
            }
        }
    }

    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        self.contains_admitted(key, &mut StoreReadAdmission::unbounded(), work)
    }

    fn contains_admitted(
        &self,
        key: ObjectKey,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<bool, StoreError> {
        match self {
            Self::Disk(store) => store.contains_admitted(key, admission, work),
            Self::Memory(store) => {
                admission.admit_catalog_lookup()?;
                work.add(crate::platform::storage::object::StoreWork { catalog_lookups: 1, ..crate::platform::storage::object::StoreWork::default() });
                Ok(store.objects.get(&key).is_some())
            }
        }
    }

    fn stage(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        self.stage_admitted(key, bytes, &mut StoreReadAdmission::unbounded(), work)
    }

    fn stage_admitted(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        match self {
            Self::Disk(store) => store.stage_admitted(key, bytes, admission, work),
            Self::Memory(store) => {
                key.verify(bytes)?;
                admission.admit_catalog_lookup()?;
                work.add(crate::platform::storage::object::StoreWork { catalog_lookups: 1, ..crate::platform::storage::object::StoreWork::default() });
                if let Some(existing) = store.objects.get(&key) {
                    admission.admit_object(existing.as_slice().len())?;
                    existing.verify(key)?;
                    work.add(crate::platform::storage::object::StoreWork { objects_read: 1, ..crate::platform::storage::object::StoreWork::default() });
                    work.bytes_read = work
                        .bytes_read
                        .saturating_add(existing.as_slice().len() as u64);
                    if existing.as_slice() != bytes {
                        return Err(memory_error(
                            StoreErrorClass::Corrupt,
                            "memory_object_collision",
                            "existing immutable object has foreign bytes",
                        ));
                    }
                    work.add(crate::platform::storage::object::StoreWork { objects_reused: 1, ..crate::platform::storage::object::StoreWork::default() });
                    return Ok(StageOutcome::Reused);
                }
                store.objects.insert(key, bytes)?;
                store.staged += 1;
                work.add(crate::platform::storage::object::StoreWork { objects_staged: 1, ..crate::platform::storage::object::StoreWork::default() });
                work.add(crate::platform::storage::object::StoreWork { bytes_staged: bytes.len() as u64, ..crate::platform::storage::object::StoreWork::default() });
                Ok(StageOutcome::Inserted)
            }
        }
    }
}
