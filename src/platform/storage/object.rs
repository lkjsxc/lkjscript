//! Typed immutable object identities and narrow store interface.

use super::contract;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ObjectDomain {
    Owner,
    Type,
    Blob,
    Sequence,
    MapPage,
    SemanticRoot,
    ValidationWitness,
    OwnerSummary,
    Revision,
    Receipt,
    Draft,
    Conflict,
    CompilerUnit,
    ArtifactManifest,
    PackageRevision,
    PackageTransport,
    BackupManifest,
    BackupSegment,
    Dependency,
    Retirement,
    Change,
    Transaction,
    SemanticDiff,
    PackageInterface,
    CompilationManifest,
}

impl ObjectDomain {
    pub const ALL: [Self; 25] = [
        Self::Owner,
        Self::Type,
        Self::Blob,
        Self::Sequence,
        Self::MapPage,
        Self::SemanticRoot,
        Self::ValidationWitness,
        Self::OwnerSummary,
        Self::Revision,
        Self::Receipt,
        Self::Draft,
        Self::Conflict,
        Self::CompilerUnit,
        Self::ArtifactManifest,
        Self::PackageRevision,
        Self::PackageTransport,
        Self::BackupManifest,
        Self::BackupSegment,
        Self::Dependency,
        Self::Retirement,
        Self::Change,
        Self::Transaction,
        Self::SemanticDiff,
        Self::PackageInterface,
        Self::CompilationManifest,
    ];

    pub const fn tag(self) -> u8 {
        match self {
            Self::Owner => 1,
            Self::Type => 2,
            Self::Blob => 3,
            Self::Sequence => 4,
            Self::MapPage => 5,
            Self::SemanticRoot => 6,
            Self::ValidationWitness => 7,
            Self::OwnerSummary => 8,
            Self::Revision => 9,
            Self::Receipt => 10,
            Self::Draft => 11,
            Self::Conflict => 12,
            Self::CompilerUnit => 13,
            Self::ArtifactManifest => 14,
            Self::PackageRevision => 25,
            Self::PackageTransport => 26,
            Self::BackupManifest => 16,
            Self::BackupSegment => 17,
            Self::Dependency => 18,
            Self::Retirement => 19,
            Self::Change => 20,
            Self::Transaction => 21,
            Self::SemanticDiff => 22,
            Self::PackageInterface => 23,
            Self::CompilationManifest => 24,
        }
    }

