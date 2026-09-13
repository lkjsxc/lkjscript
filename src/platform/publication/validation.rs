//! Current semantic admission is derived from, and never replaces, historical acceptance.

use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{FullValidationReport, KernelSnapshot};
use crate::platform::storage::directory::PackDirectoryStore;
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StageOutcome, StoreError, StoreErrorClass,
    StoreReadAdmission, StoreReadLimits, StoreWork,
};
use crate::platform::witness::{
    CanonicalWitnessFacts, ValidationWitnessDigest, ValidationWitnessManifest,
    bind_witness_manifest, rebuild_canonical_facts,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, OnceLock};

// A process can open the same immutable revision several times during one command. Reuse only
// proof this validator actually constructed, never an on-disk assertion. One bounded retained
// context is enough for those views; a new process performs fresh current admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CurrentValidationKey {
    pub root: crate::platform::kernel::SemanticRootDigest,
    pub revision: crate::platform::semantic_id::RevisionId,
    pub validator: crate::platform::witness::ValidatorContractDigest,
}
type ValidationCache = Option<(CurrentValidationKey, Arc<[ObjectKey]>, Arc<RevalidatedBase>)>;
static CURRENT_CONTEXT: OnceLock<Mutex<ValidationCache>> = OnceLock::new();

pub(super) fn cached(
    key: CurrentValidationKey,
    store: &ViewStore,
    checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
) -> Result<Option<(Arc<RevalidatedBase>, StoreWork)>, Diagnostic> {
    let entry = CURRENT_CONTEXT
        .get()
        .and_then(|cache| cache.lock().ok())
        .and_then(|cache| {
            cache
                .as_ref()
                .filter(|(binding, _, _)| *binding == key)
                .map(|(_, objects, context)| (objects.clone(), context.clone()))
        });
    let Some((objects, context)) = entry else {
        return Ok(None);
    };
    let mut work = StoreWork::default();
    // A cached proof never substitutes for the canonical bytes in this physical repository.
    // Recheck its complete original read set, including exact dependency objects, before reuse.
    let mut admission = StoreReadAdmission::new(REBUILD_LIMITS);
    for key in objects.iter() {
        checkpoint()?;
        let bytes = store
            .read_admitted(*key, key.domain.maximum_bytes(), &mut admission, &mut work)
            .map_err(|error| {
                Diagnostic::new(
                    match error.class {
                        StoreErrorClass::Resource => DiagnosticClass::Resource,
                        _ => DiagnosticClass::Corrupt,
                    },
                    error.code,
                    error.message,
                )
            })?
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Corrupt,
                    "publication_current_validation_object_missing",
                    "cached current validation references a missing canonical or dependency object",
                )
            })?;
        key.verify(&bytes).map_err(|error| {
            Diagnostic::new(DiagnosticClass::Corrupt, error.code, error.message)
        })?;
    }
    Ok(Some((context, work)))
}

pub(super) fn retain(
    key: CurrentValidationKey,
    read_bytes: u64,
    objects: BTreeSet<ObjectKey>,
    context: Arc<RevalidatedBase>,
) {
    if context.current.as_ref().err().is_some_and(|errors| {
        errors
            .iter()
            .any(|error| error.class != DiagnosticClass::Semantic)
    }) {
        return;
    }
    let derived_bytes = context
        .facts
        .summary_objects
        .values()
        .map(Vec::len)
        .chain(context.facts.pages.objects().map(|(_, bytes)| bytes.len()))
        .try_fold(0_u64, |total, bytes| total.checked_add(bytes as u64));
    let estimate = derived_bytes
        .and_then(|bytes| bytes.checked_add(read_bytes))
        .and_then(|bytes| bytes.checked_mul(8))
        .and_then(|bytes| {
            bytes.checked_add((context.snapshot.owners.len() as u64).checked_mul(256)?)
        });
    if let Ok(mut cache) = CURRENT_CONTEXT.get_or_init(|| Mutex::new(None)).lock() {
        *cache = estimate.filter(|bytes| *bytes <= 32 * 1_048_576).map(|_| {
            (
                key,
                Arc::from(objects.into_iter().collect::<Vec<_>>()),
                context,
            )
        });
    }
}

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
}

