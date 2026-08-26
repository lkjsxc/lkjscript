//! Strict frozen artifact-4 loading for the isolated service and worker runtime.
//!
//! An artifact contains independently integrity-protected package objects. Each package object
//! embeds a canonical graph root, its packed module shards, and the exact accepted revision and
//! receipt. This module is read-only in production; current Graph 5 builds use compiler artifact
//! contract 10 instead.

use super::contract::registry::{
    ARTIFACT_DOMAIN, ARTIFACT_MAGIC, PACKAGE_ARTIFACT_DOMAIN as PACKAGE_DOMAIN,
    PACKAGE_ARTIFACT_MAGIC as PACKAGE_MAGIC,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::graph::GraphRoot;
#[cfg(test)]
use super::graph::{DependencyBinding, ModuleObjectRef, TargetBinding};
#[cfg(test)]
use super::language::Declaration;
use super::meaning::MeaningModule;
#[cfg(test)]
use super::meaning::{GRAPH_CONTRACT_VERSION, MemberIdentity};
use super::package::{PackageId, Target};
use super::packed;
#[cfg(test)]
use super::revision::{
    RECEIPT_CONTRACT_VERSION, REVISION_CONTRACT_VERSION, ReceiptStatus, RevisionCore,
    ValidationFacts,
};
use super::revision::{RevisionRecord, TransactionReceipt};
use super::semantic::{ExactGraphDependency, ValidatedPackage, validate_graph_package};
use super::semantic_digest::ArtifactDigest;
#[cfg(test)]
use super::semantic_digest::{SemanticDiffDigest, TransactionDigest};
#[cfg(test)]
use super::semantic_fact::build_semantic_certificate;
use super::semantic_id::RevisionId;
#[cfg(test)]
use super::semantic_id::{RepositoryId, TargetId};
#[cfg(test)]
use super::semantic_summary::build_module_summary;
use bincode::{Decode, Encode};
#[cfg(test)]
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const ARTIFACT_CONTRACT_VERSION: u16 = 4;
pub const PACKAGE_ARTIFACT_CONTRACT_VERSION: u16 = 3;
pub const MAXIMUM_ARTIFACT_BYTES: usize = 128 * 1_048_576;
pub const MAXIMUM_ARTIFACT_PACKAGES: usize = 1_024;
const MAXIMUM_PACKAGE_ARTIFACT_BYTES: usize = 128 * 1_048_576;

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReceipt {
    pub contract_version: u16,
    pub artifact_digest: String,
    pub root_package_artifact: ArtifactDigest,
    pub root_package_id: PackageId,
    pub root_revision_digest: String,
    pub package_count: usize,
    pub module_count: usize,
    pub bytes: usize,
}

#[derive(Clone, Debug)]
pub struct LoadedArtifact {
    pub artifact_digest: String,
    pub root_package_artifact: ArtifactDigest,
    pub root_package_id: PackageId,
    pub root_revision_digest: String,
    pub root_revision: RevisionId,
    pub packages: BTreeMap<PackageId, ValidatedPackage>,
    pub package_artifacts: BTreeMap<PackageId, ArtifactDigest>,
    pub(crate) graph_roots: BTreeMap<PackageId, GraphRoot>,
    pub(crate) package_objects: BTreeMap<ArtifactDigest, Vec<u8>>,
}

impl LoadedArtifact {
    pub fn root(&self) -> Result<&ValidatedPackage, Diagnostic> {
        self.packages.get(&self.root_package_id).ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_root_missing",
                "validated artifact has no root package",
            )
        })
    }

    pub(crate) fn graph_root(&self, package: &PackageId) -> Result<&GraphRoot, Diagnostic> {
        self.graph_roots.get(package).ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_graph_root_missing",
                "validated artifact has no canonical graph root for a package",
            )
        })
    }

    pub(crate) fn root_graph(&self) -> Result<&GraphRoot, Diagnostic> {
        self.graph_root(&self.root_package_id)
    }

    pub fn target(&self, name: &str) -> Result<&Target, Diagnostic> {
        self.root()?
            .descriptor
            .targets
            .iter()
            .find(|target| target.name == name)
            .ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Source,
                    "artifact_target_missing",
                    format!("artifact has no target '{name}'"),
                )
            })
    }

    pub fn package_object(&self, digest: ArtifactDigest) -> Option<&[u8]> {
        self.package_objects.get(&digest).map(Vec::as_slice)
    }
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct ArtifactBundle {
    contract_version: u16,
    root: ArtifactDigest,
    objects: Vec<ArtifactObject>,
}

