//! Deterministic segmented Graph 9 artifact contract and strict standalone loader.

use super::manifest::{
    COMPILATION_MANIFEST_CONTRACT_VERSION, CompilationBinding, CompilationManifest,
    CompilationManifestDigest,
};
use super::unit::{
    BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_VERSION, CompilationPayload, CompilationUnit,
    CompiledParameter, CompiledPortImplementation, CompiledSignature,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    CaseReference, DeclarationPayload, DeclarationReference, DeclarationVisibility,
    EncodedOwnerKey, ExpressionOperation, ExternalVisibility, FieldReference, FunctionEffect,
    Idempotency, LocalValueReference, Name, OperationReference, OwnerKey, OwnerKind,
    OwnerObjectDigest, OwnerRecord, PackageId, PackageInterfaceDeclarationPayload,
    PackageInterfaceDigest, PackageInterfaceRecord, PackageRevisionDigest, ParameterParent,
    PortImplementation, PortReference, RequirementReference, ResourceLimit, SemanticStateDigest,
    TypeForm, TypeObject, TypeObjectDigest, decode_owner, decode_owner_binding, decode_type_object,
    encode_type_object,
};
use crate::platform::package::RunnerKind;
use crate::platform::package_interface::{
    PackageInterfaceValidation, build_package_interface, package_interface_digest,
    validate_package_interface,
};
use crate::platform::package_transport::{
    PackageRevision, validate_package_interface_closure, validate_package_revision_closure,
};
use crate::platform::persistent_map::{MapError, MapErrorClass, MapRoot, MapWork, PersistentMap};
use crate::platform::semantic_id::{
    BindingId, DeclarationId, HttpRouteId, ParameterId, RepositoryId, RevisionId, TargetId,
    TypeParameterId,
};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StageOutcome, StoreError, StoreErrorClass,
    StoreWork,
};
use crate::platform::storage::pack::{PackBuilder, PackId, PackMetadata};
use crate::platform::storage::page_store::ObjectPageReader;
use bincode::de::Decoder;
use bincode::error::DecodeError;
use bincode::{Decode, Encode};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const ARTIFACT_MANIFEST_CONTRACT_IDENTITY: &str = "lkjscript-artifact-manifest-14";
pub const ARTIFACT_BUNDLE_CONTRACT_IDENTITY: &str = "lkjscript-artifact-bundle-14";
pub const ARTIFACT_CONTRACT_VERSION: u16 = 14;
pub(crate) const ARTIFACT_MANIFEST_MAGIC: [u8; 8] = *b"LKJAMF14";
pub(crate) const ARTIFACT_BUNDLE_MAGIC: [u8; 8] = *b"LKJART14";
pub(crate) const ARTIFACT_BUNDLE_END_MAGIC: [u8; 8] = *b"LKJAEN14";
pub(crate) const ARTIFACT_MANIFEST_ENVELOPE_DOMAIN: &str =
    "lkjscript.artifact-manifest-envelope.v14";
pub(crate) const ARTIFACT_BUNDLE_DIGEST_DOMAIN: &str = "lkjscript.artifact-bundle.v14";
pub(crate) const ARTIFACT_BUNDLE_CHECKSUM_DOMAIN: &str = "lkjscript.artifact-bundle.complete.v14";
pub(crate) const ARTIFACT_CLOSURE_DIGEST_DOMAIN: &str = "lkjscript.artifact-object-closure.v14";
pub(crate) const MAXIMUM_ARTIFACT_MANIFEST_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAXIMUM_ARTIFACT_PACKAGES: usize = 10_000;
pub(crate) const MAXIMUM_ARTIFACT_RUNTIME_OWNERS: usize = 1_000_000;
pub(crate) const MAXIMUM_ARTIFACT_REFERENCE_OWNERS: u64 = 1_000_000;
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
pub struct ArtifactRuntimeOwner {
    pub owner: OwnerKey,
    pub kind: OwnerKind,
    pub object: OwnerObjectDigest,
}

#[derive(Clone, Debug, Encode, Eq, PartialEq)]
pub struct ArtifactPackage {
    pub repository_id: RepositoryId,
    pub package: PackageId,
    pub package_revision: PackageRevisionDigest,
    pub semantic_revision: RevisionId,
    pub semantic_state: SemanticStateDigest,
    pub interface: PackageInterfaceDigest,
    pub interface_owners: MapRoot,
    pub compilation: CompilationManifestDigest,
    pub runtime_owners: Vec<ArtifactRuntimeOwner>,
    /// Exact canonical declarations, expressions, and bindings used only by the independent
    /// reference tier. The Merkle map is cold artifact data and is not a runtime identity table.
    pub reference_owners: MapRoot,
}

impl<Context> Decode<Context> for ArtifactPackage {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            repository_id: RepositoryId::decode(decoder)?,
            package: PackageId::decode(decoder)?,
            package_revision: PackageRevisionDigest::decode(decoder)?,
            semantic_revision: RevisionId::decode(decoder)?,
            semantic_state: SemanticStateDigest::decode(decoder)?,
            interface: PackageInterfaceDigest::decode(decoder)?,
            interface_owners: MapRoot::decode(decoder)?,
            compilation: CompilationManifestDigest::decode(decoder)?,
            runtime_owners: decode_bounded_vec(
                decoder,
                MAXIMUM_ARTIFACT_RUNTIME_OWNERS,
                "artifact runtime-owner count",
            )?,
            reference_owners: MapRoot::decode(decoder)?,
        })
    }
}

#[derive(Clone, Debug, Encode, Eq, PartialEq)]
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

impl<Context> Decode<Context> for ArtifactManifest {
    fn decode<D: Decoder<Context = Context>>(decoder: &mut D) -> Result<Self, DecodeError> {
        Ok(Self {
            contract_version: u16::decode(decoder)?,
            graph_contract_version: u16::decode(decoder)?,
            compiler_contract_version: u16::decode(decoder)?,
            bytecode_contract_version: u16::decode(decoder)?,
            compilation_manifest_contract_version: u16::decode(decoder)?,
            root_package: PackageId::decode(decoder)?,
            packages: decode_bounded_vec(
                decoder,
                MAXIMUM_ARTIFACT_PACKAGES,
                "artifact package count",
            )?,
            closure: ArtifactClosureDigest::decode(decoder)?,
            object_count: u64::decode(decoder)?,
            object_bytes: u64::decode(decoder)?,
        })
    }
}

fn decode_bounded_vec<Context, T, D>(
    decoder: &mut D,
    maximum: usize,
    label: &'static str,
) -> Result<Vec<T>, DecodeError>
where
    T: Decode<Context>,
    D: Decoder<Context = Context>,
{
    let encoded_length = u64::decode(decoder)?;
    let length = usize::try_from(encoded_length)
        .map_err(|_| DecodeError::OutsideUsizeRange(encoded_length))?;
    if length > maximum {
        return Err(DecodeError::OtherString(format!(
            "{label} exceeds {maximum} before allocation"
        )));
    }
    decoder.claim_container_read::<T>(length)?;
    let mut values = Vec::with_capacity(length);
    for _ in 0..length {
        decoder.unclaim_bytes_read(std::mem::size_of::<T>());
        values.push(T::decode(decoder)?);
    }
    Ok(values)
}

const fn runtime_owner_kind(kind: OwnerKind) -> bool {
    matches!(
        kind,
        OwnerKind::TaskFunction
            | OwnerKind::TypeParameter
            | OwnerKind::Field
            | OwnerKind::Case
            | OwnerKind::Operation
            | OwnerKind::Parameter
            | OwnerKind::Requirement
            | OwnerKind::Port
            | OwnerKind::Target
            | OwnerKind::HttpRoute
    )
}

