//! Derived compiler-unit persistence and revision-bound manifest cache.

use super::lower::{CompilationReceipt, CompilationWork, compile_unit};
use super::manifest::{
    COMPILATION_MANIFEST_CONTRACT_VERSION, CompilationBinding, CompilationManifest,
    CompilationManifestDigest, CompilerUnitObjectDigest, MAXIMUM_COMPILATION_MANIFEST_BYTES,
    MAXIMUM_COMPILATION_UNITS, compilation_map_key, manifest_error,
};
use super::unit::{
    BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_VERSION, CompilationUnit, OptimizationPolicy,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{EncodedOwnerKey, OwnerKey};
use crate::platform::persistent_map::{
    MapEdit, MapError, MapErrorClass, MapRoot, MapWork, MemoryPageStore, OverlayPageStore,
    PersistentMap,
};
use crate::platform::publication::{
    GraphRepository, PreparedPublication, RepositoryView, RevisionRecord, TransactionBody,
};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use crate::platform::storage::directory::SealReceipt;
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StoreError, StoreErrorClass, StoreWork,
};
use crate::platform::storage::page_store::ObjectPageReader;
use bincode::{Decode, Encode};
use fs2::FileExt;
use rustix::fs::{AtFlags, Mode, OFlags};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Read, Write};

const DERIVED_DIRECTORY: &str = "derived";
const COMPILER_DIRECTORY: &str = "compiler";
const CACHE_LOCK_FILE: &str = "LOCK";
const CACHE_HEAD_FILE: &str = "CURRENT";
const CACHE_HEAD_STAGE_PREFIX: &str = ".CURRENT-stage-";
const CACHE_HEAD_CONTRACT_VERSION: u16 = 1;
const CACHE_HEAD_MAGIC: [u8; 8] = *b"LKJCMH01";
const CACHE_HEAD_ENVELOPE_DOMAIN: &str = "lkjscript.compilation-cache-head-envelope.v1";
const MAXIMUM_CACHE_HEAD_BYTES: usize = 4 * 1024;

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
struct CompilationCacheHead {
    contract_version: u16,
    repository_id: RepositoryId,
    revision: RevisionId,
    manifest: CompilationManifestDigest,
}

impl CompilationCacheHead {
    fn encode(self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        crate::platform::packed::encode(
            CACHE_HEAD_MAGIC,
            CACHE_HEAD_ENVELOPE_DOMAIN,
            &self,
            MAXIMUM_CACHE_HEAD_BYTES,
        )
    }

    fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let head: Self = crate::platform::packed::decode(
            bytes,
            CACHE_HEAD_MAGIC,
            CACHE_HEAD_ENVELOPE_DOMAIN,
            MAXIMUM_CACHE_HEAD_BYTES,
        )?;
        head.validate()?;
        if head.encode()? != bytes {
            return Err(cache_error(
                DiagnosticClass::Corrupt,
                "compilation_cache_head_canonical",
                "derived compilation cache head is not canonically encoded",
            ));
        }
        Ok(head)
    }

    fn validate(self) -> Result<(), Diagnostic> {
        if self.contract_version != CACHE_HEAD_CONTRACT_VERSION
            || self.repository_id.bytes() == [0; 16]
        {
            return Err(cache_error(
                DiagnosticClass::Source,
                "compilation_cache_head_contract",
                "derived compilation cache head uses a predecessor or foreign contract",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompilationBuildProfile {
    Clean,
    Incremental,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompilationBuildWork {
    pub inventory_map_pages: u64,
    pub inventory_bindings: u64,
    pub selection_map_pages: u64,
    pub selection_objects_read: u64,
    pub compilation: CompilationWork,
    pub manifest_map: MapWork,
    pub manifest_store: StoreWork,
    pub stage_store: StoreWork,
}

#[derive(Clone, Debug)]
pub struct CompilationBuildReceipt {
    pub profile: CompilationBuildProfile,
    pub revision: RevisionId,
    pub manifest: CompilationManifest,
    pub manifest_digest: CompilationManifestDigest,
    pub manifest_bytes: Vec<u8>,
    pub units_compiled: u64,
    pub units_reused: u64,
    pub units_removed: u64,
    pub seal: SealReceipt,
    pub work: CompilationBuildWork,
}

#[derive(Clone, Debug)]
pub struct CachedCompilation {
    pub manifest: CompilationManifest,
    pub digest: CompilationManifestDigest,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CompilationValidationReceipt {
    pub units: u64,
    pub map: MapWork,
    pub store: StoreWork,
}

pub fn load_current_compilation(
    repository: &GraphRepository,
) -> Result<Option<CachedCompilation>, Diagnostic> {
    let current = repository.current()?;
    let Some(head) = read_cache_head(repository)? else {
        return Ok(None);
    };
    if head.repository_id != current.head.repository_id {
        return Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_cache_repository",
            "derived compilation cache head belongs to another repository",
        ));
    }
    if head.revision != current.head.revision {
        return Ok(None);
    }
    let mut store = repository.object_store()?;
    let manifest = read_manifest(&mut store, head.manifest)?;
    bind_manifest_to_current(&manifest, &current)?;
    Ok(Some(CachedCompilation {
        manifest,
        digest: head.manifest,
    }))
}

pub(crate) fn load_exact_current_compilation(
    repository: &GraphRepository,
    digest: CompilationManifestDigest,
) -> Result<CachedCompilation, Diagnostic> {
    let current = repository.current()?;
    let mut store = repository.object_store()?;
    let manifest = read_manifest(&mut store, digest)?;
    bind_manifest_to_current(&manifest, &current)?;
    Ok(CachedCompilation { manifest, digest })
}

pub fn build_clean(
    repository: &GraphRepository,
    optimization: OptimizationPolicy,
) -> Result<CompilationBuildReceipt, Diagnostic> {
    let view = repository.view_current()?;
    let inventory = view.compilation_unit_owners(MAXIMUM_COMPILATION_UNITS)?;
    let mut work = CompilationBuildWork {
        inventory_map_pages: inventory.work.map.pages_read,
        inventory_bindings: inventory.work.witness_records_decoded,
        ..CompilationBuildWork::default()
    };
    build_clean_for_view(repository, &view, optimization, inventory.value, &mut work)
}

pub fn build_incremental(
    repository: &GraphRepository,
    base_manifest: CompilationManifestDigest,
    publication: &PreparedPublication,
) -> Result<CompilationBuildReceipt, Diagnostic> {
    let affected = &publication.compiler_units;
    if affected.len() as u64 > MAXIMUM_COMPILATION_UNITS {
        return Err(cache_error(
            DiagnosticClass::Resource,
            "compilation_incremental_unit_count",
            "incremental compiler impact exceeds the current derived-unit implementation bound",
        ));
    }
    if affected
        .iter()
        .any(|owner| !matches!(owner, OwnerKey::Declaration(_) | OwnerKey::Target(_)))
    {
        return Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_incremental_owner_domain",
            "compiler impact contains an owner that cannot own a compilation unit",
        ));
    }
    let view = repository.view_current()?;
    let mut store = repository.object_store()?;
    let mut manifest_store = StoreWork::default();
    let base = read_manifest_with_work(&mut store, base_manifest, &mut manifest_store)?;
    validate_incremental_base(&view, &base, publication, &mut store, &mut manifest_store)?;
    let mut work = CompilationBuildWork {
        manifest_store,
        ..CompilationBuildWork::default()
    };
    build_incremental_for_view(repository, &view, &store, base, affected, &mut work)
}

fn build_clean_for_view(
    repository: &GraphRepository,
    view: &RepositoryView,
    optimization: OptimizationPolicy,
    owners: Vec<OwnerKey>,
    work: &mut CompilationBuildWork,
) -> Result<CompilationBuildReceipt, Diagnostic> {
    let mut objects = BTreeMap::new();
    let mut entries = Vec::with_capacity(owners.len());
    for owner in owners {
        let receipt = compile_unit(view, view, owner, optimization)?;
        add_compilation_work(&mut work.compilation, receipt.work);
        let binding = binding_for_receipt(&receipt)?;
        insert_object(&mut objects, receipt.object, receipt.bytes)?;
        entries.push((compilation_map_key(owner).to_vec(), binding.encode(owner)?));
    }
    let mut pages = MemoryPageStore::default();
    let units = PersistentMap::from_sorted(&mut pages, entries, &mut work.manifest_map)
        .map_err(map_diagnostic)?
        .root();
    stage_pages(&mut objects, &pages)?;
    finish_build(
        repository,
        view,
        optimization,
        CompilationBuildProfile::Clean,
        units,
        objects,
        units.entries(),
        0,
        0,
        work,
    )
}

fn build_incremental_for_view(
    repository: &GraphRepository,
    view: &RepositoryView,
    store: &crate::platform::storage::directory::PackDirectoryStore,
    base: CompilationManifest,
    affected: &BTreeSet<OwnerKey>,
    work: &mut CompilationBuildWork,
) -> Result<CompilationBuildReceipt, Diagnostic> {
    let reader = ObjectPageReader::new(store);
    let base_map = PersistentMap::from_root(base.units);
    // Even a presentation-only revision with an empty compiler impact must prove that the reused
    // derived root exists and agrees with its committed count. Link and explicit validation remain
    // the exhaustive corruption checks for untouched subtrees.
    let _ = base_map
        .lookup(&reader, &[0_u8; 17], &mut work.manifest_map)
        .map_err(map_diagnostic)?;

    let mut objects = BTreeMap::new();
    let mut edits = Vec::with_capacity(affected.len());
    let mut compiled = 0_u64;
    let mut removed = 0_u64;
    for owner in affected {
        let before = base_map
            .lookup(
                &reader,
                &compilation_map_key(*owner),
                &mut work.manifest_map,
            )
            .map_err(map_diagnostic)?;
        let selected = view.owner(*owner)?;
        work.selection_map_pages = work
            .selection_map_pages
            .saturating_add(selected.work.map.pages_read);
        work.selection_objects_read = work
            .selection_objects_read
            .saturating_add(selected.work.store.objects_read);
        let after = match selected.value {
            Some(record) if record.kind().has_compilation_unit() => {
                let receipt = compile_unit(view, view, *owner, base.optimization)?;
                add_compilation_work(&mut work.compilation, receipt.work);
                let binding = binding_for_receipt(&receipt)?.encode(*owner)?;
                insert_object(&mut objects, receipt.object, receipt.bytes)?;
                compiled = compiled.saturating_add(1);
                Some(binding)
            }
            Some(_) => {
                return Err(cache_error(
                    DiagnosticClass::Corrupt,
                    "compilation_incremental_owner_kind",
                    "compiler impact owner exists with a non-compilable semantic kind",
                ));
            }
            None => {
                if before.is_some() {
                    removed = removed.saturating_add(1);
                }
                None
            }
        };
        if before != after {
            edits.push(MapEdit {
                key: compilation_map_key(*owner).to_vec(),
                before,
                after,
            });
        }
    }

    let mut overlay = OverlayPageStore::new(&reader);
    let (units, _) = base_map
        .apply_sorted_edits(&mut overlay, &edits, &mut work.manifest_map)
        .map_err(map_diagnostic)?;
    let pages = overlay.into_pages();
    stage_pages(&mut objects, &pages)?;
    work.manifest_store.add(reader.work());
    let changed_present = edits.iter().filter(|edit| edit.after.is_some()).count() as u64;
    let reused = units.len().saturating_sub(changed_present);
    finish_build(
        repository,
        view,
        base.optimization,
        CompilationBuildProfile::Incremental,
        units.root(),
        objects,
        compiled,
        reused,
        removed,
        work,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "one derived compilation result binds these closed inputs and counters"
)]
fn finish_build(
    repository: &GraphRepository,
    view: &RepositoryView,
    optimization: OptimizationPolicy,
    profile: CompilationBuildProfile,
    units: MapRoot,
    mut objects: BTreeMap<ObjectKey, Vec<u8>>,
    units_compiled: u64,
    units_reused: u64,
    units_removed: u64,
    work: &mut CompilationBuildWork,
) -> Result<CompilationBuildReceipt, Diagnostic> {
    let manifest = CompilationManifest {
        contract_version: COMPILATION_MANIFEST_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        compiler_contract_version: COMPILER_UNIT_CONTRACT_VERSION,
        bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
        repository_id: view.current().head.repository_id,
        package_id: view.package(),
        revision: view.revision(),
        semantic_root: view.current().accepted.semantic_root,
        validation_certificate: view.current().witness.certificate,
        optimization,
        units,
    };
    let (manifest_digest, manifest_bytes) = manifest.encode()?;
    insert_object(
        &mut objects,
        ObjectKey::from_digest(ObjectDomain::CompilationManifest, manifest_digest.bytes()),
        manifest_bytes.clone(),
    )?;
    let (seal, stage_work) = repository.stage_compilation_objects(view.current().head, &objects)?;
    work.stage_store.add(stage_work);
    write_cache_head(
        repository,
        CompilationCacheHead {
            contract_version: CACHE_HEAD_CONTRACT_VERSION,
            repository_id: manifest.repository_id,
            revision: manifest.revision,
            manifest: manifest_digest,
        },
    )?;
    Ok(CompilationBuildReceipt {
        profile,
        revision: manifest.revision,
        manifest,
        manifest_digest,
        manifest_bytes,
        units_compiled,
        units_reused,
        units_removed,
        seal,
        work: *work,
    })
}

fn binding_for_receipt(receipt: &CompilationReceipt) -> Result<CompilationBinding, Diagnostic> {
    if receipt.object.domain != ObjectDomain::CompilerUnit {
        return Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_receipt_object_domain",
            "normalized compiler returned a foreign immutable object domain",
        ));
    }
    Ok(CompilationBinding {
        kind: receipt.unit.source.kind,
        key: receipt.key,
        object: CompilerUnitObjectDigest::from_bytes(receipt.object.digest.bytes()),
    })
}

