//! Deterministic segmented Graph 5 artifact contract and strict standalone loader.

use super::manifest::{
    COMPILATION_MANIFEST_CONTRACT_VERSION, CompilationBinding, CompilationManifest,
    CompilationManifestDigest,
};
use super::unit::{
    BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_VERSION, CompilationPayload, CompilationUnit,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    OwnerKey, OwnerKind, OwnerObjectDigest, OwnerRecord, PackageId, PackageObjectDigest, TypeForm,
    decode_owner, decode_type_object,
};
use crate::platform::package_object::{PackageObject, validate_package_object_closure};
use crate::platform::persistent_map::{MapError, MapErrorClass, MapWork, PersistentMap};
use crate::platform::semantic_id::{RepositoryId, TargetId, TypeParameterId};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StageOutcome, StoreError, StoreErrorClass,
    StoreWork,
};
use crate::platform::storage::pack::{PackBuilder, PackId, PackMetadata};
use crate::platform::storage::page_store::ObjectPageReader;
use bincode::{Decode, Encode};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const ARTIFACT_MANIFEST_CONTRACT_IDENTITY: &str = "lkjscript-artifact-manifest-5";
pub const ARTIFACT_BUNDLE_CONTRACT_IDENTITY: &str = "lkjscript-artifact-bundle-5";
pub const ARTIFACT_CONTRACT_VERSION: u16 = 5;
pub(crate) const ARTIFACT_MANIFEST_MAGIC: [u8; 8] = *b"LKJAMF05";
pub(crate) const ARTIFACT_BUNDLE_MAGIC: [u8; 8] = *b"LKJART05";
pub(crate) const ARTIFACT_BUNDLE_END_MAGIC: [u8; 8] = *b"LKJAEND5";
pub(crate) const ARTIFACT_MANIFEST_ENVELOPE_DOMAIN: &str =
    "lkjscript.artifact-manifest-envelope.v5";
pub(crate) const ARTIFACT_BUNDLE_DIGEST_DOMAIN: &str = "lkjscript.artifact-bundle.v5";
pub(crate) const ARTIFACT_BUNDLE_CHECKSUM_DOMAIN: &str = "lkjscript.artifact-bundle.complete.v5";
pub(crate) const ARTIFACT_CLOSURE_DIGEST_DOMAIN: &str = "lkjscript.artifact-object-closure.v5";
pub(crate) const MAXIMUM_ARTIFACT_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAXIMUM_ARTIFACT_PACKAGES: usize = 10_000;
pub(crate) const MAXIMUM_ARTIFACT_TARGETS: usize = 1_000_000;
pub(crate) const MAXIMUM_ARTIFACT_SEGMENTS: usize = 1_000_000;
pub(crate) const MAXIMUM_ARTIFACT_BUNDLE_BYTES: u64 = 16 * 1024 * 1024 * 1024;
pub(crate) const TARGET_ARTIFACT_SEGMENT_BYTES: usize = 4 * 1024 * 1024;

const BUNDLE_HEADER_BYTES: usize = 8 + 2 + 2 + 8 + 8 + 32;
const SEGMENT_HEADER_BYTES: usize = 8 + 32;
const BUNDLE_FOOTER_BYTES: usize = 32 + 8;

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArtifactManifestDigest([u8; 32]);

impl ArtifactManifestDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }

    pub const fn object_key(self) -> ObjectKey {
        ObjectKey::from_digest(ObjectDomain::ArtifactManifest, self.0)
    }
}