#[derive(Decode, Encode, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ArtifactObject {
    digest: ArtifactDigest,
    bytes: Vec<u8>,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
pub(crate) struct GraphPackageArtifact {
    pub(crate) contract_version: u16,
    pub(crate) revision: RevisionRecord,
    pub(crate) receipt: TransactionReceipt,
    pub(crate) root: GraphRoot,
    pub(crate) modules: Vec<MeaningModule>,
}

impl GraphPackageArtifact {
    #[cfg(test)]
    fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate_shape()?;
        packed::encode(
            PACKAGE_MAGIC,
            PACKAGE_DOMAIN,
            self,
            MAXIMUM_PACKAGE_ARTIFACT_BYTES,
        )
    }

    fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = packed::decode(
            bytes,
            PACKAGE_MAGIC,
            PACKAGE_DOMAIN,
            MAXIMUM_PACKAGE_ARTIFACT_BYTES,
        )?;
        value.validate_shape()?;
        Ok(value)
    }

    fn validate_shape(&self) -> Result<(), Diagnostic> {
        if self.contract_version != PACKAGE_ARTIFACT_CONTRACT_VERSION {
            return Err(artifact_error(
                DiagnosticClass::Source,
                "artifact_package_contract",
                "package artifact uses an unknown contract",
            ));
        }
        self.root.validate_modules(&self.modules)?;
        if self.revision.core.repository_id != self.root.repository_id
            || self.revision.core.root != self.root.digest()?
            || self.receipt.repository_id != self.root.repository_id
            || self.receipt.result != self.revision.revision
            || self.receipt.transaction != self.revision.core.transaction
            || self.receipt.semantic_diff != self.revision.core.semantic_diff
            || self.revision.receipt != self.receipt.digest()?
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_package_revision",
                "package graph, revision, and receipt bindings are inconsistent",
            ));
        }
        RevisionRecord::decode(&self.revision.encode()?)?;
        TransactionReceipt::decode(&self.receipt.encode()?)?;
        Ok(())
    }

    pub(crate) fn dependencies(&self) -> impl Iterator<Item = ArtifactDigest> + '_ {
        self.root
            .dependencies
            .iter()
            .map(|dependency| dependency.artifact)
    }
}

#[cfg(test)]
#[derive(Clone)]
struct BuiltPackage {
    digest: ArtifactDigest,
    bytes: Vec<u8>,
    revision: RevisionId,
}

/// Independent test oracle for reconstructing an initial graph from textual fixtures. It is not
/// compiled into the public library and cannot publish maintained authority.
#[cfg(test)]
pub(crate) fn build_artifact(
    root: &ValidatedPackage,
    packages: &[&ValidatedPackage],
) -> Result<(Vec<u8>, ArtifactReceipt), Diagnostic> {
    let by_id = validate_build_closure(root, packages)?;
    let mut pending = by_id.clone();
    let mut built = BTreeMap::<PackageId, BuiltPackage>::new();
    while !pending.is_empty() {
        let ready = pending
            .iter()
            .filter(|(_, package)| {
                package
                    .descriptor
                    .dependencies
                    .iter()
                    .all(|dependency| built.contains_key(&dependency.package_id))
            })
            .map(|(package, _)| package.clone())
            .collect::<Vec<_>>();
        if ready.is_empty() {
            return Err(artifact_error(
                DiagnosticClass::Semantic,
                "artifact_dependency_cycle",
                "package dependency graph contains a cycle",
            ));
        }
        for package_id in ready {
            let package = pending.remove(&package_id).ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Infrastructure,
                    "artifact_build_pending",
                    "ready package disappeared during artifact construction",
                )
            })?;
            let object = migration_package_artifact(package, &built)?;
            let bytes = object.encode()?;
            let digest = ArtifactDigest::of(&bytes);
            built.insert(
                package_id,
                BuiltPackage {
                    digest,
                    bytes,
                    revision: object.revision.revision,
                },
            );
        }
    }
    let root_object = built
        .get(&root.descriptor.package_id)
        .ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Infrastructure,
                "artifact_build_root",
                "built package closure lost its root",
            )
        })?
        .clone();
    let objects = built
        .into_values()
        .map(|object| ArtifactObject {
            digest: object.digest,
            bytes: object.bytes,
        })
        .collect();
    encode_bundle(root_object.digest, objects)
}

