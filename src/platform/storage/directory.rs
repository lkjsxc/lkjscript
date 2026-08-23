//! Filesystem-backed immutable packs with an anchored, rebuildable catalog.

use super::catalog::{DuplicateObject, ObjectCatalog};
use super::contract;
use super::object::{
    ImmutableObjectStore, ObjectKey, StageOutcome, StoreError, StoreErrorClass, StoreWork,
    stage_into_map,
};
use super::pack::{PackBuilder, PackId, PackMetadata, SealedPack};
use rustix::fs::{AtFlags, Dir, Mode, OFlags};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const PACKS_DIRECTORY: &str = "packs";
const CATALOG_DIRECTORY: &str = "catalog";
const STAGING_DIRECTORY: &str = "staging";
const CURRENT_CATALOG: &str = "current.lkjc";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CatalogState {
    Loaded,
    RebuiltPersisted,
    RebuiltMemoryOnly,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SealReceipt {
    pub packs: Vec<PackId>,
    pub objects: usize,
    pub catalog_state: CatalogState,
    pub catalog_persist_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeepVerifyReceipt {
    pub packs: usize,
    pub objects: usize,
    pub bytes_read: u64,
    pub duplicate_objects: Vec<DuplicateObject>,
}

#[derive(Debug)]
pub struct PackDirectoryStore {
    root: PathBuf,
    root_directory: File,
    packs_directory: File,
    catalog_directory: File,
    staging_directory: File,
    staged: BTreeMap<ObjectKey, Vec<u8>>,
    metadata: BTreeMap<PackId, PackMetadata>,
    catalog: ObjectCatalog,
    duplicates: Vec<DuplicateObject>,
    catalog_state: CatalogState,
    catalog_rebuild_note: Option<String>,
    catalog_persist_error: Option<String>,
    staging_leftovers: Vec<String>,
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
        let staging_directory = ensure_child_directory(&root_directory, STAGING_DIRECTORY)?;
        sync_directory(&root_directory, "pack_store_root_sync")?;
        Self::load(
            root.to_path_buf(),
            root_directory,
            packs_directory,
            catalog_directory,
            staging_directory,
        )
    }

    pub fn open(root: &Path) -> Result<Self, StoreError> {
        let root_directory = open_directory(root, "pack_store_root")?;
        let packs_directory = open_child_directory(&root_directory, PACKS_DIRECTORY)?;
        let catalog_directory = open_child_directory(&root_directory, CATALOG_DIRECTORY)?;
        let staging_directory = open_child_directory(&root_directory, STAGING_DIRECTORY)?;
        Self::load(
            root.to_path_buf(),
            root_directory,
            packs_directory,
            catalog_directory,
            staging_directory,
        )
    }

    fn load(
        root: PathBuf,
        root_directory: File,
        packs_directory: File,
        catalog_directory: File,
        staging_directory: File,
    ) -> Result<Self, StoreError> {
        let metadata = scan_pack_metadata(&packs_directory)?;
        let build = ObjectCatalog::rebuild(metadata.iter().map(|(pack, data)| (*pack, data)))?;
        let staging_leftovers = list_directory_names(&staging_directory, "pack_staging_scan")?;
        let loaded = read_catalog(&catalog_directory, build.catalog.generation());
        let (catalog_state, catalog_rebuild_note, catalog_persist_error) = match loaded {
            Ok(Some(catalog)) if catalog == build.catalog => (CatalogState::Loaded, None, None),
            Ok(_) => match write_catalog(&catalog_directory, &build.catalog) {
                Ok(()) => (
                    CatalogState::RebuiltPersisted,
                    Some("catalog was missing or disagreed with scanned pack footers".to_owned()),
                    None,
                ),
                Err(error) => (
                    CatalogState::RebuiltMemoryOnly,
                    Some("catalog was missing or disagreed with scanned pack footers".to_owned()),
                    Some(error.to_string()),
                ),
            },
            Err(load_error) => match write_catalog(&catalog_directory, &build.catalog) {
                Ok(()) => (
                    CatalogState::RebuiltPersisted,
                    Some(format!("discarded catalog: {load_error}")),
                    None,
                ),
                Err(write_error) => (
                    CatalogState::RebuiltMemoryOnly,
                    Some(format!("discarded catalog: {load_error}")),
                    Some(write_error.to_string()),
                ),
            },
        };
        Ok(Self {
            root,
            root_directory,
            packs_directory,
            catalog_directory,
            staging_directory,
            staged: BTreeMap::new(),
            metadata,
            catalog: build.catalog,
            duplicates: build.duplicates,
            catalog_state,
            catalog_rebuild_note,
            catalog_persist_error,
            staging_leftovers,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn catalog(&self) -> &ObjectCatalog {
        &self.catalog
    }

    pub const fn catalog_state(&self) -> CatalogState {
        self.catalog_state
    }

    pub fn catalog_persist_error(&self) -> Option<&str> {
        self.catalog_persist_error.as_deref()
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

    pub fn staged_len(&self) -> usize {
        self.staged.len()
    }

    pub fn seal_staged(
        &mut self,
        target_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<SealReceipt, StoreError> {
        if self.staged.is_empty() {
            return Ok(SealReceipt {
                packs: Vec::new(),
                objects: 0,
                catalog_state: self.catalog_state,
                catalog_persist_error: self.catalog_persist_error.clone(),
            });
        }
        let mut builder = PackBuilder::default();
        for (key, bytes) in &self.staged {
            builder.insert(*key, bytes)?;
        }
        let packs = builder.seal_targeted(target_bytes)?;
        let ids = packs.iter().map(|pack| pack.id).collect::<Vec<_>>();
        for pack in &packs {
            install_pack(&self.packs_directory, &self.staging_directory, pack)?;
            work.packs_sealed = work.packs_sealed.saturating_add(1);
        }
        sync_directory(&self.packs_directory, "pack_directory_sync")?;
        sync_directory(&self.staging_directory, "pack_staging_sync")?;
        let metadata = scan_pack_metadata(&self.packs_directory)?;
        let build = ObjectCatalog::rebuild(metadata.iter().map(|(pack, data)| (*pack, data)))?;
        self.metadata = metadata;
        self.catalog = build.catalog;
        self.duplicates = build.duplicates;
        self.staged.clear();
        match write_catalog(&self.catalog_directory, &self.catalog) {
            Ok(()) => {
                self.catalog_state = CatalogState::RebuiltPersisted;
                self.catalog_rebuild_note = Some("sealed pack set changed".to_owned());
                self.catalog_persist_error = None;
            }
            Err(error) => {
                self.catalog_state = CatalogState::RebuiltMemoryOnly;
                self.catalog_rebuild_note = Some("sealed pack set changed".to_owned());
                self.catalog_persist_error = Some(error.to_string());
            }
        }
        Ok(SealReceipt {
            packs: ids,
            objects: self.catalog.len(),
            catalog_state: self.catalog_state,
            catalog_persist_error: self.catalog_persist_error.clone(),
        })
    }

    pub fn deep_verify(&self) -> Result<DeepVerifyReceipt, StoreError> {
        let metadata = scan_pack_metadata(&self.packs_directory)?;
        let build = ObjectCatalog::rebuild(metadata.iter().map(|(pack, data)| (*pack, data)))?;
        let mut bytes_read = 0_u64;
        for (pack, data) in &metadata {
            let mut file =
                open_regular_file_at(&self.packs_directory, &pack.file_name(), "pack_deep_open")?;
            let verification = data.verify_file(&mut file, *pack)?;
            bytes_read = bytes_read
                .checked_add(verification.bytes_read)
                .ok_or_else(|| resource("pack_deep_work", "deep verification work overflows"))?;
        }
        Ok(DeepVerifyReceipt {
            packs: metadata.len(),
            objects: build.catalog.len(),
            bytes_read,
            duplicate_objects: build.duplicates,
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
        work.catalog_lookups = work.catalog_lookups.saturating_add(1);
        if let Some(bytes) = self.staged.get(&key) {
            if bytes.len() > maximum_bytes {
                return Err(resource(
                    "pack_read_limit",
                    "staged object exceeds the caller read bound",
                ));
            }
            key.verify(bytes)?;
            work.objects_read = work.objects_read.saturating_add(1);
            work.bytes_read = work.bytes_read.saturating_add(bytes.len() as u64);
            return Ok(Some(bytes.clone()));
        }
        let Some(location) = self.catalog.get(key) else {
            return Ok(None);
        };
        let metadata = self.metadata.get(&location.pack).ok_or_else(|| {
            corrupt(
                "pack_catalog_metadata",
                "catalog names a pack without scanned metadata",
            )
        })?;
        let entry = metadata.find(key).ok_or_else(|| {
            corrupt(
                "pack_catalog_entry",
                "catalog names an object absent from the pack footer",
            )
        })?;
        if entry.offset != location.offset
            || entry.encoded_length != location.length
            || entry.checksum != location.checksum
        {
            return Err(corrupt(
                "pack_catalog_location",
                "catalog coordinates disagree with the immutable pack footer",
            ));
        }
        let mut file = open_regular_file_at(
            &self.packs_directory,
            &location.pack.file_name(),
            "pack_object_open",
        )?;
        let observed_length = file
            .metadata()
            .map_err(|error| io_error("pack_object_metadata", "failed to inspect pack", error))?
            .len();
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
        work.catalog_lookups = work.catalog_lookups.saturating_add(1);
        Ok(self.staged.contains_key(&key) || self.catalog.get(key).is_some())
    }

    fn stage(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        key.verify(bytes)?;
        if let Some(existing) = self.read(key, key.domain.maximum_bytes(), work)? {
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

fn scan_pack_metadata(directory: &File) -> Result<BTreeMap<PackId, PackMetadata>, StoreError> {
    let mut packs = BTreeMap::new();
    for name in list_directory_names(directory, "pack_directory_scan")? {
        let pack = PackId::parse_file_name(&name)?;
        let mut file = open_regular_file_at(directory, &name, "pack_footer_open")?;
        let length = file
            .metadata()
            .map_err(|error| io_error("pack_footer_metadata", "failed to inspect pack", error))?
            .len();
        let read = PackMetadata::read_footer(&mut file, length)?;
        packs.insert(pack, read.metadata);
    }
    Ok(packs)
}

fn read_catalog(
    directory: &File,
    expected_generation: [u8; 32],
) -> Result<Option<ObjectCatalog>, StoreError> {
    let Some(mut file) = open_optional_regular_file_at(
        directory,
        CURRENT_CATALOG,
        OFlags::RDONLY,
        "pack_catalog_open",
    )?
    else {
        return Ok(None);
    };
    let length = file
        .metadata()
        .map_err(|error| io_error("pack_catalog_metadata", "failed to inspect catalog", error))?
        .len();
    let length = usize::try_from(length).map_err(|_| {
        resource(
            "pack_catalog_size",
            "catalog length does not fit this platform",
        )
    })?;
    if length > contract::MAXIMUM_CATALOG_BYTES {
        return Err(resource(
            "pack_catalog_size",
            "catalog exceeds its hostile decoder bound",
        ));
    }
    let mut bytes = vec![0_u8; length];
    file.read_exact(&mut bytes)
        .map_err(|error| read_error("pack_catalog_read", "catalog is truncated", error))?;
    ObjectCatalog::decode(&bytes, expected_generation).map(Some)
}

fn write_catalog(directory: &File, catalog: &ObjectCatalog) -> Result<(), StoreError> {
    let bytes = catalog.encode()?;
    let temporary = format!(".catalog-stage-{}", random_hex()?);
    let fd = rustix::fs::openat(
        directory,
        temporary.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| {
        rustix_error(
            "pack_catalog_stage",
            "failed to create catalog stage",
            error,
        )
    })?;
    let mut file = File::from(fd);
    let write_result = file
        .write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| {
            io_error(
                "pack_catalog_write",
                "failed to persist catalog stage",
                error,
            )
        });
    drop(file);
    if let Err(error) = write_result {
        let _ = rustix::fs::unlinkat(directory, temporary.as_str(), AtFlags::empty());
        return Err(error);
    }
    if let Err(error) =
        rustix::fs::renameat(directory, temporary.as_str(), directory, CURRENT_CATALOG)
    {
        let _ = rustix::fs::unlinkat(directory, temporary.as_str(), AtFlags::empty());
        return Err(rustix_error(
            "pack_catalog_publish",
            "failed to publish rebuilt catalog",
            error,
        ));
    }
    sync_directory(directory, "pack_catalog_sync")
}

fn install_pack(
    packs_directory: &File,
    staging_directory: &File,
    pack: &SealedPack,
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
    let write_result = file
        .write_all(&pack.bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io_error("pack_stage_write", "failed to persist pack stage", error));
    drop(file);
    if let Err(error) = write_result {
        let _ = rustix::fs::unlinkat(staging_directory, temporary.as_str(), AtFlags::empty());
        return Err(error);
    }
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
    rustix::fs::unlinkat(staging_directory, temporary.as_str(), AtFlags::empty())
        .map_err(|error| rustix_error("pack_stage_remove", "failed to remove pack stage", error))?;
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
