//! Current semantic admission is derived from, and never replaces, historical acceptance.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{FullValidationReport, KernelSnapshot};
use crate::platform::storage::backend::StorageBackend;
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StageOutcome, StoreError, StoreErrorClass,
    StoreReadAdmission, StoreReadLimits, StoreWork,
};
use crate::platform::storage::project::MemoryBytes;
use crate::platform::witness::{
    CanonicalWitnessFacts, ValidationWitnessDigest, ValidationWitnessManifest,
    bind_witness_manifest, rebuild_canonical_facts,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex, OnceLock, Weak};

// Retain only proof this implementation constructed. Distinct projects share a bounded LRU;
// every reuse checks its complete read set against the selected physical store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CurrentValidationKey {
    pub root: crate::platform::kernel::SemanticRootDigest,
    pub revision: crate::platform::semantic_id::RevisionId,
    pub publication: super::RevisionObjectDigest,
    pub validator: crate::platform::witness::ValidatorContractDigest,
    pub implementation: [u8; 32],
}
// A process can interleave dozens of source authorities. The conservative decoded
// estimate charges raw bytes eightfold, so a small global bound evicts live peers.
const MAXIMUM_CACHE_BYTES: u64 = 512 * 1_048_576;
const MAXIMUM_CACHE_ENTRIES: usize = 64;

#[derive(Debug)]
struct CachedRead {
    key: ObjectKey,
    // Weak identity keeps neither the payload nor its arena reservation alive. Its control block
    // prevents pointer reuse while cached. No address enters a canonical identity or encoding.
    memory: Option<Weak<MemoryBytes>>,
}

#[derive(Debug)]
struct CachedValidation {
    key: CurrentValidationKey,
    objects: Box<[CachedRead]>,
    selections: Box<[crate::platform::package_transport::PackageTransportBinding]>,
    context: Arc<RevalidatedBase>,
    retained_bytes: u64,
}

#[derive(Debug, Default)]
struct ValidationCache {
    entries: VecDeque<Arc<CachedValidation>>,
    retained_bytes: u64,
}

impl ValidationCache {
    fn get(&mut self, key: CurrentValidationKey) -> Option<Arc<CachedValidation>> {
        let position = self.entries.iter().position(|entry| entry.key == key)?;
        let entry = self.entries.remove(position)?;
        self.entries.push_back(entry.clone());
        Some(entry)
    }

    fn retain(&mut self, entry: Arc<CachedValidation>) {
        if entry.retained_bytes > MAXIMUM_CACHE_BYTES {
            return;
        }
        if let Some(position) = self.entries.iter().position(|old| old.key == entry.key)
            && let Some(old) = self.entries.remove(position)
        {
            self.retained_bytes -= old.retained_bytes;
        }
        while self.entries.len() >= MAXIMUM_CACHE_ENTRIES
            || self.retained_bytes > MAXIMUM_CACHE_BYTES - entry.retained_bytes
        {
            let Some(old) = self.entries.pop_front() else {
                return;
            };
            self.retained_bytes -= old.retained_bytes;
        }
        self.retained_bytes += entry.retained_bytes;
        self.entries.push_back(entry);
    }
}

static CURRENT_CONTEXT: OnceLock<Mutex<ValidationCache>> = OnceLock::new();

pub(super) fn cached(
    key: CurrentValidationKey,
    store: &ViewStore,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
) -> Result<Option<(Arc<RevalidatedBase>, StoreWork)>, Diagnostic> {
    let entry = CURRENT_CONTEXT
        .get()
        .and_then(|cache| cache.lock().ok())
        .and_then(|mut cache| cache.get(key));
    let Some(entry) = entry else {
        return Ok(None);
    };
    if !selections_match(store.persistent_base(), &entry.selections, checkpoint)? {
        return Ok(None);
    }
    let work = recheck(&entry, store, checkpoint, REBUILD_LIMITS)?;
    Ok(Some((entry.context.clone(), work)))
}

pub(super) fn dependency_selections(
    snapshot: &KernelSnapshot,
    ready: &crate::platform::package_transport::source::PackageReadiness,
) -> Result<Vec<crate::platform::package_transport::PackageTransportBinding>, Diagnostic> {
    let mut selections = BTreeMap::new();
    for dependency in snapshot.representation_dependencies.values() {
        let transport = ready
            .bindings
            .get(&dependency.package_revision)
            .copied()
            .ok_or_else(selection_changed)?;
        selections.insert(dependency.package_revision, transport);
    }
    Ok(selections
        .into_iter()
        .map(|(package_revision, transport)| {
            crate::platform::package_transport::PackageTransportBinding {
                package_revision,
                transport,
            }
        })
        .collect())
}

