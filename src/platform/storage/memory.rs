//! In-memory packed object store used by kernel, map, and corruption oracles.

use super::catalog::ObjectCatalog;
use super::contract;
use super::object::{
    ImmutableObjectStore, ObjectKey, StageOutcome, StoreError, StoreErrorClass, StoreReadAdmission,
    StoreWork, stage_into_map,
};
use super::pack::{PackBuilder, PackId, PackMetadata, SealedPack};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct MemoryPackedStore {
    staged: BTreeMap<ObjectKey, Vec<u8>>,
    packs: BTreeMap<PackId, Vec<u8>>,
    metadata: BTreeMap<PackId, PackMetadata>,
    catalog: ObjectCatalog,
}

impl Default for MemoryPackedStore {
    fn default() -> Self {
        Self {
            staged: BTreeMap::new(),
            packs: BTreeMap::new(),
            metadata: BTreeMap::new(),
            catalog: ObjectCatalog::empty(),
        }
    }
}

impl MemoryPackedStore {
    pub fn staged_len(&self) -> usize {
        self.staged.len()
    }

    pub fn pack_len(&self) -> usize {
        self.packs.len()
    }

    pub fn catalog(&self) -> &ObjectCatalog {
        &self.catalog
    }

    pub fn seal_staged(
        &mut self,
        target_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<Vec<PackId>, StoreError> {
        if self.staged.is_empty() {
            return Ok(Vec::new());
        }
        let staged = std::mem::take(&mut self.staged);
        let mut builder = PackBuilder::default();
        for (key, bytes) in staged {
            builder.insert(key, &bytes)?;
        }
        let packs = builder.seal_targeted(target_bytes)?;
        let mut installed = Vec::with_capacity(packs.len());
        for pack in packs {
            installed.push(pack.id);
            self.install(pack)?;
            work.packs_sealed = work.packs_sealed.saturating_add(1);
        }
        self.rebuild_catalog()?;
        Ok(installed)
    }

    pub fn install(&mut self, pack: SealedPack) -> Result<(), StoreError> {
        if PackId::of(&pack.bytes) != pack.id {
            return Err(store_error(
                StoreErrorClass::Corrupt,
                "memory_pack_identity",
                "sealed pack bytes do not match their physical identity",
            ));
        }
        pack.metadata.verify_all(&pack.bytes)?;
        match self.packs.get(&pack.id) {
            Some(existing) if existing == &pack.bytes => return Ok(()),
            Some(_) => {
                return Err(store_error(
                    StoreErrorClass::Corrupt,
                    "memory_pack_collision",
                    "one physical pack identity is bound to different bytes",
                ));
            }
            None => {}
        }
        self.metadata.insert(pack.id, pack.metadata);
        self.packs.insert(pack.id, pack.bytes);
        Ok(())
    }

    pub fn rebuild_catalog(&mut self) -> Result<Vec<super::catalog::DuplicateObject>, StoreError> {
        let build = ObjectCatalog::rebuild(
            self.metadata
                .iter()
                .map(|(pack, metadata)| (*pack, metadata)),
        )?;
        self.catalog = build.catalog;
        Ok(build.duplicates)
    }

    #[cfg(test)]
    pub fn corrupt_pack_byte(&mut self, pack: PackId, offset: usize) {
        if let Some(bytes) = self.packs.get_mut(&pack)
            && let Some(byte) = bytes.get_mut(offset)
        {
            *byte ^= 0x80;
        }
    }
}

impl ImmutableObjectStore for MemoryPackedStore {
    fn read(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let mut admission = StoreReadAdmission::unbounded();
        self.read_admitted(key, maximum_bytes, &mut admission, work)
    }

    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        admission.admit_catalog_lookup()?;
        work.catalog_lookups = work.catalog_lookups.saturating_add(1);
        if let Some(bytes) = self.staged.get(&key) {
            if bytes.len() > maximum_bytes {
                return Err(store_error(
                    StoreErrorClass::Resource,
                    "memory_read_limit",
                    "staged object exceeds the caller read bound",
                ));
            }
            admission.admit_object(bytes.len())?;
            key.verify(bytes)?;
            work.objects_read = work.objects_read.saturating_add(1);
            work.bytes_read = work.bytes_read.saturating_add(bytes.len() as u64);
            return Ok(Some(bytes.clone()));
        }
        let Some(location) = self.catalog.get(key) else {
            return Ok(None);
        };
        if !self.packs.contains_key(&location.pack) {
            return Err(store_error(
                StoreErrorClass::Corrupt,
                "memory_pack_missing",
                "catalog names a missing immutable pack",
            ));
        }
        let metadata = self.metadata.get(&location.pack).ok_or_else(|| {
            store_error(
                StoreErrorClass::Corrupt,
                "memory_pack_metadata_missing",
                "catalog names a pack without verified metadata",
            )
        })?;
        let (entry, _) = metadata
            .bounded_read_entry(key, maximum_bytes)?
            .ok_or_else(|| {
                store_error(
                    StoreErrorClass::Corrupt,
                    "memory_catalog_entry",
                    "catalog names an object absent from the pack footer",
                )
            })?;
        if entry.offset != location.offset
            || entry.encoded_length != location.length
            || entry.checksum != location.checksum
        {
            return Err(store_error(
                StoreErrorClass::Corrupt,
                "memory_catalog_location",
                "catalog coordinates disagree with the immutable pack footer",
            ));
        }
        admission.admit_object_bytes(entry.encoded_length)?;
        let bytes = self.packs.get(&location.pack).ok_or_else(|| {
            store_error(
                StoreErrorClass::Corrupt,
                "memory_pack_missing",
                "catalog names a missing immutable pack",
            )
        })?;
        work.packs_opened = work.packs_opened.saturating_add(1);
        let value = metadata.read(bytes, key, maximum_bytes)?;
        if let Some(value) = &value {
            work.objects_read = work.objects_read.saturating_add(1);
            work.bytes_read = work.bytes_read.saturating_add(value.len() as u64);
        }
        Ok(value)
    }

    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        let mut admission = StoreReadAdmission::unbounded();
        self.contains_admitted(key, &mut admission, work)
    }

    fn contains_admitted(
        &self,
        key: ObjectKey,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<bool, StoreError> {
        admission.admit_catalog_lookup()?;
        work.catalog_lookups = work.catalog_lookups.saturating_add(1);
        Ok(self.staged.contains_key(&key) || self.catalog.get(key).is_some())
    }

    fn stage(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        let mut admission = StoreReadAdmission::unbounded();
        self.stage_admitted(key, bytes, &mut admission, work)
    }

    fn stage_admitted(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        key.verify(bytes)?;
        if let Some(existing) =
            self.read_admitted(key, key.domain.maximum_bytes(), admission, work)?
        {
            if existing == bytes {
                work.objects_reused = work.objects_reused.saturating_add(1);
                return Ok(StageOutcome::Reused);
            }
            return Err(store_error(
                StoreErrorClass::Corrupt,
                "memory_object_collision",
                "existing immutable object has foreign bytes",
            ));
        }
        let outcome = stage_into_map(&mut self.staged, key, bytes)?;
        match outcome {
            StageOutcome::Inserted => {
                work.objects_staged = work.objects_staged.saturating_add(1);
                work.bytes_staged = work.bytes_staged.saturating_add(bytes.len() as u64);
            }
            StageOutcome::Reused => {
                work.objects_reused = work.objects_reused.saturating_add(1);
            }
        }
        Ok(outcome)
    }
}

pub fn default_target_bytes() -> usize {
    contract::TARGET_PACK_BYTES
}

fn store_error(
    class: StoreErrorClass,
    code: &'static str,
    message: impl Into<String>,
) -> StoreError {
    StoreError::new(class, code, message)
}
