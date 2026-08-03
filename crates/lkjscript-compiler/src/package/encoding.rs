use std::fs;
use std::path::Path;

use lkjscript_core::{Error, Result};

use super::model::{LockFile, LockedModule, LockedPackage};

pub(super) fn encode(lock: &LockFile) -> Result<Vec<u8>> {
    let mut lines = vec![
        "{".into(),
        field("schema", &lock.schema, true)?,
        field("contract", &lock.contract, true)?,
        field("root", &lock.root, true)?,
        "  \"contracts\": {".into(),
    ];
    for (index, (name, digest)) in lock.contracts.iter().enumerate() {
        lines.push(format!(
            "    {}: {}{}",
            json(name)?,
            json(digest)?,
            comma(index + 1 < lock.contracts.len())
        ));
    }
    lines.push("  },".into());
    lines.push("  \"packages\": [".into());
    for (index, package) in lock.packages.iter().enumerate() {
        package_lines(package, index + 1 < lock.packages.len(), &mut lines)?;
    }
    lines.push("  ]".into());
    lines.push("}".into());
    Ok((lines.join("\n") + "\n").into_bytes())
}

fn package_lines(package: &LockedPackage, more: bool, lines: &mut Vec<String>) -> Result<()> {
    lines.push("    {".into());
    lines.push(field_at(6, "name", &package.name, true)?);
    lines.push(field_at(6, "origin", &package.origin, true)?);
    lines.push(field_at(
        6,
        "manifest_sha256",
        &package.manifest_sha256,
        true,
    )?);
    lines.push(field_at(
        6,
        "package_memory_interface_sha256",
        &package.package_memory_interface_sha256,
        true,
    )?);
    lines.push(field_at(
        6,
        "package_sha256",
        &package.package_sha256,
        true,
    )?);
    lines.push(format!(
        "      \"dependencies\": {},",
        serde_json::to_string(&package.dependencies)
            .map_err(|error| Error::msg(format!("encode lock dependencies: {error}")))?
    ));
    lines.push("      \"modules\": [".into());
    for (index, module) in package.modules.iter().enumerate() {
        lines.push(format!(
            "        {}{}",
            module_json(module)?,
            comma(index + 1 < package.modules.len())
        ));
    }
    lines.push("      ],".into());
    lines.push(format!(
        "      \"targets\": {}",
        serde_json::to_string(&package.targets)
            .map_err(|error| Error::msg(format!("encode lock targets: {error}")))?
    ));
    lines.push(format!("    }}{}", comma(more)));
    Ok(())
}

fn module_json(module: &LockedModule) -> Result<String> {
    serde_json::to_string(module)
        .map_err(|error| Error::msg(format!("encode locked module: {error}")))
}

fn field(name: &str, value: &str, trailing: bool) -> Result<String> {
    field_at(2, name, value, trailing)
}

fn field_at(indent: usize, name: &str, value: &str, trailing: bool) -> Result<String> {
    Ok(format!(
        "{}{}: {}{}",
        " ".repeat(indent),
        json(name)?,
        json(value)?,
        comma(trailing)
    ))
}

fn json(value: &str) -> Result<String> {
    serde_json::to_string(value).map_err(|error| Error::msg(format!("encode lock string: {error}")))
}

const fn comma(value: bool) -> &'static str {
    if value {
        ","
    } else {
        ""
    }
}

pub(super) fn read(root: &Path) -> Result<(LockFile, Vec<u8>)> {
    let path = root.join(super::manifest::LOCK);
    let bytes = fs::read(&path)
        .map_err(|error| Error::host(format!("read {}: {error}", path.display())))?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(Error::msg("package lock exceeds 16 MiB"));
    }
    let lock: LockFile = serde_json::from_slice(&bytes)
        .map_err(|error| Error::msg(format!("decode {}: {error}", path.display())))?;
    if encode(&lock)? != bytes {
        return Err(Error::msg("package lock is not canonically encoded"));
    }
    Ok((lock, bytes))
}