#[cfg(test)]
fn encode_bundle(
    root: ArtifactDigest,
    mut objects: Vec<ArtifactObject>,
) -> Result<(Vec<u8>, ArtifactReceipt), Diagnostic> {
    objects.sort();
    let bundle = ArtifactBundle {
        contract_version: ARTIFACT_CONTRACT_VERSION,
        root,
        objects,
    };
    validate_bundle_shape(&bundle)?;
    let bytes = packed::encode(
        ARTIFACT_MAGIC,
        ARTIFACT_DOMAIN,
        &bundle,
        MAXIMUM_ARTIFACT_BYTES,
    )?;
    let digest = ArtifactDigest::of(&bytes);
    let loaded = load_bundle_objects(root, &bundle.objects, digest)?;
    let module_count = loaded
        .packages
        .values()
        .try_fold(0usize, |count, package| {
            count.checked_add(package.modules.len())
        })
        .ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Resource,
                "artifact_module_count",
                "artifact module count overflowed",
            )
        })?;
    let package_count = loaded.packages.len();
    let receipt = ArtifactReceipt {
        contract_version: ARTIFACT_CONTRACT_VERSION,
        artifact_digest: digest.to_string(),
        root_package_artifact: root,
        root_package_id: loaded.root_package_id,
        root_revision_digest: loaded.root_revision_digest,
        package_count,
        module_count,
        bytes: bytes.len(),
    };
    Ok((bytes, receipt))
}

pub fn load_artifact(bytes: &[u8]) -> Result<LoadedArtifact, Diagnostic> {
    if bytes.len() > MAXIMUM_ARTIFACT_BYTES + 50 {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_too_large",
            format!("artifact exceeds {MAXIMUM_ARTIFACT_BYTES} payload bytes"),
        ));
    }
    if bytes.get(..8) != Some(ARTIFACT_MAGIC.as_slice()) {
        return Err(artifact_error(
            DiagnosticClass::Source,
            "artifact_contract",
            "artifact does not use the supported frozen service contract",
        ));
    }
    let bundle: ArtifactBundle = packed::decode(
        bytes,
        ARTIFACT_MAGIC,
        ARTIFACT_DOMAIN,
        MAXIMUM_ARTIFACT_BYTES,
    )?;
    validate_bundle_shape(&bundle)?;
    load_bundle_objects(bundle.root, &bundle.objects, ArtifactDigest::of(bytes))
}

fn validate_bundle_shape(bundle: &ArtifactBundle) -> Result<(), Diagnostic> {
    if bundle.contract_version != ARTIFACT_CONTRACT_VERSION {
        return Err(artifact_error(
            DiagnosticClass::Source,
            "artifact_contract",
            format!(
                "artifact contract {} is not supported frozen contract {ARTIFACT_CONTRACT_VERSION}",
                bundle.contract_version
            ),
        ));
    }
    if bundle.objects.is_empty() || bundle.objects.len() > MAXIMUM_ARTIFACT_PACKAGES {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_package_count",
            format!("artifact must contain 1 through {MAXIMUM_ARTIFACT_PACKAGES} package objects"),
        ));
    }
    if bundle.objects.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_object_order",
            "artifact package objects are not unique and canonically ordered",
        ));
    }
    if !bundle
        .objects
        .iter()
        .any(|object| object.digest == bundle.root)
    {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_root_missing",
            "artifact root package object is absent",
        ));
    }
    for object in &bundle.objects {
        if object.bytes.len() > MAXIMUM_PACKAGE_ARTIFACT_BYTES + 50
            || ArtifactDigest::of(&object.bytes) != object.digest
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_object_digest",
                "artifact package object exceeds its bound or has a foreign digest",
            ));
        }
    }
    Ok(())
}