fn stage_pages(
    objects: &mut BTreeMap<ObjectKey, Vec<u8>>,
    pages: &MemoryPageStore,
) -> Result<(), Diagnostic> {
    for (digest, bytes) in pages.objects() {
        insert_object(
            objects,
            ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()),
            bytes.to_vec(),
        )?;
    }
    Ok(())
}

fn insert_object(
    objects: &mut BTreeMap<ObjectKey, Vec<u8>>,
    key: ObjectKey,
    bytes: Vec<u8>,
) -> Result<(), Diagnostic> {
    key.verify(&bytes).map_err(store_diagnostic)?;
    match objects.entry(key) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(bytes);
            Ok(())
        }
        std::collections::btree_map::Entry::Occupied(entry) if entry.get() == &bytes => Ok(()),
        std::collections::btree_map::Entry::Occupied(_) => Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_object_collision",
            "one derived object key is bound to different bytes in one build",
        )),
    }
}

pub fn validate_current_compilation(
    repository: &GraphRepository,
    digest: CompilationManifestDigest,
) -> Result<CompilationValidationReceipt, Diagnostic> {
    let current = repository.current()?;
    let mut store = repository.object_store()?;
    let mut store_work = StoreWork::default();
    let manifest = read_manifest_with_work(&mut store, digest, &mut store_work)?;
    bind_manifest_to_current(&manifest, &current)?;

    let reader = ObjectPageReader::new(&store);
    let map = PersistentMap::from_root(manifest.units);
    let mut map_work = MapWork::default();
    map.verify(&reader, &mut map_work).map_err(map_diagnostic)?;
    let mut unit_work = StoreWork::default();
    let mut captured = None;
    let mut units = 0_u64;
    let result = map.for_each(&reader, &mut map_work, |key, value| {
        let operation = (|| {
            let owner = EncodedOwnerKey::decode(key)?;
            let binding = CompilationBinding::decode(value, owner)?;
            let bytes = store
                .read(
                    binding.object.object_key(),
                    ObjectDomain::CompilerUnit.maximum_bytes(),
                    &mut unit_work,
                )
                .map_err(store_diagnostic)?
                .ok_or_else(|| {
                    cache_error(
                        DiagnosticClass::Corrupt,
                        "compilation_unit_missing",
                        "compilation manifest references a missing compiler-unit object",
                    )
                })?;
            let unit = CompilationUnit::decode(&bytes, binding.object.object_key())?;
            if unit.key != binding.key
                || unit.source.package != manifest.package_id
                || unit.source.owner != owner
                || unit.source.kind != binding.kind
            {
                return Err(cache_error(
                    DiagnosticClass::Corrupt,
                    "compilation_unit_binding",
                    "compiler-unit bytes disagree with their exact manifest binding",
                ));
            }
            units = units.saturating_add(1);
            Ok::<(), Diagnostic>(())
        })();
        match operation {
            Ok(()) => Ok(()),
            Err(diagnostic) => {
                captured = Some(diagnostic);
                Err(MapError {
                    class: MapErrorClass::Corrupt,
                    code: "compilation_validation_stop",
                    message: "compilation validation stopped after an exact diagnostic".to_owned(),
                })
            }
        }
    });
    if let Some(diagnostic) = captured {
        return Err(diagnostic);
    }
    result.map_err(map_diagnostic)?;
    if units != manifest.units.entries() {
        return Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_validation_count",
            "validated compiler-unit count disagrees with the manifest map root",
        ));
    }
    store_work.add(reader.work());
    store_work.add(unit_work);
    Ok(CompilationValidationReceipt {
        units,
        map: map_work,
        store: store_work,
    })
}