    pub fn from_tag(tag: u8) -> Result<Self, StoreError> {
        Self::ALL
            .into_iter()
            .find(|domain| domain.tag() == tag)
            .ok_or_else(|| {
                StoreError::new(
                    StoreErrorClass::Corrupt,
                    "object_domain_tag",
                    format!("unknown immutable object domain tag {tag}"),
                )
            })
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Type => "type",
            Self::Blob => "blob",
            Self::Sequence => "sequence",
            Self::MapPage => "map_page",
            Self::SemanticRoot => "semantic_root",
            Self::ValidationWitness => "validation_witness",
            Self::OwnerSummary => "owner_summary",
            Self::Revision => "revision",
            Self::Receipt => "receipt",
            Self::Draft => "draft",
            Self::Conflict => "conflict",
            Self::CompilerUnit => "compiler_unit",
            Self::ArtifactManifest => "artifact_manifest",
            Self::PackageRevision => "package_revision",
            Self::PackageTransport => "package_transport",
            Self::BackupManifest => "backup_manifest",
            Self::BackupSegment => "backup_segment",
            Self::Dependency => "dependency",
            Self::Retirement => "retirement",
            Self::Change => "change",
            Self::Transaction => "transaction",
            Self::SemanticDiff => "semantic_diff",
            Self::PackageInterface => "package_interface",
            Self::CompilationManifest => "compilation_manifest",
        }
    }

    pub const fn digest_domain(self) -> &'static str {
        match self {
            Self::Owner => contract::OWNER_OBJECT_DIGEST_DOMAIN,
            Self::Type => contract::TYPE_OBJECT_DIGEST_DOMAIN,
            Self::Blob => contract::BLOB_OBJECT_DIGEST_DOMAIN,
            Self::Sequence => contract::SEQUENCE_OBJECT_DIGEST_DOMAIN,
            Self::MapPage => contract::MAP_PAGE_DIGEST_DOMAIN,
            Self::SemanticRoot => contract::SEMANTIC_ROOT_DIGEST_DOMAIN,
            Self::ValidationWitness => contract::VALIDATION_WITNESS_DIGEST_DOMAIN,
            Self::OwnerSummary => contract::OWNER_SUMMARY_DIGEST_DOMAIN,
            Self::Revision => contract::REVISION_OBJECT_DIGEST_DOMAIN,
            Self::Receipt => contract::RECEIPT_OBJECT_DIGEST_DOMAIN,
            Self::Draft => contract::DRAFT_OBJECT_DIGEST_DOMAIN,
            Self::Conflict => contract::CONFLICT_OBJECT_DIGEST_DOMAIN,
            Self::CompilerUnit => contract::COMPILER_UNIT_DIGEST_DOMAIN,
            Self::ArtifactManifest => contract::ARTIFACT_MANIFEST_DIGEST_DOMAIN,
            Self::PackageRevision => contract::PACKAGE_REVISION_DIGEST_DOMAIN,
            Self::PackageTransport => contract::PACKAGE_TRANSPORT_DIGEST_DOMAIN,
            Self::BackupManifest => contract::BACKUP_MANIFEST_DIGEST_DOMAIN,
            Self::BackupSegment => contract::BACKUP_SEGMENT_DIGEST_DOMAIN,
            Self::Dependency => contract::DEPENDENCY_OBJECT_DIGEST_DOMAIN,
            Self::Retirement => contract::RETIREMENT_OBJECT_DIGEST_DOMAIN,
            Self::Change => contract::CHANGE_DIGEST_DOMAIN,
            Self::Transaction => contract::TRANSACTION_OBJECT_DIGEST_DOMAIN,
            Self::SemanticDiff => contract::SEMANTIC_DIFF_OBJECT_DIGEST_DOMAIN,
            Self::PackageInterface => contract::PACKAGE_INTERFACE_OWNER_DIGEST_DOMAIN,
            Self::CompilationManifest => contract::COMPILATION_MANIFEST_DIGEST_DOMAIN,
        }
    }

    /// Current object-format boundary, separate from a caller's optional read policy.
    pub const fn maximum_bytes(self) -> Option<usize> {
        match self {
            Self::Transaction | Self::SemanticDiff => None,
            Self::MapPage => Some(crate::platform::persistent_map::MAXIMUM_PAGE_BYTES),
            Self::SemanticRoot
            | Self::Revision
            | Self::Receipt
            | Self::Retirement
            | Self::CompilationManifest => Some(64 * 1024),
            Self::Type | Self::OwnerSummary | Self::Dependency | Self::PackageInterface => {
                Some(1024 * 1024)
            }
            Self::Owner
            | Self::Sequence
            | Self::ValidationWitness
            | Self::Draft
            | Self::Conflict
            | Self::CompilerUnit
            | Self::ArtifactManifest
            | Self::PackageRevision
            | Self::PackageTransport
            | Self::BackupManifest
            | Self::Change => Some(4 * 1024 * 1024),
            Self::Blob | Self::BackupSegment => Some(64 * 1024 * 1024),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectDigest([u8; 32]);

impl ObjectDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ObjectDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("object_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObjectKey {
    pub domain: ObjectDomain,
    pub digest: ObjectDigest,
}

impl ObjectKey {
    pub fn for_bytes(domain: ObjectDomain, bytes: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new_derive_key(domain.digest_domain());
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
        Self {
            domain,
            digest: ObjectDigest::from_bytes(*hasher.finalize().as_bytes()),
        }
    }

    pub const fn from_digest(domain: ObjectDomain, digest: [u8; 32]) -> Self {
        Self {
            domain,
            digest: ObjectDigest::from_bytes(digest),
        }
    }

    pub fn verify(self, bytes: &[u8]) -> Result<(), StoreError> {
        if let Some(maximum) = self.domain.maximum_bytes()
            && bytes.len() > maximum
        {
            return Err(StoreError::new(
                StoreErrorClass::Resource,
                "object_domain_size",
                format!(
                    "{} object has {} bytes; maximum is {}",
                    self.domain.name(),
                    bytes.len(),
                    maximum
                ),
            ));
        }
        if Self::for_bytes(self.domain, bytes) != self {
            return Err(StoreError::new(
                StoreErrorClass::Corrupt,
                "object_digest_mismatch",
                "immutable object bytes do not match their domain-separated digest",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StageOutcome {
    Inserted,
    Reused,
}

/// One independent immutable-store observation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum StoreWorkCounter {
    CatalogLookups,
    PacksOpened,
    ObjectsRead,
    ObjectsStaged,
    ObjectsReused,
    BytesRead,
    BytesStaged,
    PagesStaged,
    PacksSealed,
}

impl StoreWorkCounter {
    const fn bit(self) -> u16 {
        1_u16 << self as u8
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StoreWork {
    pub catalog_lookups: u64,
    pub packs_opened: u64,
    pub objects_read: u64,
    pub objects_staged: u64,
    pub objects_reused: u64,
    pub bytes_read: u64,
    pub bytes_staged: u64,
    pub pages_staged: u64,
    pub packs_sealed: u64,
    /// At least one observation is a saturated lower bound.
    pub saturated: bool,
    /// Individual lower-bound flags; aggregate `saturated` is retained for summary consumers.
    pub saturated_counters: u16,
}

impl StoreWork {
    pub const fn counter_saturated(self, counter: StoreWorkCounter) -> bool {
        self.saturated_counters & counter.bit() != 0
    }

    pub fn add(&mut self, other: Self) {
        self.saturated |= other.saturated;
        self.saturated_counters |= other.saturated_counters;
        for (counter, additional, field) in [
            (
                &mut self.catalog_lookups,
                other.catalog_lookups,
                StoreWorkCounter::CatalogLookups,
            ),
            (
                &mut self.packs_opened,
                other.packs_opened,
                StoreWorkCounter::PacksOpened,
            ),
            (
                &mut self.objects_read,
                other.objects_read,
                StoreWorkCounter::ObjectsRead,
            ),
            (
                &mut self.objects_staged,
                other.objects_staged,
                StoreWorkCounter::ObjectsStaged,
            ),
            (
                &mut self.objects_reused,
                other.objects_reused,
                StoreWorkCounter::ObjectsReused,
            ),
            (
                &mut self.bytes_read,
                other.bytes_read,
                StoreWorkCounter::BytesRead,
            ),
            (
                &mut self.bytes_staged,
                other.bytes_staged,
                StoreWorkCounter::BytesStaged,
            ),
            (
                &mut self.pages_staged,
                other.pages_staged,
                StoreWorkCounter::PagesStaged,
            ),
            (
                &mut self.packs_sealed,
                other.packs_sealed,
                StoreWorkCounter::PacksSealed,
            ),
        ] {
            match counter.checked_add(additional) {
                Some(value) => *counter = value,
                None => {
                    *counter = u64::MAX;
                    self.saturated = true;
                    self.saturated_counters |= field.bit();
                }
            }
        }
    }
}

/// Exact optional allowances for immutable-object reads at one owning boundary.
/// `None` is unbounded; `Some(0)` permits no work; every finite value is exact.
/// Present objects consume object/byte allowances; missing lookups consume only a
/// catalog lookup. Payload lengths are admitted before opening, copying or hashing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StoreReadLimits {
    pub maximum_catalog_lookups: Option<u64>,
    pub maximum_objects: Option<u64>,
    pub maximum_bytes: Option<u64>,
}

impl StoreReadLimits {
    pub const fn unbounded() -> Self {
        Self {
            maximum_catalog_lookups: None,
            maximum_objects: None,
            maximum_bytes: None,
        }
    }

    /// Intersect permissions without treating the absence of a quota as zero.
    pub fn intersect(self, other: Self) -> Self {
        let intersect = |left: Option<u64>, right: Option<u64>| match (left, right) {
            (Some(left), Some(right)) => Some(left.min(right)),
            (None, right) => right,
            (left, None) => left,
        };
        Self {
            maximum_catalog_lookups: intersect(
                self.maximum_catalog_lookups,
                other.maximum_catalog_lookups,
            ),
            maximum_objects: intersect(self.maximum_objects, other.maximum_objects),
            maximum_bytes: intersect(self.maximum_bytes, other.maximum_bytes),
        }
    }
}

/// Passive observations of reservations, distinct from completed store reads.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StoreReadObservation {
    pub catalog_lookups: u64,
    pub objects: u64,
    pub bytes: u64,
    /// At least one counter is a lower bound rather than an exact observation.
    pub saturated: bool,
    pub saturated_counters: u16,
}

impl StoreReadObservation {
    pub const fn counter_saturated(self, counter: StoreWorkCounter) -> bool {
        self.saturated_counters & counter.bit() != 0
    }

    pub fn as_store_work(self) -> StoreWork {
        StoreWork {
            catalog_lookups: self.catalog_lookups,
            objects_read: self.objects,
            bytes_read: self.bytes,
            saturated: self.saturated,
            saturated_counters: self.saturated_counters,
            ..StoreWork::default()
        }
    }

    fn add(&mut self, other: Self) {
        self.saturated |= other.saturated;
        self.saturated_counters |= other.saturated_counters;
        for (counter, additional, field) in [
            (
                &mut self.catalog_lookups,
                other.catalog_lookups,
                StoreWorkCounter::CatalogLookups,
            ),
            (
                &mut self.objects,
                other.objects,
                StoreWorkCounter::ObjectsRead,
            ),
            (&mut self.bytes, other.bytes, StoreWorkCounter::BytesRead),
        ] {
            match counter.checked_add(additional) {
                Some(value) => *counter = value,
                None => {
                    *counter = u64::MAX;
                    self.saturated = true;
                    self.saturated_counters |= field.bit();
                }
            }
        }
    }
}

/// Mutable pre-consumption admission with exact finite remaining allowances.
/// Observation saturation never creates additional permission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoreReadAdmission {
    initial: StoreReadLimits,
    remaining: StoreReadLimits,
    observation: StoreReadObservation,
}

impl StoreReadAdmission {
    pub const fn new(limits: StoreReadLimits) -> Self {
        Self {
            initial: limits,
            remaining: limits,
            observation: StoreReadObservation {
                catalog_lookups: 0,
                objects: 0,
                bytes: 0,
                saturated: false,
                saturated_counters: 0,
            },
        }
    }

    pub const fn unbounded() -> Self {
        Self::new(StoreReadLimits::unbounded())
    }

    pub const fn remaining(&self) -> StoreReadLimits {
        self.remaining
    }

    pub const fn observation(&self) -> StoreReadObservation {
        self.observation
    }

    /// Carry a jointly admitted operation back to each enclosing policy, including
    /// reservations made before a rejected read. Finite consumption is derived from
    /// checked remaining permission, never from a possibly saturated observation.
    pub fn charged_from(mut self, joint: &Self) -> Result<Self, StoreError> {
        let charge = |current: Option<u64>, initial: Option<u64>, remaining: Option<u64>| match (
            current, initial, remaining,
        ) {
            (None, _, _) => Ok(None),
            (Some(current), Some(initial), Some(remaining)) => initial
                .checked_sub(remaining)
                .and_then(|used| current.checked_sub(used))
                .map(Some)
                .ok_or_else(read_admission_accounting),
            _ => Err(read_admission_accounting()),
        };
        let remaining = StoreReadLimits {
            maximum_catalog_lookups: charge(
                self.remaining.maximum_catalog_lookups,
                joint.initial.maximum_catalog_lookups,
                joint.remaining.maximum_catalog_lookups,
            )?,
            maximum_objects: charge(
                self.remaining.maximum_objects,
                joint.initial.maximum_objects,
                joint.remaining.maximum_objects,
            )?,
            maximum_bytes: charge(
                self.remaining.maximum_bytes,
                joint.initial.maximum_bytes,
                joint.remaining.maximum_bytes,
            )?,
        };
        self.remaining = remaining;
        self.observation.add(joint.observation);
        Ok(self)
    }

    pub fn admit_catalog_lookup(&mut self) -> Result<(), StoreError> {
        let remaining = read_remaining(
            self.remaining.maximum_catalog_lookups,
            1,
            "object_read_catalog_lookups_exhausted",
            "catalog lookups",
        )?;
        self.remaining.maximum_catalog_lookups = remaining;
        self.observation.add(StoreReadObservation {
            catalog_lookups: 1,
            ..StoreReadObservation::default()
        });
        Ok(())
    }

    pub fn admit_object(&mut self, byte_count: usize) -> Result<(), StoreError> {
        let byte_count = u64::try_from(byte_count).map_err(|_| {
            StoreError::new(
                StoreErrorClass::Resource,
                "object_read_byte_count",
                "immutable object byte count cannot be represented by the read admission",
            )
        })?;
        self.admit_object_bytes(byte_count)
    }

    pub fn admit_object_bytes(&mut self, byte_count: u64) -> Result<(), StoreError> {
        let objects = read_remaining(
            self.remaining.maximum_objects,
            1,
            "object_read_objects_exhausted",
            "objects",
        )?;
        let bytes = read_remaining(
            self.remaining.maximum_bytes,
            byte_count,
            "object_read_bytes_exhausted",
            "bytes",
        )?;
        self.remaining.maximum_objects = objects;
        self.remaining.maximum_bytes = bytes;
        self.observation.add(StoreReadObservation {
            objects: 1,
            bytes: byte_count,
            ..StoreReadObservation::default()
        });
        Ok(())
    }
}

fn read_remaining(
    remaining: Option<u64>,
    required: u64,
    code: &'static str,
    unit: &'static str,
) -> Result<Option<u64>, StoreError> {
    remaining
        .map(|remaining| {
            remaining
                .checked_sub(required)
                .ok_or_else(|| read_exhausted(code, unit, required, remaining))
        })
        .transpose()
}

fn read_admission_accounting() -> StoreError {
    StoreError::new(
        StoreErrorClass::Corrupt,
        "object_read_admission_accounting",
        "joint read admission does not preserve the enclosing finite allowance",
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectStageLimits {
    pub maximum_objects: Option<u64>,
    pub maximum_bytes: Option<u64>,
    pub maximum_pages: Option<u64>,
}

pub trait ImmutableObjectStore {
    fn read(
        &self,
        key: ObjectKey,
        maximum_bytes: Option<usize>,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError>;

    /// Reads through an aggregate pre-consumption admission boundary.
    ///
    /// Implementations must override this method when they can expose admitted reads. The default
    /// fails before invoking [`Self::read`], so a generic store can never silently turn a
    /// retrospective observation into an admission claim.
    fn read_admitted(
        &self,
        _key: ObjectKey,
        _maximum_bytes: Option<usize>,
        _admission: &mut StoreReadAdmission,
        _work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        Err(read_admission_unsupported())
    }

    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        Ok(self.read(key, key.domain.maximum_bytes(), work)?.is_some())
    }

    /// Checks presence while admitting its catalog access before inspection.
    fn contains_admitted(
        &self,
        _key: ObjectKey,
        _admission: &mut StoreReadAdmission,
        _work: &mut StoreWork,
    ) -> Result<bool, StoreError> {
        Err(read_admission_unsupported())
    }

    fn stage(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError>;

    /// Stages bytes while admitting any immutable-base read used for deduplication.
    fn stage_admitted(
        &mut self,
        _key: ObjectKey,
        _bytes: &[u8],
        _admission: &mut StoreReadAdmission,
        _work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        Err(read_admission_unsupported())
    }
}

/// Private read-through stage for one prepared publication. Reads observe staged objects first and
/// then the exact accepted base; writes remain isolated in memory until a publication owner moves
/// them into durable immutable packs.
pub struct ObjectStage<'a, S: ImmutableObjectStore + ?Sized> {
    base: &'a S,
    objects: BTreeMap<ObjectKey, Vec<u8>>,
    limits: Option<ObjectStageLimits>,
    read_admission: Option<Cell<StoreReadAdmission>>,
    staged_bytes: u64,
    staged_pages: u64,
}

impl<'a, S: ImmutableObjectStore + ?Sized> ObjectStage<'a, S> {
    pub const fn new(base: &'a S) -> Self {
        Self {
            base,
            objects: BTreeMap::new(),
            limits: None,
            read_admission: None,
            staged_bytes: 0,
            staged_pages: 0,
        }
    }

    pub const fn with_limits(base: &'a S, limits: ObjectStageLimits) -> Self {
        Self {
            base,
            objects: BTreeMap::new(),
            limits: Some(limits),
            read_admission: None,
            staged_bytes: 0,
            staged_pages: 0,
        }
    }

    /// Creates a request-local stage whose ordinary store operations share one accepted-base
    /// read admission.
    pub const fn with_limits_and_read_admission(
        base: &'a S,
        limits: ObjectStageLimits,
        read_admission: StoreReadAdmission,
    ) -> Self {
        Self {
            base,
            objects: BTreeMap::new(),
            limits: Some(limits),
            read_admission: Some(Cell::new(read_admission)),
            staged_bytes: 0,
            staged_pages: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }

    pub fn stored_bytes(&self) -> usize {
        usize::try_from(self.staged_bytes).unwrap_or(usize::MAX)
    }

    pub const fn staged_byte_count(&self) -> u64 {
        self.staged_bytes
    }

    pub const fn staged_page_count(&self) -> u64 {
        self.staged_pages
    }

    pub fn remaining_read_admission(&self) -> Option<StoreReadLimits> {
        self.read_admission
            .as_ref()
            .map(|admission| admission.get().remaining())
    }

    pub fn objects(&self) -> impl Iterator<Item = (ObjectKey, &[u8])> {
        self.objects
            .iter()
            .map(|(key, bytes)| (*key, bytes.as_slice()))
    }

    pub fn into_objects(self) -> BTreeMap<ObjectKey, Vec<u8>> {
        self.objects
    }

    fn read_inner(
        &self,
        key: ObjectKey,
        maximum_bytes: Option<usize>,
        admission: Option<&mut StoreReadAdmission>,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        if let Some(bytes) = self.objects.get(&key) {
            if maximum_bytes.is_some_and(|maximum| bytes.len() > maximum) {
                return Err(StoreError::new(
                    StoreErrorClass::Resource,
                    "object_stage_read_limit",
                    "staged object exceeds the caller read bound",
                ));
            }
            if let Some(admission) = admission {
                admission.admit_object(bytes.len())?;
            }
            key.verify(bytes)?;
            work.add(crate::platform::storage::object::StoreWork {
                objects_read: 1,
                ..crate::platform::storage::object::StoreWork::default()
            });
            work.add(crate::platform::storage::object::StoreWork {
                bytes_read: bytes.len() as u64,
                ..crate::platform::storage::object::StoreWork::default()
            });
            return Ok(Some(copy_object_bytes(bytes)?));
        }
        self.read_base(key, maximum_bytes, admission, work)
    }

    fn contains_inner(
        &self,
        key: ObjectKey,
        admission: Option<&mut StoreReadAdmission>,
        work: &mut StoreWork,
    ) -> Result<bool, StoreError> {
        if self.objects.contains_key(&key) {
            return Ok(true);
        }
        self.contains_base(key, admission, work)
    }

    fn stage_inner(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        admission: Option<&mut StoreReadAdmission>,
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        key.verify(bytes)?;
        if let Some(existing) = self.objects.get(&key) {
            if existing != bytes {
                return Err(StoreError::new(
                    StoreErrorClass::Corrupt,
                    "object_stage_collision",
                    "one staged immutable object identity is bound to different bytes",
                ));
            }
            work.add(crate::platform::storage::object::StoreWork {
                objects_reused: 1,
                ..crate::platform::storage::object::StoreWork::default()
            });
            return Ok(StageOutcome::Reused);
        }
        let existing = self.read_base(key, key.domain.maximum_bytes(), admission, work)?;
        if let Some(existing) = existing {
            if existing != bytes {
                return Err(StoreError::new(
                    StoreErrorClass::Corrupt,
                    "object_stage_base_collision",
                    "accepted storage binds one immutable object identity to different bytes",
                ));
            }
            work.add(crate::platform::storage::object::StoreWork {
                objects_reused: 1,
                ..crate::platform::storage::object::StoreWork::default()
            });
            return Ok(StageOutcome::Reused);
        }
        let byte_count = u64::try_from(bytes.len()).map_err(|_| {
            StoreError::new(
                StoreErrorClass::Resource,
                "change_budget_staged_bytes",
                "staged object byte count cannot be represented by the staging budget",
            )
        })?;
        let staged_objects = u64::try_from(self.objects.len()).unwrap_or(u64::MAX);
        let admitted_objects = staged_objects.checked_add(1).ok_or_else(|| {
            StoreError::new(
                StoreErrorClass::Resource,
                "change_budget_staged_objects",
                "staged object observation overflowed",
            )
        })?;
        let admitted_bytes = self.staged_bytes.checked_add(byte_count).ok_or_else(|| {
            StoreError::new(
                StoreErrorClass::Resource,
                "change_budget_staged_bytes",
                "staged byte observation overflowed",
            )
        })?;
        let admitted_pages = self
            .staged_pages
            .checked_add(u64::from(key.domain == ObjectDomain::MapPage))
            .ok_or_else(|| {
                StoreError::new(
                    StoreErrorClass::Resource,
                    "change_budget_staged_pages",
                    "staged page observation overflowed",
                )
            })?;
        if let Some(limits) = self.limits {
            check_stage_limit(
                "change_budget_staged_objects",
                "objects",
                admitted_objects,
                limits.maximum_objects,
            )?;
            check_stage_limit(
                "change_budget_staged_bytes",
                "bytes",
                admitted_bytes,
                limits.maximum_bytes,
            )?;
            check_stage_limit(
                "change_budget_staged_pages",
                "pages",
                admitted_pages,
                limits.maximum_pages,
            )?;
        }
        let outcome = stage_into_map(&mut self.objects, key, bytes)?;
        self.staged_bytes = admitted_bytes;
        self.staged_pages = admitted_pages;
        work.add(crate::platform::storage::object::StoreWork {
            objects_staged: 1,
            ..crate::platform::storage::object::StoreWork::default()
        });
        work.add(crate::platform::storage::object::StoreWork {
            bytes_staged: byte_count,
            ..crate::platform::storage::object::StoreWork::default()
        });
        work.add(StoreWork {
            pages_staged: u64::from(key.domain == ObjectDomain::MapPage),
            ..StoreWork::default()
        });
        Ok(outcome)
    }

    fn read_base(
        &self,
        key: ObjectKey,
        maximum_bytes: Option<usize>,
        admission: Option<&mut StoreReadAdmission>,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        if let Some(admission) = admission {
            return self.base.read_admitted(key, maximum_bytes, admission, work);
        }
        let Some(shared) = &self.read_admission else {
            return self.base.read(key, maximum_bytes, work);
        };
        let mut admission = shared.get();
        let result = self
            .base
            .read_admitted(key, maximum_bytes, &mut admission, work);
        shared.set(admission);
        result
    }

    fn contains_base(
        &self,
        key: ObjectKey,
        admission: Option<&mut StoreReadAdmission>,
        work: &mut StoreWork,
    ) -> Result<bool, StoreError> {
        if let Some(admission) = admission {
            return self.base.contains_admitted(key, admission, work);
        }
        let Some(shared) = &self.read_admission else {
            return self.base.contains(key, work);
        };
        let mut admission = shared.get();
        let result = self.base.contains_admitted(key, &mut admission, work);
        shared.set(admission);
        result
    }
}

impl<S: ImmutableObjectStore + ?Sized> ImmutableObjectStore for ObjectStage<'_, S> {
    fn read(
        &self,
        key: ObjectKey,
        maximum_bytes: Option<usize>,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.read_inner(key, maximum_bytes, None, work)
    }

    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum_bytes: Option<usize>,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.read_inner(key, maximum_bytes, Some(admission), work)
    }

    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        self.contains_inner(key, None, work)
    }

    fn contains_admitted(
        &self,
        key: ObjectKey,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<bool, StoreError> {
        self.contains_inner(key, Some(admission), work)
    }

    fn stage(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        self.stage_inner(key, bytes, None, work)
    }

    fn stage_admitted(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        self.stage_inner(key, bytes, Some(admission), work)
    }
}

fn check_stage_limit(
    code: &'static str,
    unit: &'static str,
    observed: u64,
    maximum: Option<u64>,
) -> Result<(), StoreError> {
    if let Some(maximum) = maximum
        && observed > maximum
    {
        return Err(StoreError::new(
            StoreErrorClass::Resource,
            code,
            format!("staging requires {observed} {unit}, exceeding the declared maximum {maximum}"),
        ));
    }
    Ok(())
}

impl<S: ImmutableObjectStore + ?Sized> ImmutableObjectStore for &mut S {
    fn read(
        &self,
        key: ObjectKey,
        maximum_bytes: Option<usize>,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        (**self).read(key, maximum_bytes, work)
    }

    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum_bytes: Option<usize>,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        (**self).read_admitted(key, maximum_bytes, admission, work)
    }

    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        (**self).contains(key, work)
    }

    fn contains_admitted(
        &self,
        key: ObjectKey,
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<bool, StoreError> {
        (**self).contains_admitted(key, admission, work)
    }

    fn stage(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        (**self).stage(key, bytes, work)
    }

    fn stage_admitted(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        admission: &mut StoreReadAdmission,
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        (**self).stage_admitted(key, bytes, admission, work)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreErrorClass {
    Input,
    Resource,
    Corrupt,
    Io,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoreError {
    pub class: StoreErrorClass,
    pub code: &'static str,
    pub message: String,
}

impl StoreError {
    pub fn new(class: StoreErrorClass, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            class,
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for StoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for StoreError {}

fn read_admission_unsupported() -> StoreError {
    StoreError::new(
        StoreErrorClass::Input,
        "object_read_admission_unsupported",
        "immutable object store does not implement pre-consumption read admission",
    )
}

fn read_exhausted(
    code: &'static str,
    unit: &'static str,
    required: u64,
    remaining: u64,
) -> StoreError {
    StoreError::new(
        StoreErrorClass::Resource,
        code,
        format!("immutable object read requires {required} {unit}, but only {remaining} remain"),
    )
}

pub(crate) fn copy_object_bytes(bytes: &[u8]) -> Result<Vec<u8>, StoreError> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len()).map_err(|_| {
        StoreError::new(
            StoreErrorClass::Resource,
            "object_allocation",
            "immutable object allocation exceeds available capacity",
        )
    })?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

pub fn stage_into_map(
    objects: &mut BTreeMap<ObjectKey, Vec<u8>>,
    key: ObjectKey,
    bytes: &[u8],
) -> Result<StageOutcome, StoreError> {
    key.verify(bytes)?;
    match objects.get(&key) {
        Some(existing) if existing == bytes => Ok(StageOutcome::Reused),
        Some(_) => Err(StoreError::new(
            StoreErrorClass::Corrupt,
            "object_digest_collision",
            "one immutable object identity is bound to different bytes",
        )),
        None => {
            objects.insert(key, copy_object_bytes(bytes)?);
            Ok(StageOutcome::Inserted)
        }
    }
}

#[cfg(test)]
mod admission_policy_tests {
    use super::*;

    #[test]
    fn every_store_read_field_preserves_the_complete_policy_boundary_matrix() {
        for (dimension, code) in [
            "object_read_catalog_lookups_exhausted",
            "object_read_objects_exhausted",
            "object_read_bytes_exhausted",
        ]
        .into_iter()
        .enumerate()
        {
            for maximum in [None, Some(0), Some(2), Some(3), Some(u64::MAX)] {
                let mut limits = StoreReadLimits::unbounded();
                match dimension {
                    0 => limits.maximum_catalog_lookups = maximum,
                    1 => limits.maximum_objects = maximum,
                    _ => limits.maximum_bytes = maximum,
                }
                let mut admission = StoreReadAdmission::new(limits);
                for ordinal in 1_u64..=4 {
                    let before = admission;
                    let result = match dimension {
                        0 => admission.admit_catalog_lookup(),
                        1 => admission.admit_object_bytes(0),
                        _ => admission.admit_object_bytes(1),
                    };
                    assert_eq!(
                        result.is_ok(),
                        maximum.is_none_or(|limit| ordinal <= limit),
                        "dimension={dimension} maximum={maximum:?} ordinal={ordinal}"
                    );
                    if let Err(error) = result {
                        assert_eq!(error.class, StoreErrorClass::Resource);
                        assert_eq!(error.code, code);
                        assert_eq!(admission, before);
                    }
                }
                let consumed = maximum.map_or(4, |limit| limit.min(4));
                let mut expected_remaining = [None; 3];
                expected_remaining[dimension] = maximum.map(|limit| limit - consumed);
                let remaining = admission.remaining();
                assert_eq!(
                    [
                        remaining.maximum_catalog_lookups,
                        remaining.maximum_objects,
                        remaining.maximum_bytes,
                    ],
                    expected_remaining
                );
                let observation = admission.observation();
                let expected_observation = match dimension {
                    0 => [consumed, 0, 0],
                    1 => [0, consumed, 0],
                    _ => [0, consumed, consumed],
                };
                assert_eq!(
                    [
                        observation.catalog_lookups,
                        observation.objects,
                        observation.bytes
                    ],
                    expected_observation
                );
                assert!(!observation.saturated);
                assert_eq!(observation.saturated_counters, 0);
            }
        }
    }

    #[test]
    fn default_and_unbounded_are_distinct_from_finite_zero_and_maximum() {
        assert_eq!(StoreReadLimits::default(), StoreReadLimits::unbounded());
        let mut unbounded = StoreReadAdmission::unbounded();
        unbounded.admit_object_bytes(u64::MAX).unwrap();
        assert!(!unbounded.observation().saturated);
        unbounded.admit_object_bytes(1).unwrap();
        assert_eq!(unbounded.remaining(), StoreReadLimits::unbounded());
        assert_eq!(unbounded.observation().bytes, u64::MAX);
        assert!(unbounded.observation().saturated);
        let mut finite = StoreReadAdmission::new(StoreReadLimits {
            maximum_bytes: Some(u64::MAX),
            ..StoreReadLimits::unbounded()
        });
        finite.admit_object_bytes(u64::MAX).unwrap();
        assert_eq!(finite.remaining().maximum_bytes, Some(0));
        assert!(finite.admit_object_bytes(1).is_err());
        finite.admit_object_bytes(0).unwrap();
        assert!(!finite.observation().saturated);
        let mut zero = StoreReadAdmission::new(StoreReadLimits {
            maximum_catalog_lookups: Some(0),
            maximum_objects: Some(0),
            maximum_bytes: Some(0),
        });
        assert!(zero.admit_catalog_lookup().is_err());
        assert!(zero.admit_object_bytes(0).is_err());
        assert_eq!(zero.observation(), StoreReadObservation::default());
    }

    #[test]
    fn finite_fit_and_failure_reserve_both_dimensions_atomically() {
        let mut admission = StoreReadAdmission::new(StoreReadLimits {
            maximum_catalog_lookups: Some(1),
            maximum_objects: Some(1),
            maximum_bytes: Some(536_870_913),
        });
        admission.admit_catalog_lookup().unwrap();
        let before = admission;
        assert!(admission.admit_object_bytes(536_870_914).is_err());
        assert_eq!(admission, before);
        admission.admit_object_bytes(536_870_913).unwrap();
        assert_eq!(
            admission.remaining(),
            StoreReadLimits {
                maximum_catalog_lookups: Some(0),
                maximum_objects: Some(0),
                maximum_bytes: Some(0),
            }
        );
        assert!(admission.admit_object_bytes(0).is_err());
    }

    #[test]
    fn joint_reads_preserve_every_optional_policy_and_failed_read_charge() {
        for inner_limit in [None, Some(0), Some(3), Some(u64::MAX)] {
            for outer_limit in [None, Some(0), Some(3), Some(u64::MAX)] {
                let inner = StoreReadAdmission::new(StoreReadLimits {
                    maximum_bytes: inner_limit,
                    ..StoreReadLimits::unbounded()
                });
                let outer = StoreReadAdmission::new(StoreReadLimits {
                    maximum_bytes: outer_limit,
                    ..StoreReadLimits::unbounded()
                });
                let limits = inner.remaining().intersect(outer.remaining());
                let mut joint = StoreReadAdmission::new(limits);
                joint.admit_catalog_lookup().unwrap();
                let accepted = joint.admit_object_bytes(3).is_ok();
                assert_eq!(accepted, inner_limit != Some(0) && outer_limit != Some(0));
                let next_inner = inner.charged_from(&joint).unwrap();
                let next_outer = outer.charged_from(&joint).unwrap();
                for (before, after) in [(inner, next_inner), (outer, next_outer)] {
                    assert_eq!(
                        after.remaining().maximum_bytes,
                        before
                            .remaining()
                            .maximum_bytes
                            .map(|n| n - if accepted { 3 } else { 0 })
                    );
                    assert_eq!(after.observation().catalog_lookups, 1);
                    assert_eq!(after.observation().objects, u64::from(accepted));
                }
            }
        }
    }

    #[test]
    fn observation_saturation_cannot_expand_a_finite_enclosing_policy() {
        let outer = StoreReadAdmission::new(StoreReadLimits {
            maximum_objects: Some(2),
            ..StoreReadLimits::unbounded()
        });
        let mut joint = StoreReadAdmission::new(outer.remaining());
        joint.admit_object_bytes(u64::MAX).unwrap();
        joint.admit_object_bytes(1).unwrap();
        let mut outer = outer.charged_from(&joint).unwrap();
        assert_eq!(outer.remaining().maximum_objects, Some(0));
        assert!(outer.observation().saturated);
        assert!(outer.admit_object_bytes(0).is_err());
    }

    #[test]
    fn joint_policy_cannot_drop_an_enclosing_finite_dimension() {
        let finite = StoreReadAdmission::new(StoreReadLimits {
            maximum_bytes: Some(1),
            ..StoreReadLimits::unbounded()
        });
        let mut unbounded = StoreReadAdmission::unbounded();
        unbounded.admit_object_bytes(2).unwrap();
        assert_eq!(
            finite.charged_from(&unbounded).unwrap_err().code,
            "object_read_admission_accounting"
        );
        assert_eq!(finite.remaining().maximum_bytes, Some(1));
    }
    #[test]
    fn independent_store_saturation_preserves_an_exact_maximum_in_another_dimension() {
        let mut work = StoreWork {
            bytes_read: u64::MAX,
            objects_read: u64::MAX - 1,
            ..StoreWork::default()
        };
        work.add(StoreWork {
            bytes_read: 1,
            objects_read: 1,
            ..StoreWork::default()
        });
        assert!(work.saturated);
        assert!(work.counter_saturated(StoreWorkCounter::BytesRead));
        assert!(!work.counter_saturated(StoreWorkCounter::ObjectsRead));
        assert_eq!(work.objects_read, u64::MAX);
        let mut combined = StoreWork::default();
        combined.add(work);
        assert_eq!(combined, work);

        let mut admission = StoreReadAdmission::new(StoreReadLimits {
            maximum_objects: Some(2),
            ..StoreReadLimits::unbounded()
        });
        admission.admit_object_bytes(u64::MAX).unwrap();
        admission.admit_object_bytes(1).unwrap();
        let observation = admission.observation();
        assert!(observation.counter_saturated(StoreWorkCounter::BytesRead));
        assert!(!observation.counter_saturated(StoreWorkCounter::ObjectsRead));
        assert!(
            observation
                .as_store_work()
                .counter_saturated(StoreWorkCounter::BytesRead)
        );
        assert_eq!(admission.remaining().maximum_objects, Some(0));
        assert!(admission.admit_object_bytes(0).is_err());
    }
}
