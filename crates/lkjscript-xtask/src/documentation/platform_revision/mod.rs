use std::fs;
use std::path::Path;

use crate::util::walk;

mod git;

use git::comparison;

const RULE: &str = "LKJ-PLATFORM-REVISION";
const FIXED_CARGO_VERSION: &str = "0.0.0";
const FORBIDDEN_FIELDS: &[&str] = &[
    "language_version",
    "schema_version",
    "protocol_version",
    "database_version",
    "gui_version",
    "runtime_version",
    "abi_version",
    "format_version",
    "language-version",
    "schema-version",
    "protocol-version",
    "database-version",
    "gui-version",
    "runtime-version",
    "abi-version",
    "format-version",
];
const ENVELOPE_PRODUCERS: &[&str] = &[
    "crates/lkjscript-app/src/describe.rs",
    "crates/lkjscript-runtime/src/control/mod.rs",
];

pub(super) fn check(root: &Path) -> usize {
    let mut failures = 0;
    let revision = match read_revision(&root.join("meta/platform-revision")) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{RULE}: {error}");
            return 1;
        }
    };
    failures += cargo_metadata(root);
    failures += forbidden_fields(root);
    failures += envelopes(root);
    match comparison(root) {
        Ok((parent, changed)) => {
            if let Some(parent) = parent {
                if revision < parent {
                    eprintln!("{RULE}: revision decreased from {parent} to {revision}");
                    failures += 1;
                }
                if changed.iter().any(|path| public_contract_path(path)) && revision <= parent {
                    eprintln!("{RULE}: public contract change requires revision above {parent}");
                    failures += 1;
                }
            }
        }
        Err(error) => {
            eprintln!("{RULE}: {error}");
            failures += 1;
        }
    }
    failures
}

pub(super) fn read_revision(path: &Path) -> Result<u64, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    if bytes.len() < 2 || bytes.last() != Some(&b'\n') || bytes[..bytes.len() - 1].contains(&b'\n')
    {
        return Err("meta/platform-revision must be one decimal line".to_string());
    }
    let digits = &bytes[..bytes.len() - 1];
    if digits[0] == b'0' || !digits.iter().all(u8::is_ascii_digit) {
        return Err("meta/platform-revision must be canonical nonzero decimal".to_string());
    }
    std::str::from_utf8(digits)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value != 0)
        .ok_or_else(|| "meta/platform-revision exceeds nonzero u64".to_string())
}

fn cargo_metadata(root: &Path) -> usize {
    let mut failures = 0;
    let workspace = fs::read_to_string(root.join("Cargo.toml")).unwrap_or_default();
    if !workspace.contains(&format!("version = \"{FIXED_CARGO_VERSION}\"")) {
        eprintln!("{RULE}: workspace Cargo version must be fixed {FIXED_CARGO_VERSION}");
        failures += 1;
    }
    let Ok(entries) = fs::read_dir(root.join("crates")) else {
        eprintln!("{RULE}: cannot enumerate workspace crates");
        return failures + 1;
    };
    for manifest in entries
        .flatten()
        .map(|entry| entry.path().join("Cargo.toml"))
    {
        if manifest.is_file()
            && !fs::read_to_string(&manifest)
                .unwrap_or_default()
                .contains("version.workspace = true")
        {
            eprintln!("{RULE}: {} must inherit Cargo version", manifest.display());
            failures += 1;
        }
    }
    for package in fs::read_to_string(root.join("Cargo.lock"))
        .unwrap_or_default()
        .split("[[package]]")
    {
        if package.contains("name = \"lkjscript-")
            && !package.contains(&format!("version = \"{FIXED_CARGO_VERSION}\""))
        {
            eprintln!("{RULE}: workspace package has stale Cargo.lock metadata");
            failures += 1;
        }
    }
    failures
}

fn forbidden_fields(root: &Path) -> usize {
    let mut paths = Vec::new();
    for directory in ["crates", "docs", "meta", "src"] {
        let _ = walk(&root.join(directory), &mut paths);
    }
    let mut failures = 0;
    for path in paths {
        let relative = path.strip_prefix(root).unwrap_or(&path);
        if relative.starts_with("docs/history")
            || relative.starts_with("meta/results")
            || relative.starts_with("crates/lkjscript-xtask/src/documentation/platform_revision")
        {
            continue;
        }
        let content = fs::read_to_string(&path).unwrap_or_default();
        for field in FORBIDDEN_FIELDS {
            if content.contains(&format!("\"{field}\"")) {
                eprintln!(
                    "{RULE}: forbidden subsystem field {field} in {}",
                    relative.display()
                );
                failures += 1;
            }
        }
    }
    failures
}

fn envelopes(root: &Path) -> usize {
    ENVELOPE_PRODUCERS
        .iter()
        .filter(|path| {
            let content = fs::read_to_string(root.join(path)).unwrap_or_default();
            let invalid =
                !content.contains("platform_revision") || !content.contains("contract_digest");
            if invalid {
                eprintln!("{RULE}: public envelope lacks revision or digest: {path}");
            }
            invalid
        })
        .count()
}

fn public_contract_path(path: &Path) -> bool {
    matches!(path.to_str(), Some("meta/platform-revision" | "Cargo.toml"))
        || path.starts_with("docs/decisions")
        || path.starts_with("docs/language")
        || path.starts_with("crates/lkjscript-contracts")
        || path.starts_with("crates/lkjscript-runtime")
        || path.starts_with("crates/lkjscript-host")
        || path.starts_with("crates/lkjscript-database")
        || path.starts_with("crates/lkjscript-vm")
        || path.starts_with("crates/lkjscript-app")
        || path.starts_with("src")
}