const fn reference_owner_kind(kind: OwnerKind) -> bool {
    matches!(
        kind,
        OwnerKind::External
            | OwnerKind::PureFunction
            | OwnerKind::TaskFunction
            | OwnerKind::Constant
            | OwnerKind::Test
            | OwnerKind::Binding
            | OwnerKind::Expression
    )
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
            if package.runtime_owners.len() > MAXIMUM_ARTIFACT_RUNTIME_OWNERS {
                return Err(artifact_error(
                    DiagnosticClass::Resource,
                    "artifact_manifest_runtime_owner_count",
                    "artifact package runtime-owner count exceeds the current implementation bound",
                ));
            }
            if package.repository_id.bytes() == [0; 16]
                || package.package.bytes() == [0; 16]
                || package.reference_owners.entries() > MAXIMUM_ARTIFACT_REFERENCE_OWNERS
                || package
                    .runtime_owners
                    .windows(2)
                    .any(|pair| pair[0].owner >= pair[1].owner)
                || package.runtime_owners.iter().any(|binding| {
                    !binding.kind.accepts_owner(binding.owner) || !runtime_owner_kind(binding.kind)
                })
            {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_manifest_package",
                    "artifact package identity or runtime-owner binding is invalid",
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

    pub(crate) fn runtime_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
        expected_kind: OwnerKind,
        work: &mut StoreWork,
    ) -> Result<OwnerRecord, Diagnostic> {
        let package = self.package(package).ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_runtime_owner_package_missing",
                "runtime-owner lookup names a package outside the artifact closure",
            )
        })?;
        let binding = package
            .runtime_owners
            .binary_search_by_key(&owner, |entry| entry.owner)
            .ok()
            .map(|index| &package.runtime_owners[index])
            .ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_runtime_owner_missing",
                    "runtime-owner lookup names no exact artifact metadata binding",
                )
            })?;
        if binding.kind != expected_kind {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_runtime_owner_kind",
                "runtime-owner metadata has another exact owner kind",
            ));
        }
        let key = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
        let bytes = self
            .read(key, ObjectDomain::Owner.maximum_bytes(), work)
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_runtime_owner_object_missing",
                    "runtime-owner metadata references a missing exact owner object",
                )
            })?;
        decode_owner(&bytes, owner, expected_kind, binding.object)
    }

    pub(crate) fn reference_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
        map_work: &mut MapWork,
        store_work: &mut StoreWork,
    ) -> Result<Option<OwnerRecord>, Diagnostic> {
        let package = self.package(package).ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_reference_package_missing",
                "reference-owner lookup names a package outside the exact artifact closure",
            )
        })?;
        let reader = ObjectPageReader::new(self);
        let binding = PersistentMap::from_root(package.reference_owners)
            .lookup(&reader, &EncodedOwnerKey::new(owner).bytes(), map_work)
            .map_err(map_diagnostic)?;
        store_work.add(reader.work());
        let Some(binding) = binding else {
            return Ok(None);
        };
        let binding = decode_owner_binding(&binding, owner)?;
        if !reference_owner_kind(binding.kind) {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_reference_owner_kind",
                "reference-owner map contains a non-executable owner kind",
            ));
        }
        let key = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
        let bytes = self
            .read(key, ObjectDomain::Owner.maximum_bytes(), store_work)
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_reference_owner_object_missing",
                    "reference-owner map names a missing exact canonical owner object",
                )
            })?;
        decode_owner(&bytes, owner, binding.kind, binding.object).map(Some)
    }
}

impl ImmutableObjectStore for LoadedArtifact {
    fn read(
        &self,
        key: ObjectKey,
        maximum_bytes: usize,
        work: &mut StoreWork,
    ) -> Result<Option<Vec<u8>>, StoreError> {
        work.catalog_lookups = work.catalog_lookups.saturating_add(1);
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
        work.objects_read = work.objects_read.saturating_add(1);
        work.bytes_read = work.bytes_read.saturating_add(bytes.len() as u64);
        Ok(Some(bytes.clone()))
    }