pub(crate) fn read_current_binding(
    repository: &GraphRepository,
    manifest: CompilationManifestDigest,
    owner: OwnerKey,
) -> Result<Option<CompilationBinding>, Diagnostic> {
    let current = repository.current()?;
    let mut store = repository.object_store()?;
    let decoded = read_manifest(&mut store, manifest)?;
    bind_manifest_to_current(&decoded, &current)?;
    let reader = ObjectPageReader::new(&store);
    let mut map_work = MapWork::default();
    PersistentMap::from_root(decoded.units)
        .lookup(&reader, &compilation_map_key(owner), &mut map_work)
        .map_err(map_diagnostic)?
        .as_deref()
        .map(|bytes| CompilationBinding::decode(bytes, owner))
        .transpose()
}

fn read_manifest(
    store: &mut impl ImmutableObjectStore,
    digest: CompilationManifestDigest,
) -> Result<CompilationManifest, Diagnostic> {
    read_manifest_with_work(store, digest, &mut StoreWork::default())
}

fn read_manifest_with_work(
    store: &mut impl ImmutableObjectStore,
    digest: CompilationManifestDigest,
    work: &mut StoreWork,
) -> Result<CompilationManifest, Diagnostic> {
    let key = ObjectKey::from_digest(ObjectDomain::CompilationManifest, digest.bytes());
    let bytes = store
        .read(key, MAXIMUM_COMPILATION_MANIFEST_BYTES, work)
        .map_err(store_diagnostic)?
        .ok_or_else(|| {
            cache_error(
                DiagnosticClass::Corrupt,
                "compilation_manifest_missing",
                "derived compilation cache references a missing manifest object",
            )
        })?;
    CompilationManifest::decode(&bytes, digest)
}