pub(super) fn selections_match(
    store: &StorageBackend,
    selections: &[crate::platform::package_transport::PackageTransportBinding],
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
) -> Result<bool, Diagnostic> {
    if selections.is_empty() {
        return Ok(true);
    }
    checkpoint()?;
    let ready = super::repository::read_store_readiness(store)?;
    for selection in selections {
        checkpoint()?;
        if ready.bindings.get(&selection.package_revision) != Some(&selection.transport) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(super) fn selection_changed() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Semantic,
        "publication_current_validation_selection_changed",
        "an exact dependency transport selection changed during current validation; retry the selected closure",
    )
}

pub(super) fn recheck_selected_objects(
    store: &StorageBackend,
    objects: &BTreeSet<ObjectKey>,
    selections: &[crate::platform::package_transport::PackageTransportBinding],
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
) -> Result<(), Diagnostic> {
    if !selections_match(store, selections, checkpoint)? {
        return Err(selection_changed());
    }
    let mut admission = StoreReadAdmission::new(REBUILD_LIMITS);
    let mut work = StoreWork::default();
    for key in objects {
        checkpoint()?;
        let bytes = store
            .read_admitted(*key, key.domain.maximum_bytes(), &mut admission, &mut work)
            .map_err(read_error)?
            .ok_or_else(missing_object)?;
        key.verify(&bytes).map_err(read_error)?;
    }
    checkpoint()
}

fn recheck(
    entry: &CachedValidation,
    store: &ViewStore,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    limits: StoreReadLimits,
) -> Result<StoreWork, Diagnostic> {
    let mut work = StoreWork::default();
    let mut admission = StoreReadAdmission::new(limits);
    for object in entry.objects.iter() {
        checkpoint()?;
        let key = object.key;
        if let StorageBackend::Memory(memory) = &store.base
            && !store.derived.contains_key(&key)
        {
            admission.admit_catalog_lookup().map_err(read_error)?;
            work.add(crate::platform::storage::object::StoreWork { catalog_lookups: 1, ..crate::platform::storage::object::StoreWork::default() });
            let value = memory.objects.get(&key).ok_or_else(missing_object)?;
            let length = value.as_slice().len();
            if length > key.domain.maximum_bytes() {
                return Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "publication_validation_object_size",
                    "cached validation object exceeds its domain read bound",
                ));
            }
            // Identical immutable allocation proves unchanged bytes, but it grants no extra
            // read capacity. Charge the same complete object/byte allowance before reuse.
            admission.admit_object(length).map_err(read_error)?;
            if object
                .memory
                .as_ref()
                .is_some_and(|old| old.as_ptr() == Arc::as_ptr(value))
            {
                work.add(crate::platform::storage::object::StoreWork { objects_reused: 1, ..crate::platform::storage::object::StoreWork::default() });
            } else {
                key.verify(value.as_slice()).map_err(read_error)?;
                work.add(crate::platform::storage::object::StoreWork { objects_read: 1, ..crate::platform::storage::object::StoreWork::default() });
                work.add(crate::platform::storage::object::StoreWork { bytes_read: length as u64, ..crate::platform::storage::object::StoreWork::default() });
            }
            continue;
        }
        // Disk bytes can change independently of this process; re-read and verify every object.
        // Derived overlays also require their own content proof, independent of any base identity.
        let bytes = store
            .read_admitted(key, key.domain.maximum_bytes(), &mut admission, &mut work)
            .map_err(read_error)?
            .ok_or_else(missing_object)?;
        key.verify(&bytes).map_err(read_error)?;
    }
    Ok(work)
}

fn missing_object() -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "publication_current_validation_object_missing",
        "cached current validation references a missing canonical or dependency object",
    )
}

fn read_error(error: StoreError) -> Diagnostic {
    Diagnostic::new(
        match error.class {
            StoreErrorClass::Resource => DiagnosticClass::Resource,
            _ => DiagnosticClass::Corrupt,
        },
        error.code,
        error.message,
    )
}

pub(super) fn retain(
    key: CurrentValidationKey,
    read_bytes: u64,
    objects: BTreeSet<ObjectKey>,
    selections: Vec<crate::platform::package_transport::PackageTransportBinding>,
    context: Arc<RevalidatedBase>,
    store: &ViewStore,
) {
    let Some(entry) = cache_entry(key, read_bytes, objects, selections, context, store) else {
        return;
    };
    if let Ok(mut cache) = CURRENT_CONTEXT
        .get_or_init(|| Mutex::new(ValidationCache::default()))
        .lock()
    {
        cache.retain(Arc::new(entry));
    }
}