impl fmt::Display for ArtifactManifestDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("artifact_manifest_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArtifactBundleDigest([u8; 32]);

impl ArtifactBundleDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for ArtifactBundleDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("artifact_bundle_")?;
        formatter.write_str(&crate::platform::semantic_id::encode_hex(&self.0))
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ArtifactClosureDigest([u8; 32]);

impl ArtifactClosureDigest {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ArtifactTarget {
    pub target: TargetId,
    pub owner: OwnerObjectDigest,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ArtifactPackage {
    pub repository_id: RepositoryId,
    pub package: PackageId,
    pub package_object: PackageObjectDigest,
    pub compilation: CompilationManifestDigest,
    pub targets: Vec<ArtifactTarget>,
}

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub struct ArtifactManifest {
    pub contract_version: u16,
    pub graph_contract_version: u16,
    pub compiler_contract_version: u16,
    pub bytecode_contract_version: u16,
    pub compilation_manifest_contract_version: u16,
    pub root_package: PackageId,
    pub packages: Vec<ArtifactPackage>,
    pub closure: ArtifactClosureDigest,
    pub object_count: u64,
    pub object_bytes: u64,
}

impl ArtifactManifest {
    pub fn encode(&self) -> Result<(ArtifactManifestDigest, Vec<u8>), Diagnostic> {
        self.validate()?;
        let bytes = crate::platform::packed::encode(
            ARTIFACT_MANIFEST_MAGIC,
            ARTIFACT_MANIFEST_ENVELOPE_DOMAIN,
            self,
            MAXIMUM_ARTIFACT_MANIFEST_BYTES,
        )?;
        let key = ObjectKey::for_bytes(ObjectDomain::ArtifactManifest, &bytes);
        Ok((
            ArtifactManifestDigest::from_bytes(key.digest.bytes()),
            bytes,
        ))
    }

    pub fn decode(bytes: &[u8], expected: ArtifactManifestDigest) -> Result<Self, Diagnostic> {
        expected
            .object_key()
            .verify(bytes)
            .map_err(store_diagnostic)?;
        let manifest: Self = crate::platform::packed::decode(
            bytes,
            ARTIFACT_MANIFEST_MAGIC,
            ARTIFACT_MANIFEST_ENVELOPE_DOMAIN,
            MAXIMUM_ARTIFACT_MANIFEST_BYTES,
        )?;
        manifest.validate()?;
        let (digest, canonical) = manifest.encode()?;
        if digest != expected || canonical != bytes {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_manifest_canonical",
                "artifact manifest is not canonically encoded",
            ));
        }
        Ok(manifest)
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != ARTIFACT_CONTRACT_VERSION
            || self.graph_contract_version
                != crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
            || self.compiler_contract_version != COMPILER_UNIT_CONTRACT_VERSION
            || self.bytecode_contract_version != BYTECODE_CONTRACT_VERSION
            || self.compilation_manifest_contract_version != COMPILATION_MANIFEST_CONTRACT_VERSION
        {
            return Err(artifact_error(
                DiagnosticClass::Source,
                "artifact_manifest_contract",
                "artifact manifest uses a predecessor or foreign graph, compiler, or artifact contract",
            ));
        }
        if self.packages.is_empty() || self.packages.len() > MAXIMUM_ARTIFACT_PACKAGES {
            return Err(artifact_error(
                DiagnosticClass::Resource,
                "artifact_manifest_package_count",
                "artifact manifest package count is outside the current implementation bound",
            ));
        }
        if self
            .packages
            .windows(2)
            .any(|pair| pair[0].package >= pair[1].package)
            || !self
                .packages
                .iter()
                .any(|package| package.package == self.root_package)
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_manifest_package_order",
                "artifact packages are not unique, ordered, and rooted exactly",
            ));
        }
        for package in &self.packages {
            if package.targets.len() > MAXIMUM_ARTIFACT_TARGETS {
                return Err(artifact_error(
                    DiagnosticClass::Resource,
                    "artifact_manifest_target_count",
                    "artifact package target count exceeds the current implementation bound",
                ));
            }
            if package.repository_id.bytes() == [0; 16]
                || package.package.bytes() == [0; 16]
                || package
                    .targets
                    .windows(2)
                    .any(|pair| pair[0].target >= pair[1].target)
            {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_manifest_package",
                    "artifact package identity or target ordering is invalid",
                ));
            }
        }
        if self.object_count == 0 || self.object_bytes == 0 {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_manifest_closure",
                "artifact manifest declares an empty immutable object closure",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ArtifactLoadWork {
    pub segments: u64,
    pub objects: u64,
    pub object_bytes: u64,
    pub map: MapWork,
    pub store: StoreWork,
}

#[derive(Clone, Debug)]
pub struct LoadedArtifact {
    pub manifest: ArtifactManifest,
    pub manifest_digest: ArtifactManifestDigest,
    pub bundle_digest: ArtifactBundleDigest,
    pub segment_count: u64,
    pub work: ArtifactLoadWork,
    pub(crate) objects: BTreeMap<ObjectKey, Vec<u8>>,
}

impl LoadedArtifact {
    pub fn package(&self, package: PackageId) -> Option<&ArtifactPackage> {
        self.manifest
            .packages
            .binary_search_by_key(&package, |entry| entry.package)
            .ok()
            .map(|index| &self.manifest.packages[index])
    }

    pub fn root_package(&self) -> Option<&ArtifactPackage> {
        self.package(self.manifest.root_package)
    }
}

#[derive(Clone, Debug)]
pub struct EncodedArtifact {
    pub manifest: ArtifactManifest,
    pub manifest_digest: ArtifactManifestDigest,
    pub bundle_digest: ArtifactBundleDigest,
    pub bytes: Vec<u8>,
    pub segment_count: u64,
}

pub(crate) fn encode_artifact(
    manifest: ArtifactManifest,
    objects: &BTreeMap<ObjectKey, Vec<u8>>,
) -> Result<EncodedArtifact, Diagnostic> {
    validate_declared_closure(&manifest, objects)?;
    let (manifest_digest, manifest_bytes) = manifest.encode()?;
    let mut builder = PackBuilder::default();
    for (key, bytes) in objects {
        if *key == manifest_digest.object_key() || key.domain == ObjectDomain::ArtifactManifest {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_bundle_manifest_object",
                "artifact payload closure must not contain an artifact-manifest object",
            ));
        }
        builder.insert(*key, bytes).map_err(store_diagnostic)?;
    }
    let segments = builder
        .seal_targeted(TARGET_ARTIFACT_SEGMENT_BYTES)
        .map_err(store_diagnostic)?;
    if segments.is_empty() || segments.len() > MAXIMUM_ARTIFACT_SEGMENTS {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_bundle_segment_count",
            "artifact segment count is outside the current implementation bound",
        ));
    }
    let manifest_length = manifest_bytes.len() as u64;
    let segment_count = segments.len() as u64;
    let core_length = BUNDLE_HEADER_BYTES as u64
        + manifest_length
        + segments.iter().try_fold(0_u64, |total, segment| {
            total
                .checked_add(SEGMENT_HEADER_BYTES as u64)
                .and_then(|value| value.checked_add(segment.bytes.len() as u64))
                .ok_or_else(|| {
                    artifact_error(
                        DiagnosticClass::Resource,
                        "artifact_bundle_length",
                        "artifact bundle length overflows its contract domain",
                    )
                })
        })?;
    let total_length = core_length
        .checked_add(BUNDLE_FOOTER_BYTES as u64)
        .ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Resource,
                "artifact_bundle_length",
                "artifact bundle length overflows its contract domain",
            )
        })?;
    if total_length > MAXIMUM_ARTIFACT_BUNDLE_BYTES {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_bundle_length",
            "artifact bundle exceeds its current hostile decoder bound",
        ));
    }
    let capacity = usize::try_from(total_length).map_err(|_| {
        artifact_error(
            DiagnosticClass::Resource,
            "artifact_bundle_platform_length",
            "artifact bundle length does not fit this platform",
        )
    })?;
    let mut bytes = Vec::with_capacity(capacity);
    bytes.extend_from_slice(&ARTIFACT_BUNDLE_MAGIC);
    bytes.extend_from_slice(&ARTIFACT_CONTRACT_VERSION.to_be_bytes());
    bytes.extend_from_slice(&0_u16.to_be_bytes());
    bytes.extend_from_slice(&manifest_length.to_be_bytes());
    bytes.extend_from_slice(&segment_count.to_be_bytes());
    bytes.extend_from_slice(&manifest_digest.bytes());
    bytes.extend_from_slice(&manifest_bytes);
    for segment in &segments {
        bytes.extend_from_slice(&(segment.bytes.len() as u64).to_be_bytes());
        bytes.extend_from_slice(&segment.id.bytes());
        bytes.extend_from_slice(&segment.bytes);
    }
    if bytes.len() as u64 != core_length {
        return Err(artifact_error(
            DiagnosticClass::Infrastructure,
            "artifact_bundle_encode_length",
            "artifact bundle encoder produced an unexpected core length",
        ));
    }
    let checksum = domain_digest(ARTIFACT_BUNDLE_CHECKSUM_DOMAIN, &bytes);
    bytes.extend_from_slice(&checksum);
    bytes.extend_from_slice(&ARTIFACT_BUNDLE_END_MAGIC);
    let bundle_digest =
        ArtifactBundleDigest::from_bytes(domain_digest(ARTIFACT_BUNDLE_DIGEST_DOMAIN, &bytes));
    let loaded = load_artifact(&bytes)?;
    if loaded.manifest_digest != manifest_digest
        || loaded.bundle_digest != bundle_digest
        || loaded.manifest != manifest
    {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_bundle_self_validation",
            "artifact changed during strict standalone self-validation",
        ));
    }
    Ok(EncodedArtifact {
        manifest,
        manifest_digest,
        bundle_digest,
        bytes,
        segment_count,
    })
}

