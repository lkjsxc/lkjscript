//! Typed immutable object identities and narrow store interface.

use super::contract;
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
    PackageObject,
    BackupManifest,
    BackupSegment,
    Dependency,
    Retirement,
    Change,
}

impl ObjectDomain {
    pub const ALL: [Self; 20] = [
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
        Self::PackageObject,
        Self::BackupManifest,
        Self::BackupSegment,
        Self::Dependency,
        Self::Retirement,
        Self::Change,
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
            Self::PackageObject => 15,
            Self::BackupManifest => 16,
            Self::BackupSegment => 17,
            Self::Dependency => 18,
            Self::Retirement => 19,
            Self::Change => 20,
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
            Self::PackageObject => "package_object",
            Self::BackupManifest => "backup_manifest",
            Self::BackupSegment => "backup_segment",
            Self::Dependency => "dependency",
            Self::Retirement => "retirement",
            Self::Change => "change",
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
            Self::PackageObject => contract::PACKAGE_OBJECT_DIGEST_DOMAIN,
            Self::BackupManifest => contract::BACKUP_MANIFEST_DIGEST_DOMAIN,
            Self::BackupSegment => contract::BACKUP_SEGMENT_DIGEST_DOMAIN,
            Self::Dependency => contract::DEPENDENCY_OBJECT_DIGEST_DOMAIN,
            Self::Retirement => contract::RETIREMENT_OBJECT_DIGEST_DOMAIN,
            Self::Change => contract::CHANGE_DIGEST_DOMAIN,
        }
    }

    pub const fn maximum_bytes(self) -> usize {
        match self {
            Self::MapPage => crate::platform::persistent_map::MAXIMUM_PAGE_BYTES,
            Self::SemanticRoot | Self::Revision | Self::Receipt | Self::Retirement => 64 * 1024,
            Self::Type | Self::OwnerSummary | Self::Dependency => 1024 * 1024,
            Self::Owner
            | Self::Sequence
            | Self::ValidationWitness
            | Self::Draft
            | Self::Conflict
            | Self::CompilerUnit
            | Self::ArtifactManifest
            | Self::PackageObject
            | Self::BackupManifest
            | Self::Change => 4 * 1024 * 1024,
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
    pub packs_sealed: u64,
}

pub trait ImmutableObjectStore {
    fn read(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError>;

    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        Ok(self.read(key, key.domain.maximum_bytes(), work)?.is_some())
    }

    fn stage(
        &mut self,
        key: ObjectKey,
        bytes: &[u8],
        work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError>;
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