fn load_bundle_objects(
    root_digest: ArtifactDigest,
    objects: &[ArtifactObject],
    bundle_digest: ArtifactDigest,
) -> Result<LoadedArtifact, Diagnostic> {
    let mut decoded = BTreeMap::new();
    let mut object_bytes = BTreeMap::new();
    for object in objects {
        if ArtifactDigest::of(&object.bytes) != object.digest {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_object_digest",
                "artifact package object digest does not match its bytes",
            ));
        }
        if decoded
            .insert(object.digest, GraphPackageArtifact::decode(&object.bytes)?)
            .is_some()
        {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_object_duplicate",
                "artifact repeats a package object digest",
            ));
        }
        object_bytes.insert(object.digest, object.bytes.clone());
    }
    let reachable = reachable_objects(root_digest, &decoded)?;
    if reachable.len() != decoded.len() {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_object_foreign",
            "artifact contains a package object outside the root dependency closure",
        ));
    }

    let mut pending = decoded;
    let mut validated_by_digest = BTreeMap::<ArtifactDigest, ValidatedPackage>::new();
    let mut artifacts_by_package = BTreeMap::new();
    let mut graph_roots = BTreeMap::new();
    while !pending.is_empty() {
        let ready = pending
            .iter()
            .filter(|(_, package)| {
                package
                    .root
                    .dependencies
                    .iter()
                    .all(|dependency| validated_by_digest.contains_key(&dependency.artifact))
            })
            .map(|(digest, _)| *digest)
            .collect::<Vec<_>>();
        if ready.is_empty() {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_dependency_cycle",
                "artifact package dependency graph contains a cycle",
            ));
        }
        for digest in ready {
            let package = pending.remove(&digest).ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Infrastructure,
                    "artifact_decode_pending",
                    "ready package object disappeared",
                )
            })?;
            let exact = package
                .root
                .dependencies
                .iter()
                .map(|dependency| {
                    let supplied =
                        validated_by_digest
                            .get(&dependency.artifact)
                            .ok_or_else(|| {
                                artifact_error(
                                    DiagnosticClass::Corrupt,
                                    "artifact_dependency_missing",
                                    "ready package dependency disappeared",
                                )
                            })?;
                    Ok(ExactGraphDependency {
                        alias: &dependency.alias,
                        package: supplied,
                        artifact: dependency.artifact,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?;
            let validated = validate_graph_package(
                &package.root,
                package.modules.clone(),
                &exact,
                Some(package.revision.revision),
            )?;
            if artifacts_by_package
                .insert(package.root.package_id.clone(), digest)
                .is_some()
            {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_package_duplicate",
                    "artifact contains two objects for one package identity",
                ));
            }
            graph_roots.insert(package.root.package_id.clone(), package.root);
            validated_by_digest.insert(digest, validated);
        }
    }
    let root = validated_by_digest.get(&root_digest).ok_or_else(|| {
        artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_root_missing",
            "validated artifact lost its root package",
        )
    })?;
    let root_package_id = root.descriptor.package_id.clone();
    let root_revision = root.accepted_revision.ok_or_else(|| {
        artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_root_revision",
            "artifact root package has no accepted revision",
        )
    })?;
    let packages = validated_by_digest
        .into_values()
        .map(|package| (package.descriptor.package_id.clone(), package))
        .collect();
    Ok(LoadedArtifact {
        artifact_digest: bundle_digest.to_string(),
        root_package_artifact: root_digest,
        root_package_id,
        root_revision_digest: root_revision.to_string(),
        root_revision,
        packages,
        package_artifacts: artifacts_by_package,
        graph_roots,
        package_objects: object_bytes,
    })
}

fn reachable_objects(
    root: ArtifactDigest,
    objects: &BTreeMap<ArtifactDigest, GraphPackageArtifact>,
) -> Result<BTreeSet<ArtifactDigest>, Diagnostic> {
    let mut reachable = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(digest) = pending.pop() {
        if !reachable.insert(digest) {
            continue;
        }
        let object = objects.get(&digest).ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_dependency_missing",
                format!("artifact closure omits package object '{digest}'"),
            )
        })?;
        pending.extend(object.dependencies());
        if reachable.len() > MAXIMUM_ARTIFACT_PACKAGES {
            return Err(artifact_error(
                DiagnosticClass::Resource,
                "artifact_package_count",
                "artifact dependency traversal exceeded its package bound",
            ));
        }
    }
    Ok(reachable)
}