fn bind_manifest_to_current(
    manifest: &CompilationManifest,
    current: &crate::platform::publication::CurrentPublication,
) -> Result<(), Diagnostic> {
    if manifest.repository_id != current.head.repository_id
        || manifest.package_id != current.semantic_root.package_id
        || manifest.revision != current.head.revision
        || manifest.semantic_root != current.accepted.semantic_root
        || manifest.validation_certificate != current.witness.certificate
    {
        return Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_manifest_revision_binding",
            "compilation manifest does not bind the exact accepted semantic revision",
        ));
    }
    Ok(())
}

fn validate_incremental_base(
    current: &RepositoryView,
    base: &CompilationManifest,
    publication: &PreparedPublication,
    store: &mut impl ImmutableObjectStore,
    store_work: &mut StoreWork,
) -> Result<(), Diagnostic> {
    if current.current().head != publication.head {
        return Err(cache_error(
            DiagnosticClass::Semantic,
            "compilation_incremental_result_stale",
            "prepared compiler impact does not produce the current accepted HEAD",
        ));
    }
    if publication.accepted.head != publication.head
        || publication.receipt.result != publication.head.revision
        || publication.authority.semantic.digest != current.current().accepted.semantic_root
        || publication.authority.witness.manifest.certificate
            != current.current().witness.certificate
        || publication.receipt.validation.compiler_units_planned
            != publication.compiler_units.len() as u64
    {
        return Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_incremental_prepared_binding",
            "prepared compiler impact disagrees with its exact publication evidence",
        ));
    }
    let Some(expected_base) = publication.expected_base else {
        return Err(cache_error(
            DiagnosticClass::Semantic,
            "compilation_incremental_initial_publication",
            "initial publication requires a clean normalized compilation",
        ));
    };
    let parents = &current.current().revision.publication.parents;
    if parents.len() != 1
        || parents[0].revision != base.revision
        || parents[0].revision != expected_base.revision
        || parents[0].record != expected_base.record
        || base.repository_id != current.current().head.repository_id
        || base.package_id != current.package()
    {
        return Err(cache_error(
            DiagnosticClass::Semantic,
            "compilation_incremental_base",
            "incremental compilation manifest is not the exact sole parent of current HEAD",
        ));
    }
    let (transaction_base, transaction_base_root, transaction_result_root) =
        match publication.transaction.body {
            TransactionBody::Change {
                base,
                base_root,
                result_root,
                ..
            } => (base, base_root, result_root),
            TransactionBody::Bootstrap { .. } => {
                return Err(cache_error(
                    DiagnosticClass::Corrupt,
                    "compilation_incremental_transaction",
                    "incremental compilation received a bootstrap transaction",
                ));
            }
        };
    if transaction_base != base.revision
        || transaction_base_root != base.semantic_root
        || transaction_result_root != current.current().accepted.semantic_root
    {
        return Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_incremental_transaction_binding",
            "incremental compiler base disagrees with the normalized accepted transaction",
        ));
    }
    let parent_key = ObjectKey::from_digest(ObjectDomain::Revision, expected_base.record.bytes());
    let parent_bytes = store
        .read(
            parent_key,
            ObjectDomain::Revision.maximum_bytes(),
            store_work,
        )
        .map_err(store_diagnostic)?
        .ok_or_else(|| {
            cache_error(
                DiagnosticClass::Corrupt,
                "compilation_incremental_parent_missing",
                "incremental compiler base revision record is missing",
            )
        })?;
    let parent = RevisionRecord::decode(&parent_bytes, expected_base.record)?;
    if parent.revision != expected_base.revision
        || parent.core.repository_id != base.repository_id
        || parent.publication.semantic_root != base.semantic_root
        || parent.publication.validation.certificate != base.validation_certificate
    {
        return Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_incremental_parent_binding",
            "base compilation manifest disagrees with its exact accepted parent revision",
        ));
    }
    Ok(())
}

