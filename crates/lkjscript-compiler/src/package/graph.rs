use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use lkjscript_contracts::{sha256, ContractDigest};
use lkjscript_core::{Error, Result};

use super::analysis::ModuleAnalysis;
use super::model::{LockFile, LockedModule, LockedPackage, LockedSourceIdentity, Manifest};

pub(super) fn build(root: &Path) -> Result<LockFile> {
    let canonical = root
        .canonicalize()
        .map_err(|error| Error::host(format!("canonicalize package root: {error}")))?;
    let mut visiting = BTreeSet::new();
    let mut packages = BTreeMap::new();
    let root_hash = visit(&canonical, &canonical, &mut visiting, &mut packages)?;
    Ok(LockFile {
        schema: "lkjscript.package-lock".into(),
        contract: super::contracts::expected(lkjscript_contracts::PACKAGE_LOCK)?.to_hex(),
        root: root_hash,
        contracts: super::contracts::all()?,
        packages: packages.into_values().collect(),
    })
}

fn visit(
    root: &Path,
    package_root: &Path,
    visiting: &mut BTreeSet<std::path::PathBuf>,
    packages: &mut BTreeMap<String, LockedPackage>,
) -> Result<String> {
    let canonical = package_root
        .canonicalize()
        .map_err(|error| Error::host(format!("canonicalize local dependency: {error}")))?;
    if !canonical.starts_with(root) {
        return Err(Error::msg("local dependency escapes the root package"));
    }
    if !visiting.insert(canonical.clone()) {
        return Err(Error::msg("local package dependency cycle"));
    }
    let (manifest, _) = super::manifest::load(&canonical)?;
    let (modules, analyses) = modules(&canonical, &manifest)?;
    let targets = super::target_memory::build(&manifest.targets, &analyses)?;
    let manifest_bytes = serde_json::to_vec(&manifest)
        .map_err(|error| Error::msg(format!("encode canonical package manifest: {error}")))?;
    let manifest_hash = hex(sha256(&manifest_bytes));
    let memory_hash = package_memory_hash(&modules)?;
    let package_hash = package_hash(&manifest_hash, &memory_hash, &modules, &targets)?;
    let mut dependencies = Vec::new();
    for dependency in &manifest.dependencies {
        let dependency_hash = visit(root, &canonical.join(&dependency.path), visiting, packages)?;
        if dependency.content_sha256 != dependency_hash {
            return Err(Error::msg(format!(
                "dependency {} content mismatch: expected {}, received {}",
                dependency.name, dependency.content_sha256, dependency_hash
            )));
        }
        dependencies.push(dependency_hash);
    }
    dependencies.sort();
    let origin = canonical
        .strip_prefix(root)
        .map_err(|_| Error::msg("dependency origin escapes root"))?
        .to_str()
        .ok_or_else(|| Error::msg("dependency origin is not UTF-8"))?;
    let locked = LockedPackage {
        name: manifest.name,
        origin: if origin.is_empty() {
            ".".into()
        } else {
            origin.replace('\\', "/")
        },
        manifest_sha256: manifest_hash,
        package_memory_interface_sha256: memory_hash,
        package_sha256: package_hash.clone(),
        dependencies,
        modules,
        targets,
    };
    if packages.insert(package_hash.clone(), locked).is_some() {
        return Err(Error::msg("duplicate local package content identity"));
    }
    visiting.remove(&canonical);
    Ok(package_hash)
}

fn modules(
    root: &Path,
    manifest: &Manifest,
) -> Result<(Vec<LockedModule>, BTreeMap<String, ModuleAnalysis>)> {
    let contract = super::contracts::expected(lkjscript_contracts::MODULE_INTERFACE)?;
    let mut locked = Vec::with_capacity(manifest.modules.len());
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

fn package_memory_hash(modules: &[LockedModule]) -> Result<String> {
    let bytes = serde_json::to_vec(
        &modules
            .iter()
            .map(|module| (&module.id, &module.module_memory_interface_sha256))
            .collect::<Vec<_>>(),
    )
    .map_err(|error| Error::msg(format!("encode package memory interfaces: {error}")))?;
    framed_hash(b"lkjscript.package-memory-interfaces", &[&bytes])
}

fn package_hash(
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
