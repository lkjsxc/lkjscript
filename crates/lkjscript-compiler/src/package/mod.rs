mod analysis;
mod compilation;
mod contracts;
mod encoding;
mod error;
mod graph;
mod interface;
mod interface_function;
mod interface_values;
mod manifest;
mod model;
mod read;
mod target_memory;

use std::path::{Path, PathBuf};

use lkjscript_core::{Error, Result as CoreResult};

pub use error::{PackageError, PackageResult};
pub use model::{
    Dependency, LockFile, LockedConstraintSupport, LockedMemoryConstraint,
    LockedMemoryParameterMode, LockedMemoryRequirement, LockedMemoryResultMode, LockedModule,
    LockedPackage, LockedSourceIdentity, LockedTargetMemory, LockedTraitParameter,
    LockedWitnessDependency, LockedWitnessDependencyRole, LockedWitnessGroup,
    LockedWitnessGroupMember, Manifest, PackageMemoryInterface, Target,
};

pub const MANIFEST_FILE: &str = manifest::MANIFEST;
pub const LOCK_FILE: &str = manifest::LOCK;

pub(crate) struct VerifiedCompilationPackage {
    root: PathBuf,
    lock: LockFile,
    entry_module: String,
    capabilities: Vec<lkjscript_core::CapabilityKind>,
}

impl VerifiedCompilationPackage {
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) const fn lock(&self) -> &LockFile {
        &self.lock
    }

    pub(crate) fn entry_module(&self) -> &str {
        &self.entry_module
    }

    pub(crate) fn capabilities(&self) -> &[lkjscript_core::CapabilityKind] {
        &self.capabilities
    }
}
pub(crate) use compilation::{capture as capture_compilation, CapturedPackageCompilation};

pub fn root(entry: &Path) -> CoreResult<PathBuf> {
    manifest::find_root(entry)
}

pub fn load_manifest(entry: &Path) -> CoreResult<(PathBuf, Manifest)> {
    let root = root(entry)?;
    let (manifest, _) = manifest::load(&root)?;
    Ok((root, manifest))
}

pub fn create_lock(entry: &Path) -> PackageResult<(PathBuf, Vec<u8>)> {
    let root = root(entry)?;
    let lock = graph::build(&root)?;
    let bytes = encoding::encode(&lock)?;
    Ok((root.join(LOCK_FILE), bytes))
}

pub fn verify(entry: &Path) -> PackageResult<(PathBuf, Manifest)> {
    verify_with_lock(entry).map(|(root, manifest, _, _)| (root, manifest))
}

fn verify_with_lock(entry: &Path) -> PackageResult<(PathBuf, Manifest, LockFile, Option<String>)> {
    let root = root(entry)?;
    let (current, manifest) = graph::build_with_root_manifest(&root)?;
    let (locked, locked_bytes) = encoding::read(&root)?;
    contracts::require(&locked.contract, lkjscript_contracts::PACKAGE_LOCK)?;
    if locked.contracts != contracts::all()? {
        return Err(Error::msg("package lock contract set is stale").into());
    }
    if current != locked || encoding::encode(&current)? != locked_bytes {
        return Err(Error::msg("package lock is stale; run `lkjscript package lock`").into());
    }
    let entry_module = if entry.is_dir() {
        None
    } else {
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
            return Err(Error::msg(format!("entry module is not declared: {module}")).into());
        }
        Some(module)
    };
    Ok((root, manifest, locked, entry_module))
}

pub(crate) fn verify_for_compilation(
    entry: &Path,
) -> PackageResult<Option<VerifiedCompilationPackage>> {
    let start = entry.parent().unwrap_or_else(|| Path::new("."));
    let mut current = start
        .canonicalize()
        .map_err(|error| Error::host(format!("canonicalize package search root: {error}")))?;
    loop {
        if current.join(MANIFEST_FILE).is_file() {
            return verify_with_lock(entry).and_then(|(root, manifest, lock, entry_module)| {
                let mut capabilities = Vec::new();
                capabilities
                    .try_reserve(manifest.capabilities.len())
                    .map_err(|_| Error::host("package capability grant allocation failed"))?;
                for name in &manifest.capabilities {
                    capabilities.push(lkjscript_core::CapabilityKind::parse(name).ok_or_else(
                        || Error::msg(format!("unknown package capability: {name}")),
                    )?);
                }
                Ok(Some(VerifiedCompilationPackage {
                    root,
                    lock,
                    entry_module: entry_module.ok_or_else(|| {
                        Error::msg("compiled package entry is not a source module")
                    })?,
                    capabilities,
                }))
            });
        }
        if !current.pop() {
            return Ok(None);
        }
    }
}

pub(crate) fn verify_loaded_sources(
    verified: &VerifiedCompilationPackage,
    loaded: &crate::source::ValidatedSourceTree,
) -> PackageResult<()> {
    let root = verified.root();
    let package = verified
        .lock()
        .packages
        .iter()
        .find(|package| package.origin == ".")
        .ok_or_else(|| Error::msg("package lock omits root package"))?;
    let id = verified.entry_module();
    let module = package
        .modules
        .iter()
        .find(|module| module.id == id)
        .ok_or_else(|| Error::msg(format!("compiled module is absent from package lock: {id}")))?;
    if module.source_closure.len() != loaded.files().len() {
        return Err(Error::msg("compiled source closure differs from package lock").into());
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
            return Err(
                Error::msg(format!("compiled source identity differs from lock: {id}")).into(),
            );
        }
    }
    Ok(())
}

pub fn verify_content(
    entry: &Path,
) -> PackageResult<(PathBuf, Manifest, lkjscript_contracts::ContractDigest)> {
    let (root, manifest, locked, _) = verify_with_lock(entry)?;
    let identity = lkjscript_contracts::ContractDigest::from_hex(&locked.root)
        .ok_or_else(|| Error::msg("package lock root is not a full lowercase SHA-256"))?;
    Ok((root, manifest, identity))
}

pub fn target(entry: &Path, name: &str) -> PackageResult<PathBuf> {
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