pub fn load_artifact(bytes: &[u8]) -> Result<LoadedArtifact, Diagnostic> {
    if bytes.len() as u64 > MAXIMUM_ARTIFACT_BUNDLE_BYTES
        || bytes.len() < BUNDLE_HEADER_BYTES + BUNDLE_FOOTER_BYTES
    {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_bundle_length",
            "artifact bundle byte length is outside its hostile decoder bound",
        ));
    }
    if bytes.get(..8) != Some(ARTIFACT_BUNDLE_MAGIC.as_slice()) {
        return Err(artifact_error(
            DiagnosticClass::Source,
            "artifact_bundle_contract",
            "artifact bundle uses a predecessor or foreign contract",
        ));
    }
    let version = read_u16(bytes, 8, "artifact_bundle_contract")?;
    let flags = read_u16(bytes, 10, "artifact_bundle_flags")?;
    if version != ARTIFACT_CONTRACT_VERSION || flags != 0 {
        return Err(artifact_error(
            DiagnosticClass::Source,
            "artifact_bundle_contract",
            "artifact bundle version or reserved flags are not current",
        ));
    }
    let manifest_length = usize_from_u64(
        read_u64(bytes, 12, "artifact_bundle_manifest_length")?,
        "artifact_bundle_manifest_length",
    )?;
    if manifest_length == 0 || manifest_length > MAXIMUM_ARTIFACT_MANIFEST_BYTES {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_bundle_manifest_length",
            "artifact manifest length is outside its hostile decoder bound",
        ));
    }
    let segment_count = usize_from_u64(
        read_u64(bytes, 20, "artifact_bundle_segment_count")?,
        "artifact_bundle_segment_count",
    )?;
    if segment_count == 0 || segment_count > MAXIMUM_ARTIFACT_SEGMENTS {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_bundle_segment_count",
            "artifact segment count is outside its hostile decoder bound",
        ));
    }
    let manifest_digest = ArtifactManifestDigest::from_bytes(read_array::<32>(
        bytes,
        28,
        "artifact_bundle_manifest_digest",
    )?);
    let manifest_end = BUNDLE_HEADER_BYTES
        .checked_add(manifest_length)
        .ok_or_else(|| bundle_overflow("artifact_bundle_manifest_bounds"))?;
    let footer_start = bytes
        .len()
        .checked_sub(BUNDLE_FOOTER_BYTES)
        .ok_or_else(|| bundle_overflow("artifact_bundle_footer"))?;
    if manifest_end > footer_start {
        return Err(bundle_truncated("artifact_bundle_manifest_bounds"));
    }
    let manifest =
        ArtifactManifest::decode(&bytes[BUNDLE_HEADER_BYTES..manifest_end], manifest_digest)?;
    let mut position = manifest_end;
    let mut objects = BTreeMap::new();
    let mut previous = None;
    let mut work = ArtifactLoadWork::default();
    for _ in 0..segment_count {
        let segment_header_end = position
            .checked_add(SEGMENT_HEADER_BYTES)
            .ok_or_else(|| bundle_overflow("artifact_bundle_segment_header"))?;
        if segment_header_end > footer_start {
            return Err(bundle_truncated("artifact_bundle_segment_header"));
        }
        let segment_length = usize_from_u64(
            read_u64(bytes, position, "artifact_bundle_segment_length")?,
            "artifact_bundle_segment_length",
        )?;
        let expected_id = PackId::from_bytes(read_array::<32>(
            bytes,
            position + 8,
            "artifact_bundle_segment_identity",
        )?);
        let segment_end = segment_header_end
            .checked_add(segment_length)
            .ok_or_else(|| bundle_overflow("artifact_bundle_segment_bounds"))?;
        if segment_end > footer_start {
            return Err(bundle_truncated("artifact_bundle_segment_bounds"));
        }
        let segment = &bytes[segment_header_end..segment_end];
        if PackId::of(segment) != expected_id {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_bundle_segment_identity",
                "artifact segment bytes disagree with their exact pack identity",
            ));
        }
        let metadata = PackMetadata::decode(segment, true).map_err(store_diagnostic)?;
        for entry in &metadata.entries {
            if previous.is_some_and(|previous| previous >= entry.key) {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_bundle_object_order",
                    "artifact segment entries are not globally unique and ordered",
                ));
            }
            previous = Some(entry.key);
            let value = metadata
                .read(segment, entry.key, entry.key.domain.maximum_bytes())
                .map_err(store_diagnostic)?
                .ok_or_else(|| {
                    artifact_error(
                        DiagnosticClass::Corrupt,
                        "artifact_bundle_object_missing",
                        "artifact pack index lost one exact object",
                    )
                })?;
            if objects.insert(entry.key, value).is_some() {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_bundle_object_duplicate",
                    "artifact bundle repeats one immutable object key",
                ));
            }
        }
        work.segments = work.segments.saturating_add(1);
        position = segment_end;
    }
    if position != footer_start {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_bundle_trailing",
            "artifact bundle contains bytes outside its exact segment sequence",
        ));
    }
    if bytes[footer_start + 32..] != ARTIFACT_BUNDLE_END_MAGIC {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_bundle_end_magic",
            "artifact bundle closing magic is corrupt or truncated",
        ));
    }
    let expected_checksum = read_array::<32>(bytes, footer_start, "artifact_bundle_checksum")?;
    if domain_digest(ARTIFACT_BUNDLE_CHECKSUM_DOMAIN, &bytes[..footer_start]) != expected_checksum {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_bundle_checksum",
            "artifact bundle checksum does not match its exact core bytes",
        ));
    }
    validate_declared_closure(&manifest, &objects)?;
    validate_object_closure(&manifest, &objects, &mut work)?;
    let bundle_digest =
        ArtifactBundleDigest::from_bytes(domain_digest(ARTIFACT_BUNDLE_DIGEST_DOMAIN, bytes));
    work.objects = objects.len() as u64;
    work.object_bytes = objects.values().fold(0_u64, |total, value| {
        total.saturating_add(value.len() as u64)
    });
    Ok(LoadedArtifact {
        manifest,
        manifest_digest,
        bundle_digest,
        segment_count: segment_count as u64,
        work,
        objects,
    })
}