#[cfg(test)]
fn validate_build_closure<'a>(
    root: &ValidatedPackage,
    packages: &'a [&'a ValidatedPackage],
) -> Result<BTreeMap<PackageId, &'a ValidatedPackage>, Diagnostic> {
    if packages.is_empty() || packages.len() > MAXIMUM_ARTIFACT_PACKAGES {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_package_count",
            format!("artifact closure must contain 1 through {MAXIMUM_ARTIFACT_PACKAGES} packages"),
        ));
    }
    let mut by_id = BTreeMap::new();
    for package in packages {
        if by_id
            .insert(package.descriptor.package_id.clone(), *package)
            .is_some()
        {
            return Err(artifact_error(
                DiagnosticClass::Semantic,
                "artifact_package_duplicate",
                "artifact closure repeats a package identity",
            ));
        }
    }
    if !by_id.contains_key(&root.descriptor.package_id) {
        return Err(artifact_error(
            DiagnosticClass::Semantic,
            "artifact_root_absent",
            "artifact closure omits its root package",
        ));
    }
    for package in by_id.values() {
        for dependency in &package.descriptor.dependencies {
            let supplied = by_id.get(&dependency.package_id).ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Semantic,
                    "artifact_dependency_missing",
                    format!(
                        "package '{}' omits dependency '{}' from the artifact closure",
                        package.descriptor.name, dependency.alias
                    ),
                )
            })?;
            if supplied.revision_digest != dependency.revision_digest {
                return Err(artifact_error(
                    DiagnosticClass::Semantic,
                    "artifact_dependency_identity",
                    format!(
                        "package '{}' dependency '{}' has a foreign semantic revision",
                        package.descriptor.name, dependency.alias
                    ),
                ));
            }
        }
    }
    Ok(by_id)
}

#[cfg(test)]
fn migration_package_artifact(
    package: &ValidatedPackage,
    built: &BTreeMap<PackageId, BuiltPackage>,
) -> Result<GraphPackageArtifact, Diagnostic> {
    let repository_id = RepositoryId::migrate(&package.descriptor.package_id.bytes(), 1);
    let mut modules = package
        .modules
        .iter()
        .map(|module| {
            let mut semantic_module = module.module.clone();
            super::meaning::normalize_module_spans(&mut semantic_module);
            MeaningModule {
                graph_contract_version: GRAPH_CONTRACT_VERSION,
                module_id: module.module_id,
                module: semantic_module,
                declarations: module.declaration_identities.clone(),
                relations: module.relations.clone(),
                documentation: Vec::new(),
                annotations: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    modules.sort_by_key(|module| module.module_id);
    let mut module_references = modules
        .iter()
        .map(|module| {
            Ok(ModuleObjectRef {
                id: module.module_id,
                name: module.module.name.clone(),
                object: module.digest()?,
            })
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    module_references.sort();
    let mut dependencies = package
        .descriptor
        .dependencies
        .iter()
        .map(|dependency| {
            let exact = built.get(&dependency.package_id).ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Infrastructure,
                    "artifact_dependency_build_order",
                    format!("dependency '{}' was not built first", dependency.alias),
                )
            })?;
            Ok(DependencyBinding {
                alias: dependency.alias.clone(),
                package_id: dependency.package_id.clone(),
                semantic_revision: exact.revision,
                artifact: exact.digest,
            })
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    dependencies.sort();
    let mut targets = package
        .descriptor
        .targets
        .iter()
        .enumerate()
        .map(|(index, target)| migration_target(package, target, index))
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    targets.sort();
    let root = GraphRoot {
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id,
        package_id: package.descriptor.package_id.clone(),
        package_name: package.descriptor.name.clone(),
        modules: module_references,
        dependencies,
        targets,
        tombstones: Vec::new(),
    };
    root.validate_modules(&modules)?;
    let root_bytes = root.encode()?;
    let transaction = TransactionDigest::of(&root_bytes);
    let semantic_diff = SemanticDiffDigest::of(&root_bytes);
    let summaries = modules
        .iter()
        .map(|module| build_module_summary(&root.package_id, module))
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    let semantic_certificate = build_semantic_certificate(&summaries)?;
    let core = RevisionCore {
        contract_version: REVISION_CONTRACT_VERSION,
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id,
        parents: Vec::new(),
        root: root.digest()?,
        semantic_certificate,
        semantic_diff,
        transaction,
    };
    let revision = core.revision_id()?;
    let module_count = u64::try_from(modules.len()).map_err(|_| count_limit("module"))?;
    let declaration_count = modules.iter().try_fold(0u64, |total, module| {
        total.checked_add(u64::try_from(module.declarations.len()).ok()?)
    });
    let receipt = TransactionReceipt {
        contract_version: RECEIPT_CONTRACT_VERSION,
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id,
        status: ReceiptStatus::ImportAccepted,
        base: None,
        result: revision,
        transaction,
        idempotency_key: None,
        semantic_diff,
        affected_owners: Vec::new(),
        validation: ValidationFacts {
            profile: "source_import_full_graph_and_reconstruction".to_owned(),
            graph_valid: true,
            full_oracle_equal: true,
            modules_checked: module_count,
            declarations_checked: declaration_count.ok_or_else(|| count_limit("declaration"))?,
        },
        intent: Some("one-time source authority import".to_owned()),
    };
    let revision = RevisionRecord::new(core, receipt.digest()?)?;
    Ok(GraphPackageArtifact {
        contract_version: PACKAGE_ARTIFACT_CONTRACT_VERSION,
        revision,
        receipt,
        root,
        modules,
    })
}

#[cfg(test)]
fn migration_target(
    package: &ValidatedPackage,
    target: &Target,
    index: usize,
) -> Result<TargetBinding, Diagnostic> {
    let (module_name, component_name) = target.component.rsplit_once('.').ok_or_else(|| {
        artifact_error(
            DiagnosticClass::Semantic,
            "artifact_target_component",
            format!("target '{}' component is not qualified", target.name),
        )
    })?;
    let module = package
        .modules
        .iter()
        .find(|module| module.module.name == module_name)
        .ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Semantic,
                "artifact_target_module",
                format!("target '{}' module is absent", target.name),
            )
        })?;
    let (declaration, identity) = module
        .module
        .declarations
        .iter()
        .zip(&module.declaration_identities)
        .find(|(declaration, _)| declaration.name() == component_name)
        .ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Semantic,
                "artifact_target_component",
                format!("target '{}' component is absent", target.name),
            )
        })?;
    if !matches!(declaration, Declaration::Component(_)) {
        return Err(artifact_error(
            DiagnosticClass::Semantic,
            "artifact_target_component_kind",
            format!("target '{}' owner is not a component", target.name),
        ));
    }
    let port = identity
        .members
        .iter()
        .find_map(|member| match member {
            MemberIdentity::Port { id, name } if name == &target.port => Some(*id),
            _ => None,
        })
        .ok_or_else(|| {
            artifact_error(
                DiagnosticClass::Semantic,
                "artifact_target_port",
                format!("target '{}' port is absent", target.name),
            )
        })?;
    let mut seed = package.descriptor.package_id.bytes().to_vec();
    seed.extend_from_slice(b"migration-target");
    Ok(TargetBinding {
        id: TargetId::migrate(
            &seed,
            u64::try_from(index)
                .map_err(|_| count_limit("target"))?
                .checked_add(1)
                .ok_or_else(|| count_limit("target"))?,
        ),
        name: target.name.clone(),
        component_module: module.module_id,
        component: identity.id,
        port,
        runner: target.runner,
    })
}

