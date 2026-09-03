//! Filesystem-backed immutable packs with a rebuildable incremental location catalog.

use super::catalog::{
    self, CatalogCommitment, CatalogEntry, CatalogHistory, CatalogIndex, CatalogLocation,
    CatalogManifest, CatalogWork, DuplicateObject, ObjectCatalog, PackDescriptor, SegmentId,
    SegmentMetadata,
};
use super::contract;
use super::object::{
    ImmutableObjectStore, ObjectKey, StageOutcome, StoreError, StoreErrorClass, StoreReadAdmission,
    StoreWork, stage_into_map,
};
use super::pack::{PackBuilder, PackId, PackIndexEntry, PackMetadata, SealedPack};
use rustix::fs::{AtFlags, Dir, Mode, OFlags};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const PACKS_DIRECTORY: &str = "packs";
const CATALOG_DIRECTORY: &str = "catalog";
const SEGMENTS_DIRECTORY: &str = "segments";
const STAGING_DIRECTORY: &str = "staging";
const CURRENT_CATALOG: &str = "current.lkjc";
const MANIFEST_STAGE_PREFIX: &str = ".manifest-stage-";
const SEGMENT_STAGE_PREFIX: &str = ".segment-stage-";
const PACK_STAGE_PREFIX: &str = ".pack-stage-";
const INJECTED_INTERRUPTION_CODE: &str = "pack_store_injected_interruption";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogState {
    Loaded,
    RebuiltPersisted,
    IncrementalPersisted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SealCheckpoint {
    PackStageCreated,
    PackPayloadWritten,
    PackFooterWritten,
    PackFileSynced,
    PackPublished,
    PackStageRemoved,
    PackDirectorySynced,
    StagingDirectorySynced,
    SegmentStageCreated,
    SegmentBytesWritten,
    SegmentFileSynced,
    SegmentPublished,
    SegmentStageRemoved,
    SegmentDirectorySynced,
    ManifestStageCreated,
    ManifestBytesWritten,
    ManifestFileSynced,
    ManifestPublished,
    CatalogDirectorySynced,
    ObsoleteSegmentsRemoved,
    DerivedCleanupSynced,
}

impl SealCheckpoint {
    pub(crate) const ALL: [Self; 21] = [
        Self::PackStageCreated,
        Self::PackPayloadWritten,
        Self::PackFooterWritten,
        Self::PackFileSynced,
        Self::PackPublished,
        Self::PackStageRemoved,
        Self::PackDirectorySynced,
        Self::StagingDirectorySynced,
        Self::SegmentStageCreated,
        Self::SegmentBytesWritten,
        Self::SegmentFileSynced,
        Self::SegmentPublished,
        Self::SegmentStageRemoved,
        Self::SegmentDirectorySynced,
        Self::ManifestStageCreated,
        Self::ManifestBytesWritten,
        Self::ManifestFileSynced,
        Self::ManifestPublished,
        Self::CatalogDirectorySynced,
        Self::ObsoleteSegmentsRemoved,
        Self::DerivedCleanupSynced,
    ];

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::PackStageCreated => "pack_stage_created",
            Self::PackPayloadWritten => "pack_payload_written",
            Self::PackFooterWritten => "pack_footer_written",
            Self::PackFileSynced => "pack_file_synced",
            Self::PackPublished => "pack_published",
            Self::PackStageRemoved => "pack_stage_removed",
            Self::PackDirectorySynced => "pack_directory_synced",
            Self::StagingDirectorySynced => "staging_directory_synced",
            Self::SegmentStageCreated => "catalog_segment_stage_created",
            Self::SegmentBytesWritten => "catalog_segment_bytes_written",
            Self::SegmentFileSynced => "catalog_segment_file_synced",
            Self::SegmentPublished => "catalog_segment_published",
            Self::SegmentStageRemoved => "catalog_segment_stage_removed",
            Self::SegmentDirectorySynced => "catalog_segment_directory_synced",
            Self::ManifestStageCreated => "catalog_manifest_stage_created",
            Self::ManifestBytesWritten => "catalog_manifest_bytes_written",
            Self::ManifestFileSynced => "catalog_manifest_file_synced",
            Self::ManifestPublished => "catalog_manifest_published",
            Self::CatalogDirectorySynced => "catalog_directory_synced",
            Self::ObsoleteSegmentsRemoved => "catalog_obsolete_segments_removed",
            Self::DerivedCleanupSynced => "catalog_derived_cleanup_synced",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealReceipt {
    pub packs: Vec<PackId>,
    pub objects: usize,
    pub catalog_state: CatalogState,
    pub catalog_work: Box<CatalogWork>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeepVerifyReceipt {
    pub packs: usize,
    pub selected_packs: usize,
    pub objects: usize,
    pub bytes_read: u64,
    pub duplicate_objects: Vec<DuplicateObject>,
    pub oracle_commitment: CatalogCommitment,
    pub manifest_commitment: CatalogCommitment,
    pub catalog_equal: bool,
    pub footer_scan_runs: u64,
    pub pack_footers_scanned: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CatalogFooterVerifyReceipt {
    pub packs: u64,
    pub entries: u64,
    pub footer_bytes_read: u64,
    pub duplicate_objects: u64,
    pub oracle_commitment: CatalogCommitment,
    pub manifest_commitment: CatalogCommitment,
    pub equal: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogObservation {
    pub identity: &'static str,
    pub contract_version: u16,
    pub state: CatalogState,
    pub generation: u64,
    pub commitment: CatalogCommitment,
    pub entries: u64,
    pub packs: u64,
    pub segments: usize,
    pub segment_bytes: u64,
    pub segment_metadata_bytes: u64,
    pub maximum_level: Option<u16>,
    pub maximum_live_segments: usize,
    pub maximum_lookup_segments: usize,
    pub block_entries: usize,
    pub history: CatalogHistory,
    pub work: CatalogWork,
    pub leftovers: Vec<String>,
}

struct StoreLayout {
    root: PathBuf,
    root_directory: File,
    packs_directory: File,
    catalog_directory: File,
    segments_directory: File,
    staging_directory: File,
}

#[derive(Debug)]
pub struct PackDirectoryStore {
    root: PathBuf,
    root_directory: File,
    packs_directory: File,
    catalog_directory: File,
    segments_directory: File,
    staging_directory: File,
    staged: BTreeMap<ObjectKey, Vec<u8>>,
    metadata: RefCell<BTreeMap<PackId, PackMetadata>>,
    catalog: CatalogIndex,
    segment_files: BTreeMap<SegmentId, RefCell<File>>,
    duplicates: Vec<DuplicateObject>,
    catalog_state: CatalogState,
    catalog_rebuild_note: Option<String>,
    staging_leftovers: Vec<String>,
    catalog_leftovers: Vec<String>,
    catalog_work: Cell<CatalogWork>,
}

impl PackDirectoryStore {
    pub fn initialize(root: &Path) -> Result<Self, StoreError> {
        match fs::create_dir(root) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(io_error(
                    "pack_store_root_create",
                    "failed to create immutable store root",
                    error,
                ));
            }
        }
        let root_directory = open_directory(root, "pack_store_root")?;
        let packs_directory = ensure_child_directory(&root_directory, PACKS_DIRECTORY)?;
        let catalog_directory = ensure_child_directory(&root_directory, CATALOG_DIRECTORY)?;
        let segments_directory = ensure_child_directory(&catalog_directory, SEGMENTS_DIRECTORY)?;
        let staging_directory = ensure_child_directory(&root_directory, STAGING_DIRECTORY)?;
        sync_directory(&root_directory, "pack_store_root_sync")?;
        if !regular_file_exists(&catalog_directory, CURRENT_CATALOG, "pack_catalog_open")? {
            if list_directory_names(&packs_directory, "pack_directory_scan")?.is_empty() {
                write_manifest_with_checkpoints(
                    &catalog_directory,
                    &CatalogManifest::empty(),
                    &mut |_| Ok(()),
                )?;
            } else {
                return Err(corrupt(
                    "catalog_manifest_missing",
                    "catalog manifest is missing while immutable packs exist; exclusive recovery is required",
                ));
            }
        }
        Self::load(
            StoreLayout {
                root: root.to_path_buf(),
                root_directory,
                packs_directory,
                catalog_directory,
                segments_directory,
                staging_directory,
            },
            CatalogState::Loaded,
            None,
            CatalogWork::default(),
            Vec::new(),
        )
    }

    pub fn open(root: &Path) -> Result<Self, StoreError> {
        let root_directory = open_directory(root, "pack_store_root")?;
        let packs_directory = open_child_directory(&root_directory, PACKS_DIRECTORY)?;
        let catalog_directory = open_child_directory(&root_directory, CATALOG_DIRECTORY)?;
        let segments_directory = ensure_child_directory(&catalog_directory, SEGMENTS_DIRECTORY)?;
        // Staging contains no accepted or derived identity and is intentionally absent from a
        // checked-in repository. Recreate that disposable boundary on first local open.
        let staging_directory = ensure_child_directory(&root_directory, STAGING_DIRECTORY)?;
        Self::load(
            StoreLayout {
                root: root.to_path_buf(),
                root_directory,
                packs_directory,
                catalog_directory,
                segments_directory,
                staging_directory,
            },
            CatalogState::Loaded,
            None,
            CatalogWork::default(),
            Vec::new(),
        )
    }

    /// Reconstructs disposable catalog state from immutable pack footers.
    ///
    /// Callers must hold the repository's exclusive publication lock. The reconstructed segment
    /// and manifest become durable before this method returns.
    pub(crate) fn recover_catalog(
        root: &Path,
        reason: impl Into<String>,
    ) -> Result<Self, StoreError> {
        Self::recover_catalog_inner(root, reason.into(), &mut |_| Ok(()))
    }

    #[cfg(test)]
    pub(crate) fn recover_catalog_with_fault(
        root: &Path,
        reason: impl Into<String>,
        fault: SealCheckpoint,
    ) -> Result<Self, StoreError> {
        let mut fired = false;
        Self::recover_catalog_inner(root, reason.into(), &mut |observed| {
            if !fired && observed == fault {
                fired = true;
                return Err(injected_interruption(observed));
            }
            Ok(())
        })
    }

    fn recover_catalog_inner(
        root: &Path,
        reason: String,
        checkpoint: &mut dyn FnMut(SealCheckpoint) -> Result<(), StoreError>,
    ) -> Result<Self, StoreError> {
        let root_directory = open_directory(root, "pack_store_root")?;
        let packs_directory = open_child_directory(&root_directory, PACKS_DIRECTORY)?;
        let catalog_directory = ensure_child_directory(&root_directory, CATALOG_DIRECTORY)?;
        let segments_directory = ensure_child_directory(&catalog_directory, SEGMENTS_DIRECTORY)?;
        let staging_directory = ensure_child_directory(&root_directory, STAGING_DIRECTORY)?;
        let scan = scan_pack_metadata(&packs_directory)?;
        let build = ObjectCatalog::rebuild(
            scan.metadata
                .iter()
                .map(|(pack, metadata)| (*pack, metadata)),
        )?;
        let mut recovery_work = CatalogWork {
            full_rebuilds: 1,
            full_footer_scan_runs: 1,
            pack_footers_scanned: scan.metadata.len() as u64,
            ..CatalogWork::default()
        };
        let index = write_rebuilt_catalog(
            &catalog_directory,
            &segments_directory,
            &build.catalog,
            &scan.metadata,
            &mut recovery_work,
            checkpoint,
        )?;
        cleanup_derived(
            &staging_directory,
            &catalog_directory,
            &segments_directory,
            &index,
            &mut recovery_work,
            checkpoint,
        )?;
        Self::load_with_index(
            StoreLayout {
                root: root.to_path_buf(),
                root_directory,
                packs_directory,
                catalog_directory,
                segments_directory,
                staging_directory,
            },
            index,
            CatalogState::RebuiltPersisted,
            Some(reason),
            recovery_work,
            build.duplicates,
        )
    }

    fn load(
        layout: StoreLayout,
        catalog_state: CatalogState,
        catalog_rebuild_note: Option<String>,
        mut catalog_work: CatalogWork,
        duplicates: Vec<DuplicateObject>,
    ) -> Result<Self, StoreError> {
        catalog_work.healthy_opens = catalog_work.healthy_opens.saturating_add(1);
        let manifest = read_manifest(&layout.catalog_directory, &mut catalog_work)?;
        let mut segments = Vec::with_capacity(manifest.segments.len());
        for expected in &manifest.segments {
            let mut file = open_regular_file_at(
                &layout.segments_directory,
                &expected.id.file_name(),
                "catalog_segment_open",
            )?;
            let length = regular_file_length(&file, "catalog_segment_metadata")?;
            if length != expected.file_bytes {
                return Err(corrupt(
                    "catalog_segment_length",
                    "manifest-selected segment length changed",
                ));
            }
            let metadata = catalog::read_segment_metadata(&mut file, length, expected.id)?;
            catalog_work.segment_metadata_read =
                catalog_work.segment_metadata_read.saturating_add(1);
            catalog_work.segment_metadata_bytes_read = catalog_work
                .segment_metadata_bytes_read
                .saturating_add(metadata.metadata_bytes);
            segments.push(metadata);
        }
        let index = CatalogIndex::new(manifest, segments)?;
        Self::load_with_index(
            layout,
            index,
            catalog_state,
            catalog_rebuild_note,
            catalog_work,
            duplicates,
        )
    }

    fn load_with_index(
        layout: StoreLayout,
        catalog: CatalogIndex,
        catalog_state: CatalogState,
        catalog_rebuild_note: Option<String>,
        catalog_work: CatalogWork,
        duplicates: Vec<DuplicateObject>,
    ) -> Result<Self, StoreError> {
        let segment_files = open_segment_handles(&layout.segments_directory, &catalog)?;
        let staging_leftovers = classify_staging_leftovers(&layout.staging_directory)?;
        let catalog_leftovers = classify_catalog_leftovers(
            &layout.catalog_directory,
            &layout.segments_directory,
            &catalog,
        )?;
        Ok(Self {
            root: layout.root,
            root_directory: layout.root_directory,
            packs_directory: layout.packs_directory,
            catalog_directory: layout.catalog_directory,
            segments_directory: layout.segments_directory,
            staging_directory: layout.staging_directory,
            staged: BTreeMap::new(),
            metadata: RefCell::new(BTreeMap::new()),
            catalog,
            segment_files,
            duplicates,
            catalog_state,
            catalog_rebuild_note,
            staging_leftovers,
            catalog_leftovers,
            catalog_work: Cell::new(catalog_work),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub const fn catalog_state(&self) -> CatalogState {
        self.catalog_state
    }

    pub fn catalog_rebuild_note(&self) -> Option<&str> {
        self.catalog_rebuild_note.as_deref()
    }

    pub fn duplicate_objects(&self) -> &[DuplicateObject] {
        &self.duplicates
    }

    pub fn staging_leftovers(&self) -> &[String] {
        &self.staging_leftovers
    }

    pub fn catalog_leftovers(&self) -> &[String] {
        &self.catalog_leftovers
    }

    pub fn catalog_work(&self) -> CatalogWork {
        self.catalog_work.get()
    }

    pub fn catalog_observation(&self) -> CatalogObservation {
        let manifest = self.catalog.manifest();
        CatalogObservation {
            identity: contract::CATALOG_CONTRACT_IDENTITY,
            contract_version: contract::CATALOG_CONTRACT_VERSION,
            state: self.catalog_state,
            generation: manifest.generation,
            commitment: manifest.logical_commitment,
            entries: manifest.total_entries,
            packs: manifest.total_packs,
            segments: self.catalog.segments().len(),
            segment_bytes: self
                .catalog
                .segments()
                .iter()
                .map(|segment| segment.file_bytes)
                .fold(0_u64, u64::saturating_add),
            segment_metadata_bytes: self
                .catalog
                .segments()
                .iter()
                .map(|segment| segment.metadata_bytes)
                .fold(0_u64, u64::saturating_add),
            maximum_level: self
                .catalog
                .segments()
                .iter()
                .map(|segment| segment.level)
                .max(),
            maximum_live_segments: contract::MAXIMUM_CATALOG_SEGMENTS,
            maximum_lookup_segments: self.catalog.segments().len(),
            block_entries: contract::CATALOG_BLOCK_ENTRIES,
            history: manifest.history,
            work: self.catalog_work.get(),
            leftovers: self.catalog_leftovers.clone(),
        }
    }

    pub(crate) fn catalog_location(
        &self,
        key: ObjectKey,
    ) -> Result<Option<CatalogLocation>, StoreError> {
        self.lookup_location(key)
    }

    pub fn staged_len(&self) -> usize {
        self.staged.len()
    }

    fn lookup_location(&self, key: ObjectKey) -> Result<Option<CatalogLocation>, StoreError> {
        let mut work = CatalogWork::default();
        let result = (|| {
            let mut found = None;
            for segment in self.catalog.segments() {
                work.segment_lookups = work.segment_lookups.saturating_add(1);
                let Some(block_index) = segment.find_block(key) else {
                    continue;
                };
                let block = segment.blocks.get(block_index).ok_or_else(|| {
                    corrupt(
                        "catalog_block_index",
                        "catalog block selection escaped metadata bounds",
                    )
                })?;
                if !block.might_contain(key) {
                    continue;
                }
                let file = self.segment_files.get(&segment.id).ok_or_else(|| {
                    corrupt(
                        "catalog_segment_handle",
                        "manifest-selected segment has no immutable open handle",
                    )
                })?;
                let entries = segment.read_block(&mut *file.borrow_mut(), block_index)?;
                work.segment_blocks_read = work.segment_blocks_read.saturating_add(1);
                work.segment_block_bytes_read = work.segment_block_bytes_read.saturating_add(
                    (entries.len() as u64).saturating_mul(catalog::CATALOG_ENTRY_BYTES as u64),
                );
                work.segment_entries_examined = work
                    .segment_entries_examined
                    .saturating_add(entries.len() as u64);
                if let Ok(index) = entries.binary_search_by_key(&key, |entry| entry.key) {
                    let location =
                        entries
                            .get(index)
                            .map(|entry| entry.location)
                            .ok_or_else(|| {
                                corrupt(
                                    "catalog_block_index",
                                    "catalog block search escaped decoded entry bounds",
                                )
                            })?;
                    if found.replace(location).is_some() {
                        return Err(corrupt(
                            "catalog_object_duplicate",
                            "one object key appears in multiple live catalog segments",
                        ));
                    }
                }
            }
            Ok(found)
        })();
        let mut accumulated = self.catalog_work.get();
        accumulated.add(work);
        self.catalog_work.set(accumulated);
        result
    }

    fn validated_pack_entry(
        &self,
        key: ObjectKey,
        location: CatalogLocation,
        maximum_bytes: usize,
    ) -> Result<(PackMetadata, PackIndexEntry), StoreError> {
        let descriptor = self.catalog.pack(location.pack).cloned().ok_or_else(|| {
            corrupt(
                "catalog_pack_descriptor",
                "catalog location names a pack absent from live segment metadata",
            )
        })?;
        let cached = self.metadata.borrow().get(&location.pack).cloned();
        let metadata = match cached {
            Some(metadata) => metadata,
            None => {
                let mut file = open_regular_file_at(
                    &self.packs_directory,
                    &location.pack.file_name(),
                    "catalog_pack_footer_open",
                )?;
                let length = regular_file_length(&file, "catalog_pack_footer_metadata")?;
                if length != descriptor.byte_length {
                    return Err(corrupt(
                        "catalog_pack_length",
                        "catalog pack descriptor disagrees with the immutable file length",
                    ));
                }
                let read = PackMetadata::read_footer(&mut file, length)?;
                if !descriptor.matches(&read.metadata) {
                    return Err(corrupt(
                        "catalog_pack_binding",
                        "catalog pack descriptor disagrees with the immutable pack footer",
                    ));
                }
                let observed = CatalogWork {
                    targeted_pack_footers_read: 1,
                    targeted_pack_footer_bytes_read: read.bytes_read,
                    ..CatalogWork::default()
                };
                let mut accumulated = self.catalog_work.get();
                accumulated.add(observed);
                self.catalog_work.set(accumulated);
                self.metadata
                    .borrow_mut()
                    .insert(location.pack, read.metadata.clone());
                read.metadata
            }
        };
        let (entry, _) = metadata
            .bounded_read_entry(key, maximum_bytes)?
            .ok_or_else(|| {
                corrupt(
                    "catalog_pack_entry",
                    "catalog names an object absent from the selected pack footer",
                )
            })?;
        if entry.offset != location.offset
            || entry.encoded_length != location.length
            || entry.checksum != location.checksum
            || entry.key != key
        {
            return Err(corrupt(
                "catalog_pack_location",
                "catalog coordinates disagree with the exact typed pack entry",
            ));
        }
        Ok((metadata.clone(), entry.clone()))
    }

    pub fn seal_staged(
        &mut self,
        target_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<SealReceipt, StoreError> {
        self.seal_staged_inner(target_bytes, work, &mut |_| Ok(()))
    }

    #[cfg(test)]
    pub fn seal_staged_with_fault(
        &mut self,
        target_bytes: usize,
        work: &mut StoreWork,
        fault: SealCheckpoint,
    ) -> Result<SealReceipt, StoreError> {
        let mut fired = false;
        self.seal_staged_inner(target_bytes, work, &mut |observed| {
            if !fired && observed == fault {
                fired = true;
                return Err(injected_interruption(observed));
            }
            Ok(())
        })
    }

    fn seal_staged_inner(
        &mut self,
        target_bytes: usize,
        work: &mut StoreWork,
        checkpoint: &mut dyn FnMut(SealCheckpoint) -> Result<(), StoreError>,
    ) -> Result<SealReceipt, StoreError> {
        if self.staged.is_empty() {
            return Ok(SealReceipt {
                packs: Vec::new(),
                objects: 0,
                catalog_state: self.catalog_state,
                catalog_work: Box::new(CatalogWork::default()),
            });
        }
        let mut builder = PackBuilder::default();
        for (key, bytes) in &self.staged {
            builder.insert(*key, bytes)?;
        }
        let packs = builder.seal_targeted(target_bytes)?;
        let ids = packs.iter().map(|pack| pack.id).collect::<Vec<_>>();
        for pack in &packs {
            install_pack(
                &self.packs_directory,
                &self.staging_directory,
                pack,
                checkpoint,
            )?;
            work.packs_sealed = work.packs_sealed.saturating_add(1);
        }
        sync_directory(&self.packs_directory, "pack_directory_sync")?;
        checkpoint(SealCheckpoint::PackDirectorySynced)?;
        sync_directory(&self.staging_directory, "pack_staging_sync")?;
        checkpoint(SealCheckpoint::StagingDirectorySynced)?;
        let mut catalog_work = CatalogWork::default();
        let next = append_catalog(
            &self.catalog_directory,
            &self.segments_directory,
            &self.staging_directory,
            &self.catalog,
            &packs,
            &mut catalog_work,
            checkpoint,
        )?;
        {
            let mut metadata = self.metadata.borrow_mut();
            for pack in &packs {
                metadata.insert(pack.id, pack.metadata.clone());
            }
        }
        let segment_files = open_segment_handles(&self.segments_directory, &next)?;
        self.catalog = next;
        self.segment_files = segment_files;
        self.catalog_state = CatalogState::IncrementalPersisted;
        self.catalog_rebuild_note = None;
        self.duplicates.clear();
        self.staged.clear();
        let mut accumulated = self.catalog_work.get();
        accumulated.add(catalog_work);
        self.catalog_work.set(accumulated);
        self.staging_leftovers = classify_staging_leftovers(&self.staging_directory)?;
        self.catalog_leftovers = classify_catalog_leftovers(
            &self.catalog_directory,
            &self.segments_directory,
            &self.catalog,
        )?;
        Ok(SealReceipt {
            packs: ids,
            objects: self.catalog.len(),
            catalog_state: self.catalog_state,
            catalog_work: Box::new(catalog_work),
        })
    }

    pub fn deep_verify(&self) -> Result<DeepVerifyReceipt, StoreError> {
        let scan = scan_pack_metadata(&self.packs_directory)?;
        let oracle = independent_footer_oracle(&scan.metadata)?;
        let mut bytes_read = 0_u64;
        for (pack, data) in &scan.metadata {
            let mut file =
                open_regular_file_at(&self.packs_directory, &pack.file_name(), "pack_deep_open")?;
            let verification = data.verify_file(&mut file, *pack)?;
            bytes_read = bytes_read
                .checked_add(verification.bytes_read)
                .ok_or_else(|| resource("pack_deep_work", "deep verification work overflows"))?;
        }
        bytes_read = bytes_read
            .checked_add(scan.bytes_read)
            .ok_or_else(|| resource("pack_deep_work", "deep verification work overflows"))?;
        let manifest = self.catalog.manifest();
        Ok(DeepVerifyReceipt {
            packs: scan.metadata.len(),
            selected_packs: self.catalog.manifest().total_packs as usize,
            objects: oracle.entries,
            bytes_read,
            duplicate_objects: oracle.duplicates,
            oracle_commitment: oracle.commitment,
            manifest_commitment: manifest.logical_commitment,
            catalog_equal: oracle.entries as u64 == manifest.total_entries
                && scan.metadata.len() as u64 == manifest.total_packs
                && oracle.commitment == manifest.logical_commitment,
            footer_scan_runs: 1,
            pack_footers_scanned: scan.metadata.len() as u64,
        })
    }

    pub(crate) fn verify_catalog_from_footers(
        &self,
    ) -> Result<CatalogFooterVerifyReceipt, StoreError> {
        let scan = scan_pack_metadata(&self.packs_directory)?;
        let oracle = independent_footer_oracle(&scan.metadata)?;
        let manifest = self.catalog.manifest();
        Ok(CatalogFooterVerifyReceipt {
            packs: scan.metadata.len() as u64,
            entries: oracle.entries as u64,
            footer_bytes_read: scan.bytes_read,
            duplicate_objects: oracle.duplicates.len() as u64,
            oracle_commitment: oracle.commitment,
            manifest_commitment: manifest.logical_commitment,
            equal: oracle.entries as u64 == manifest.total_entries
                && scan.metadata.len() as u64 == manifest.total_packs
                && oracle.commitment == manifest.logical_commitment,
        })
    }
}

impl ImmutableObjectStore for PackDirectoryStore {
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
                return Err(resource(
                    "pack_read_limit",
                    "staged object exceeds the caller read bound",
                ));
            }
            admission.admit_object(bytes.len())?;
            key.verify(bytes)?;
            work.objects_read = work.objects_read.saturating_add(1);
            work.bytes_read = work.bytes_read.saturating_add(bytes.len() as u64);
            return Ok(Some(bytes.clone()));
        }
        let Some(location) = self.lookup_location(key)? else {
            return Ok(None);
        };
        let object_bytes = usize::try_from(location.length).map_err(|_| {
            resource(
                "pack_read_length",
                "catalog object length does not fit this platform",
            )
        })?;
        if object_bytes > maximum_bytes || object_bytes > key.domain.maximum_bytes() {
            return Err(resource(
                "pack_read_limit",
                format!("object has {object_bytes} bytes; caller allowed {maximum_bytes}"),
            ));
        }
        admission.admit_object_bytes(location.length)?;
        let (metadata, _) = self.validated_pack_entry(key, location, maximum_bytes)?;
        let mut file = open_regular_file_at(
            &self.packs_directory,
            &location.pack.file_name(),
            "pack_object_open",
        )?;
        let observed_length = regular_file_length(&file, "pack_object_metadata")?;
        if observed_length != metadata.byte_length {
            return Err(corrupt(
                "pack_length_changed",
                "pack length changed after catalog construction",
            ));
        }
        work.packs_opened = work.packs_opened.saturating_add(1);
        let value = metadata.read_from(&mut file, key, maximum_bytes)?;
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
        if self.staged.contains_key(&key) {
            return Ok(true);
        }
        let Some(location) = self.lookup_location(key)? else {
            return Ok(false);
        };
        let _ = self.validated_pack_entry(key, location, key.domain.maximum_bytes())?;
        Ok(true)
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
            return Err(corrupt(
                "pack_object_collision",
                "one immutable object identity is bound to different bytes",
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

struct PackScan {
    metadata: BTreeMap<PackId, PackMetadata>,
    bytes_read: u64,
}

fn scan_pack_metadata(directory: &File) -> Result<PackScan, StoreError> {
    let names = list_directory_names(directory, "pack_directory_scan")?;
    if names.len() > contract::MAXIMUM_CATALOG_PACKS {
        return Err(resource(
            "catalog_pack_count",
            "immutable pack count exceeds the catalog reconstruction bound",
        ));
    }
    let mut metadata = BTreeMap::new();
    let mut bytes_read = 0_u64;
    for name in names {
        let pack = PackId::parse_file_name(&name)?;
        let mut file = open_regular_file_at(directory, &name, "pack_footer_open")?;
        let length = regular_file_length(&file, "pack_footer_metadata")?;
        let read = PackMetadata::read_footer(&mut file, length)?;
        bytes_read = bytes_read
            .checked_add(read.bytes_read)
            .ok_or_else(|| resource("catalog_rebuild_work", "footer scan work overflows"))?;
        if metadata.insert(pack, read.metadata).is_some() {
            return Err(corrupt(
                "pack_file_duplicate",
                "pack directory contains duplicate canonical identities",
            ));
        }
    }
    Ok(PackScan {
        metadata,
        bytes_read,
    })
}

fn read_manifest(directory: &File, work: &mut CatalogWork) -> Result<CatalogManifest, StoreError> {
    let mut file = open_optional_regular_file_at(
        directory,
        CURRENT_CATALOG,
        OFlags::RDONLY,
        "catalog_manifest_open",
    )?
    .ok_or_else(|| {
        corrupt(
            "catalog_manifest_missing",
            "catalog manifest is missing; exclusive footer reconstruction is required",
        )
    })?;
    let length = regular_file_length(&file, "catalog_manifest_metadata")?;
    let length = usize::try_from(length).map_err(|_| {
        resource(
            "catalog_manifest_size",
            "catalog manifest length does not fit this platform",
        )
    })?;
    if length > contract::MAXIMUM_CATALOG_MANIFEST_BYTES {
        return Err(resource(
            "catalog_manifest_size",
            "catalog manifest exceeds its hostile decoder bound",
        ));
    }
    let mut bytes = vec![0_u8; length];
    file.read_exact(&mut bytes)
        .map_err(|error| read_error("catalog_manifest_read", "manifest is truncated", error))?;
    work.manifests_read = work.manifests_read.saturating_add(1);
    work.manifest_bytes_read = work.manifest_bytes_read.saturating_add(length as u64);
    CatalogManifest::decode(&bytes)
}

fn write_manifest_with_checkpoints(
    directory: &File,
    manifest: &CatalogManifest,
    checkpoint: &mut dyn FnMut(SealCheckpoint) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    let bytes = manifest.encode()?;
    let temporary = format!("{MANIFEST_STAGE_PREFIX}{}", random_hex()?);
    let fd = rustix::fs::openat(
        directory,
        temporary.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| {
        rustix_error(
            "catalog_manifest_stage",
            "failed to create manifest stage",
            error,
        )
    })?;
    let mut file = File::from(fd);
    checkpoint(SealCheckpoint::ManifestStageCreated)?;
    if let Err(error) = file.write_all(&bytes) {
        drop(file);
        let _ = rustix::fs::unlinkat(directory, temporary.as_str(), AtFlags::empty());
        return Err(io_error(
            "catalog_manifest_write",
            "failed to write manifest stage",
            error,
        ));
    }
    checkpoint(SealCheckpoint::ManifestBytesWritten)?;
    if let Err(error) = file.sync_all() {
        drop(file);
        let _ = rustix::fs::unlinkat(directory, temporary.as_str(), AtFlags::empty());
        return Err(io_error(
            "catalog_manifest_write",
            "failed to synchronize manifest stage",
            error,
        ));
    }
    checkpoint(SealCheckpoint::ManifestFileSynced)?;
    drop(file);
    if let Err(error) =
        rustix::fs::renameat(directory, temporary.as_str(), directory, CURRENT_CATALOG)
    {
        let _ = rustix::fs::unlinkat(directory, temporary.as_str(), AtFlags::empty());
        return Err(rustix_error(
            "catalog_manifest_publish",
            "failed to publish catalog manifest",
            error,
        ));
    }
    checkpoint(SealCheckpoint::ManifestPublished)?;
    sync_directory(directory, "catalog_directory_sync")?;
    checkpoint(SealCheckpoint::CatalogDirectorySynced)
}

fn write_rebuilt_catalog(
    catalog_directory: &File,
    segments_directory: &File,
    rebuilt: &ObjectCatalog,
    metadata: &BTreeMap<PackId, PackMetadata>,
    work: &mut CatalogWork,
    checkpoint: &mut dyn FnMut(SealCheckpoint) -> Result<(), StoreError>,
) -> Result<CatalogIndex, StoreError> {
    let mut descriptors = BTreeMap::new();
    for (pack, pack_metadata) in metadata {
        descriptors.insert(*pack, PackDescriptor::from_metadata(*pack, pack_metadata)?);
    }
    let mut segments = Vec::new();
    if !rebuilt.is_empty() {
        let segment = publish_segment(
            segments_directory,
            contract::CATALOG_RECOVERY_LEVEL,
            0,
            &descriptors,
            rebuilt.entries().map(Ok),
            work,
            checkpoint,
        )?;
        segments.push(segment);
    }
    let history = history_from_work(CatalogHistory::default(), *work)?;
    let manifest = CatalogManifest::from_segments(0, history, &segments)?;
    let index = CatalogIndex::new(manifest.clone(), segments)?;
    write_manifest_with_checkpoints(catalog_directory, &manifest, checkpoint)?;
    work.manifests_written = work.manifests_written.saturating_add(1);
    Ok(index)
}

fn append_catalog(
    catalog_directory: &File,
    segments_directory: &File,
    staging_directory: &File,
    current: &CatalogIndex,
    packs: &[SealedPack],
    work: &mut CatalogWork,
    checkpoint: &mut dyn FnMut(SealCheckpoint) -> Result<(), StoreError>,
) -> Result<CatalogIndex, StoreError> {
    let generation = current
        .manifest()
        .generation
        .checked_add(1)
        .ok_or_else(|| resource("catalog_generation", "catalog generation is exhausted"))?;
    let mut descriptors = BTreeMap::new();
    let mut entries = BTreeMap::<ObjectKey, CatalogLocation>::new();
    for pack in packs {
        descriptors.insert(
            pack.id,
            PackDescriptor::from_metadata(pack.id, &pack.metadata)?,
        );
        for entry in &pack.metadata.entries {
            let location = CatalogLocation {
                pack: pack.id,
                offset: entry.offset,
                length: entry.encoded_length,
                checksum: entry.checksum,
            };
            match entries.get(&entry.key) {
                Some(existing) if existing.pack <= pack.id => {}
                _ => {
                    entries.insert(entry.key, location);
                }
            }
        }
    }
    if entries.is_empty() {
        return Err(corrupt(
            "catalog_delta_empty",
            "a non-empty sealed pack set produced no catalog delta",
        ));
    }
    let delta_entries = entries
        .into_iter()
        .map(|(key, location)| Ok(CatalogEntry { key, location }));
    let mut carry = publish_segment(
        segments_directory,
        0,
        generation,
        &descriptors,
        delta_entries,
        work,
        checkpoint,
    )?;
    work.delta_segments_written = work.delta_segments_written.saturating_add(1);
    let mut live = current.segments().to_vec();
    let mut obsolete = BTreeSet::new();
    while let Some(position) = live.iter().position(|segment| segment.level == carry.level) {
        if carry.level >= contract::MAXIMUM_CATALOG_LEVEL {
            return Err(resource(
                "catalog_level_exhausted",
                "catalog leveled merge capacity is exhausted",
            ));
        }
        let existing = live.remove(position);
        let merged_descriptors = merge_pack_descriptors(&existing, &carry)?;
        let input_entries = existing
            .entry_count
            .checked_add(carry.entry_count)
            .ok_or_else(|| resource("catalog_merge_entries", "catalog merge work overflows"))?;
        let input_bytes = input_entries
            .checked_mul(catalog::CATALOG_ENTRY_BYTES as u64)
            .ok_or_else(|| resource("catalog_merge_bytes", "catalog merge bytes overflow"))?;
        let left = SegmentEntryStream::open(segments_directory, existing.clone())?;
        let right = SegmentEntryStream::open(segments_directory, carry.clone())?;
        let next = publish_segment(
            segments_directory,
            carry.level + 1,
            generation,
            &merged_descriptors,
            MergedEntries::new(left, right),
            work,
            checkpoint,
        )?;
        obsolete.insert(existing.id);
        obsolete.insert(carry.id);
        carry = next;
        work.merge_operations = work.merge_operations.saturating_add(1);
        work.merge_entries_read = work.merge_entries_read.saturating_add(input_entries);
        work.merge_bytes_read = work.merge_bytes_read.saturating_add(input_bytes);
    }
    live.push(carry);
    live.sort_by_key(|segment| segment.level);
    let history = history_from_work(current.manifest().history, *work)?;
    let manifest = CatalogManifest::from_segments(generation, history, &live)?;
    let index = CatalogIndex::new(manifest.clone(), live)?;
    write_manifest_with_checkpoints(catalog_directory, &manifest, checkpoint)?;
    work.manifests_written = work.manifests_written.saturating_add(1);
    cleanup_derived(
        staging_directory,
        catalog_directory,
        segments_directory,
        &index,
        work,
        checkpoint,
    )?;
    for id in obsolete {
        if index.segments().iter().any(|segment| segment.id == id) {
            return Err(corrupt(
                "catalog_cleanup_selection",
                "obsolete catalog segment remains selected by the new manifest",
            ));
        }
    }
    Ok(index)
}

fn history_from_work(
    previous: CatalogHistory,
    work: CatalogWork,
) -> Result<CatalogHistory, StoreError> {
    Ok(CatalogHistory {
        delta_segments: checked_history_add(previous.delta_segments, work.delta_segments_written)?,
        merge_operations: checked_history_add(previous.merge_operations, work.merge_operations)?,
        merge_entries_read: checked_history_add(
            previous.merge_entries_read,
            work.merge_entries_read,
        )?,
        merge_bytes_read: checked_history_add(previous.merge_bytes_read, work.merge_bytes_read)?,
        segments_written: checked_history_add(previous.segments_written, work.segments_written)?,
        segment_entries_written: checked_history_add(
            previous.segment_entries_written,
            work.segment_entries_written,
        )?,
        full_rebuilds: checked_history_add(previous.full_rebuilds, work.full_rebuilds)?,
        full_footer_scan_runs: checked_history_add(
            previous.full_footer_scan_runs,
            work.full_footer_scan_runs,
        )?,
        pack_footers_scanned: checked_history_add(
            previous.pack_footers_scanned,
            work.pack_footers_scanned,
        )?,
    })
}

fn checked_history_add(left: u64, right: u64) -> Result<u64, StoreError> {
    left.checked_add(right).ok_or_else(|| {
        resource(
            "catalog_history_overflow",
            "catalog cumulative work history is exhausted",
        )
    })
}

fn merge_pack_descriptors(
    left: &SegmentMetadata,
    right: &SegmentMetadata,
) -> Result<BTreeMap<PackId, PackDescriptor>, StoreError> {
    let mut merged = left.packs.clone();
    for (pack, descriptor) in &right.packs {
        if merged.insert(*pack, descriptor.clone()).is_some() {
            return Err(corrupt(
                "catalog_merge_pack_duplicate",
                "one immutable pack appears in both merged catalog segments",
            ));
        }
    }
    if merged.len() > contract::MAXIMUM_CATALOG_PACKS {
        return Err(resource(
            "catalog_merge_packs",
            "catalog merge exceeds the pack descriptor bound",
        ));
    }
    Ok(merged)
}

fn publish_segment<I>(
    directory: &File,
    level: u16,
    generation: u64,
    descriptors: &BTreeMap<PackId, PackDescriptor>,
    entries: I,
    work: &mut CatalogWork,
    checkpoint: &mut dyn FnMut(SealCheckpoint) -> Result<(), StoreError>,
) -> Result<SegmentMetadata, StoreError>
where
    I: IntoIterator<Item = Result<CatalogEntry, StoreError>>,
{
    let temporary = format!("{SEGMENT_STAGE_PREFIX}{}", random_hex()?);
    let fd = rustix::fs::openat(
        directory,
        temporary.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| {
        rustix_error(
            "catalog_segment_stage",
            "failed to create catalog segment stage",
            error,
        )
    })?;
    let mut file = File::from(fd);
    checkpoint(SealCheckpoint::SegmentStageCreated)?;
    let written = match catalog::write_segment(&mut file, level, generation, descriptors, entries) {
        Ok(written) => written,
        Err(error) => {
            drop(file);
            let _ = rustix::fs::unlinkat(directory, temporary.as_str(), AtFlags::empty());
            return Err(error);
        }
    };
    checkpoint(SealCheckpoint::SegmentBytesWritten)?;
    if let Err(error) = file.sync_all() {
        drop(file);
        let _ = rustix::fs::unlinkat(directory, temporary.as_str(), AtFlags::empty());
        return Err(io_error(
            "catalog_segment_sync",
            "failed to synchronize catalog segment stage",
            error,
        ));
    }
    checkpoint(SealCheckpoint::SegmentFileSynced)?;
    drop(file);
    let final_name = written.metadata.id.file_name();
    match rustix::fs::linkat(
        directory,
        temporary.as_str(),
        directory,
        final_name.as_str(),
        AtFlags::empty(),
    ) {
        Ok(()) => {}
        Err(error) if error == rustix::io::Errno::EXIST => {
            verify_existing_segment(directory, &written.metadata)?;
        }
        Err(error) => {
            let _ = rustix::fs::unlinkat(directory, temporary.as_str(), AtFlags::empty());
            return Err(rustix_error(
                "catalog_segment_publish",
                "failed to publish immutable catalog segment",
                error,
            ));
        }
    }
    checkpoint(SealCheckpoint::SegmentPublished)?;
    rustix::fs::unlinkat(directory, temporary.as_str(), AtFlags::empty()).map_err(|error| {
        rustix_error(
            "catalog_segment_stage_remove",
            "failed to remove catalog segment stage",
            error,
        )
    })?;
    checkpoint(SealCheckpoint::SegmentStageRemoved)?;
    sync_directory(directory, "catalog_segment_directory_sync")?;
    checkpoint(SealCheckpoint::SegmentDirectorySynced)?;
    verify_existing_segment(directory, &written.metadata)?;
    work.segments_written = work.segments_written.saturating_add(1);
    work.segment_entries_written = work
        .segment_entries_written
        .saturating_add(written.metadata.entry_count);
    Ok(written.metadata)
}

fn verify_existing_segment(directory: &File, expected: &SegmentMetadata) -> Result<(), StoreError> {
    let mut file = open_regular_file_at(
        directory,
        &expected.id.file_name(),
        "catalog_segment_existing_open",
    )?;
    let length = regular_file_length(&file, "catalog_segment_existing_metadata")?;
    let observed = catalog::read_segment_metadata(&mut file, length, expected.id)?;
    if &observed != expected {
        return Err(corrupt(
            "catalog_segment_existing",
            "existing content-addressed catalog segment disagrees with staged metadata",
        ));
    }
    Ok(())
}

struct SegmentEntryStream {
    file: File,
    metadata: SegmentMetadata,
    next_block: usize,
    current: std::vec::IntoIter<CatalogEntry>,
    observed_entries: u64,
    observed_sum: [u8; 32],
    finished: bool,
}

impl SegmentEntryStream {
    fn open(directory: &File, metadata: SegmentMetadata) -> Result<Self, StoreError> {
        let file = open_regular_file_at(
            directory,
            &metadata.id.file_name(),
            "catalog_merge_segment_open",
        )?;
        if regular_file_length(&file, "catalog_merge_segment_metadata")? != metadata.file_bytes {
            return Err(corrupt(
                "catalog_merge_segment_length",
                "catalog merge input length changed after manifest load",
            ));
        }
        Ok(Self {
            file,
            metadata,
            next_block: 0,
            current: Vec::new().into_iter(),
            observed_entries: 0,
            observed_sum: [0_u8; 32],
            finished: false,
        })
    }
}

impl Iterator for SegmentEntryStream {
    type Item = Result<CatalogEntry, StoreError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(entry) = self.current.next() {
                self.observed_entries = match self.observed_entries.checked_add(1) {
                    Some(value) => value,
                    None => {
                        self.finished = true;
                        return Some(Err(resource(
                            "catalog_merge_entries",
                            "catalog merge input count overflows",
                        )));
                    }
                };
                catalog::add_logical_entry(&mut self.observed_sum, entry);
                return Some(Ok(entry));
            }
            if self.finished {
                return None;
            }
            if self.next_block < self.metadata.blocks.len() {
                let block = self.next_block;
                self.next_block += 1;
                match self.metadata.read_block(&mut self.file, block) {
                    Ok(entries) => {
                        self.current = entries.into_iter();
                    }
                    Err(error) => {
                        self.finished = true;
                        return Some(Err(error));
                    }
                }
                continue;
            }
            self.finished = true;
            if self.observed_entries != self.metadata.entry_count
                || self.observed_sum != self.metadata.logical_sum
            {
                return Some(Err(corrupt(
                    "catalog_merge_input_summary",
                    "streamed catalog merge input disagrees with authenticated metadata",
                )));
            }
            return None;
        }
    }
}

struct MergedEntries<L, R>
where
    L: Iterator<Item = Result<CatalogEntry, StoreError>>,
    R: Iterator<Item = Result<CatalogEntry, StoreError>>,
{
    left: L,
    right: R,
    left_next: Option<Result<CatalogEntry, StoreError>>,
    right_next: Option<Result<CatalogEntry, StoreError>>,
    failed: bool,
}

impl<L, R> MergedEntries<L, R>
where
    L: Iterator<Item = Result<CatalogEntry, StoreError>>,
    R: Iterator<Item = Result<CatalogEntry, StoreError>>,
{
    fn new(mut left: L, mut right: R) -> Self {
        let left_next = left.next();
        let right_next = right.next();
        Self {
            left,
            right,
            left_next,
            right_next,
            failed: false,
        }
    }

    fn take_left(&mut self) -> Option<Result<CatalogEntry, StoreError>> {
        let current = self.left_next.take();
        self.left_next = self.left.next();
        current
    }

    fn take_right(&mut self) -> Option<Result<CatalogEntry, StoreError>> {
        let current = self.right_next.take();
        self.right_next = self.right.next();
        current
    }
}

impl<L, R> Iterator for MergedEntries<L, R>
where
    L: Iterator<Item = Result<CatalogEntry, StoreError>>,
    R: Iterator<Item = Result<CatalogEntry, StoreError>>,
{
    type Item = Result<CatalogEntry, StoreError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        if matches!(self.left_next, Some(Err(_))) {
            self.failed = true;
            return self.left_next.take();
        }
        if matches!(self.right_next, Some(Err(_))) {
            self.failed = true;
            return self.right_next.take();
        }
        let left = match self.left_next.as_ref() {
            Some(Ok(entry)) => Some(*entry),
            _ => None,
        };
        let right = match self.right_next.as_ref() {
            Some(Ok(entry)) => Some(*entry),
            _ => None,
        };
        match (left, right) {
            (None, None) => None,
            (Some(_), None) => self.take_left(),
            (None, Some(_)) => self.take_right(),
            (Some(left), Some(right)) if left.key < right.key => self.take_left(),
            (Some(left), Some(right)) if right.key < left.key => self.take_right(),
            (Some(left), Some(right)) => {
                let selected = if left.location.pack <= right.location.pack {
                    left
                } else {
                    right
                };
                let _ = self.take_left();
                let _ = self.take_right();
                Some(Ok(selected))
            }
        }
    }
}

fn classify_staging_leftovers(directory: &File) -> Result<Vec<String>, StoreError> {
    let names = list_directory_names(directory, "pack_staging_scan")?;
    if names.len() > contract::MAXIMUM_CATALOG_LEFTOVERS {
        return Err(resource(
            "pack_staging_leftovers",
            "pack staging leftovers exceed their bounded inspection limit",
        ));
    }
    for name in &names {
        let _ = open_regular_file_at(directory, name, "pack_staging_leftover_type")?;
    }
    Ok(names)
}

fn open_segment_handles(
    directory: &File,
    index: &CatalogIndex,
) -> Result<BTreeMap<SegmentId, RefCell<File>>, StoreError> {
    let mut files = BTreeMap::new();
    for segment in index.segments() {
        let file = open_regular_file_at(
            directory,
            &segment.id.file_name(),
            "catalog_segment_handle_open",
        )?;
        if regular_file_length(&file, "catalog_segment_handle_metadata")? != segment.file_bytes {
            return Err(corrupt(
                "catalog_segment_length",
                "manifest-selected segment length changed while opening its handle",
            ));
        }
        files.insert(segment.id, RefCell::new(file));
    }
    Ok(files)
}

fn classify_catalog_leftovers(
    catalog_directory: &File,
    segments_directory: &File,
    index: &CatalogIndex,
) -> Result<Vec<String>, StoreError> {
    let mut leftovers = Vec::new();
    for name in list_directory_names(catalog_directory, "catalog_directory_scan")? {
        if name == CURRENT_CATALOG || name == SEGMENTS_DIRECTORY {
            continue;
        }
        if !is_stage_name(&name, MANIFEST_STAGE_PREFIX) {
            return Err(corrupt(
                "catalog_directory_entry",
                format!("unknown catalog directory entry '{name}'"),
            ));
        }
        let _ = open_regular_file_at(catalog_directory, &name, "catalog_manifest_leftover_type")?;
        leftovers.push(name);
    }
    let selected = index
        .segments()
        .iter()
        .map(|segment| segment.id)
        .collect::<BTreeSet<_>>();
    for name in list_directory_names(segments_directory, "catalog_segment_directory_scan")? {
        let parsed = SegmentId::parse_file_name(&name);
        match parsed {
            Ok(id) => {
                let _ = open_regular_file_at(
                    segments_directory,
                    &name,
                    "catalog_segment_leftover_type",
                )?;
                if !selected.contains(&id) {
                    leftovers.push(format!("{SEGMENTS_DIRECTORY}/{name}"));
                }
            }
            Err(_) if is_stage_name(&name, SEGMENT_STAGE_PREFIX) => {
                let _ =
                    open_regular_file_at(segments_directory, &name, "catalog_segment_stage_type")?;
                leftovers.push(format!("{SEGMENTS_DIRECTORY}/{name}"));
            }
            Err(error) => return Err(error),
        }
    }
    if leftovers.len() > contract::MAXIMUM_CATALOG_LEFTOVERS {
        return Err(resource(
            "catalog_leftovers",
            "catalog derived leftovers exceed their bounded inspection limit",
        ));
    }
    leftovers.sort();
    Ok(leftovers)
}

fn cleanup_derived(
    staging_directory: &File,
    catalog_directory: &File,
    segments_directory: &File,
    index: &CatalogIndex,
    work: &mut CatalogWork,
    checkpoint: &mut dyn FnMut(SealCheckpoint) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    let staging = classify_staging_leftovers(staging_directory)?;
    for name in staging {
        if is_stage_name(&name, PACK_STAGE_PREFIX) {
            rustix::fs::unlinkat(staging_directory, name.as_str(), AtFlags::empty()).map_err(
                |error| {
                    rustix_error(
                        "pack_staging_cleanup",
                        "failed to remove an owned pack staging leftover",
                        error,
                    )
                },
            )?;
        }
    }
    let leftovers = classify_catalog_leftovers(catalog_directory, segments_directory, index)?;
    for relative in leftovers {
        if let Some(name) = relative.strip_prefix("segments/") {
            if SegmentId::parse_file_name(name).is_ok() {
                work.obsolete_segments_removed = work.obsolete_segments_removed.saturating_add(1);
            }
            rustix::fs::unlinkat(segments_directory, name, AtFlags::empty()).map_err(|error| {
                rustix_error(
                    "catalog_segment_cleanup",
                    "failed to remove an owned derived catalog segment",
                    error,
                )
            })?;
        } else {
            rustix::fs::unlinkat(catalog_directory, relative.as_str(), AtFlags::empty()).map_err(
                |error| {
                    rustix_error(
                        "catalog_manifest_cleanup",
                        "failed to remove an owned manifest staging leftover",
                        error,
                    )
                },
            )?;
        }
    }
    checkpoint(SealCheckpoint::ObsoleteSegmentsRemoved)?;
    sync_directory(staging_directory, "pack_staging_cleanup_sync")?;
    sync_directory(segments_directory, "catalog_segment_cleanup_sync")?;
    sync_directory(catalog_directory, "catalog_cleanup_sync")?;
    checkpoint(SealCheckpoint::DerivedCleanupSynced)
}

fn is_stage_name(name: &str, prefix: &str) -> bool {
    let Some(suffix) = name.strip_prefix(prefix) else {
        return false;
    };
    suffix.len() == 32
        && suffix
            .bytes()
            .all(|value| value.is_ascii_digit() || (b'a'..=b'f').contains(&value))
}

struct FooterOracle {
    entries: usize,
    duplicates: Vec<DuplicateObject>,
    commitment: CatalogCommitment,
}

fn independent_footer_oracle(
    metadata: &BTreeMap<PackId, PackMetadata>,
) -> Result<FooterOracle, StoreError> {
    let mut locations = BTreeMap::<ObjectKey, CatalogLocation>::new();
    let mut duplicate_packs = BTreeMap::<ObjectKey, Vec<PackId>>::new();
    let mut visited = 0_usize;
    for (pack, pack_metadata) in metadata {
        for entry in &pack_metadata.entries {
            visited = visited.checked_add(1).ok_or_else(|| {
                resource(
                    "catalog_oracle_entries",
                    "independent footer oracle work overflows",
                )
            })?;
            if visited > contract::MAXIMUM_CATALOG_ENTRIES {
                return Err(resource(
                    "catalog_oracle_entries",
                    "independent footer oracle exceeds the entry bound",
                ));
            }
            let location = CatalogLocation {
                pack: *pack,
                offset: entry.offset,
                length: entry.encoded_length,
                checksum: entry.checksum,
            };
            match locations.get(&entry.key).copied() {
                None => {
                    locations.insert(entry.key, location);
                }
                Some(existing) => {
                    let packs = duplicate_packs
                        .entry(entry.key)
                        .or_insert_with(|| vec![existing.pack]);
                    if !packs.contains(pack) {
                        packs.push(*pack);
                    }
                    if *pack < existing.pack {
                        locations.insert(entry.key, location);
                    }
                }
            }
        }
    }
    let duplicates = duplicate_packs
        .into_iter()
        .map(|(key, mut packs)| {
            packs.sort();
            DuplicateObject { key, packs }
        })
        .collect::<Vec<_>>();
    let mut sum = [0_u8; 32];
    for (key, location) in &locations {
        oracle_add_entry(&mut sum, *key, *location);
    }
    let count = locations.len() as u64;
    Ok(FooterOracle {
        entries: locations.len(),
        duplicates,
        commitment: oracle_commitment(count, sum),
    })
}

fn oracle_add_entry(sum: &mut [u8; 32], key: ObjectKey, location: CatalogLocation) {
    let mut bytes = Vec::with_capacity(1 + 32 + 8 + 32);
    bytes.push(key.domain.tag());
    bytes.extend_from_slice(&key.digest.bytes());
    bytes.extend_from_slice(&location.length.to_be_bytes());
    bytes.extend_from_slice(&location.checksum);
    let mut hasher = blake3::Hasher::new_derive_key(contract::CATALOG_LOGICAL_ENTRY_DOMAIN);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    let value = hasher.finalize();
    let mut carry = 0_u16;
    for index in (0..32).rev() {
        let total = u16::from(sum[index]) + u16::from(value.as_bytes()[index]) + carry;
        sum[index] = total as u8;
        carry = total >> 8;
    }
}

fn oracle_commitment(count: u64, sum: [u8; 32]) -> CatalogCommitment {
    let mut bytes = Vec::with_capacity(40);
    bytes.extend_from_slice(&count.to_be_bytes());
    bytes.extend_from_slice(&sum);
    let mut hasher = blake3::Hasher::new_derive_key(contract::CATALOG_LOGICAL_COMMITMENT_DOMAIN);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    CatalogCommitment::from_bytes(*hasher.finalize().as_bytes())
}

fn install_pack(
    packs_directory: &File,
    staging_directory: &File,
    pack: &SealedPack,
    checkpoint: &mut dyn FnMut(SealCheckpoint) -> Result<(), StoreError>,
) -> Result<(), StoreError> {
    let temporary = format!(".pack-stage-{}", random_hex()?);
    let fd = rustix::fs::openat(
        staging_directory,
        temporary.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| rustix_error("pack_stage_create", "failed to create pack stage", error))?;
    let mut file = File::from(fd);
    checkpoint(SealCheckpoint::PackStageCreated)?;
    let payload_end = super::pack::HEADER_BYTES
        .checked_add(usize::try_from(pack.metadata.payload_bytes).map_err(|_| {
            resource(
                "pack_payload_size",
                "sealed pack payload length does not fit this platform",
            )
        })?)
        .filter(|end| *end <= pack.bytes.len())
        .ok_or_else(|| {
            corrupt(
                "pack_payload_size",
                "sealed pack payload bounds are invalid",
            )
        })?;
    if let Err(error) = file.write_all(&pack.bytes[..payload_end]) {
        drop(file);
        let _ = rustix::fs::unlinkat(staging_directory, temporary.as_str(), AtFlags::empty());
        return Err(io_error(
            "pack_stage_write",
            "failed to persist pack payload",
            error,
        ));
    }
    checkpoint(SealCheckpoint::PackPayloadWritten)?;
    if let Err(error) = file.write_all(&pack.bytes[payload_end..]) {
        drop(file);
        let _ = rustix::fs::unlinkat(staging_directory, temporary.as_str(), AtFlags::empty());
        return Err(io_error(
            "pack_stage_write",
            "failed to persist pack footer",
            error,
        ));
    }
    checkpoint(SealCheckpoint::PackFooterWritten)?;
    if let Err(error) = file.sync_all() {
        drop(file);
        let _ = rustix::fs::unlinkat(staging_directory, temporary.as_str(), AtFlags::empty());
        return Err(io_error(
            "pack_stage_write",
            "failed to synchronize pack stage",
            error,
        ));
    }
    checkpoint(SealCheckpoint::PackFileSynced)?;
    drop(file);
    let final_name = pack.id.file_name();
    match rustix::fs::linkat(
        staging_directory,
        temporary.as_str(),
        packs_directory,
        final_name.as_str(),
        AtFlags::empty(),
    ) {
        Ok(()) => {}
        Err(error) if error == rustix::io::Errno::EXIST => {
            verify_existing_pack(packs_directory, pack.id)?;
        }
        Err(error) => {
            let _ = rustix::fs::unlinkat(staging_directory, temporary.as_str(), AtFlags::empty());
            return Err(rustix_error(
                "pack_publish",
                "failed to publish immutable pack",
                error,
            ));
        }
    }
    checkpoint(SealCheckpoint::PackPublished)?;
    rustix::fs::unlinkat(staging_directory, temporary.as_str(), AtFlags::empty())
        .map_err(|error| rustix_error("pack_stage_remove", "failed to remove pack stage", error))?;
    checkpoint(SealCheckpoint::PackStageRemoved)?;
    verify_existing_pack(packs_directory, pack.id)
}

fn verify_existing_pack(directory: &File, pack: PackId) -> Result<(), StoreError> {
    let mut file = open_regular_file_at(directory, &pack.file_name(), "pack_existing_open")?;
    let length = file
        .metadata()
        .map_err(|error| io_error("pack_existing_metadata", "failed to inspect pack", error))?
        .len();
    let metadata = PackMetadata::read_footer(&mut file, length)?.metadata;
    metadata.verify_file(&mut file, pack)?;
    Ok(())
}

fn open_directory(path: &Path, code: &'static str) -> Result<File, StoreError> {
    let fd = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| rustix_error(code, "failed to open non-symlink directory", error))?;
    Ok(File::from(fd))
}

fn ensure_child_directory(parent: &File, name: &str) -> Result<File, StoreError> {
    match rustix::fs::mkdirat(parent, name, Mode::from_raw_mode(0o700)) {
        Ok(()) => {}
        Err(error) if error == rustix::io::Errno::EXIST => {}
        Err(error) => {
            return Err(rustix_error(
                "pack_store_directory_create",
                "failed to create immutable store directory",
                error,
            ));
        }
    }
    open_child_directory(parent, name)
}

fn open_child_directory(parent: &File, name: &str) -> Result<File, StoreError> {
    let fd = rustix::fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| {
        rustix_error(
            "pack_store_directory_open",
            "failed to open immutable store directory",
            error,
        )
    })?;
    Ok(File::from(fd))
}

fn open_regular_file_at(
    directory: &File,
    name: &str,
    code: &'static str,
) -> Result<File, StoreError> {
    open_optional_regular_file_at(directory, name, OFlags::RDONLY, code)?.ok_or_else(|| {
        corrupt(
            "pack_file_missing",
            format!("immutable pack file '{name}' is missing"),
        )
    })
}

fn regular_file_exists(
    directory: &File,
    name: &str,
    code: &'static str,
) -> Result<bool, StoreError> {
    Ok(open_optional_regular_file_at(directory, name, OFlags::RDONLY, code)?.is_some())
}

fn regular_file_length(file: &File, code: &'static str) -> Result<u64, StoreError> {
    file.metadata()
        .map(|metadata| metadata.len())
        .map_err(|error| io_error(code, "failed to inspect regular file", error))
}

fn open_optional_regular_file_at(
    directory: &File,
    name: &str,
    access: OFlags,
    code: &'static str,
) -> Result<Option<File>, StoreError> {
    let fd = match rustix::fs::openat(
        directory,
        name,
        access | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
        Err(error) => {
            return Err(rustix_error(
                code,
                "failed to open regular non-symlink file",
                error,
            ));
        }
    };
    let file = File::from(fd);
    let metadata = file
        .metadata()
        .map_err(|error| io_error(code, "failed to inspect opened file", error))?;
    if !metadata.is_file() {
        return Err(corrupt(code, "store entry is not a regular file"));
    }
    Ok(Some(file))
}

fn list_directory_names(directory: &File, code: &'static str) -> Result<Vec<String>, StoreError> {
    let entries = Dir::read_from(directory)
        .map_err(|error| rustix_error(code, "failed to open directory stream", error))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|error| rustix_error(code, "failed to enumerate directory", error))?;
        let bytes = entry.file_name().to_bytes();
        if bytes == b"." || bytes == b".." {
            continue;
        }
        let name = std::str::from_utf8(bytes)
            .map_err(|_| corrupt(code, "store directory contains a non-UTF-8 entry"))?;
        names.push(name.to_owned());
    }
    names.sort();
    Ok(names)
}

fn random_hex() -> Result<String, StoreError> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        StoreError::new(
            StoreErrorClass::Io,
            "pack_stage_random",
            format!("secure randomness for private staging is unavailable: {error}"),
        )
    })?;
    Ok(crate::platform::semantic_id::encode_hex(&bytes))
}

fn injected_interruption(checkpoint: SealCheckpoint) -> StoreError {
    StoreError::new(
        StoreErrorClass::Io,
        INJECTED_INTERRUPTION_CODE,
        format!(
            "deterministic packed-store interruption at {}",
            checkpoint.name()
        ),
    )
}

fn sync_directory(directory: &File, code: &'static str) -> Result<(), StoreError> {
    directory
        .sync_all()
        .map_err(|error| io_error(code, "failed to synchronize directory", error))
}

fn read_error(code: &'static str, message: &'static str, error: std::io::Error) -> StoreError {
    if error.kind() == std::io::ErrorKind::UnexpectedEof {
        corrupt(code, message)
    } else {
        io_error(code, "failed to read store bytes", error)
    }
}

fn rustix_error(code: &'static str, message: &'static str, error: rustix::io::Errno) -> StoreError {
    StoreError::new(StoreErrorClass::Io, code, format!("{message}: {error}"))
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