pub(crate) fn closure_facts(
    objects: &BTreeMap<ObjectKey, Vec<u8>>,
) -> Result<(ArtifactClosureDigest, u64, u64), Diagnostic> {
    let object_count = u64::try_from(objects.len()).map_err(|_| {
        artifact_error(
            DiagnosticClass::Resource,
            "artifact_closure_count",
            "artifact object count does not fit its contract domain",
        )
    })?;
    let object_bytes = objects.values().try_fold(0_u64, |total, bytes| {
        total.checked_add(bytes.len() as u64).ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Resource,
                "artifact_closure_bytes",
                "artifact object bytes overflow their contract domain",
            )
        })
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(ARTIFACT_CLOSURE_DIGEST_DOMAIN);
    hasher.update(&object_count.to_be_bytes());
    hasher.update(&object_bytes.to_be_bytes());
    for (key, bytes) in objects {
        hasher.update(&[key.domain.tag()]);
        hasher.update(&key.digest.bytes());
        hasher.update(&(bytes.len() as u64).to_be_bytes());
    }
    Ok((
        ArtifactClosureDigest::from_bytes(*hasher.finalize().as_bytes()),
        object_count,
        object_bytes,
    ))
}

fn validate_declared_closure(
    manifest: &ArtifactManifest,
    objects: &BTreeMap<ObjectKey, Vec<u8>>,
) -> Result<(), Diagnostic> {
    manifest.validate()?;
    if objects.keys().any(|key| {
        !matches!(
            key.domain,
            ObjectDomain::Owner
                | ObjectDomain::Type
                | ObjectDomain::Blob
                | ObjectDomain::MapPage
                | ObjectDomain::CompilerUnit
                | ObjectDomain::PackageObject
                | ObjectDomain::PackageInterface
                | ObjectDomain::CompilationManifest
        )
    }) {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_closure_domain",
            "artifact closure contains an object domain outside executable package authority",
        ));
    }
    for (key, value) in objects {
        key.verify(value).map_err(store_diagnostic)?;
    }
    let (digest, count, bytes) = closure_facts(objects)?;
    if manifest.closure != digest
        || manifest.object_count != count
        || manifest.object_bytes != bytes
    {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_closure_digest",
            "artifact manifest disagrees with its exact immutable object closure",
        ));
    }
    Ok(())
}

