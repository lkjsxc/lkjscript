//! Catalog publication precedes reclamation under exclusive reader and writer leases.

use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub(crate) struct PackCompactionReceipt {
    pub objects_retained: usize,
    pub packs_before: usize,
    pub packs_after: usize,
    pub pack_bytes_before: u64,
    pub pack_bytes_after: u64,
    pub pack_bytes_reclaimed: u64,
}

impl PackDirectoryStore {
    /// The caller must already hold the repository's exclusive publication lock.
    /// Never wait for old readers while holding that lock: they may need to publish.
    pub(crate) fn open_for_gc(root: &Path) -> Result<Self, StoreError> {
        let root_directory = open_directory(root, "pack_store_root")?;
        FileExt::try_lock_exclusive(&root_directory).map_err(|error| {
            if error.kind() == std::io::ErrorKind::WouldBlock {
                resource(
                    "gc_readers_active",
                    "repository readers still retain pack snapshots; retry after they finish",
                )
            } else {
                io_error(
                    "gc_reader_lease",
                    "failed to acquire exclusive pack lease",
                    error,
                )
            }
        })?;
        let mut store = Self::open_with_root(root, root_directory)?;
        store.gc_exclusive = true;
        Ok(store)
    }

    pub(crate) fn compact_reachable(
        &mut self,
        objects: BTreeMap<ObjectKey, Vec<u8>>,
    ) -> Result<PackCompactionReceipt, StoreError> {
        self.compact_inner(objects, contract::COMPACTION_TARGET_PACK_BYTES, &mut |_| {
            Ok(())
        })
    }

    fn compact_inner(
        &mut self,
        objects: BTreeMap<ObjectKey, Vec<u8>>,
        target_bytes: usize,
        checkpoint: &mut dyn FnMut(SealCheckpoint) -> Result<(), StoreError>,
    ) -> Result<PackCompactionReceipt, StoreError> {
        if !self.gc_exclusive || !self.staged.is_empty() {
            return Err(corrupt(
                "gc_lease",
                "compaction requires exclusive pack ownership and no staged objects",
            ));
        }
        let mut old = Vec::new();
        let mut before = 0_u64;
        for (id, name) in owned_pack_names(&self.packs_directory)? {
            let file = open_regular_file_at(&self.packs_directory, &name, "gc_pack_type")?;
            before = before
                .checked_add(regular_file_length(&file, "gc_pack_length")?)
                .ok_or_else(|| resource("gc_pack_bytes", "physical pack byte count overflowed"))?;
            old.push((id, name));
        }
        let retained = objects.len();
        let mut builder = PackBuilder::default();
        for (key, bytes) in objects {
            builder.insert(key, &bytes)?;
        }
        let packs = builder.seal_compressed_targeted(target_bytes)?;
        let after = packs.iter().try_fold(0_u64, |total, pack| {
            total
                .checked_add(pack.bytes.len() as u64)
                .ok_or_else(|| resource("gc_pack_bytes", "physical pack byte count overflowed"))
        })?;
        for pack in &packs {
            install_pack(
                &self.packs_directory,
                &self.staging_directory,
                pack,
                checkpoint,
            )?;
        }
        sync_directory(&self.packs_directory, "gc_pack_directory_sync")?;
        checkpoint(SealCheckpoint::PackDirectorySynced)?;
        sync_directory(&self.staging_directory, "gc_staging_directory_sync")?;
        checkpoint(SealCheckpoint::StagingDirectorySynced)?;
        let metadata = packs
            .into_iter()
            .map(|pack| (pack.id, pack.metadata))
            .collect::<BTreeMap<_, _>>();
        let rebuilt = ObjectCatalog::rebuild(metadata.iter().map(|(id, value)| (*id, value)))?;
        let mut work = CatalogWork::default();
        let next = write_rebuilt_catalog(
            &self.catalog_directory,
            &self.segments_directory,
            &rebuilt.catalog,
            &metadata,
            &mut work,
            checkpoint,
        )?;
        // From this durable catalog switch onward, every reachable object has a selected copy.
        // Interruption may leave extra old files, but cannot strand the accepted HEAD.
        self.segment_files = open_segment_handles(&self.segments_directory, &next)?;
        self.catalog = next;
        self.catalog_blocks.get_mut().clear();
        self.metadata.get_mut().clear();
        for (id, name) in &old {
            if !metadata.contains_key(id) {
                rustix::fs::unlinkat(&self.packs_directory, name.as_str(), AtFlags::empty())
                    .map_err(|error| {
                        rustix_error(
                            "gc_pack_remove",
                            "compacted catalog is published but old pack cleanup failed",
                            error,
                        )
                    })?;
            }
        }
        sync_directory(&self.packs_directory, "gc_pack_cleanup_sync")?;
        cleanup_derived(
            &self.staging_directory,
            &self.catalog_directory,
            &self.segments_directory,
            &self.catalog,
            &mut work,
            checkpoint,
        )?;
        self.duplicates.clear();
        self.staging_leftovers = classify_staging_leftovers(&self.staging_directory)?;
        self.catalog_leftovers = classify_catalog_leftovers(
            &self.catalog_directory,
            &self.segments_directory,
            &self.catalog,
        )?;
        self.catalog_state = CatalogState::IncrementalPersisted;
        self.catalog_rebuild_note = None;
        let mut total = self.catalog_work.get();
        total.add(work);
        self.catalog_work.set(total);
        Ok(PackCompactionReceipt {
            objects_retained: retained,
            packs_before: old.len(),
            packs_after: metadata.len(),
            pack_bytes_before: before,
            pack_bytes_after: after,
            pack_bytes_reclaimed: before.saturating_sub(after),
        })
    }
}

fn owned_pack_names(directory: &File) -> Result<Vec<(PackId, String)>, StoreError> {
    let entries = Dir::read_from(directory)
        .map_err(|error| rustix_error("gc_pack_scan", "failed to open pack directory", error))?;
    let mut result = Vec::new();
    let mut visited = 0;
    for entry in entries {
        let entry = entry.map_err(|error| {
            rustix_error("gc_pack_scan", "failed to read pack directory", error)
        })?;
        let bytes = entry.file_name().to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        if visited == contract::MAXIMUM_CATALOG_PACKS + contract::MAXIMUM_CATALOG_LEFTOVERS {
            return Err(resource(
                "gc_directory_capacity",
                "compaction directory scan exceeds its finite entry bound",
            ));
        }
        visited += 1;
        let Ok(name) = std::str::from_utf8(bytes) else {
            continue;
        };
        let Ok(id) = PackId::parse_file_name(name) else {
            continue;
        };
        if result.len() == contract::MAXIMUM_CATALOG_PACKS {
            return Err(resource(
                "gc_pack_count",
                "compaction pack inventory exceeds its bound",
            ));
        }
        result.push((id, name.to_owned()));
    }
    result.sort_unstable();
    Ok(result)
}

#[cfg(test)]
mod tests;
