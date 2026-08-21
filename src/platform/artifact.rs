//! Deterministic package-closure artifact for components and every runner topology.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::package::{Dependency, ModuleLocator, PackageDescriptor, PackageId, RunnerKind, Target};
use super::semantic::{ExactDependency, ValidatedPackage, validate_package_documents};
use super::syntax::{SourceLimits, parse_source};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, BTreeSet};

pub const ARTIFACT_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_ARTIFACT_BYTES: usize = 128 * 1_048_576;
pub const MAXIMUM_ARTIFACT_PACKAGES: usize = 1_024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReceipt {
    pub contract_version: u16,
    pub artifact_digest: String,
    pub root_package_id: PackageId,
    pub root_revision_digest: String,
    pub package_count: usize,
    pub module_count: usize,
    pub bytes: usize,
}

#[derive(Clone, Debug)]
pub struct LoadedArtifact {
    pub artifact_digest: String,
    pub root_package_id: PackageId,
    pub root_revision_digest: String,
    pub packages: BTreeMap<PackageId, ValidatedPackage>,
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactEnvelope {
    contract_version: u16,
    artifact_digest: String,
    root_package_id: PackageId,
    root_revision_digest: String,
    packages: Vec<ArtifactPackage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactCore {
    contract_version: u16,
    root_package_id: PackageId,
    root_revision_digest: String,
    packages: Vec<ArtifactPackage>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactPackage {
    package_id: PackageId,
    name: String,
    revision_digest: String,
    package_artifact_digest: String,
    modules: Vec<ArtifactModule>,
    dependencies: Vec<ArtifactDependency>,
    targets: Vec<ArtifactTarget>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactModule {
    name: String,
    source: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactDependency {
    alias: String,
    package_id: PackageId,
    revision_digest: String,
    artifact_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactTarget {
    name: String,
    component: String,
    port: String,
    runner: RunnerKind,
}

pub fn package_artifact_digest(package: &ValidatedPackage) -> String {
    domain_digest(
        "lkjscript.package-artifact.v1",
        package.revision_digest.as_bytes(),
    )
}

/// Builds one artifact containing an exact package closure. `packages` may be in any order but
/// must contain the root and every transitive dependency exactly once.
pub fn build_artifact(
    root: &ValidatedPackage,
    packages: &[&ValidatedPackage],
) -> Result<(Vec<u8>, ArtifactReceipt), Diagnostic> {
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
    validate_build_closure(&by_id)?;
    let mut encoded = Vec::with_capacity(by_id.len());
    let mut module_count = 0usize;
    for package in by_id.values() {
        module_count = module_count.saturating_add(package.modules.len());
        encoded.push(encode_package(package)?);
    }
    let core = ArtifactCore {
        contract_version: ARTIFACT_CONTRACT_VERSION,
        root_package_id: root.descriptor.package_id.clone(),
        root_revision_digest: root.revision_digest.clone(),
        packages: encoded,
    };
    let core_bytes = canonical_json(&core)?;
    let artifact_digest = domain_digest("lkjscript.component-artifact.v1", &core_bytes);
    let envelope = ArtifactEnvelope {
        contract_version: core.contract_version,
        artifact_digest: artifact_digest.clone(),
        root_package_id: core.root_package_id.clone(),
        root_revision_digest: core.root_revision_digest.clone(),
        packages: core.packages,
    };
    let bytes = canonical_json(&envelope)?;
    if bytes.len() > MAXIMUM_ARTIFACT_BYTES {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_too_large",
            format!(
                "artifact has {} bytes; the limit is {MAXIMUM_ARTIFACT_BYTES}",
                bytes.len()
            ),
        ));
    }
    let receipt = ArtifactReceipt {
        contract_version: ARTIFACT_CONTRACT_VERSION,
        artifact_digest,
        root_package_id: root.descriptor.package_id.clone(),
        root_revision_digest: root.revision_digest.clone(),
        package_count: packages.len(),
        module_count,
        bytes: bytes.len(),
    };
    Ok((bytes, receipt))
}

pub fn load_artifact(bytes: &[u8]) -> Result<LoadedArtifact, Diagnostic> {
    if bytes.len() > MAXIMUM_ARTIFACT_BYTES {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_too_large",
            format!(
                "artifact has {} bytes; the limit is {MAXIMUM_ARTIFACT_BYTES}",
                bytes.len()
            ),
        ));
    }
    let envelope: ArtifactEnvelope = strict_json(bytes, "component artifact")?;
    if envelope.contract_version != ARTIFACT_CONTRACT_VERSION {
        return Err(artifact_error(
            DiagnosticClass::Source,
            "artifact_contract",
            format!(
                "artifact contract {} is not current contract {ARTIFACT_CONTRACT_VERSION}",
                envelope.contract_version
            ),
        ));
    }
    if envelope.packages.is_empty() || envelope.packages.len() > MAXIMUM_ARTIFACT_PACKAGES {
        return Err(artifact_error(
            DiagnosticClass::Resource,
            "artifact_package_count",
            format!("artifact closure must contain 1 through {MAXIMUM_ARTIFACT_PACKAGES} packages"),
        ));
    }
    validate_digest(&envelope.artifact_digest, "artifact digest")?;
    validate_digest(&envelope.root_revision_digest, "root revision digest")?;
    let core = ArtifactCore {
        contract_version: envelope.contract_version,
        root_package_id: envelope.root_package_id.clone(),
        root_revision_digest: envelope.root_revision_digest.clone(),
        packages: envelope.packages.clone(),
    };
    let actual_digest = domain_digest("lkjscript.component-artifact.v1", &canonical_json(&core)?);
    if actual_digest != envelope.artifact_digest {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_digest",
            "component artifact digest does not match its canonical content",
        ));
    }
    validate_encoded_closure(&envelope.packages)?;
    let packages = decode_packages(&envelope.packages)?;
    let root = packages.get(&envelope.root_package_id).ok_or_else(|| {
        artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_root_missing",
            "artifact closure omits the root package",
        )
    })?;
    if root.revision_digest != envelope.root_revision_digest {
        return Err(artifact_error(
            DiagnosticClass::Corrupt,
            "artifact_root_revision",
            "artifact root revision digest is inconsistent",
        ));
    }
    Ok(LoadedArtifact {
        artifact_digest: envelope.artifact_digest,
        root_package_id: envelope.root_package_id,
        root_revision_digest: envelope.root_revision_digest,
        packages,
    })
}