    fn contains(&self, key: ObjectKey, work: &mut StoreWork) -> Result<bool, StoreError> {
        work.catalog_lookups = work.catalog_lookups.saturating_add(1);
        Ok(self.objects.contains_key(&key))
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

#[derive(Debug)]
pub(crate) struct CanonicalDependencyArtifact {
    pub packages: Vec<ArtifactPackage>,
    pub objects: BTreeMap<ObjectKey, Vec<u8>>,
    pub work: ArtifactLoadWork,
}

/// Rebuilds every package interface from its validated logical entries and retains only the exact
/// executable closure reachable through the rebuilt roots. Dependency artifact page partitioning
/// is therefore never inherited by a newly linked artifact.
pub(crate) fn canonicalize_dependency_artifact_interfaces(
    artifact: &LoadedArtifact,
) -> Result<CanonicalDependencyArtifact, Diagnostic> {
    let mut packages = artifact.manifest.packages.clone();
    let mut objects = artifact.objects.clone();
    let mut work = ArtifactLoadWork::default();
    for package in &mut packages {
        let mut store_work = StoreWork::default();
        let interface = validate_package_interface(
            package.package,
            package.interface_owners,
            artifact,
            &mut store_work,
        )?;
        work.store.add(store_work);
        add_map_work(&mut work.map, interface.map_work);
        let mut types = BTreeMap::new();
        for (digest, object) in interface.type_objects {
            let (encoded_digest, bytes) = encode_type_object(&object)?;
            if encoded_digest != digest {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_dependency_interface_type_digest",
                    "dependency interface type changed during canonical encoding",
                ));
            }
            types.insert(digest, bytes);
        }
        let rebuilt = build_package_interface(&interface.owners, &types)?;
        if package_interface_digest(package.package, rebuilt.root.content_root())?
            != package.interface
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_dependency_interface_digest",
                "rebuilt dependency interface disagrees with its logical package commitment",
            ));
        }
        add_map_work(&mut work.map, rebuilt.map_work);
        work.store.add(rebuilt.store_work);
        for (key, bytes) in rebuilt.objects {
            match objects.entry(key) {
                std::collections::btree_map::Entry::Vacant(entry) => {
                    entry.insert(bytes);
                }
                std::collections::btree_map::Entry::Occupied(entry) if entry.get() == &bytes => {}
                std::collections::btree_map::Entry::Occupied(_) => {
                    return Err(artifact_error(
                        DiagnosticClass::Corrupt,
                        "artifact_dependency_interface_object_collision",
                        "canonical dependency interface collides with different immutable bytes",
                    ));
                }
            }
        }
        package.interface_owners = rebuilt.root;
    }
    let mut manifest = artifact.manifest.clone();
    manifest.packages = packages.clone();
    manifest.validate()?;
    let mut trace_work = ArtifactLoadWork::default();
    let reachable = trace_object_closure(&manifest, &objects, &mut trace_work)?;
    add_map_work(&mut work.map, trace_work.map);
    work.store.add(trace_work.store);
    objects.retain(|key, _| reachable.contains(key));
    work.objects = objects.len() as u64;
    work.object_bytes = objects.values().fold(0_u64, |total, bytes| {
        total.saturating_add(bytes.len() as u64)
    });
    Ok(CanonicalDependencyArtifact {
        packages,
        objects,
        work,
    })
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
                | ObjectDomain::PackageRevision
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeOwnerExpectation {
    ResourceFunction {
        parameters: Vec<ParameterId>,
        result: TypeObjectDigest,
        requirements: Vec<RequirementReference>,
    },
    TypeParameter {
        declaration: DeclarationId,
    },
    Field {
        declaration: DeclarationId,
        ty: TypeObjectDigest,
    },
    Case {
        declaration: DeclarationId,
        payload: Option<TypeObjectDigest>,
    },
    Operation {
        declaration: DeclarationId,
        parameters: Vec<ParameterId>,
        result: TypeObjectDigest,
        idempotency: Idempotency,
        external_visibility: ExternalVisibility,
    },
    Parameter {
        parent: ParameterParent,
        ty: TypeObjectDigest,
        use_mode: crate::platform::kernel::ParameterUse,
        resource_requirement: Option<RequirementReference>,
    },
    Requirement {
        declaration: DeclarationId,
        interface: DeclarationReference,
        operations: Vec<OperationReference>,
        limits: Vec<ResourceLimit>,
    },
    TaskRequirement,
    Port {
        declaration: DeclarationId,
        function_type: TypeObjectDigest,
        implementation: RuntimePortImplementation,
    },
    Target {
        component: DeclarationReference,
        port: Option<PortReference>,
        runner: crate::platform::package::RunnerKind,
    },
    HttpRoute {
        target: TargetId,
        method: String,
        path: String,
        port: PortReference,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimePortImplementation {
    Function(DeclarationReference),
    Expression,
}

impl RuntimeOwnerExpectation {
    pub(crate) const fn kind(&self) -> OwnerKind {
        match self {
            Self::ResourceFunction { .. } => OwnerKind::TaskFunction,
            Self::TypeParameter { .. } => OwnerKind::TypeParameter,
            Self::Field { .. } => OwnerKind::Field,
            Self::Case { .. } => OwnerKind::Case,
            Self::Operation { .. } => OwnerKind::Operation,
            Self::Parameter { .. } => OwnerKind::Parameter,
            Self::Requirement { .. } | Self::TaskRequirement { .. } => OwnerKind::Requirement,
            Self::Port { .. } => OwnerKind::Port,
            Self::Target { .. } => OwnerKind::Target,
            Self::HttpRoute { .. } => OwnerKind::HttpRoute,
        }
    }

    fn matches(&self, record: &OwnerRecord) -> bool {
        match (self, record) {
            (
                Self::ResourceFunction {
                    parameters,
                    result,
                    requirements,
                },
                OwnerRecord::Declaration(record),
            ) => {
                matches!(
                    (&record.visibility, &record.payload),
                    (
                        DeclarationVisibility::Private,
                        DeclarationPayload::Function(function),
                    ) if function.type_parameters.is_empty()
                        && function.parameters == *parameters
                        && function.result == *result
                        && matches!(
                            &function.effect,
                            FunctionEffect::Task {
                                requirements: actual,
                            } if actual == requirements
                        )
                )
            }
            (Self::TypeParameter { declaration }, OwnerRecord::TypeParameter(record)) => {
                record.declaration == *declaration
            }
            (Self::Field { declaration, ty }, OwnerRecord::Field(record)) => {
                record.declaration == *declaration && record.ty == *ty
            }
            (
                Self::Case {
                    declaration,
                    payload,
                },
                OwnerRecord::Case(record),
            ) => record.declaration == *declaration && record.payload == *payload,
            (
                Self::Operation {
                    declaration,
                    parameters,
                    result,
                    idempotency,
                    external_visibility,
                },
                OwnerRecord::Operation(record),
            ) => {
                record.declaration == *declaration
                    && record.parameters == *parameters
                    && record.result == *result
                    && record.idempotency == *idempotency
                    && record.external_visibility == *external_visibility
            }
            (
                Self::Parameter {
                    parent,
                    ty,
                    use_mode,
                    resource_requirement,
                },
                OwnerRecord::Parameter(record),
            ) => {
                record.parent == *parent
                    && record.ty == *ty
                    && record.use_mode == *use_mode
                    && record.resource_requirement == *resource_requirement
            }
            (
                Self::Requirement {
                    declaration,
                    interface,
                    operations,
                    limits,
                },
                OwnerRecord::Requirement(record),
            ) => {
                record.declaration == *declaration
                    && record.interface == *interface
                    && record.operations == *operations
                    && record.limits == *limits
            }
            (Self::TaskRequirement, OwnerRecord::Requirement(_)) => true,
            (
                Self::Port {
                    declaration,
                    function_type,
                    implementation,
                },
                OwnerRecord::Port(record),
            ) => {
                let implementation_matches = match (implementation, &record.implementation) {
                    (
                        RuntimePortImplementation::Function(expected),
                        PortImplementation::Function(actual),
                    ) => expected == actual,
                    (RuntimePortImplementation::Expression, PortImplementation::Expression(_)) => {
                        true
                    }
                    _ => false,
                };
                record.declaration == *declaration
                    && record.function_type == *function_type
                    && implementation_matches
            }
            (
                Self::Target {
                    component,
                    port,
                    runner,
                },
                OwnerRecord::Target(record),
            ) => record.component == *component && record.port == *port && record.runner == *runner,
            (
                Self::HttpRoute {
                    target,
                    method,
                    path,
                    port,
                },
                OwnerRecord::HttpRoute(record),
            ) => {
                record.target == *target
                    && record.method == *method
                    && record.path == *path
                    && record.port == *port
            }
            _ => false,
        }
    }
}

pub(crate) fn runtime_owner_expectations(
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
) -> Result<BTreeMap<(PackageId, OwnerKey), RuntimeOwnerExpectation>, Diagnostic> {
    let mut expected = BTreeMap::new();
    let mut current_package = None;
    let mut current_package_count = 0_usize;
    for ((package, owner), unit) in units {
        if current_package != Some(*package) {
            current_package = Some(*package);
            current_package_count = 0;
        }
        let before = expected.len();
        match &unit.payload {
            CompilationPayload::Record { fields } => {
                let declaration = declaration_owner(*owner, "record")?;
                for field in fields {
                    let reference =
                        table_value(&unit.tables.fields, field.field, "record field reference")?;
                    require_local_reference(*package, reference.package, "record field")?;
                    insert_runtime_expectation(
                        &mut expected,
                        (*package, OwnerKey::Field(reference.field)),
                        RuntimeOwnerExpectation::Field {
                            declaration,
                            ty: table_value(&unit.tables.types, field.ty, "record field type")?,
                        },
                    )?;
                }
            }
            CompilationPayload::Variant { cases } => {
                let declaration = declaration_owner(*owner, "variant")?;
                for case in cases {
                    let reference =
                        table_value(&unit.tables.cases, case.case, "variant case reference")?;
                    require_local_reference(*package, reference.package, "variant case")?;
                    insert_runtime_expectation(
                        &mut expected,
                        (*package, OwnerKey::Case(reference.case)),
                        RuntimeOwnerExpectation::Case {
                            declaration,
                            payload: case
                                .payload
                                .map(|index| {
                                    table_value(
                                        &unit.tables.types,
                                        index,
                                        "variant case payload type",
                                    )
                                })
                                .transpose()?,
                        },
                    )?;
                }
            }
            CompilationPayload::Interface { operations } => {
                let declaration = declaration_owner(*owner, "interface")?;
                for operation in operations {
                    let reference = table_value(
                        &unit.tables.operations,
                        operation.operation,
                        "interface operation reference",
                    )?;
                    require_local_reference(*package, reference.package, "interface operation")?;
                    let parameters = operation
                        .parameters
                        .iter()
                        .map(|parameter| parameter.parameter)
                        .collect::<Vec<_>>();
                    insert_runtime_expectation(
                        &mut expected,
                        (*package, OwnerKey::Operation(reference.operation)),
                        RuntimeOwnerExpectation::Operation {
                            declaration,
                            parameters: parameters.clone(),
                            result: table_value(
                                &unit.tables.types,
                                operation.result,
                                "interface operation result type",
                            )?,
                            idempotency: operation.idempotency,
                            external_visibility: operation.external_visibility,
                        },
                    )?;
                    for parameter in &operation.parameters {
                        insert_parameter_expectation(
                            &mut expected,
                            *package,
                            ParameterParent::Operation(reference.operation),
                            parameter,
                            unit,
                        )?;
                    }
                }
            }
            CompilationPayload::External { signature, .. } => {
                let declaration = declaration_owner(*owner, "function")?;
                insert_signature_expectations(
                    &mut expected,
                    *package,
                    declaration,
                    signature,
                    unit,
                )?;
            }
            CompilationPayload::Function { signature, .. } => {
                let declaration = declaration_owner(*owner, "function")?;
                insert_signature_expectations(
                    &mut expected,
                    *package,
                    declaration,
                    signature,
                    unit,
                )?;
                if signature
                    .parameters
                    .iter()
                    .any(|parameter| parameter.resource_requirement.is_some())
                {
                    let parameters = signature
                        .parameters
                        .iter()
                        .map(|parameter| parameter.parameter)
                        .collect();
                    let result = table_value(
                        &unit.tables.types,
                        signature.result,
                        "resource function result type",
                    )?;
                    let requirements = signature
                        .task_requirements
                        .iter()
                        .map(|requirement| {
                            table_value(
                                &unit.tables.requirements,
                                *requirement,
                                "resource function requirement",
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    insert_runtime_expectation(
                        &mut expected,
                        (*package, OwnerKey::Declaration(declaration)),
                        RuntimeOwnerExpectation::ResourceFunction {
                            parameters,
                            result,
                            requirements,
                        },
                    )?;
                }
            }
            CompilationPayload::Component {
                requirements,
                ports,
            } => {
                let declaration = declaration_owner(*owner, "component")?;
                for requirement in requirements {
                    let reference = table_value(
                        &unit.tables.requirements,
                        requirement.requirement,
                        "component requirement reference",
                    )?;
                    require_local_reference(*package, reference.package, "component requirement")?;
                    let operations = requirement
                        .operations
                        .iter()
                        .map(|index| {
                            table_value(
                                &unit.tables.operations,
                                *index,
                                "component requirement operation",
                            )
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    insert_runtime_expectation(
                        &mut expected,
                        (*package, OwnerKey::Requirement(reference.requirement)),
                        RuntimeOwnerExpectation::Requirement {
                            declaration,
                            interface: table_value(
                                &unit.tables.declarations,
                                requirement.interface,
                                "component requirement interface",
                            )?,
                            operations,
                            limits: requirement.limits.clone(),
                        },
                    )?;
                }
                for port in ports {
                    let reference =
                        table_value(&unit.tables.ports, port.port, "component port reference")?;
                    require_local_reference(*package, reference.package, "component port")?;
                    let implementation = match &port.implementation {
                        CompiledPortImplementation::Function(index) => {
                            RuntimePortImplementation::Function(table_value(
                                &unit.tables.declarations,
                                *index,
                                "component port function",
                            )?)
                        }
                        CompiledPortImplementation::Expression(_) => {
                            RuntimePortImplementation::Expression
                        }
                    };
                    insert_runtime_expectation(
                        &mut expected,
                        (*package, OwnerKey::Port(reference.port)),
                        RuntimeOwnerExpectation::Port {
                            declaration,
                            function_type: table_value(
                                &unit.tables.types,
                                port.function_type,
                                "component port function type",
                            )?,
                            implementation,
                        },
                    )?;
                }
            }
            CompilationPayload::Target {
                component,
                port,
                routes,
                runner,
            } => {
                let OwnerKey::Target(target) = owner else {
                    return Err(artifact_error(
                        DiagnosticClass::Corrupt,
                        "artifact_target_owner",
                        "target compiler payload is not bound to a target owner",
                    ));
                };
                insert_runtime_expectation(
                    &mut expected,
                    (*package, OwnerKey::Target(*target)),
                    RuntimeOwnerExpectation::Target {
                        component: table_value(
                            &unit.tables.declarations,
                            *component,
                            "target component",
                        )?,
                        port: port
                            .map(|port| table_value(&unit.tables.ports, port, "target port"))
                            .transpose()?,
                        runner: *runner,
                    },
                )?;
                for route in routes {
                    insert_runtime_expectation(
                        &mut expected,
                        (*package, OwnerKey::HttpRoute(route.route)),
                        RuntimeOwnerExpectation::HttpRoute {
                            target: *target,
                            method: route.method.clone(),
                            path: route.path.clone(),
                            port: table_value(&unit.tables.ports, route.port, "HTTP route port")?,
                        },
                    )?;
                }
            }
            CompilationPayload::Constant { .. } | CompilationPayload::Test { .. } => {}
        }
        current_package_count = current_package_count
            .checked_add(expected.len().saturating_sub(before))
            .ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Resource,
                    "artifact_runtime_owner_count",
                    "runtime-owner expectation count overflowed its platform domain",
                )
            })?;
        if current_package_count > MAXIMUM_ARTIFACT_RUNTIME_OWNERS {
            return Err(artifact_error(
                DiagnosticClass::Resource,
                "artifact_runtime_owner_count",
                "compiler units require more runtime owners than the artifact contract permits",
            ));
        }
    }
    Ok(expected)
}

fn insert_signature_expectations(
    expected: &mut BTreeMap<(PackageId, OwnerKey), RuntimeOwnerExpectation>,
    package: PackageId,
    declaration: DeclarationId,
    signature: &CompiledSignature,
    unit: &CompilationUnit,
) -> Result<(), Diagnostic> {
    for parameter in &signature.type_parameters {
        insert_runtime_expectation(
            expected,
            (package, OwnerKey::TypeParameter(*parameter)),
            RuntimeOwnerExpectation::TypeParameter { declaration },
        )?;
    }
    for parameter in &signature.parameters {
        insert_parameter_expectation(
            expected,
            package,
            ParameterParent::Function(declaration),
            parameter,
            unit,
        )?;
    }
    for requirement in &signature.task_requirements {
        let reference = table_value(
            &unit.tables.requirements,
            *requirement,
            "task signature requirement reference",
        )?;
        require_local_reference(package, reference.package, "task signature requirement")?;
        insert_runtime_expectation(
            expected,
            (package, OwnerKey::Requirement(reference.requirement)),
            RuntimeOwnerExpectation::TaskRequirement,
        )?;
    }
    Ok(())
}

fn insert_parameter_expectation(
    expected: &mut BTreeMap<(PackageId, OwnerKey), RuntimeOwnerExpectation>,
    package: PackageId,
    parent: ParameterParent,
    parameter: &CompiledParameter,
    unit: &CompilationUnit,
) -> Result<(), Diagnostic> {
    insert_runtime_expectation(
        expected,
        (package, OwnerKey::Parameter(parameter.parameter)),
        RuntimeOwnerExpectation::Parameter {
            parent,
            ty: table_value(&unit.tables.types, parameter.ty, "runtime parameter type")?,
            use_mode: parameter.use_mode,
            resource_requirement: parameter
                .resource_requirement
                .map(|requirement| {
                    table_value(
                        &unit.tables.requirements,
                        requirement,
                        "runtime parameter requirement",
                    )
                })
                .transpose()?,
        },
    )
}

fn insert_runtime_expectation(
    expected: &mut BTreeMap<(PackageId, OwnerKey), RuntimeOwnerExpectation>,
    key: (PackageId, OwnerKey),
    value: RuntimeOwnerExpectation,
) -> Result<(), Diagnostic> {
    match expected.entry(key) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(value);
        }
        std::collections::btree_map::Entry::Occupied(_)
            if matches!(&value, RuntimeOwnerExpectation::TaskRequirement) =>
        {
            // A task signature names the exact requirement it may use. Multiple task signatures
            // may name one component-owned requirement, so the signature contributes a closure
            // requirement rather than a second semantic definition.
        }
        std::collections::btree_map::Entry::Occupied(mut entry)
            if matches!(entry.get(), RuntimeOwnerExpectation::TaskRequirement)
                && matches!(&value, RuntimeOwnerExpectation::Requirement { .. }) =>
        {
            // The component unit carries the complete defining semantics and supersedes the
            // closure-only expectation regardless of stable owner ordering.
            entry.insert(value);
        }
        std::collections::btree_map::Entry::Occupied(_) => {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_runtime_owner_duplicate",
                "one stable runtime owner is defined by multiple compiler-unit records",
            ));
        }
    }
    Ok(())
}

