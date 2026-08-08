use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use lkjscript_contracts::{sha256, ContractDigest};
use lkjscript_core::{Error, Result};

use super::analysis::ModuleAnalysis;
use super::model::{
    Dependency, LockFile, LockedModule, LockedPackage, LockedSourceIdentity, LockedTargetMemory,
    Manifest,
};

enum VisitState {
    Visiting,
    Complete(String),
}

struct PackageFrame {
    canonical: PathBuf,
    manifest: Manifest,
    manifest_hash: String,
    memory_hash: String,
    package_hash: String,
    modules: Vec<LockedModule>,
    targets: Vec<LockedTargetMemory>,
    dependencies: Vec<String>,
    next_dependency: usize,
}

pub(super) fn build(root: &Path) -> Result<LockFile> {
    build_with_root_manifest(root).map(|(lock, _)| lock)
}

pub(super) fn build_with_root_manifest(root: &Path) -> Result<(LockFile, Manifest)> {
    let canonical = root
        .canonicalize()
        .map_err(|error| Error::host(format!("canonicalize package root: {error}")))?;
    let mut states = HashMap::new();
    states
        .try_reserve(1)
        .map_err(|_| Error::host("package graph state allocation failed"))?;
    states.insert(canonical.clone(), VisitState::Visiting);

    let mut frames = Vec::new();
    frames
        .try_reserve(1)
        .map_err(|_| Error::host("package graph work stack allocation failed"))?;
    let root_frame = prepare(canonical.clone())?;
    let root_manifest = root_frame.manifest.clone();
    frames.push(root_frame);

    let mut packages = Vec::new();
    let mut package_identities = HashMap::new();
    let root_hash = loop {
        let dependency = {
            let frame = frames
                .last_mut()
                .ok_or_else(|| Error::host("package graph work stack is empty"))?;
            let dependency = frame
                .manifest
                .dependencies
                .get(frame.next_dependency)
                .cloned();
            if dependency.is_some() {
                frame.next_dependency = frame
                    .next_dependency
                    .checked_add(1)
                    .ok_or_else(|| Error::host("package dependency index overflow"))?;
            }
            dependency
        };

        if let Some(dependency) = dependency {
            let parent = &frames
                .last()
                .ok_or_else(|| Error::host("package graph parent frame is missing"))?
                .canonical;
            let canonical_dependency = parent
                .join(&dependency.path)
                .canonicalize()
                .map_err(|error| Error::host(format!("canonicalize local dependency: {error}")))?;
            if !canonical_dependency.starts_with(&canonical) {
                return Err(Error::msg("local dependency escapes the root package"));
            }
            match states.get(&canonical_dependency) {
                Some(VisitState::Visiting) => {
                    return Err(Error::msg("local package dependency cycle"));
                }
                Some(VisitState::Complete(hash)) => {
                    verify_dependency(&dependency, hash)?;
                    push_dependency(
                        &mut frames
                            .last_mut()
                            .ok_or_else(|| Error::host("package graph parent frame is missing"))?
                            .dependencies,
                        hash,
                    )?;
                }
                None => {
                    let child = prepare(canonical_dependency.clone())?;
                    states
                        .try_reserve(1)
                        .map_err(|_| Error::host("package graph state allocation failed"))?;
                    frames
                        .try_reserve(1)
                        .map_err(|_| Error::host("package graph work stack allocation failed"))?;
                    states.insert(canonical_dependency, VisitState::Visiting);
                    frames.push(child);
                }
            }
            continue;
        }

        let frame = frames
            .pop()
            .ok_or_else(|| Error::host("package graph completion frame is missing"))?;
        let canonical_package = frame.canonical.clone();
        let package_hash = clone_string(&frame.package_hash, "package graph identity")?;
        let locked = finish(&canonical, frame)?;
        package_identities
            .try_reserve(1)
            .map_err(|_| Error::host("package identity index allocation failed"))?;
        packages
            .try_reserve(1)
            .map_err(|_| Error::host("locked package allocation failed"))?;
        if package_identities
            .insert(
                clone_string(&package_hash, "package identity index")?,
                packages.len(),
            )
            .is_some()
        {
            return Err(Error::msg("duplicate local package content identity"));
        }
        packages.push(locked);
        let state = states
            .get_mut(&canonical_package)
            .ok_or_else(|| Error::host("package graph state is missing"))?;
        *state = VisitState::Complete(clone_string(&package_hash, "completed package identity")?);

        if let Some(parent) = frames.last_mut() {
            let dependency_index = parent
                .next_dependency
                .checked_sub(1)
                .ok_or_else(|| Error::host("package dependency completion index underflow"))?;
            let expected = parent
                .manifest
                .dependencies
                .get(dependency_index)
                .ok_or_else(|| Error::host("completed package dependency is missing"))?;
            verify_dependency(expected, &package_hash)?;
            push_dependency(&mut parent.dependencies, &package_hash)?;
        } else {
            break package_hash;
        }
    };

    packages.sort_by(|left, right| left.package_sha256.cmp(&right.package_sha256));
    Ok((
        LockFile {
            schema: "lkjscript.package-lock".into(),
            contract: super::contracts::expected(lkjscript_contracts::PACKAGE_LOCK)?.to_hex(),
            root: root_hash,
            contracts: super::contracts::all()?,
            packages,
        },
        root_manifest,
    ))
}

