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

    pub const fn maximum_bytes(self) -> usize {
        match self {
            Self::MapPage => crate::platform::persistent_map::MAXIMUM_PAGE_BYTES,
            Self::SemanticRoot
            | Self::Revision
            | Self::Receipt
            | Self::Retirement
            | Self::CompilationManifest => 64 * 1024,
            Self::Type | Self::OwnerSummary | Self::Dependency | Self::PackageInterface => {
                1024 * 1024
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
            | Self::Change
            | Self::Transaction
            | Self::SemanticDiff => 4 * 1024 * 1024,
            Self::Blob | Self::BackupSegment => contract::MAXIMUM_PACK_ENTRY_BYTES,
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
        if bytes.len() > self.domain.maximum_bytes() {
            return Err(StoreError::new(
                StoreErrorClass::Resource,
                "object_domain_size",
                format!(
                    "{} object has {} bytes; maximum is {}",
                    self.domain.name(),
                    bytes.len(),
                    self.domain.maximum_bytes()
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
}

impl StoreWork {
    pub fn add(&mut self, other: Self) {
        self.catalog_lookups = self.catalog_lookups.saturating_add(other.catalog_lookups);
        self.packs_opened = self.packs_opened.saturating_add(other.packs_opened);
        self.objects_read = self.objects_read.saturating_add(other.objects_read);
        self.objects_staged = self.objects_staged.saturating_add(other.objects_staged);
        self.objects_reused = self.objects_reused.saturating_add(other.objects_reused);
        self.bytes_read = self.bytes_read.saturating_add(other.bytes_read);
        self.bytes_staged = self.bytes_staged.saturating_add(other.bytes_staged);
        self.pages_staged = self.pages_staged.saturating_add(other.pages_staged);
        self.packs_sealed = self.packs_sealed.saturating_add(other.packs_sealed);
    }
}

/// Exact remaining aggregate allowance for immutable-object reads at one owning boundary.
///
/// Object and byte allowances apply only to present objects. A missing lookup consumes one
/// catalog lookup and no object or byte allowance. Store implementations that support admitted
/// reads must inspect already-retained catalog metadata, admit the exact payload length, and only
/// then open, copy, hash, or otherwise consume the object payload.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StoreReadLimits {
    pub maximum_catalog_lookups: u64,
    pub maximum_objects: u64,
    pub maximum_bytes: u64,
}

/// Mutable pre-consumption admission for one aggregate immutable-object read boundary.
///
/// [`StoreWork`] remains the observation of work actually completed. This value owns only the
/// remaining permission and deliberately does not infer observations after a read has occurred.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StoreReadAdmission {
    remaining: StoreReadLimits,
}

impl StoreReadAdmission {
    pub const fn new(limits: StoreReadLimits) -> Self {
        Self { remaining: limits }
    }

    pub const fn unbounded() -> Self {
        Self::new(StoreReadLimits {
            maximum_catalog_lookups: u64::MAX,
            maximum_objects: u64::MAX,
            maximum_bytes: u64::MAX,
        })
    }

    pub const fn remaining(&self) -> StoreReadLimits {
        self.remaining
    }

    pub fn admit_catalog_lookup(&mut self) -> Result<(), StoreError> {
        let Some(remaining) = self.remaining.maximum_catalog_lookups.checked_sub(1) else {
            return Err(read_exhausted(
                "object_read_catalog_lookups_exhausted",
                "catalog lookups",
                1,
                self.remaining.maximum_catalog_lookups,
            ));
        };
        self.remaining.maximum_catalog_lookups = remaining;
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
        if self.remaining.maximum_objects == 0 {
            return Err(read_exhausted(
                "object_read_objects_exhausted",
                "objects",
                1,
                0,
            ));
        }
        if byte_count > self.remaining.maximum_bytes {
            return Err(read_exhausted(
                "object_read_bytes_exhausted",
                "bytes",
                byte_count,
                self.remaining.maximum_bytes,
            ));
        }
        self.remaining.maximum_objects -= 1;
        self.remaining.maximum_bytes -= byte_count;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ObjectStageLimits {
    pub maximum_objects: u64,
    pub maximum_bytes: u64,
    pub maximum_pages: u64,
}

pub trait ImmutableObjectStore {
    fn read(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
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
        _maximum_bytes: usize,
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
        maximum_bytes: usize,
        admission: Option<&mut StoreReadAdmission>,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        if let Some(bytes) = self.objects.get(&key) {
            if bytes.len() > maximum_bytes {
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
            work.objects_read = work.objects_read.saturating_add(1);
            work.bytes_read = work.bytes_read.saturating_add(bytes.len() as u64);
            return Ok(Some(bytes.clone()));
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
            work.objects_reused = work.objects_reused.saturating_add(1);
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
            work.objects_reused = work.objects_reused.saturating_add(1);
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
        work.objects_staged = work.objects_staged.saturating_add(1);
        work.bytes_staged = work.bytes_staged.saturating_add(byte_count);
        work.pages_staged = work
            .pages_staged
            .saturating_add(u64::from(key.domain == ObjectDomain::MapPage));
        Ok(outcome)
    }

    fn read_base(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
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
        maximum_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        self.read_inner(key, maximum_bytes, None, work)
    }

    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
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
    maximum: u64,
) -> Result<(), StoreError> {
    if observed > maximum {
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
        maximum_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        (**self).read(key, maximum_bytes, work)
    }

    fn read_admitted(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
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
            objects.insert(key, bytes.to_vec());
            Ok(StageOutcome::Inserted)
        }
    }
}