fn validate_object_closure(
    manifest: &ArtifactManifest,
    objects: &BTreeMap<ObjectKey, Vec<u8>>,
    work: &mut ArtifactLoadWork,
) -> Result<(), Diagnostic> {
    let store = TrackingObjectStore::new(objects);
    let root = manifest
        .packages
        .iter()
        .find(|package| package.package == manifest.root_package)
        .ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_root_package",
                "artifact root package disappeared after manifest validation",
            )
        })?;
    let mut store_work = StoreWork::default();
    let root_object =
        validate_package_object_closure(&store, root.package_object, None, &mut store_work)?;
    if root_object.package != manifest.root_package {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_root_package_binding",
            "artifact root package object has another package identity",
        ));
    }
    let reachable_packages = store.visited_for_domain(ObjectDomain::PackageObject);
    let expected_packages = manifest
        .packages
        .iter()
        .map(|package| {
            ObjectKey::from_digest(ObjectDomain::PackageObject, package.package_object.bytes())
        })
        .collect::<BTreeSet<_>>();
    if reachable_packages != expected_packages {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_package_closure",
            "artifact package manifests do not equal the exact root dependency closure",
        ));
    }

    let mut units = BTreeMap::<(PackageId, OwnerKey), CompilationUnit>::new();
    let mut type_roots = BTreeSet::new();
    let mut blobs = BTreeMap::new();
    for package in &manifest.packages {
        let package_key =
            ObjectKey::from_digest(ObjectDomain::PackageObject, package.package_object.bytes());
        let package_bytes = required_object(
            &store,
            package_key,
            "artifact package object is missing",
            &mut store_work,
        )?;
        let package_object = PackageObject::decode(&package_bytes, package.package_object)?;
        let compilation_key = package.compilation.object_key();
        let compilation_bytes = required_object(
            &store,
            compilation_key,
            "artifact compilation manifest is missing",
            &mut store_work,
        )?;
        let compilation = CompilationManifest::decode(&compilation_bytes, package.compilation)?;
        if package.repository_id != package_object.repository_id
            || package.package != package_object.package
            || compilation.repository_id != package.repository_id
            || compilation.package_id != package.package
            || compilation.revision != package_object.semantic_revision
            || compilation.semantic_root != package_object.semantic_root
            || compilation.validation_certificate != package_object.witness.certificate
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_package_compilation_binding",
                "package object and compilation manifest do not bind one exact accepted package",
            ));
        }
        let reader = ObjectPageReader::new(&store);
        let mut map_work = MapWork::default();
        let mut captured = None;
        let result = PersistentMap::from_root(compilation.units).for_each(
            &reader,
            &mut map_work,
            |key, value| {
                let operation = (|| {
                    let owner = crate::platform::kernel::EncodedOwnerKey::decode(key)?;
                    let binding = CompilationBinding::decode(value, owner)?;
                    let unit_bytes = required_object(
                        &store,
                        binding.object.object_key(),
                        "artifact compilation manifest references a missing compiler unit",
                        &mut store_work,
                    )?;
                    let unit = CompilationUnit::decode(&unit_bytes, binding.object.object_key())?;
                    if unit.key != binding.key
                        || unit.source.package != package.package
                        || unit.source.owner != owner
                        || unit.source.kind != binding.kind
                    {
                        return Err(artifact_error(
                            DiagnosticClass::Corrupt,
                            "artifact_compiler_unit_binding",
                            "compiler unit disagrees with its package compilation manifest",
                        ));
                    }
                    type_roots.extend(unit.tables.types.iter().copied());
                    for text in &unit.tables.texts {
                        if let super::unit::CompiledText::Blob { digest, bytes } = text
                            && let Some(previous) = blobs.insert(*digest, *bytes)
                            && previous != *bytes
                        {
                            return Err(artifact_error(
                                DiagnosticClass::Corrupt,
                                "artifact_blob_length_binding",
                                "compiler units bind one blob digest to conflicting lengths",
                            ));
                        }
                    }
                    if units.insert((package.package, owner), unit).is_some() {
                        return Err(artifact_error(
                            DiagnosticClass::Corrupt,
                            "artifact_compiler_unit_duplicate",
                            "artifact repeats one package compiler-unit owner",
                        ));
                    }
                    Ok::<(), Diagnostic>(())
                })();
                match operation {
                    Ok(()) => Ok(()),
                    Err(diagnostic) => {
                        captured = Some(diagnostic);
                        Err(MapError {
                            class: MapErrorClass::Corrupt,
                            code: "artifact_compilation_iteration_stop",
                            message:
                                "artifact compilation iteration stopped after an exact diagnostic"
                                    .to_owned(),
                        })
                    }
                }
            },
        );
        add_map_work(&mut work.map, map_work);
        work.store.add(reader.work());
        if let Some(diagnostic) = captured {
            return Err(diagnostic);
        }
        result.map_err(map_diagnostic)?;
        validate_targets(package, &units, &store, &mut store_work)?;
    }
    let relocations = validate_unit_relocations(&units)?;

    let mut types = BTreeSet::new();
    while let Some(digest) = type_roots.pop_first() {
        if !types.insert(digest) {
            continue;
        }
        let key = ObjectKey::from_digest(ObjectDomain::Type, digest.bytes());
        let bytes = required_object(
            &store,
            key,
            "artifact compiler unit references a missing type object",
            &mut store_work,
        )?;
        let object = decode_type_object(&bytes, digest)?;
        match &object.form {
            TypeForm::Named { declaration }
                if !relocations
                    .declarations
                    .contains(&(declaration.package, declaration.declaration)) =>
            {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_type_declaration_missing",
                    "artifact type object names a declaration outside the executable package closure",
                ));
            }
            TypeForm::TypeParameter { parameter }
                if !relocations.type_parameters.contains(parameter) =>
            {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_type_parameter_missing",
                    "artifact type object names no exact compiled type parameter",
                ));
            }
            _ => {}
        }
        type_roots.extend(object.child_types());
    }
    for (digest, expected_length) in blobs {
        let key = ObjectKey::from_digest(ObjectDomain::Blob, digest.bytes());
        let bytes = required_object(
            &store,
            key,
            "artifact compiler unit references a missing blob object",
            &mut store_work,
        )?;
        if bytes.len() as u64 != expected_length {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_blob_length",
                "artifact blob bytes disagree with the compiler-unit length binding",
            ));
        }
    }
    work.store.add(store_work);
    let visited = store.visited();
    let expected = objects.keys().copied().collect::<BTreeSet<_>>();
    if visited != expected {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_unreachable_object",
            "artifact bundle contains an object outside its exact executable closure",
        ));
    }
    Ok(())
}