impl RevalidatedBase {
    pub fn rebuild(
        snapshot: KernelSnapshot,
        checkpoint: &dyn Fn() -> Result<(), Diagnostic>,
    ) -> Result<Self, Diagnostic> {
        checkpoint()?;
        let facts = rebuild_canonical_facts(&snapshot)?;
        let mut semantic_work = 0;
        let current = crate::platform::kernel::validate_full_checked(
            &snapshot,
            crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK,
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
            semantic_work,
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
    base: PackDirectoryStore,
    derived: BTreeMap<ObjectKey, Vec<u8>>,
    rebuild_reads: std::cell::Cell<StoreReadAdmission>,
    rebuild_objects: std::cell::RefCell<Option<BTreeSet<ObjectKey>>>,
}

const REBUILD_LIMITS: StoreReadLimits = StoreReadLimits {
    maximum_catalog_lookups: 2_000_000,
    maximum_objects: 1_000_000,
    maximum_bytes: 256 * 1_048_576,
};

impl ViewStore {
    pub const fn new(base: PackDirectoryStore) -> Self {
        Self {
            base,
            derived: BTreeMap::new(),
            rebuild_reads: std::cell::Cell::new(StoreReadAdmission::unbounded()),
            rebuild_objects: std::cell::RefCell::new(None),
        }
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
        let remaining = self.rebuild_reads.get().remaining();
        StoreWork {
            catalog_lookups: REBUILD_LIMITS.maximum_catalog_lookups
                - remaining.maximum_catalog_lookups,
            objects_read: REBUILD_LIMITS.maximum_objects - remaining.maximum_objects,
            bytes_read: REBUILD_LIMITS.maximum_bytes - remaining.maximum_bytes,
            ..StoreWork::default()
        }
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
}

impl std::ops::Deref for ViewStore {
    type Target = PackDirectoryStore;
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
        let caller = admission.remaining();
        let rebuild = self.rebuild_reads.get().remaining();
        let allowed = StoreReadLimits {
            maximum_catalog_lookups: caller
                .maximum_catalog_lookups
                .min(rebuild.maximum_catalog_lookups),
            maximum_objects: caller.maximum_objects.min(rebuild.maximum_objects),
            maximum_bytes: caller.maximum_bytes.min(rebuild.maximum_bytes),
        };
        let mut joint = StoreReadAdmission::new(allowed);
        let result = self.read_joint(key, maximum_bytes, &mut joint, work);
        if matches!(&result, Ok(Some(_)))
            && let Some(objects) = self.rebuild_objects.borrow_mut().as_mut()
        {
            objects.insert(key);
        }
        let remaining = joint.remaining();
        let charge = |original: StoreReadLimits| {
            StoreReadAdmission::new(StoreReadLimits {
                maximum_catalog_lookups: original.maximum_catalog_lookups
                    - (allowed.maximum_catalog_lookups - remaining.maximum_catalog_lookups),
                maximum_objects: original.maximum_objects
                    - (allowed.maximum_objects - remaining.maximum_objects),
                maximum_bytes: original.maximum_bytes
                    - (allowed.maximum_bytes - remaining.maximum_bytes),
            })
        };
        *admission = charge(caller);
        self.rebuild_reads.set(charge(rebuild));
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
            work.catalog_lookups = work.catalog_lookups.saturating_add(1);
            work.objects_read = work.objects_read.saturating_add(1);
            work.bytes_read = work.bytes_read.saturating_add(bytes.len() as u64);
            Ok(Some(bytes.clone()))
        } else {
            self.base.read_admitted(key, maximum_bytes, admission, work)
        }
    }
}