fn validate_build_closure(
    packages: &BTreeMap<PackageId, &ValidatedPackage>,
) -> Result<(), Diagnostic> {
    for package in packages.values() {
        for dependency in &package.descriptor.dependencies {
            let supplied = packages.get(&dependency.package_id).ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Semantic,
                    "artifact_dependency_missing",
                    format!(
                        "package '{}' omits dependency '{}' from the artifact closure",
                        package.descriptor.name, dependency.alias
                    ),
                )
            })?;
            if supplied.revision_digest != dependency.revision_digest
                || package_artifact_digest(supplied) != dependency.artifact_digest
            {
                return Err(artifact_error(
                    DiagnosticClass::Semantic,
                    "artifact_dependency_identity",
                    format!(
                        "package '{}' dependency '{}' does not match the supplied exact package",
                        package.descriptor.name, dependency.alias
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn validate_encoded_closure(packages: &[ArtifactPackage]) -> Result<(), Diagnostic> {
    let mut identities = BTreeSet::new();
    for package in packages {
        if !identities.insert(package.package_id.clone()) {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_package_duplicate",
                "artifact repeats a package identity",
            ));
        }
        validate_digest(&package.revision_digest, "package revision digest")?;
        validate_digest(&package.package_artifact_digest, "package artifact digest")?;
        if package.modules.is_empty() {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_package_modules",
                "artifact package has no modules",
            ));
        }
        let mut module_names = BTreeSet::new();
        for module in &package.modules {
            if !module_names.insert(&module.name) {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_module_duplicate",
                    "artifact package repeats a module name",
                ));
            }
            if module.source.len() > SourceLimits::default().maximum_bytes {
                return Err(artifact_error(
                    DiagnosticClass::Resource,
                    "artifact_module_too_large",
                    "artifact module exceeds the source byte bound",
                ));
            }
        }
    }
    for package in packages {
        for dependency in &package.dependencies {
            if !identities.contains(&dependency.package_id) {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_dependency_missing",
                    "artifact package references an absent dependency",
                ));
            }
        }
    }
    Ok(())
}