fn declaration_owner(owner: OwnerKey, label: &'static str) -> Result<DeclarationId, Diagnostic> {
    let OwnerKey::Declaration(declaration) = owner else {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_runtime_declaration_owner",
            format!("{label} compiler payload is not bound to a declaration owner"),
        ));
    };
    Ok(declaration)
}

fn require_local_reference(
    package: PackageId,
    referenced_package: PackageId,
    label: &'static str,
) -> Result<(), Diagnostic> {
    if package != referenced_package {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_runtime_owner_package",
            format!("{label} definition uses a foreign package identity"),
        ));
    }
    Ok(())
}

fn table_value<T: Copy>(values: &[T], index: u32, label: &'static str) -> Result<T, Diagnostic> {
    values.get(index as usize).copied().ok_or_else(|| {
        artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_runtime_table_index",
            format!("{label} index is outside its compiler-unit table"),
        )
    })
}

fn validate_object_closure(
    manifest: &ArtifactManifest,
    objects: &BTreeMap<ObjectKey, Vec<u8>>,
    work: &mut ArtifactLoadWork,
) -> Result<(), Diagnostic> {
    let visited = trace_object_closure(manifest, objects, work)?;
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

fn trace_object_closure(
    manifest: &ArtifactManifest,
    objects: &BTreeMap<ObjectKey, Vec<u8>>,
    work: &mut ArtifactLoadWork,
) -> Result<BTreeSet<ObjectKey>, Diagnostic> {
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
    let logical =
        validate_package_revision_closure(&store, root.package_revision, None, &mut store_work)?;
    if logical.root_revision.package != manifest.root_package {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_root_package_binding",
            "artifact root logical revision has another package identity",
        ));
    }
    let reachable_revisions = store.visited_for_domain(ObjectDomain::PackageRevision);
    let expected_revisions = manifest
        .packages
        .iter()
        .map(|package| {
            ObjectKey::from_digest(
                ObjectDomain::PackageRevision,
                package.package_revision.bytes(),
            )
        })
        .collect::<BTreeSet<_>>();
    if reachable_revisions != expected_revisions
        || logical.revisions.len() != manifest.packages.len()
    {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_package_closure",
            "artifact packages do not equal the exact logical package-revision closure",
        ));
    }

    let mut units = BTreeMap::<(PackageId, OwnerKey), CompilationUnit>::new();
    let mut type_roots = BTreeSet::new();
    let mut blobs = BTreeMap::new();
    let mut interfaces = BTreeMap::new();
    for package in &manifest.packages {
        let revision = logical
            .revisions
            .get(&package.package_revision)
            .ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_package_revision_binding",
                    "artifact package manifest names a revision outside the logical closure",
                )
            })?;
        let interface = validate_package_interface(
            package.package,
            package.interface_owners,
            &store,
            &mut store_work,
        )?;
        if package_interface_digest(package.package, package.interface_owners.content_root())?
            != package.interface
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_package_interface_digest",
                "artifact package interface root disagrees with its logical content digest",
            ));
        }
        interfaces.insert(package.package_revision, interface);
        let compilation_key = package.compilation.object_key();
        let compilation_bytes = required_object(
            &store,
            compilation_key,
            "artifact compilation manifest is missing",
            &mut store_work,
        )?;
        let compilation = CompilationManifest::decode(&compilation_bytes, package.compilation)?;
        if package.repository_id != revision.revision.repository_id
            || package.package != revision.package
            || package.semantic_revision != revision.revision.revision_id()?
            || package.semantic_state != revision.revision.semantic_state
            || package.interface != revision.interface
            || compilation.repository_id != package.repository_id
            || compilation.package_id != package.package
            || compilation.revision != package.semantic_revision
            || compilation.package_revision != package.package_revision
            || compilation.semantic_state != package.semantic_state
            || compilation.package_interface != package.interface
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_package_compilation_binding",
                "logical package revision, deterministic interface, and compilation do not bind one package meaning",
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
    }
    validate_package_interface_closure(&logical.revisions, &interfaces)?;
    let runtime_owners = validate_runtime_owners(manifest, &units, &store, &mut store_work)?;
    validate_reference_owners(
        manifest,
        &units,
        &runtime_owners,
        &store,
        &mut store_work,
        &mut work.map,
    )?;
    let relocations = validate_unit_relocations(&units)?;

    let mut types = BTreeMap::new();
    while let Some(digest) = type_roots.pop_first() {
        if types.contains_key(&digest) {
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
        types.insert(digest, object);
    }
    validate_artifact_session_relations(manifest, &units, &runtime_owners, &interfaces, &types)?;
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
    Ok(store.visited())
}