fn validate_targets(
    package: &ArtifactPackage,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    store: &TrackingObjectStore<'_>,
    work: &mut StoreWork,
) -> Result<(), Diagnostic> {
    let unit_targets = units
        .iter()
        .filter_map(|((unit_package, owner), unit)| {
            (*unit_package == package.package && matches!(owner, OwnerKey::Target(_)))
                .then_some((*owner, unit))
        })
        .collect::<BTreeMap<_, _>>();
    if unit_targets.len() != package.targets.len() {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_target_count",
            "artifact target metadata does not cover every target compiler unit exactly",
        ));
    }
    for target in &package.targets {
        let owner = OwnerKey::Target(target.target);
        let unit = unit_targets.get(&owner).ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_target_unit_missing",
                "artifact target metadata names no exact target compiler unit",
            )
        })?;
        let owner_key = ObjectKey::from_digest(ObjectDomain::Owner, target.owner.bytes());
        let bytes = required_object(
            store,
            owner_key,
            "artifact target metadata references a missing owner object",
            work,
        )?;
        let record = decode_owner(&bytes, owner, OwnerKind::Target, target.owner)?;
        let OwnerRecord::Target(record) = record else {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_target_owner_kind",
                "artifact target metadata decoded another owner kind",
            ));
        };
        let CompilationPayload::Target {
            component,
            port,
            runner,
        } = unit.payload
        else {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_target_payload",
                "target compiler unit has another payload class",
            ));
        };
        if unit.tables.declarations.get(component as usize) != Some(&record.component)
            || unit.tables.ports.get(port as usize) != Some(&record.port)
            || runner != record.runner
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_target_binding",
                "target owner metadata disagrees with its exact compiler unit",
            ));
        }
    }
    Ok(())
}

