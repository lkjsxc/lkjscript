use std::fs;
use std::path::{Component, Path, PathBuf};

use lkjscript_core::{Error, Result};

use super::model::Manifest;

pub(super) const MANIFEST: &str = "lkjscript.package.json";
pub(super) const LOCK: &str = "lkjscript.lock.json";

pub(super) fn find_root(entry: &Path) -> Result<PathBuf> {
    let start = if entry.is_dir() {
        entry
    } else {
        entry.parent().unwrap_or_else(|| Path::new("."))
    };
    let mut current = start
        .canonicalize()
        .map_err(|error| Error::host(format!("canonicalize package search root: {error}")))?;
    loop {
        if current.join(MANIFEST).is_file() {
            return Ok(current);
        }
        if !current.pop() {
            return Err(Error::msg(format!(
                "no {MANIFEST} contains {}",
                entry.display()
            )));
        }
    }
}

pub(super) fn load(root: &Path) -> Result<(Manifest, Vec<u8>)> {
    let path = root.join(MANIFEST);
    reject_symlink(root, &path)?;
    let bytes = super::read::unchanged_file(&path, "package manifest")?;
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|error| Error::msg(format!("decode {}: {error}", path.display())))?;
    validate(root, &manifest)?;
    Ok((manifest, bytes))
}

fn validate(root: &Path, manifest: &Manifest) -> Result<()> {
    if manifest.schema != "lkjscript.package" {
        return Err(Error::msg("package schema must be lkjscript.package"));
    }
    super::contracts::require(&manifest.contract, lkjscript_contracts::PACKAGE_MANIFEST)?;
    name("package", &manifest.name)?;
    path("source_root", &manifest.source_root)?;
    sorted("modules", &manifest.modules)?;
    sorted("public", &manifest.public)?;
    sorted("capabilities", &manifest.capabilities)?;
    for capability in &manifest.capabilities {
        if lkjscript_core::CapabilityKind::parse(capability).is_none() {
            return Err(Error::msg(format!(
                "unknown package capability: {capability}"
            )));
        }
    }
    let source_root = root.join(&manifest.source_root);
    reject_symlink(root, &source_root)?;
    for module in &manifest.modules {
        path("module", module)?;
        if !module.ends_with(".lkjscript") {
            return Err(Error::msg(format!(
                "module must end in .lkjscript: {module}"
            )));
        }
        let module_path = root.join(module);
        reject_symlink(root, &module_path)?;
        if !module_path.is_file() || !module_path.starts_with(&source_root) {
            return Err(Error::msg(format!(
                "module is outside source_root: {module}"
            )));
        }
    }
    for public in &manifest.public {
        if manifest.modules.binary_search(public).is_err() {
            return Err(Error::msg(format!(
                "public module is not declared: {public}"
            )));
        }
    }
    targets(manifest)?;
    dependencies(root, manifest)?;
    Ok(())
}

fn targets(manifest: &Manifest) -> Result<()> {
    let names: Vec<_> = manifest
        .targets
        .iter()
        .map(|target| target.name.clone())
        .collect();
    sorted("target names", &names)?;
    for target in &manifest.targets {
        name("target", &target.name)?;
        if manifest.public.binary_search(&target.module).is_err() {
            return Err(Error::msg(format!(
                "target module is not public: {}",
                target.module
            )));
        }
    }
    Ok(())
}

fn dependencies(root: &Path, manifest: &Manifest) -> Result<()> {
    let names: Vec<_> = manifest
        .dependencies
        .iter()
        .map(|item| item.name.clone())
        .collect();
    sorted("dependency names", &names)?;
    for dependency in &manifest.dependencies {
        name("dependency", &dependency.name)?;
        path("dependency path", &dependency.path)?;
        exact_digest("dependency content_sha256", &dependency.content_sha256)?;
        let target = root.join(&dependency.path);
        reject_symlink(root, &target)?;
        if !target.join(MANIFEST).is_file() {
            return Err(Error::msg(format!(
                "dependency has no manifest: {}",
                dependency.path
            )));
        }
    }
    Ok(())
}

pub(super) fn path(label: &str, value: &str) -> Result<()> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(Error::msg(format!(
            "{label} must be a contained relative path: {value}"
        )));
    }
    Ok(())
}

pub(super) fn reject_symlink(root: &Path, path: &Path) -> Result<()> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| Error::msg("package path escapes root"))?;
    let mut current = root.to_path_buf();
    for part in relative.components() {
        current.push(part);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| Error::host(format!("inspect {}: {error}", current.display())))?;
        if metadata.file_type().is_symlink() {
            return Err(Error::msg(format!(
                "package symlink is forbidden: {}",
                current.display()
            )));
        }
    }
    Ok(())
}

fn sorted(label: &str, values: &[String]) -> Result<()> {
    if values.windows(2).all(|pair| pair[0] < pair[1]) {
        Ok(())
    } else {
        Err(Error::msg(format!("{label} must be sorted and unique")))
    }
}

fn name(label: &str, value: &str) -> Result<()> {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        Ok(())
    } else {
        Err(Error::msg(format!("invalid {label} name: {value}")))
    }
}

fn exact_digest(label: &str, value: &str) -> Result<()> {
    if lkjscript_contracts::ContractDigest::from_hex(value).is_some() {
        Ok(())
    } else {
        Err(Error::msg(format!(
            "{label} must be a full lowercase SHA-256 digest"
        )))
    }
}
