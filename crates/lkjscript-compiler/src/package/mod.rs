mod contracts;
mod encoding;
mod graph;
mod interface;
mod manifest;
mod model;

use std::path::{Path, PathBuf};

use lkjscript_core::{Error, Result};

pub use model::{
    Dependency, LockFile, LockedModule, LockedPackage, LockedWitnessRequirement, Manifest, Target,
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

pub fn verify_content(
    entry: &Path,
) -> Result<(PathBuf, Manifest, lkjscript_contracts::ContractDigest)> {
    let (root, manifest) = verify(entry)?;
    let (locked, _) = encoding::read(&root)?;
    let identity = lkjscript_contracts::ContractDigest::from_hex(&locked.root)
        .ok_or_else(|| Error::msg("package lock root is not a full lowercase SHA-256"))?;
    Ok((root, manifest, identity))
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