fn cache_entry(
    key: CurrentValidationKey,
    read_bytes: u64,
    objects: BTreeSet<ObjectKey>,
    selections: Vec<crate::platform::package_transport::PackageTransportBinding>,
    context: Arc<RevalidatedBase>,
    store: &ViewStore,
) -> Option<CachedValidation> {
    if context.current.as_ref().err().is_some_and(|errors| {
        errors
            .iter()
            .any(|error| error.class != DiagnosticClass::Semantic)
    }) {
        return None;
    }
    let derived_bytes = context
        .facts
        .summary_objects
        .values()
        .map(Vec::len)
        .chain(context.facts.pages.objects().map(|(_, bytes)| bytes.len()))
        .try_fold(0_u64, |total, bytes| total.checked_add(bytes as u64));
    // Preserve the conservative context estimate and include all retained read-set/control-block
    // metadata. An oversized proof does not displace unrelated, admissible cached contexts.
    let object_metadata = (std::mem::size_of::<CachedRead>()
        + std::mem::size_of::<MemoryBytes>()
        + 4 * std::mem::size_of::<usize>()) as u64;
    let estimate = derived_bytes
        .and_then(|bytes| bytes.checked_add(read_bytes))
        .and_then(|bytes| bytes.checked_mul(8))
        .and_then(|bytes| {
            bytes.checked_add((context.snapshot.owners.len() as u64).checked_mul(256)?)
        })
        .and_then(|bytes| bytes.checked_add((objects.len() as u64).checked_mul(object_metadata)?))
        .and_then(|bytes| {
            bytes.checked_add((selections.len() as u64).checked_mul(std::mem::size_of::<
                crate::platform::package_transport::PackageTransportBinding,
            >() as u64)?)
        })
        .and_then(|bytes| bytes.checked_add(256))
        .filter(|bytes| *bytes <= MAXIMUM_CACHE_BYTES)?;
    let objects = objects
        .into_iter()
        .map(|key| {
            let memory = match &store.base {
                StorageBackend::Memory(memory) if !store.derived.contains_key(&key) => {
                    memory.objects.get(&key).map(Arc::downgrade)
                }
                _ => None,
            };
            CachedRead { key, memory }
        })
        .collect();
    Some(CachedValidation {
        key,
        objects,
        selections: selections.into_boxed_slice(),
        context,
        retained_bytes: estimate,
    })
}

#[cfg(test)]
#[path = "validation_cache_tests.rs"]
mod tests;

#[derive(Debug)]
pub(super) struct CurrentValidation {
    pub manifest: ValidationWitnessManifest,
    pub digest: ValidationWitnessDigest,
    pub bytes: Vec<u8>,
    pub report: FullValidationReport,
}

#[derive(Debug)]
pub(super) struct RevalidatedBase {
    pub snapshot: KernelSnapshot,
    pub facts: CanonicalWitnessFacts,
    pub current: Result<CurrentValidation, Vec<Diagnostic>>,
    pub semantic_work: u64,
    pub semantic_work_saturated: bool,
}

impl RevalidatedBase {
    pub fn rebuild(
        snapshot: KernelSnapshot,
        checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    ) -> Result<Self, Diagnostic> {
        checkpoint()?;
        let facts = rebuild_canonical_facts(&snapshot)?;
        let mut semantic_work = crate::platform::kernel::ValidationWork::default();
        let current = crate::platform::kernel::validate_full_checked(
            &snapshot,
            None,
            &mut semantic_work,
            checkpoint,
        )
        .and_then(|report| {
            let root = crate::platform::kernel::encode_root(&snapshot.root)
                .map_err(|error| vec![error])?
                .0;
            let (manifest, digest, bytes) = bind_witness_manifest(
                snapshot.root.repository_id,
                snapshot.root.package_id,
                root,
                facts.roots,
            )
            .map_err(|error| vec![error])?;
            Ok(CurrentValidation {
                manifest,
                digest,
                bytes,
                report,
            })
        });
        checkpoint()?;
        Ok(Self {
            snapshot,
            facts,
            current,
            semantic_work: semantic_work.consumed as u64,
            semantic_work_saturated: semantic_work.saturated,
        })
    }