fn read_cache_head(
    repository: &GraphRepository,
) -> Result<Option<CompilationCacheHead>, Diagnostic> {
    let Some(directory) = open_existing_cache_directory(repository)? else {
        return Ok(None);
    };
    let Some(mut file) = open_optional_regular(&directory, CACHE_HEAD_FILE)? else {
        return Ok(None);
    };
    let length = file
        .metadata()
        .map_err(|error| io_diagnostic("compilation_cache_head_metadata", error))?
        .len();
    let length = usize::try_from(length).map_err(|_| {
        cache_error(
            DiagnosticClass::Resource,
            "compilation_cache_head_length",
            "derived compilation cache head length does not fit this platform",
        )
    })?;
    if length > MAXIMUM_CACHE_HEAD_BYTES {
        return Err(cache_error(
            DiagnosticClass::Resource,
            "compilation_cache_head_length",
            "derived compilation cache head exceeds its hostile decoder bound",
        ));
    }
    let mut bytes = vec![0_u8; length];
    file.read_exact(&mut bytes)
        .map_err(|error| io_diagnostic("compilation_cache_head_read", error))?;
    CompilationCacheHead::decode(&bytes).map(Some)
}

fn write_cache_head(
    repository: &GraphRepository,
    head: CompilationCacheHead,
) -> Result<(), Diagnostic> {
    let directory = ensure_cache_directory(repository)?;
    let lock = open_cache_lock(&directory)?;
    FileExt::lock_exclusive(&lock)
        .map_err(|error| io_diagnostic("compilation_cache_lock", error))?;
    let bytes = head.encode()?;
    let temporary = format!("{CACHE_HEAD_STAGE_PREFIX}{}", random_hex()?);
    let fd = rustix::fs::openat(
        &directory,
        temporary.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| rustix_diagnostic("compilation_cache_head_stage", error))?;
    let mut file = File::from(fd);
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = rustix::fs::unlinkat(&directory, temporary.as_str(), AtFlags::empty());
        return Err(io_diagnostic("compilation_cache_head_write", error));
    }
    drop(file);
    if let Err(error) =
        rustix::fs::renameat(&directory, temporary.as_str(), &directory, CACHE_HEAD_FILE)
    {
        let _ = rustix::fs::unlinkat(&directory, temporary.as_str(), AtFlags::empty());
        return Err(rustix_diagnostic("compilation_cache_head_publish", error));
    }
    directory
        .sync_all()
        .map_err(|error| io_diagnostic("compilation_cache_head_sync", error))
}