enum ArtifactSessionRecord {
    Local(OwnerRecord),
    Interface(PackageInterfaceRecord),
}

struct ArtifactSessionRead<'a> {
    types: &'a BTreeMap<TypeObjectDigest, TypeObject>,
    units: &'a BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    runtime_owners: &'a BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    interfaces: BTreeMap<PackageId, &'a PackageInterfaceValidation>,
}

impl ArtifactSessionRead<'_> {
    fn record(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<ArtifactSessionRecord, Diagnostic> {
        if let Some(record) = self.runtime_owners.get(&(package, owner)) {
            return Ok(ArtifactSessionRecord::Local(record.clone()));
        }
        self.interfaces
            .get(&package)
            .and_then(|interface| interface.owners.get(&owner))
            .map(|owner| ArtifactSessionRecord::Interface(owner.record.clone()))
            .ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_session_nominal_owner",
                    "session relation references a nominal owner outside the exact artifact closure",
                )
            })
    }
}

impl crate::platform::session::SessionShapeRead for ArtifactSessionRead<'_> {
    fn type_object(&self, digest: TypeObjectDigest) -> Result<TypeObject, Diagnostic> {
        self.types.get(&digest).cloned().ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_session_type",
                "session relation references a type outside the exact artifact closure",
            )
        })
    }

    fn nominal_shape(
        &self,
        declaration: DeclarationReference,
    ) -> Result<crate::platform::session::SessionNominalShape, Diagnostic> {
        if let Some(unit) = self.units.get(&(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
        )) {
            match &unit.payload {
                CompilationPayload::Record { fields } => {
                    let mut shape = BTreeMap::new();
                    for field in fields {
                        let reference = table_value(
                            &unit.tables.fields,
                            field.field,
                            "session record field reference",
                        )?;
                        let ty =
                            table_value(&unit.tables.types, field.ty, "session record field type")?;
                        let name = match self
                            .record(declaration.package, OwnerKey::Field(reference.field))?
                        {
                            ArtifactSessionRecord::Local(OwnerRecord::Field(record)) => {
                                if record.ty != ty {
                                    return Err(artifact_error(
                                        DiagnosticClass::Corrupt,
                                        "artifact_session_nominal_member",
                                        "compiled session field type disagrees with runtime metadata",
                                    ));
                                }
                                record.name
                            }
                            ArtifactSessionRecord::Interface(PackageInterfaceRecord::Field(
                                record,
                            )) => {
                                if record.ty != ty {
                                    return Err(artifact_error(
                                        DiagnosticClass::Corrupt,
                                        "artifact_session_nominal_member",
                                        "compiled session field type disagrees with package interface",
                                    ));
                                }
                                record.name
                            }
                            _ => {
                                return Err(artifact_error(
                                    DiagnosticClass::Corrupt,
                                    "artifact_session_nominal_member",
                                    "compiled session field names another owner kind",
                                ));
                            }
                        };
                        if shape.insert(name, ty).is_some() {
                            return Err(artifact_error(
                                DiagnosticClass::Corrupt,
                                "artifact_session_nominal_member",
                                "compiled session record repeats one field name",
                            ));
                        }
                    }
                    return Ok(crate::platform::session::SessionNominalShape::Record(shape));
                }
                CompilationPayload::Variant { cases } => {
                    let mut shape = BTreeMap::new();
                    for case in cases {
                        let reference = table_value(
                            &unit.tables.cases,
                            case.case,
                            "session variant case reference",
                        )?;
                        let payload = case
                            .payload
                            .map(|index| {
                                table_value(
                                    &unit.tables.types,
                                    index,
                                    "session variant payload type",
                                )
                            })
                            .transpose()?;
                        let name = match self
                            .record(declaration.package, OwnerKey::Case(reference.case))?
                        {
                            ArtifactSessionRecord::Local(OwnerRecord::Case(record)) => {
                                if record.payload != payload {
                                    return Err(artifact_error(
                                        DiagnosticClass::Corrupt,
                                        "artifact_session_nominal_member",
                                        "compiled session case type disagrees with runtime metadata",
                                    ));
                                }
                                record.name
                            }
                            ArtifactSessionRecord::Interface(PackageInterfaceRecord::Case(
                                record,
                            )) => {
                                if record.payload != payload {
                                    return Err(artifact_error(
                                        DiagnosticClass::Corrupt,
                                        "artifact_session_nominal_member",
                                        "compiled session case type disagrees with package interface",
                                    ));
                                }
                                record.name
                            }
                            _ => {
                                return Err(artifact_error(
                                    DiagnosticClass::Corrupt,
                                    "artifact_session_nominal_member",
                                    "compiled session case names another owner kind",
                                ));
                            }
                        };
                        if shape.insert(name, payload).is_some() {
                            return Err(artifact_error(
                                DiagnosticClass::Corrupt,
                                "artifact_session_nominal_member",
                                "compiled session variant repeats one case name",
                            ));
                        }
                    }
                    return Ok(crate::platform::session::SessionNominalShape::Variant(
                        shape,
                    ));
                }
                _ => {}
            }
        }
        let record = self.record(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
        )?;
        let (record, members) = match record {
            ArtifactSessionRecord::Local(OwnerRecord::Declaration(record)) => {
                match record.payload {
                    DeclarationPayload::Record { fields } => (
                        true,
                        fields.into_iter().map(OwnerKey::Field).collect::<Vec<_>>(),
                    ),
                    DeclarationPayload::Variant { cases } => (
                        false,
                        cases.into_iter().map(OwnerKey::Case).collect::<Vec<_>>(),
                    ),
                    _ => {
                        return Err(artifact_error(
                            DiagnosticClass::Corrupt,
                            "artifact_session_nominal_kind",
                            "session nominal declaration is not a record or variant",
                        ));
                    }
                }
            }
            ArtifactSessionRecord::Interface(PackageInterfaceRecord::Declaration(record)) => {
                match record.payload {
                    PackageInterfaceDeclarationPayload::Record { fields } => (
                        true,
                        fields.into_iter().map(OwnerKey::Field).collect::<Vec<_>>(),
                    ),
                    PackageInterfaceDeclarationPayload::Variant { cases } => (
                        false,
                        cases.into_iter().map(OwnerKey::Case).collect::<Vec<_>>(),
                    ),
                    _ => {
                        return Err(artifact_error(
                            DiagnosticClass::Corrupt,
                            "artifact_session_nominal_kind",
                            "session interface declaration is not a record or variant",
                        ));
                    }
                }
            }
            _ => {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_session_nominal_kind",
                    "session nominal identity names another owner kind",
                ));
            }
        };
        let mut shape = BTreeMap::new();
        for owner in members {
            let (name, ty) = match self.record(declaration.package, owner)? {
                ArtifactSessionRecord::Local(OwnerRecord::Field(field)) if record => {
                    (field.name, Some(field.ty))
                }
                ArtifactSessionRecord::Interface(PackageInterfaceRecord::Field(field))
                    if record =>
                {
                    (field.name, Some(field.ty))
                }
                ArtifactSessionRecord::Local(OwnerRecord::Case(case)) if !record => {
                    (case.name, case.payload)
                }
                ArtifactSessionRecord::Interface(PackageInterfaceRecord::Case(case)) if !record => {
                    (case.name, case.payload)
                }
                _ => {
                    return Err(artifact_error(
                        DiagnosticClass::Corrupt,
                        "artifact_session_nominal_member",
                        "session nominal member has another owner kind",
                    ));
                }
            };
            if shape.insert(name, ty).is_some() {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_session_nominal_member",
                    "session nominal shape repeats one member name",
                ));
            }
        }
        if record {
            Ok(crate::platform::session::SessionNominalShape::Record(
                shape
                    .into_iter()
                    .map(|(name, ty)| {
                        ty.map(|ty| (name, ty)).ok_or_else(|| {
                            artifact_error(
                                DiagnosticClass::Corrupt,
                                "artifact_session_nominal_member",
                                "session record field omits its type",
                            )
                        })
                    })
                    .collect::<Result<BTreeMap<Name, TypeObjectDigest>, Diagnostic>>()?,
            ))
        } else {
            Ok(crate::platform::session::SessionNominalShape::Variant(
                shape,
            ))
        }
    }
}