    pub fn require_current(&self) -> Result<&CurrentValidation, Diagnostic> {
        self.current.as_ref().map_err(|errors| {
            let mut diagnostic = errors.first().cloned().unwrap_or_else(|| Diagnostic::new(
                DiagnosticClass::Semantic, "publication_current_validation",
                "historically accepted program has no current semantic proof",
            ));
            diagnostic.notes.push("Historical acceptance and HEAD are intact. Inspect the canonical definition and repair it with change plan / change apply; the complete candidate must pass the current validator before publication.".to_owned());
            diagnostic
        })
    }
}

/// Read-only in-memory derived objects over the pinned physical store. No read-side catalog or
/// publication mutation is needed to inspect or revalidate a compatible historical revision.
#[derive(Debug)]
pub(super) struct ViewStore {
    base: StorageBackend,
    derived: BTreeMap<ObjectKey, Vec<u8>>,
    rebuild_reads: std::cell::Cell<StoreReadAdmission>,
    rebuild_objects: std::cell::RefCell<Option<BTreeSet<ObjectKey>>>,
    link_read_set: std::cell::RefCell<Option<LinkReadSet>>,
}

#[derive(Debug, Default)]
struct LinkReadSet {
    objects: BTreeSet<ObjectKey>,
    visits: u64,
    modeled_bytes: usize,
}

// Revalidation observes complete input reads; only explicit enclosing policies
// may impose an aggregate read quota.
const REBUILD_LIMITS: StoreReadLimits = StoreReadLimits::unbounded();

impl ViewStore {
    pub(super) fn persistent_base(&self) -> &StorageBackend {
        &self.base
    }

    pub const fn new(base: StorageBackend) -> Self {
        Self {
            base,
            derived: BTreeMap::new(),
            rebuild_reads: std::cell::Cell::new(StoreReadAdmission::unbounded()),
            rebuild_objects: std::cell::RefCell::new(None),
            link_read_set: std::cell::RefCell::new(None),
        }
    }

    pub(super) fn begin_link_read_set(&self) -> Result<(), StoreError> {
        let mut recorded = self.link_read_set.borrow_mut();
        if recorded.is_some() {
            return Err(StoreError::new(
                StoreErrorClass::Corrupt,
                "publication_link_read_scope",
                "link read-set recording is already active",
            ));
        }
        *recorded = Some(LinkReadSet::default());
        Ok(())
    }

    pub(super) fn take_link_read_set(&self) -> BTreeSet<ObjectKey> {
        self.link_read_set
            .borrow_mut()
            .take()
            .map(|set| set.objects)
            .unwrap_or_default()
    }

    fn record_link_read(&self, key: ObjectKey) -> Result<(), StoreError> {
        let mut recorded = self.link_read_set.borrow_mut();
        let Some(recorded) = recorded.as_mut() else {
            return Ok(());
        };
        let visits = recorded
            .visits
            .checked_add(1)
            .filter(|visits| {
                *visits <= crate::platform::package_transport::source::MAXIMUM_VALIDATION_VISITS
            })
            .ok_or_else(|| {
                StoreError::new(
                    StoreErrorClass::Resource,
                    "publication_link_read_set_work",
                    "link read-set recording exceeds the source validation visit bound",
                )
            })?;
        recorded.visits = visits;
        if !recorded.objects.contains(&key) {
            let bytes = recorded
                .modeled_bytes
                .checked_add(std::mem::size_of::<ObjectKey>() + 3 * std::mem::size_of::<usize>())
                .filter(|bytes| {
                    *bytes <= crate::platform::package_transport::source::MAXIMUM_CONTAINER_BYTES
                })
                .ok_or_else(|| {
                    StoreError::new(
                        StoreErrorClass::Resource,
                        "publication_link_read_set_bytes",
                        "link read-set metadata exceeds the source container byte bound",
                    )
                })?;
            if recorded.objects.len()
                >= crate::platform::package_transport::source::MAXIMUM_CONTAINER_OBJECTS
            {
                return Err(StoreError::new(
                    StoreErrorClass::Resource,
                    "publication_link_read_set_objects",
                    "link read-set metadata exceeds the source container object bound",
                ));
            }
            // Reserve modeled key/node growth before inserting. Separate from the
            // current-validation rebuild tracker and disabled before final recheck.
            recorded.modeled_bytes = bytes;
            recorded.objects.insert(key);
        }
        Ok(())
    }

    pub fn bound_current_rebuild(&self, bounded: bool) {
        self.rebuild_reads.set(if bounded {
            *self.rebuild_objects.borrow_mut() = Some(BTreeSet::new());
            StoreReadAdmission::new(REBUILD_LIMITS)
        } else {
            StoreReadAdmission::unbounded()
        });
    }