#[derive(Default)]
struct ArtifactRelocations {
    declarations: BTreeSet<(PackageId, crate::platform::semantic_id::DeclarationId)>,
    type_parameters: BTreeSet<TypeParameterId>,
}

fn validate_unit_relocations(
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
) -> Result<ArtifactRelocations, Diagnostic> {
    let declarations = units
        .keys()
        .filter_map(|(package, owner)| match owner {
            OwnerKey::Declaration(declaration) => Some((*package, *declaration)),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let mut type_parameters = BTreeSet::new();
    let mut fields = BTreeSet::new();
    let mut cases = BTreeSet::new();
    let mut requirements = BTreeSet::new();
    let mut operations = BTreeSet::new();
    let mut ports = BTreeSet::new();
    for unit in units.values() {
        match &unit.payload {
            CompilationPayload::Record { fields: layouts } => {
                for layout in layouts {
                    fields.insert(unit.tables.fields[layout.field as usize]);
                }
            }
            CompilationPayload::Variant { cases: layouts } => {
                for layout in layouts {
                    cases.insert(unit.tables.cases[layout.case as usize]);
                }
            }
            CompilationPayload::Interface {
                operations: layouts,
            } => {
                for layout in layouts {
                    operations.insert(unit.tables.operations[layout.operation as usize]);
                }
            }
            CompilationPayload::Component {
                requirements: compiled_requirements,
                ports: compiled_ports,
            } => {
                for requirement in compiled_requirements {
                    requirements.insert(unit.tables.requirements[requirement.requirement as usize]);
                }
                for port in compiled_ports {
                    ports.insert(unit.tables.ports[port.port as usize]);
                }
            }
            CompilationPayload::External { signature, .. }
            | CompilationPayload::Function { signature, .. } => {
                type_parameters.extend(signature.type_parameters.iter().copied());
            }
            CompilationPayload::Constant { .. }
            | CompilationPayload::Test { .. }
            | CompilationPayload::Target { .. } => {}
        }
    }
    for unit in units.values() {
        if unit
            .tables
            .declarations
            .iter()
            .any(|reference| !declarations.contains(&(reference.package, reference.declaration)))
            || unit
                .tables
                .fields
                .iter()
                .any(|reference| !fields.contains(reference))
            || unit
                .tables
                .cases
                .iter()
                .any(|reference| !cases.contains(reference))
            || unit
                .tables
                .requirements
                .iter()
                .any(|reference| !requirements.contains(reference))
            || unit
                .tables
                .operations
                .iter()
                .any(|reference| !operations.contains(reference))
            || unit
                .tables
                .ports
                .iter()
                .any(|reference| !ports.contains(reference))
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_unit_relocation_missing",
                "compiler-unit relocation does not resolve inside the exact artifact package closure",
            ));
        }
    }
    Ok(ArtifactRelocations {
        declarations,
        type_parameters,
    })
}

struct TrackingObjectStore<'a> {
    objects: &'a BTreeMap<ObjectKey, Vec<u8>>,
    visited: RefCell<BTreeSet<ObjectKey>>,
}

impl<'a> TrackingObjectStore<'a> {
    fn new(objects: &'a BTreeMap<ObjectKey, Vec<u8>>) -> Self {
        Self {
            objects,
            visited: RefCell::new(BTreeSet::new()),
        }
    }