fn prepare(canonical: PathBuf) -> Result<PackageFrame> {
    let (manifest, _) = super::manifest::load(&canonical)?;
    let (modules, analyses) = modules(&canonical, &manifest)?;
    let targets = super::target_memory::build(&manifest.targets, &analyses)?;
    let manifest_bytes = serde_json::to_vec(&manifest)
        .map_err(|error| Error::msg(format!("encode canonical package manifest: {error}")))?;
    let manifest_hash = hex(sha256(&manifest_bytes));
    let memory_hash = package_memory_hash(&modules)?;
    let package_hash = package_hash(&manifest_hash, &memory_hash, &modules, &targets)?;
    let mut dependencies = Vec::new();
    dependencies
        .try_reserve(manifest.dependencies.len())
        .map_err(|_| Error::host("locked dependency allocation failed"))?;
    Ok(PackageFrame {
        canonical,
        manifest,
        manifest_hash,
        memory_hash,
        package_hash,
        modules,
        targets,
        dependencies,
        next_dependency: 0,
    })
}

fn finish(root: &Path, mut frame: PackageFrame) -> Result<LockedPackage> {
    frame.dependencies.sort();
    let origin = frame
        .canonical
        .strip_prefix(root)
        .map_err(|_| Error::msg("dependency origin escapes root"))?
        .to_str()
        .ok_or_else(|| Error::msg("dependency origin is not UTF-8"))?;
    Ok(LockedPackage {
        name: frame.manifest.name,
        origin: if origin.is_empty() {
            ".".into()
        } else {
            origin.replace('\\', "/")
        },
        manifest_sha256: frame.manifest_hash,
        package_memory_interface_sha256: frame.memory_hash,
        package_sha256: frame.package_hash,
        dependencies: frame.dependencies,
        modules: frame.modules,
        targets: frame.targets,
    })
}

fn verify_dependency(dependency: &Dependency, received: &str) -> Result<()> {
    if dependency.content_sha256 == received {
        Ok(())
    } else {
        Err(Error::msg(format!(
            "dependency {} content mismatch: expected {}, received {}",
            dependency.name, dependency.content_sha256, received
        )))
    }
}

fn push_dependency(dependencies: &mut Vec<String>, hash: &str) -> Result<()> {
    dependencies
        .try_reserve(1)
        .map_err(|_| Error::host("locked dependency allocation failed"))?;
    dependencies.push(clone_string(hash, "locked dependency identity")?);
    Ok(())
}