fn artifact_standard_session_declarations(
    read: &ArtifactSessionRead<'_>,
) -> Result<crate::platform::session::SessionStandardDeclarations, Diagnostic> {
    let package =
        crate::platform::builtin_standard::builtin_standard_package().map_err(|error| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_session_standard_package",
                format!(
                    "built-in standard package identity is invalid: {}",
                    error.code
                ),
            )
        })?;
    let interface = read.interfaces.get(&package).ok_or_else(|| {
        artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_session_standard_package",
            "interactive artifact omits the canonical standard package interface",
        )
    })?;
    let names = [
        crate::platform::session::SESSION_EVENT_NAME,
        crate::platform::session::SESSION_MESSAGE_KIND_NAME,
        crate::platform::session::SESSION_DECISION_KIND_NAME,
        crate::platform::session::SESSION_OUTBOUND_NAME,
        crate::platform::session::SESSION_REJECT_NAME,
        crate::platform::session::SESSION_CLOSE_NAME,
    ];
    let mut declarations = BTreeMap::new();
    for (owner, value) in &interface.owners {
        let (OwnerKey::Declaration(declaration), PackageInterfaceRecord::Declaration(record)) =
            (owner, &value.record)
        else {
            continue;
        };
        if names.contains(&record.name.as_str())
            && declarations
                .insert(record.name.as_str().to_owned(), *declaration)
                .is_some()
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_session_standard_declaration",
                "canonical standard interface repeats a session declaration name",
            ));
        }
    }
    let reference = |name: &'static str| {
        declarations
            .get(name)
            .copied()
            .map(|declaration| DeclarationReference {
                package,
                declaration,
            })
            .ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_session_standard_declaration",
                    format!("canonical standard interface omits {name}"),
                )
            })
    };
    Ok(crate::platform::session::SessionStandardDeclarations {
        event: reference(crate::platform::session::SESSION_EVENT_NAME)?,
        message_kind: reference(crate::platform::session::SESSION_MESSAGE_KIND_NAME)?,
        decision_kind: reference(crate::platform::session::SESSION_DECISION_KIND_NAME)?,
        outbound: reference(crate::platform::session::SESSION_OUTBOUND_NAME)?,
        reject: reference(crate::platform::session::SESSION_REJECT_NAME)?,
        close: reference(crate::platform::session::SESSION_CLOSE_NAME)?,
    })
}