#[cfg(test)]
fn count_limit(label: &str) -> Diagnostic {
    artifact_error(
        DiagnosticClass::Resource,
        "artifact_count_limit",
        format!("artifact {label} count cannot be represented"),
    )
}

fn artifact_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{SourceLimits, decode_package, parse_source, validate_package_documents};

    fn package() -> ValidatedPackage {
        let descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"sample","modules":[{"name":"main","path":"src/main.lkj"}],"dependencies":[],"targets":[{"name":"run","component":"main.App","port":"main","runner":"command"}]}"#,
        )
        .expect("descriptor");
        let document = parse_source(
            "src/main.lkj",
            b"(module main (export App) (fn identity ((value Text)) Text value) (component App (port main (Function (Text) Text) (function identity))))\n",
            SourceLimits::default(),
        )
        .expect("source");
        validate_package_documents(descriptor, vec![document], &[]).expect("package")
    }

    #[test]
    fn deterministic_graph_round_trip_has_no_source_payload() {
        let package = package();
        let (first, receipt) = build_artifact(&package, &[&package]).expect("build artifact");
        let (second, _) = build_artifact(&package, &[&package]).expect("repeat build");
        assert_eq!(first, second);
        assert_eq!(receipt.package_count, 1);
        assert!(!first.windows(7).any(|window| window == b"(module"));
        let loaded = load_artifact(&first).expect("load artifact");
        assert_eq!(
            loaded.root_revision,
            loaded
                .root()
                .expect("root")
                .accepted_revision
                .expect("accepted revision")
        );
        assert_eq!(loaded.root().expect("root").descriptor.targets.len(), 1);

        let mut corrupt = first;
        corrupt[18] ^= 1;
        let error = load_artifact(&corrupt).expect_err("checksum must reject corruption");
        assert_eq!(error.code, "packed_checksum");
    }

    #[test]
    fn predecessor_contract_rejects() {
        let error = load_artifact(br#"{"contract_version":1}"#)
            .expect_err("source-containing predecessor rejects");
        assert_eq!(error.code, "artifact_contract");
    }
}
