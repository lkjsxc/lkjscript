mod contracts;
mod encoding;
mod graph;
mod manifest;
mod model;

use std::path::{Path, PathBuf};

use lkjscript_core::{Error, Result};

pub use model::{
    Dependency, ExecutionIdentity, LockFile, LockedModule, LockedPackage, Manifest, Target,
};

pub const MANIFEST_FILE: &str = manifest::MANIFEST;
pub const LOCK_FILE: &str = manifest::LOCK;

pub fn root(entry: &Path) -> Result<PathBuf> {
    manifest::find_root(entry)
}

pub fn load_manifest(entry: &Path) -> Result<(PathBuf, Manifest)> {
    let root = root(entry)?;
    let (manifest, _) = manifest::load(&root)?;
    Ok((root, manifest))
}

pub fn create_lock(entry: &Path) -> Result<(PathBuf, Vec<u8>)> {
    let root = root(entry)?;
    let lock = graph::build(&root)?;
    let bytes = encoding::encode(&lock)?;
    Ok((root.join(LOCK_FILE), bytes))
}

pub fn verify(entry: &Path) -> Result<(PathBuf, Manifest)> {
    let root = root(entry)?;
    let (manifest, _) = manifest::load(&root)?;
    let current = graph::build(&root)?;
    let (locked, locked_bytes) = encoding::read(&root)?;
    contracts::require(&locked.contract, lkjscript_contracts::PACKAGE_LOCK)?;
    if locked.contracts != contracts::all()? {
        return Err(Error::msg("package lock contract set is stale"));
    }
    if current != locked || encoding::encode(&current)? != locked_bytes {
        return Err(Error::msg(
            "package lock is stale; run `lkjscript package lock`",
        ));
    }
    if !entry.is_dir() {
        let canonical = entry
            .canonicalize()
            .map_err(|error| Error::host(format!("canonicalize package entry: {error}")))?;
        let module = canonical
            .strip_prefix(&root)
            .map_err(|_| Error::msg("package entry escapes root"))?
            .to_str()
            .ok_or_else(|| Error::msg("package entry is not UTF-8"))?
            .replace('\\', "/");
        if manifest.modules.binary_search(&module).is_err() {
            return Err(Error::msg(format!(
                "entry module is not declared: {module}"
            )));
        }
    }
    Ok((root, manifest))
}

pub fn execution_identity(entry: &Path) -> Result<(PathBuf, Manifest, ExecutionIdentity)> {
    let (root, manifest) = verify(entry)?;
    if entry.is_dir() {
        return Err(Error::msg("execution identity requires an entry module"));
    }
    let module_path = entry
        .canonicalize()
        .map_err(|error| Error::host(format!("canonicalize package entry: {error}")))?
        .strip_prefix(&root)
        .map_err(|_| Error::msg("package entry escapes root"))?
        .to_str()
        .ok_or_else(|| Error::msg("package entry is not UTF-8"))?
        .replace('\\', "/");
    let (lock, bytes) = encoding::read(&root)?;
    let package = lock
        .packages
        .iter()
        .find(|package| package.package_sha256 == lock.root)
        .ok_or_else(|| Error::msg("root package identity is absent"))?;
    let module = package
        .modules
        .iter()
        .find(|module| module.id == module_path)
        .ok_or_else(|| Error::msg("entry module identity is absent"))?;
    let identity = ExecutionIdentity {
        module_path,
        source_sha256: parse_digest(&module.source_sha256)?,
        module_sha256: parse_digest(&module.module_sha256)?,
        package_sha256: parse_digest(&package.package_sha256)?,
        lock_sha256: lkjscript_contracts::sha256(&bytes),
    };
    Ok((root, manifest, identity))
}

fn parse_digest(value: &str) -> Result<[u8; 32]> {
    lkjscript_contracts::ContractDigest::from_hex(value)
        .map(lkjscript_contracts::ContractDigest::as_bytes)
        .ok_or_else(|| Error::msg("package identity digest is malformed"))
}

pub fn target(entry: &Path, name: &str) -> Result<PathBuf> {
    let (root, manifest) = verify(entry)?;
    let target = manifest
        .targets
        .iter()
        .find(|target| target.name == name)
        .ok_or_else(|| Error::msg(format!("unknown package target: {name}")))?;
    Ok(root.join(&target.module))
}

#[cfg(test)]
mod tests;