fn validate_artifact_session_relations(
    manifest: &ArtifactManifest,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    runtime_owners: &BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    interfaces: &BTreeMap<PackageRevisionDigest, PackageInterfaceValidation>,
    types: &BTreeMap<TypeObjectDigest, TypeObject>,
) -> Result<(), Diagnostic> {
    let package_interfaces = manifest
        .packages
        .iter()
        .map(|package| {
            interfaces
                .get(&package.package_revision)
                .map(|interface| (package.package, interface))
                .ok_or_else(|| {
                    artifact_error(
                        DiagnosticClass::Corrupt,
                        "artifact_session_package_interface",
                        "artifact session validation lost one package interface",
                    )
                })
        })
        .collect::<Result<BTreeMap<_, _>, Diagnostic>>()?;
    let read = ArtifactSessionRead {
        types,
        units,
        runtime_owners,
        interfaces: package_interfaces,
    };
    let interactive = runtime_owners
        .iter()
        .filter_map(|((package, _), record)| {
            let OwnerRecord::Target(target) = record else {
                return None;
            };
            (target.runner == RunnerKind::Interactive).then_some((*package, target))
        })
        .collect::<Vec<_>>();
    if interactive.is_empty() {
        return Ok(());
    }
    let standard = artifact_standard_session_declarations(&read)?;
    for (package, target) in interactive {
        let port_reference = target.port.ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_session_port_missing",
                "interactive target has no exact runtime port",
            )
        })?;
        if port_reference.package != package {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_session_port_package",
                "interactive target port belongs to another package",
            ));
        }
        let Some(OwnerRecord::Port(port)) =
            runtime_owners.get(&(package, OwnerKey::Port(port_reference.port)))
        else {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_session_port",
                "interactive target references a missing or foreign runtime port",
            ));
        };
        crate::platform::session::validate_session_function_type(
            &read,
            standard,
            port.function_type,
        )
        .map_err(|error| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_session_relation",
                format!(
                    "interactive port failed independent relation reconstruction: {}",
                    error.code
                ),
            )
        })?;
    }
    Ok(())
}

fn validate_runtime_owners(
    manifest: &ArtifactManifest,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    store: &TrackingObjectStore<'_>,
    work: &mut StoreWork,
) -> Result<BTreeMap<(PackageId, OwnerKey), OwnerRecord>, Diagnostic> {
    let expected = runtime_owner_expectations(units)?;
    let package_ids = manifest
        .packages
        .iter()
        .map(|package| package.package)
        .collect::<BTreeSet<_>>();
    if expected
        .keys()
        .any(|(package, _)| !package_ids.contains(package))
    {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_runtime_owner_package_missing",
            "compiler units define runtime metadata for a package outside the artifact closure",
        ));
    }
    let mut expected_counts = BTreeMap::<PackageId, usize>::new();
    for package in expected.keys().map(|(package, _)| *package) {
        *expected_counts.entry(package).or_default() += 1;
    }
    let mut visited = BTreeSet::new();
    let mut records = BTreeMap::new();
    for package in &manifest.packages {
        let expected_count = expected_counts.get(&package.package).copied().unwrap_or(0);
        if expected_count != package.runtime_owners.len() {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_runtime_owner_count",
                "artifact runtime metadata does not cover the exact compiler-unit boundary owner set",
            ));
        }
        for binding in &package.runtime_owners {
            let key = (package.package, binding.owner);
            let expectation = expected.get(&key).ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_runtime_owner_unexpected",
                    "artifact package names runtime metadata not required by its compiler units",
                )
            })?;
            if binding.kind != expectation.kind() || !visited.insert(key) {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_runtime_owner_binding",
                    "artifact runtime-owner kind or uniqueness disagrees with compiler units",
                ));
            }
            let owner_key = ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
            let bytes = required_object(
                store,
                owner_key,
                "artifact runtime metadata references a missing owner object",
                work,
            )?;
            let record = decode_owner(&bytes, binding.owner, binding.kind, binding.object)?;
            if !expectation.matches(&record) {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_runtime_owner_semantics",
                    "artifact runtime-owner metadata disagrees with its exact compiler-unit semantics",
                ));
            }
            if records.insert(key, record).is_some() {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_runtime_owner_duplicate",
                    "artifact repeats one exact runtime-owner binding",
                ));
            }
        }
    }
    if visited.len() != expected.len() {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_runtime_owner_incomplete",
            "artifact runtime metadata omitted one exact compiler-unit boundary owner",
        ));
    }
    Ok(records)
}