fn clone_string(value: &str, label: &str) -> Result<String> {
    let mut output = String::new();
    output
        .try_reserve(value.len())
        .map_err(|_| Error::host(format!("{label} allocation failed")))?;
    output.push_str(value);
    Ok(output)
}

fn modules(
    root: &Path,
    manifest: &Manifest,
) -> Result<(Vec<LockedModule>, BTreeMap<String, ModuleAnalysis>)> {
    let contract = super::contracts::expected(lkjscript_contracts::MODULE_INTERFACE)?;
    let mut locked = Vec::new();
    locked
        .try_reserve(manifest.modules.len())
        .map_err(|_| Error::host("locked module allocation failed"))?;
    let mut analyses = BTreeMap::new();
    for id in &manifest.modules {
        let analysis = super::analysis::module(root, id)?;
        let public = manifest.public.binary_search(id).is_ok();
        let interface = super::interface::build(id, public, contract, &analysis)?;
        let source_hash = analysis.source_sha256;
        let mut source_closure = Vec::with_capacity(analysis.source.files().len());
        for source in analysis.source.files() {
            let source_id = source
                .path
                .strip_prefix(root)
                .map_err(|_| Error::msg("loaded module source escapes package root"))?
                .to_str()
                .ok_or_else(|| Error::msg("loaded module source path is not UTF-8"))?
                .replace('\\', "/");
            source_closure.push(LockedSourceIdentity {
                id: source_id,
                source_sha256: hex(source.exact_source_sha256),
            });
        }
        source_closure.sort_by(|left, right| left.id.cmp(&right.id));
        let closure_bytes = serde_json::to_vec(&source_closure)
            .map_err(|error| Error::msg(format!("encode module source closure: {error}")))?;
        let module_hash = framed_hash(
            b"lkjscript.module",
            &[
                &contract.as_bytes(),
                id.as_bytes(),
                &source_hash,
                &closure_bytes,
                interface.interface_digest.as_bytes(),
                interface.memory_digest.as_bytes(),
            ],
        )?;
        locked.push(LockedModule {
            id: id.clone(),
            source_sha256: hex(source_hash),
            source_closure,
            module_interface_contract: contract.to_hex(),
            module_interface_sha256: interface.interface_digest,
            module_memory_interface_sha256: interface.memory_digest,
            module_sha256: module_hash,
            exports: interface.exports,
            memory_interfaces: interface.records,
        });
        analyses.insert(id.clone(), analysis);
    }
    Ok((locked, analyses))
}

pub(super) fn package_memory_hash(modules: &[LockedModule]) -> Result<String> {
    let bytes = serde_json::to_vec(
        &modules
            .iter()
            .map(|module| (&module.id, &module.module_memory_interface_sha256))
            .collect::<Vec<_>>(),
    )
    .map_err(|error| Error::msg(format!("encode package memory interfaces: {error}")))?;
    framed_hash(b"lkjscript.package-memory-interfaces", &[&bytes])
}

pub(super) fn package_hash(
    manifest_hash: &str,
    memory_hash: &str,
    modules: &[LockedModule],
    targets: &[super::model::LockedTargetMemory],
) -> Result<String> {
    let contract = super::contracts::expected(lkjscript_contracts::PACKAGE_MANIFEST)?;
    let records = serde_json::to_vec(&(manifest_hash, memory_hash, modules, targets))
        .map_err(|error| Error::msg(format!("encode package identity: {error}")))?;
    framed_hash(b"lkjscript.package", &[&contract.as_bytes(), &records])
}

pub(super) fn framed_hash(domain: &[u8], fields: &[&[u8]]) -> Result<String> {
    let mut bytes = Vec::new();
    frame(&mut bytes, domain)?;
    for field in fields {
        frame(&mut bytes, field)?;
    }
    Ok(hex(sha256(&bytes)))
}

fn frame(output: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let length =
        u64::try_from(value.len()).map_err(|_| Error::msg("package identity field overflow"))?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

fn hex(bytes: [u8; 32]) -> String {
    ContractDigest::from_bytes(bytes).to_hex()
}