fn decode_packages(
    encoded: &[ArtifactPackage],
) -> Result<BTreeMap<PackageId, ValidatedPackage>, Diagnostic> {
    let mut pending: BTreeMap<PackageId, &ArtifactPackage> = encoded
        .iter()
        .map(|package| (package.package_id.clone(), package))
        .collect();
    let mut validated = BTreeMap::new();
    while !pending.is_empty() {
        let ready: Vec<_> = pending
            .iter()
            .filter(|(_, package)| {
                package
                    .dependencies
                    .iter()
                    .all(|dependency| validated.contains_key(&dependency.package_id))
            })
            .map(|(identity, _)| identity.clone())
            .collect();
        if ready.is_empty() {
            return Err(artifact_error(
                DiagnosticClass::Corrupt,
                "artifact_dependency_cycle",
                "package dependency graph contains a cycle",
            ));
        }
        for identity in ready {
            let package = pending.remove(&identity).ok_or_else(|| {
                artifact_error(
                    DiagnosticClass::Infrastructure,
                    "artifact_decode_pending",
                    "ready artifact package disappeared",
                )
            })?;
            let descriptor = descriptor_from_artifact(package);
            let documents = package
                .modules
                .iter()
                .enumerate()
                .map(|(index, module)| {
                    parse_source(
                        format!("src/{index:04}.lkj"),
                        module.source.as_bytes(),
                        SourceLimits::default(),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?;
            let dependencies = package
                .dependencies
                .iter()
                .map(|dependency| {
                    let supplied = validated.get(&dependency.package_id).ok_or_else(|| {
                        artifact_error(
                            DiagnosticClass::Corrupt,
                            "artifact_dependency_missing",
                            "ready artifact package dependency disappeared",
                        )
                    })?;
                    Ok(ExactDependency {
                        alias: dependency.alias.as_str(),
                        package: supplied,
                        artifact_digest: dependency.artifact_digest.as_str(),
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?;
            let value = validate_package_documents(descriptor, documents, &dependencies)?;
            if value.revision_digest != package.revision_digest
                || package_artifact_digest(&value) != package.package_artifact_digest
            {
                return Err(artifact_error(
                    DiagnosticClass::Corrupt,
                    "artifact_package_digest",
                    "artifact package does not reconstruct to its declared identity",
                ));
            }
            validated.insert(identity, value);
        }
    }
    Ok(validated)
}

fn descriptor_from_artifact(package: &ArtifactPackage) -> PackageDescriptor {
    PackageDescriptor {
        contract_version: super::package::PACKAGE_CONTRACT_VERSION,
        package_id: package.package_id.clone(),
        name: package.name.clone(),
        modules: package
            .modules
            .iter()
            .enumerate()
            .map(|(index, module)| ModuleLocator {
                name: module.name.clone(),
                path: format!("src/{index:04}.lkj"),
            })
            .collect(),
        dependencies: package
            .dependencies
            .iter()
            .map(|dependency| Dependency {
                alias: dependency.alias.clone(),
                package_id: dependency.package_id.clone(),
                revision_digest: dependency.revision_digest.clone(),
                artifact_digest: dependency.artifact_digest.clone(),
                artifact: format!("packages/{}", dependency.package_id.as_str()),
            })
            .collect(),
        targets: package
            .targets
            .iter()
            .map(|target| Target {
                name: target.name.clone(),
                component: target.component.clone(),
                port: target.port.clone(),
                runner: target.runner,
            })
            .collect(),
    }
}

fn encode_package(package: &ValidatedPackage) -> Result<ArtifactPackage, Diagnostic> {
    let mut modules = package
        .modules
        .iter()
        .map(|module| {
            let source = std::str::from_utf8(&module.semantic_bytes).map_err(|_| {
                artifact_error(
                    DiagnosticClass::Infrastructure,
                    "artifact_module_utf8",
                    "validated canonical module source is not UTF-8",
                )
            })?;
            Ok(ArtifactModule {
                name: module.module.name.clone(),
                source: source.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    modules.sort_by(|left, right| left.name.cmp(&right.name));
    let mut dependencies = package
        .descriptor
        .dependencies
        .iter()
        .map(|dependency| ArtifactDependency {
            alias: dependency.alias.clone(),
            package_id: dependency.package_id.clone(),
            revision_digest: dependency.revision_digest.clone(),
            artifact_digest: dependency.artifact_digest.clone(),
        })
        .collect::<Vec<_>>();
    dependencies.sort_by(|left, right| left.alias.cmp(&right.alias));
    let mut targets = package
        .descriptor
        .targets
        .iter()
        .map(|target| ArtifactTarget {
            name: target.name.clone(),
            component: target.component.clone(),
            port: target.port.clone(),
            runner: target.runner,
        })
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(ArtifactPackage {
        package_id: package.descriptor.package_id.clone(),
        name: package.descriptor.name.clone(),
        revision_digest: package.revision_digest.clone(),
        package_artifact_digest: package_artifact_digest(package),
        modules,
        dependencies,
        targets,
    })
}

fn canonical_json(value: &impl Serialize) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = serde_json::to_vec(value).map_err(|error| {
        artifact_error(
            DiagnosticClass::Infrastructure,
            "artifact_encode",
            format!("artifact encoding failed: {error}"),
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn strict_json<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T, Diagnostic> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value = T::deserialize(&mut decoder).map_err(|error| {
        artifact_error(
            DiagnosticClass::Source,
            "artifact_json",
            format!("{label} is malformed: {error}"),
        )
    })?;
    decoder.end().map_err(|error| {
        artifact_error(
            DiagnosticClass::Source,
            "artifact_trailing",
            format!("{label} has trailing input: {error}"),
        )
    })?;
    Ok(value)
}

fn validate_digest(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(artifact_error(
            DiagnosticClass::Source,
            "artifact_digest_encoding",
            format!("{label} is not 64 lowercase hexadecimal characters"),
        ));
    }
    Ok(())
}

fn domain_digest(domain: &str, bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    hex(hasher.finalize().as_bytes())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    result
}

fn artifact_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{decode_package, parse_source};

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
    fn deterministic_round_trip_and_corruption_rejection() {
        let package = package();
        let (first, receipt) = build_artifact(&package, &[&package]).expect("build artifact");
        let (second, _) = build_artifact(&package, &[&package]).expect("repeat build");
        assert_eq!(first, second);
        assert_eq!(receipt.package_count, 1);
        let loaded = load_artifact(&first).expect("load artifact");
        assert_eq!(loaded.root_revision_digest, package.revision_digest);
        assert_eq!(loaded.root().expect("root").descriptor.targets.len(), 1);

        let mut corrupt: serde_json::Value = serde_json::from_slice(&first).expect("artifact json");
        corrupt["packages"][0]["name"] = serde_json::Value::String("changed".to_owned());
        let bytes = serde_json::to_vec(&corrupt).expect("corrupt encoding");
        let error = load_artifact(&bytes).expect_err("digest must reject corruption");
        assert_eq!(error.code, "artifact_digest");
    }

    #[test]
    fn predecessor_contract_rejects() {
        let package = package();
        let (bytes, _) = build_artifact(&package, &[&package]).expect("build artifact");
        let mut value: serde_json::Value = serde_json::from_slice(&bytes).expect("artifact json");
        value["contract_version"] = serde_json::Value::from(0);
        let bytes = serde_json::to_vec(&value).expect("predecessor encoding");
        let error = load_artifact(&bytes).expect_err("predecessor rejects");
        assert_eq!(error.code, "artifact_contract");
    }
}