fn validate_reference_owners(
    manifest: &ArtifactManifest,
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    runtime_owners: &BTreeMap<(PackageId, OwnerKey), OwnerRecord>,
    store: &TrackingObjectStore<'_>,
    store_work: &mut StoreWork,
    total_map_work: &mut MapWork,
) -> Result<(), Diagnostic> {
    let mut records = BTreeMap::<(PackageId, OwnerKey), OwnerRecord>::new();
    for package in &manifest.packages {
        let reader = ObjectPageReader::new(store);
        let mut map_work = MapWork::default();
        let mut captured = None;
        let result = PersistentMap::from_root(package.reference_owners).for_each(
            &reader,
            &mut map_work,
            |key, value| {
                let operation = (|| {
                    let owner = EncodedOwnerKey::decode(key)?;
                    let binding = decode_owner_binding(value, owner)?;
                    if !reference_owner_kind(binding.kind) {
                        return Err(artifact_error(
                            DiagnosticClass::Corrupt,
                            "artifact_reference_owner_kind",
                            "reference-execution closure contains a non-executable owner kind",
                        ));
                    }
                    let owner_key =
                        ObjectKey::from_digest(ObjectDomain::Owner, binding.object.bytes());
                    let bytes = required_object(
                        store,
                        owner_key,
                        "reference-execution closure names a missing owner object",
                        store_work,
                    )?;
                    let record = decode_owner(&bytes, owner, binding.kind, binding.object)?;
                    if records.insert((package.package, owner), record).is_some() {
                        return Err(artifact_error(
                            DiagnosticClass::Corrupt,
                            "artifact_reference_owner_duplicate",
                            "reference-execution closure repeats one exact package owner",
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
                            code: "artifact_reference_iteration_stop",
                            message: "reference-execution map iteration stopped after an exact diagnostic"
                                .to_owned(),
                        })
                    }
                }
            },
        );
        add_map_work(total_map_work, map_work);
        store_work.add(reader.work());
        if let Some(diagnostic) = captured {
            return Err(diagnostic);
        }
        result.map_err(map_diagnostic)?;
    }

    let mut pending = BTreeSet::<(PackageId, OwnerKey)>::new();
    for ((package, owner), unit) in units {
        if reference_compilation_payload(&unit.payload) {
            pending.insert((*package, *owner));
        }
    }
    for ((package, _), record) in runtime_owners {
        if let OwnerRecord::Port(port) = record {
            match port.implementation {
                PortImplementation::Function(reference) => {
                    require_reference_callable(units, reference)?;
                }
                PortImplementation::Expression(expression) => {
                    pending.insert((*package, OwnerKey::Expression(expression)));
                }
            }
        }
    }

    let mut reachable = BTreeSet::new();
    let mut package_counts = BTreeMap::<PackageId, u64>::new();
    while let Some(key @ (package, _owner)) = pending.pop_first() {
        if !reachable.insert(key) {
            continue;
        }
        let count = package_counts.entry(package).or_default();
        *count = count.saturating_add(1);
        if *count > MAXIMUM_ARTIFACT_REFERENCE_OWNERS {
            return Err(artifact_error(
                DiagnosticClass::Resource,
                "artifact_reference_owner_count",
                "reference-execution closure exceeds its per-package owner bound",
            ));
        }
        let record = records.get(&key).ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_reference_owner_missing",
                "reference-execution closure omits a required exact canonical owner",
            )
        })?;
        match record {
            OwnerRecord::Declaration(declaration) => {
                let unit = units.get(&key).ok_or_else(|| {
                    artifact_error(
                        DiagnosticClass::Corrupt,
                        "artifact_reference_declaration_unit",
                        "reference declaration has no exact compiler unit in the artifact closure",
                    )
                })?;
                if !reference_payload_matches(&unit.payload, &declaration.payload) {
                    return Err(artifact_error(
                        DiagnosticClass::Corrupt,
                        "artifact_reference_declaration_payload",
                        "reference declaration kind disagrees with its exact compiler unit",
                    ));
                }
                pending.extend(
                    record
                        .expression_roots()
                        .into_iter()
                        .map(|expression| (package, OwnerKey::Expression(expression))),
                );
            }
            OwnerRecord::Expression(expression) => {
                pending.extend(
                    expression
                        .children()
                        .into_iter()
                        .map(|child| (package, OwnerKey::Expression(child.expression))),
                );
                pending.extend(
                    reference_expression_bindings(&expression.operation)
                        .into_iter()
                        .map(|binding| (package, OwnerKey::Binding(binding))),
                );
                for declaration in reference_expression_declarations(&expression.operation) {
                    require_reference_callable(units, declaration)?;
                }
            }
            OwnerRecord::Binding(_) => {
                pending.extend(
                    record
                        .expression_roots()
                        .into_iter()
                        .map(|expression| (package, OwnerKey::Expression(expression))),
                );
            }
            _ => {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_reference_owner_kind",
                    "reference-execution closure contains a non-executable owner record",
                ));
            }
        }
    }

    let observed = records.keys().copied().collect::<BTreeSet<_>>();
    if observed != reachable {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_reference_owner_unreachable",
            "reference-execution closure contains an owner outside its exact executable roots",
        ));
    }
    for package in &manifest.packages {
        let observed = package_counts.get(&package.package).copied().unwrap_or(0);
        if observed != package.reference_owners.entries() {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_reference_owner_count",
                "reference-execution map count disagrees with its reachable executable owners",
            ));
        }
    }
    Ok(())
}

fn reference_compilation_payload(payload: &CompilationPayload) -> bool {
    matches!(
        payload,
        CompilationPayload::External { .. }
            | CompilationPayload::Function { .. }
            | CompilationPayload::Constant { .. }
            | CompilationPayload::Test { .. }
    )
}

fn reference_callable_payload(payload: &CompilationPayload) -> bool {
    matches!(
        payload,
        CompilationPayload::External { .. }
            | CompilationPayload::Function { .. }
            | CompilationPayload::Constant { .. }
    )
}

fn reference_payload_matches(
    compiled: &CompilationPayload,
    canonical: &DeclarationPayload,
) -> bool {
    matches!(
        (compiled, canonical),
        (
            CompilationPayload::External { .. },
            DeclarationPayload::External(_)
        ) | (
            CompilationPayload::Function { .. },
            DeclarationPayload::Function(_)
        ) | (
            CompilationPayload::Constant { .. },
            DeclarationPayload::Constant { .. }
        ) | (
            CompilationPayload::Test { .. },
            DeclarationPayload::Test { .. }
        )
    )
}

fn require_reference_callable(
    units: &BTreeMap<(PackageId, OwnerKey), CompilationUnit>,
    reference: DeclarationReference,
) -> Result<(), Diagnostic> {
    let key = (
        reference.package,
        OwnerKey::Declaration(reference.declaration),
    );
    if units
        .get(&key)
        .is_none_or(|unit| !reference_callable_payload(&unit.payload))
    {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_reference_callable_missing",
            "canonical reference execution names no exact callable compiler unit in the linked artifact",
        ));
    }
    Ok(())
}

fn reference_expression_declarations(operation: &ExpressionOperation) -> Vec<DeclarationReference> {
    match operation {
        ExpressionOperation::Constant { declaration } => vec![*declaration],
        ExpressionOperation::Call { function, .. }
        | ExpressionOperation::FunctionValue { function, .. } => vec![*function],
        _ => Vec::new(),
    }
}

fn reference_expression_bindings(operation: &ExpressionOperation) -> Vec<BindingId> {
    let mut bindings = Vec::new();
    match operation {
        ExpressionOperation::Local {
            value:
                LocalValueReference::LexicalBinding(binding)
                | LocalValueReference::MatchPayload(binding)
                | LocalValueReference::TransactionBinding(binding),
        } => bindings.push(*binding),
        ExpressionOperation::Let {
            bindings: declared, ..
        } => bindings.extend(declared.iter().copied()),
        ExpressionOperation::Match { arms, .. } => bindings.extend(
            arms.iter()
                .filter_map(|arm| arm.payload_binding)
                .collect::<Vec<_>>(),
        ),
        ExpressionOperation::Transaction { binding, .. } => bindings.push(*binding),
        _ => {}
    }
    bindings
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
                for requirement in &signature.task_requirements {
                    requirements.insert(table_value(
                        &unit.tables.requirements,
                        *requirement,
                        "task signature requirement relocation",
                    )?);
                }
            }
            CompilationPayload::Constant { .. }
            | CompilationPayload::Test { .. }
            | CompilationPayload::Target { .. } => {}
        }
    }
    for unit in units.values() {
        let missing = unit
            .tables
            .declarations
            .iter()
            .find(|reference| !declarations.contains(&(reference.package, reference.declaration)))
            .map(|reference| format!("declaration {reference:?}"))
            .or_else(|| {
                unit.tables
                    .fields
                    .iter()
                    .find(|reference| !fields.contains(reference))
                    .map(|reference| format!("field {reference:?}"))
            })
            .or_else(|| {
                unit.tables
                    .cases
                    .iter()
                    .find(|reference| !cases.contains(reference))
                    .map(|reference| format!("case {reference:?}"))
            })
            .or_else(|| {
                unit.tables
                    .requirements
                    .iter()
                    .find(|reference| !requirements.contains(reference))
                    .map(|reference| format!("requirement {reference:?}"))
            })
            .or_else(|| {
                unit.tables
                    .operations
                    .iter()
                    .find(|reference| !operations.contains(reference))
                    .map(|reference| format!("operation {reference:?}"))
            })
            .or_else(|| {
                unit.tables
                    .ports
                    .iter()
                    .find(|reference| !ports.contains(reference))
                    .map(|reference| format!("port {reference:?}"))
            });
        if let Some(missing) = missing {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_unit_relocation_missing",
                format!(
                    "compiler unit {:?} has unresolved {missing} relocation inside the exact artifact package closure",
                    unit.source
                ),
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
