use std::path::Path;

use lkjscript_core::{Error, Result};

use super::{encoding, model::Target};

pub(crate) struct PreparedPackageFacts {
    pub package_content: [u8; 32],
    pub package_root: [u8; 32],
    pub entry: [u8; 32],
    pub module_memory_closure: [u8; 32],
    pub witness_closure: [u8; 32],
}

pub(crate) fn facts(
    root: &Path,
    entry: &Path,
    plan: &crate::HirMemoryPlan,
) -> Result<PreparedPackageFacts> {
    let (lock, _) = encoding::read(root)?;
    let package = lock
        .packages
        .iter()
        .find(|package| package.origin == ".")
        .ok_or_else(|| Error::msg("package lock omits root package"))?;
    let relative = entry
        .canonicalize()
        .map_err(|error| Error::host(format!("canonicalize package entry: {error}")))?
        .strip_prefix(root)
        .map_err(|_| Error::msg("package entry escapes root"))?
        .to_str()
        .ok_or_else(|| Error::msg("package entry is not UTF-8"))?
        .replace('\\', "/");
    let locked_target = package
        .targets
        .iter()
        .find(|target| target.module == relative)
        .ok_or_else(|| Error::msg("compiled package module is not an executable target"))?;
    let generated = super::target_memory::target_record(
        &Target {
            name: locked_target.name.clone(),
            module: locked_target.module.clone(),
        },
        plan,
    )?;
    if generated != *locked_target {
        return Err(Error::msg(
            "compiled memory plan or witness closure differs from locked target",
        ));
    }
    let module = package
        .modules
        .iter()
        .find(|module| module.id == relative)
        .ok_or_else(|| Error::msg("compiled package module is absent from lock"))?;
    let package_content = digest(&package.package_sha256, "package content")?;
    let package_root = digest(&lock.root, "package root")?;
    let mut memory = Vec::new();
    push_digest(
        &mut memory,
        &package.package_memory_interface_sha256,
        "package memory",
    )?;
    push_digest(
        &mut memory,
        &module.module_memory_interface_sha256,
        "module memory",
    )?;
    for dependency in &package.dependencies {
        push_digest(&mut memory, dependency, "dependency package")?;
    }
    let module_memory_closure = domain_hash(b"lkjscript.locked-module-memory-closure", &memory);
    let encoded_target = serde_json::to_vec(locked_target)
        .map_err(|error| Error::msg(format!("encode locked target closure: {error}")))?;
    let witness_closure = domain_hash(b"lkjscript.locked-target-witness-closure", &encoded_target);
    Ok(PreparedPackageFacts {
        package_content,
        package_root,
        entry: domain_hash(b"lkjscript.locked-package-entry", relative.as_bytes()),
        module_memory_closure,
        witness_closure,
    })
}

fn push_digest(output: &mut Vec<u8>, text: &str, name: &str) -> Result<()> {
    output.extend_from_slice(&digest(text, name)?);
    Ok(())
}

fn digest(text: &str, name: &str) -> Result<[u8; 32]> {
    lkjscript_contracts::ContractDigest::from_hex(text)
        .map(lkjscript_contracts::ContractDigest::as_bytes)
        .filter(|value| *value != [0; 32])
        .ok_or_else(|| Error::msg(format!("{name} is not a nonzero canonical digest")))
}

fn domain_hash(domain: &[u8], value: &[u8]) -> [u8; 32] {
    let mut bytes = Vec::with_capacity(domain.len() + 8 + value.len());
    bytes.extend_from_slice(domain);
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value);
    lkjscript_contracts::sha256(&bytes)
}