    fn visited(&self) -> BTreeSet<ObjectKey> {
        self.visited.borrow().clone()
    }

    fn visited_for_domain(&self, domain: ObjectDomain) -> BTreeSet<ObjectKey> {
        self.visited
            .borrow()
            .iter()
            .filter(|key| key.domain == domain)
            .copied()
            .collect()
    }
}

impl ImmutableObjectStore for TrackingObjectStore<'_> {
    fn read(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        let Some(bytes) = self.objects.get(&key) else {
            return Ok(None);
        };
        if bytes.len() > maximum_bytes {
            return Err(StoreError::new(
                StoreErrorClass::Resource,
                "artifact_object_read_limit",
                "artifact object exceeds the caller read bound",
            ));
        }
        key.verify(bytes)?;
        self.visited.borrow_mut().insert(key);
        work.objects_read = work.objects_read.saturating_add(1);
        work.bytes_read = work.bytes_read.saturating_add(bytes.len() as u64);
        Ok(Some(bytes.clone()))
    }

    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        work.catalog_lookups = work.catalog_lookups.saturating_add(1);
        if self.objects.contains_key(&key) {
            self.visited.borrow_mut().insert(key);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn stage(
        &mut self,
        _key: ObjectKey,
        _bytes: &[u8],
        _work: &mut StoreWork,
    ) -> Result<StageOutcome, StoreError> {
        Err(StoreError::new(
            StoreErrorClass::Input,
            "artifact_object_read_only",
            "loaded artifact object storage is immutable",
        ))
    }
}

fn required_object(
    store: &impl ImmutableObjectStore,
    key: ObjectKey,
    missing: &'static str,
    work: &mut StoreWork,
) -> Result<Vec<u8>, Diagnostic> {
    store
        .read(key, key.domain.maximum_bytes(), work)
        .map_err(store_diagnostic)?
        .ok_or_else(|| artifact_error(DiagnosticClass::Corrupt, "artifact_object_missing", missing))
}

fn read_u16(bytes: &[u8], offset: usize, code: &'static str) -> Result<u16, Diagnostic> {
    Ok(u16::from_be_bytes(read_array::<2>(bytes, offset, code)?))
}

fn read_u64(bytes: &[u8], offset: usize, code: &'static str) -> Result<u64, Diagnostic> {
    Ok(u64::from_be_bytes(read_array::<8>(bytes, offset, code)?))
}

fn read_array<const N: usize>(
    bytes: &[u8],
    offset: usize,
    code: &'static str,
) -> Result<[u8; N], Diagnostic> {
    let end = offset.checked_add(N).ok_or_else(|| bundle_overflow(code))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| bundle_truncated(code))?
        .try_into()
        .map_err(|_| bundle_truncated(code))
}

fn usize_from_u64(value: u64, code: &'static str) -> Result<usize, Diagnostic> {
    usize::try_from(value).map_err(|_| {
        artifact_error(
            DiagnosticClass::Resource,
            code,
            "artifact length does not fit this platform",
        )
    })
}

fn domain_digest(domain: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn bundle_overflow(code: &'static str) -> Diagnostic {
    artifact_error(
        DiagnosticClass::Corrupt,
        code,
        "artifact bundle coordinate arithmetic overflowed",
    )
}

fn bundle_truncated(code: &'static str) -> Diagnostic {
    artifact_error(
        DiagnosticClass::Corrupt,
        code,
        "artifact bundle is truncated inside a declared field",
    )
}

fn map_diagnostic(error: MapError) -> Diagnostic {
    artifact_error(
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

fn add_map_work(total: &mut MapWork, other: MapWork) {
    total.pages_read = total.pages_read.saturating_add(other.pages_read);
    total.pages_decoded = total.pages_decoded.saturating_add(other.pages_decoded);
    total.pages_encoded = total.pages_encoded.saturating_add(other.pages_encoded);
    total.pages_written = total.pages_written.saturating_add(other.pages_written);
    total.pages_reused = total.pages_reused.saturating_add(other.pages_reused);
    total.bytes_read = total.bytes_read.saturating_add(other.bytes_read);
    total.bytes_encoded = total.bytes_encoded.saturating_add(other.bytes_encoded);
    total.bytes_written = total.bytes_written.saturating_add(other.bytes_written);
    total.key_comparisons = total.key_comparisons.saturating_add(other.key_comparisons);
    total.entries_visited = total.entries_visited.saturating_add(other.entries_visited);
    total.differences_emitted = total
        .differences_emitted
        .saturating_add(other.differences_emitted);
    total.subtrees_skipped = total
        .subtrees_skipped
        .saturating_add(other.subtrees_skipped);
    total.entries_skipped = total.entries_skipped.saturating_add(other.entries_skipped);
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    artifact_error(
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

pub(crate) fn artifact_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
