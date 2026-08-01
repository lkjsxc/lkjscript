use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use lkjscript_contracts::{sha256, ContractDigest};
use lkjscript_core::{Error, Result};

use super::model::{LockFile, LockedModule, LockedPackage, Manifest};

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
    let modules = modules(&canonical, &manifest)?;
    let manifest_bytes = serde_json::to_vec(&manifest)
        .map_err(|error| Error::msg(format!("encode canonical package manifest: {error}")))?;
    let manifest_hash = hex(sha256(&manifest_bytes));
    let package_hash = package_hash(&manifest_hash, &modules)?;
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
        package_sha256: package_hash.clone(),
        dependencies,
        modules,
    };
    if packages.insert(package_hash.clone(), locked).is_some() {
        return Err(Error::msg("duplicate local package content identity"));
    }
    visiting.remove(&canonical);
    Ok(package_hash)
}

fn modules(root: &Path, manifest: &Manifest) -> Result<Vec<LockedModule>> {
    let module_contract = super::contracts::expected(lkjscript_contracts::MODULE_INTERFACE)?;
    let mut result = Vec::with_capacity(manifest.modules.len());
    for id in &manifest.modules {
        let bytes = fs::read(root.join(id))
            .map_err(|error| Error::host(format!("read module {id}: {error}")))?;
        let source_hash = sha256(&bytes);
        let (exports, witness_requirements, interface_sha256) =
            super::interface::build(&bytes, id, module_contract)?;
        let module_hash = framed_hash(
            b"lkjscript.module",
            &[
                &module_contract.as_bytes(),
                id.as_bytes(),
                &source_hash,
                interface_sha256.as_bytes(),
            ],
        )?;
        result.push(LockedModule {
            id: id.clone(),
            source_sha256: hex(source_hash),
            interface_sha256,
            module_sha256: module_hash,
            exports,
            witness_requirements,
        });
    }
    Ok(result)
}

fn package_hash(manifest_hash: &str, modules: &[LockedModule]) -> Result<String> {
    let contract = super::contracts::expected(lkjscript_contracts::PACKAGE_MANIFEST)?;
    let mut records = Vec::new();
    records.extend_from_slice(manifest_hash.as_bytes());
    for module in modules {
        records.extend_from_slice(module.id.as_bytes());
        records.extend_from_slice(module.module_sha256.as_bytes());
    }
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