fn open_existing_cache_directory(repository: &GraphRepository) -> Result<Option<File>, Diagnostic> {
    let root = open_directory(repository.root(), "compilation_cache_root_open")?;
    let Some(derived) = open_optional_directory(&root, DERIVED_DIRECTORY)? else {
        return Ok(None);
    };
    open_optional_directory(&derived, COMPILER_DIRECTORY)
}

fn ensure_cache_directory(repository: &GraphRepository) -> Result<File, Diagnostic> {
    let root = open_directory(repository.root(), "compilation_cache_root_open")?;
    let derived = ensure_child_directory(&root, DERIVED_DIRECTORY)?;
    let compiler = ensure_child_directory(&derived, COMPILER_DIRECTORY)?;
    root.sync_all()
        .map_err(|error| io_diagnostic("compilation_cache_root_sync", error))?;
    derived
        .sync_all()
        .map_err(|error| io_diagnostic("compilation_cache_derived_sync", error))?;
    Ok(compiler)
}

fn open_directory(path: &std::path::Path, code: &'static str) -> Result<File, Diagnostic> {
    let fd = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| rustix_diagnostic(code, error))?;
    Ok(File::from(fd))
}

fn ensure_child_directory(parent: &File, name: &str) -> Result<File, Diagnostic> {
    match rustix::fs::mkdirat(parent, name, Mode::from_raw_mode(0o700)) {
        Ok(()) => {}
        Err(error) if error == rustix::io::Errno::EXIST => {}
        Err(error) => {
            return Err(rustix_diagnostic(
                "compilation_cache_directory_create",
                error,
            ));
        }
    }
    open_optional_directory(parent, name)?.ok_or_else(|| {
        cache_error(
            DiagnosticClass::Infrastructure,
            "compilation_cache_directory_missing",
            "created derived compilation directory is not visible",
        )
    })
}