    pub fn take_rebuild_objects(&self) -> BTreeSet<ObjectKey> {
        self.rebuild_objects.borrow_mut().take().unwrap_or_default()
    }

    pub fn rebuild_work(&self) -> StoreWork {
        self.rebuild_reads.get().observation().as_store_work()
    }

    pub fn install(&mut self, validation: &RevalidatedBase) -> Result<(), Diagnostic> {
        for (digest, bytes) in &validation.facts.summary_objects {
            self.insert(
                ObjectKey::from_digest(ObjectDomain::OwnerSummary, digest.bytes()),
                bytes,
            )?;
        }
        for (digest, bytes) in validation.facts.pages.objects() {
            self.insert(
                ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes()),
                bytes,
            )?;
        }
        if let Ok(current) = &validation.current {
            self.insert(
                ObjectKey::from_digest(ObjectDomain::ValidationWitness, current.digest.bytes()),
                &current.bytes,
            )?;
        }
        Ok(())
    }

    fn insert(&mut self, key: ObjectKey, bytes: &[u8]) -> Result<(), Diagnostic> {
        key.verify(bytes).map_err(|error| {
            Diagnostic::new(DiagnosticClass::Corrupt, error.code, error.message)
        })?;
        self.derived.insert(key, bytes.to_vec());
        Ok(())
    }

    pub(super) fn install_candidate(
        &mut self,
        objects: &BTreeMap<ObjectKey, Vec<u8>>,
    ) -> Result<(), Diagnostic> {
        let bytes = objects.values().try_fold(0_usize, |total, bytes| {
            total.checked_add(bytes.len())?.checked_add(192)
        });
        if bytes.is_none_or(|bytes| bytes > 64 * 1_048_576) {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "transaction_preview_capacity",
                "candidate preview exceeds its 64 MiB staged-object admission",
            ));
        }
        for (key, bytes) in objects {
            self.insert(*key, bytes)?;
        }
        Ok(())
    }
}

impl std::ops::Deref for ViewStore {
    type Target = StorageBackend;
    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl ImmutableObjectStore for ViewStore {
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
        let caller = *admission;
        let rebuild = self.rebuild_reads.get();
        let mut joint = StoreReadAdmission::new(caller.remaining().intersect(rebuild.remaining()));
        let result = self.read_joint(key, maximum_bytes, &mut joint, work);
        if matches!(&result, Ok(Some(_)))
            && let Some(objects) = self.rebuild_objects.borrow_mut().as_mut()
        {
            objects.insert(key);
        }
        let next_caller = caller.charged_from(&joint)?;
        let next_rebuild = rebuild.charged_from(&joint)?;
        *admission = next_caller;
        self.rebuild_reads.set(next_rebuild);
        if matches!(&result, Ok(Some(_))) {
            self.record_link_read(key)?;
        }
        result
    }

    fn contains_admitted(
        &self,
        key: ObjectKey,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<bool, StoreError> {
        Ok(self
            .read_admitted(key, key.domain.maximum_bytes(), admission, work)?
            .is_some())
    }

    fn stage(
        &mut self,
        _key: ObjectKey,
        _bytes: &[u8],
        _work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        Err(StoreError::new(
            StoreErrorClass::Input,
            "publication_validation_read_only",
            "current validation is read-only derived context",
        ))
    }
}

impl ViewStore {
    fn read_joint(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        // Rebuilt facts do not replace equal-key physical authority. Re-read that authority so
        // later disk corruption cannot be hidden by an immutable in-process reconstruction.
        if let Some(bytes) = self
            .base
            .read_admitted(key, maximum_bytes, admission, work)?
        {
            return Ok(Some(bytes));
        }
        if let Some(bytes) = self.derived.get(&key) {
            admission.admit_catalog_lookup()?;
            admission.admit_object(bytes.len())?;
            if bytes.len() > maximum_bytes {
                return Err(StoreError::new(
                    StoreErrorClass::Resource,
                    "publication_validation_object_size",
                    "derived validation object exceeds read admission",
                ));
            }
            key.verify(bytes)?;
            work.add(crate::platform::storage::object::StoreWork { catalog_lookups: 1, ..crate::platform::storage::object::StoreWork::default() });
            work.add(crate::platform::storage::object::StoreWork { objects_read: 1, ..crate::platform::storage::object::StoreWork::default() });
            work.add(crate::platform::storage::object::StoreWork { bytes_read: bytes.len() as u64, ..crate::platform::storage::object::StoreWork::default() });
            Ok(Some(bytes.clone()))
        } else {
            Ok(None)
        }
    }
}
