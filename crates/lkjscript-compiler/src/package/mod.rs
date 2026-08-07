mod analysis;
mod contracts;
mod encoding;
mod graph;
mod interface;
mod interface_function;
mod interface_values;
mod manifest;
mod model;
mod prepared;
pub(crate) mod program;
mod read;
mod target_memory;

use std::path::{Path, PathBuf};

use lkjscript_core::{Error, Result};

pub use model::{
    Dependency, LockFile, LockedConstraintSupport, LockedMemoryConstraint,
    LockedMemoryParameterMode, LockedMemoryRequirement, LockedMemoryResultMode, LockedModule,
    LockedPackage, LockedSourceIdentity, LockedTargetMemory, LockedTraitParameter,
    LockedWitnessDependency, LockedWitnessDependencyRole, LockedWitnessGroup,
    LockedWitnessGroupMember, Manifest, PackageMemoryInterface, Target,
};

pub const MANIFEST_FILE: &str = manifest::MANIFEST;
pub const LOCK_FILE: &str = manifest::LOCK;
pub(crate) use prepared::{
    capture as capture_preparation, CapturedPackageProvenance, PreparedPackageFacts,
};

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

pub(crate) fn verify_for_compilation(entry: &Path) -> Result<Option<PathBuf>> {
    let start = entry.parent().unwrap_or_else(|| Path::new("."));
    let mut current = start
        .canonicalize()
        .map_err(|error| Error::host(format!("canonicalize package search root: {error}")))?;
    loop {
        if current.join(MANIFEST_FILE).is_file() {
            return verify(entry).map(|(root, _)| Some(root));
        }
        if !current.pop() {
            return Ok(None);
        }
    }
}

pub(crate) fn verify_loaded_sources(
    root: &Path,
    entry: &Path,
    loaded: &crate::source::ValidatedSourceTree,
) -> Result<()> {
    let (lock, _) = encoding::read(root)?;
    let package = lock
        .packages
        .iter()
        .find(|package| package.origin == ".")
        .ok_or_else(|| Error::msg("package lock omits root package"))?;
    let canonical = entry
        .canonicalize()
        .map_err(|error| Error::host(format!("canonicalize compiled entry: {error}")))?;
    let id = canonical
        .strip_prefix(root)
        .map_err(|_| Error::msg("compiled package entry escapes root"))?
        .to_str()
        .ok_or_else(|| Error::msg("compiled package entry is not UTF-8"))?
        .replace('\\', "/");
    let module = package
        .modules
        .iter()
        .find(|module| module.id == id)
        .ok_or_else(|| Error::msg(format!("compiled module is absent from package lock: {id}")))?;
    if module.source_closure.len() != loaded.files().len() {
        return Err(Error::msg(
            "compiled source closure differs from package lock",
        ));
    }
    for source in loaded.files() {
        let id = source
            .path
            .strip_prefix(root)
            .map_err(|_| Error::msg("compiled source escapes package root"))?
            .to_str()
            .ok_or_else(|| Error::msg("compiled source path is not UTF-8"))?
            .replace('\\', "/");
        let locked = module
            .source_closure
            .iter()
            .find(|locked| locked.id == id)
            .ok_or_else(|| Error::msg(format!("compiled source is absent from lock: {id}")))?;
        let digest =
            lkjscript_contracts::ContractDigest::from_bytes(source.exact_source_sha256).to_hex();
        if locked.source_sha256 != digest {
            return Err(Error::msg(format!(
                "compiled source identity differs from lock: {id}"
            )));
        }
    }
    Ok(())
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