fn open_optional_directory(parent: &File, name: &str) -> Result<Option<File>, Diagnostic> {
    let fd = match rustix::fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
        Err(error) => {
            return Err(rustix_diagnostic("compilation_cache_directory_open", error));
        }
    };
    Ok(Some(File::from(fd)))
}

fn open_optional_regular(directory: &File, name: &str) -> Result<Option<File>, Diagnostic> {
    let fd = match rustix::fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
        Err(error) => {
            return Err(rustix_diagnostic("compilation_cache_regular_open", error));
        }
    };
    let file = File::from(fd);
    if !file
        .metadata()
        .map_err(|error| io_diagnostic("compilation_cache_regular_metadata", error))?
        .is_file()
    {
        return Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_cache_regular_type",
            "derived compilation cache entry is not a regular non-symlink file",
        ));
    }
    Ok(Some(file))
}

fn open_cache_lock(directory: &File) -> Result<File, Diagnostic> {
    let fd = rustix::fs::openat(
        directory,
        CACHE_LOCK_FILE,
        OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| rustix_diagnostic("compilation_cache_lock_open", error))?;
    let file = File::from(fd);
    if !file
        .metadata()
        .map_err(|error| io_diagnostic("compilation_cache_lock_metadata", error))?
        .is_file()
    {
        return Err(cache_error(
            DiagnosticClass::Corrupt,
            "compilation_cache_lock_type",
            "derived compilation cache lock is not a regular non-symlink file",
        ));
    }
    Ok(file)
}

fn random_hex() -> Result<String, Diagnostic> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        cache_error(
            DiagnosticClass::Infrastructure,
            "compilation_cache_stage_random",
            format!("secure randomness for compiler cache staging is unavailable: {error}"),
        )
    })?;
    Ok(crate::platform::semantic_id::encode_hex(&bytes))
}

fn add_compilation_work(total: &mut CompilationWork, other: CompilationWork) {
    total.canonical.add(other.canonical);
    total.witness.add(other.witness);
    total.owner_records_read = total
        .owner_records_read
        .saturating_add(other.owner_records_read);
    total.expression_records_read = total
        .expression_records_read
        .saturating_add(other.expression_records_read);
    total.instructions_emitted = total
        .instructions_emitted
        .saturating_add(other.instructions_emitted);
    total.maximum_expression_depth = total
        .maximum_expression_depth
        .max(other.maximum_expression_depth);
    total.bytes_encoded = total.bytes_encoded.saturating_add(other.bytes_encoded);
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    cache_error(
        match error.class {
            MapErrorClass::Input => DiagnosticClass::Source,
            MapErrorClass::Resource => DiagnosticClass::Resource,
            MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
            MapErrorClass::Store => DiagnosticClass::Infrastructure,
        },
        error.code,
        error.message,
    )
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    cache_error(
        match error.class {
            StoreErrorClass::Input => DiagnosticClass::Source,
            StoreErrorClass::Resource => DiagnosticClass::Resource,
            StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
            StoreErrorClass::Io => DiagnosticClass::Infrastructure,
        },
        error.code,
        error.message,
    )
}

fn rustix_diagnostic(code: &'static str, error: rustix::io::Errno) -> Diagnostic {
    cache_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("derived compilation cache filesystem operation failed: {error}"),
    )
}

fn io_diagnostic(code: &'static str, error: std::io::Error) -> Diagnostic {
    cache_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("derived compilation cache I/O failed: {error}"),
    )
}

fn cache_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    manifest_error(class, code, message)
}
