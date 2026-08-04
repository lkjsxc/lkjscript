use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use serde::de::DeserializeOwned;

use super::digest;
use super::model::{LocatedFact, Manifest, Registry, Shard};
use super::validate;

const ROOT: &str = "meta/config/public-facts";
const MANIFEST: &str = "manifest.json";
const MAX_INPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_CONTENT_BYTES: u64 = 16 * 1024 * 1024;
const MAX_SHARDS: usize = 16;
const MAX_FACTS: usize = 256;

pub fn load(root: &Path) -> Result<Registry, String> {
    if root.join("meta/config/capability-status.json").exists() {
        return Err("obsolete capability-status authority remains".into());
    }
    let base = contained(root, Path::new(ROOT))?;
    let manifest_path = contained(&base, Path::new(MANIFEST))?;
    let (manifest, mut charged): (Manifest, u64) = read_json(&manifest_path, 32 * 1024)?;
    if manifest.schema != "lkjscript.public-facts" {
        return Err("public-fact manifest schema mismatch".into());
    }
    let expected = lkjscript_contracts::current_contracts()
        .map_err(|error| error.to_string())?
        .get(lkjscript_contracts::PUBLIC_FACTS)
        .map(lkjscript_contracts::RegisteredContract::digest);
    if lkjscript_contracts::ContractDigest::from_hex(&manifest.contract) != expected {
        return Err("public-fact contract mismatch".into());
    }
    if manifest.platform_revision != lkjscript_contracts::PLATFORM_REVISION {
        return Err("public-fact platform revision mismatch".into());
    }
    validate::manifest(&manifest, MAX_SHARDS)?;
    validate_entries(&base, &manifest)?;
    let mut facts = BTreeMap::new();
    for spec in &manifest.shards {
        let relative = safe_relative(&spec.path)?;
        let path = contained(&base, &relative)?;
        let remaining = MAX_INPUT_BYTES
            .checked_sub(charged)
            .ok_or("public-fact input exceeds byte limit")?;
        let (shard, bytes): (Shard, u64) = read_json(&path, remaining)?;
        charged = charged
            .checked_add(bytes)
            .ok_or("public-fact input byte overflow")?;
        if charged > MAX_INPUT_BYTES {
            return Err("public-fact input exceeds byte limit".into());
        }
        if shard.schema != "lkjscript.public-fact-shard" || shard.contract != manifest.contract {
            return Err(format!(
                "public-fact shard contract mismatch: {}",
                spec.path
            ));
        }
        validate::shard(spec, &shard)?;
        for fact in shard.facts {
            if facts.len() >= MAX_FACTS {
                return Err("public-fact count exceeds limit".into());
            }
            let id = fact.id.clone();
            if facts.contains_key(&id) {
                return Err(format!("duplicate public fact: {id}"));
            }
            facts.insert(
                id,
                LocatedFact {
                    fact,
                    shard: spec.path.clone(),
                    digest: String::new(),
                    content_digests: Vec::new(),
                },
            );
        }
    }
    validate::registry(root, &manifest, &facts)?;
    let mut content_cache = BTreeMap::new();
    let mut content_charged = 0u64;
    for located in facts.values_mut() {
        let (value, content) = digest::closure(
            root,
            &manifest.contract,
            &located.shard,
            &located.fact,
            &mut content_cache,
            &mut content_charged,
            MAX_CONTENT_BYTES,
        )?;
        located.digest = value;
        located.content_digests = content;
    }
    Ok(Registry { manifest, facts })
}

fn read_json<T: DeserializeOwned>(path: &Path, limit: u64) -> Result<(T, u64), String> {
    let bytes = crate::structure::repository_support::read_bounded(path, limit)?;
    let length = u64::try_from(bytes.len()).map_err(|_| "public-fact input size overflow")?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("decode {}: {error}", path.display()))?;
    Ok((value, length))
}

fn validate_entries(base: &Path, manifest: &Manifest) -> Result<(), String> {
    let mut expected: BTreeSet<_> = manifest
        .shards
        .iter()
        .map(|item| item.path.as_str())
        .collect();
    expected.insert(MANIFEST);
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(base).map_err(|error| format!("read {}: {error}", base.display()))? {
        if actual.len() > MAX_SHARDS {
            return Err("public-fact authority directory exceeds entry limit".into());
        }
        let entry = entry.map_err(|error| format!("read public-fact authority entry: {error}"))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "public-fact authority entry is not UTF-8")?;
        if !entry
            .file_type()
            .map_err(|error| format!("inspect public-fact authority entry: {error}"))?
            .is_file()
            || !actual.insert(name)
        {
            return Err("public-fact authority contains a non-file or duplicate entry".into());
        }
    }
    if actual.iter().map(String::as_str).collect::<BTreeSet<_>>() != expected {
        return Err("public-fact authority contains an unlisted or missing shard".into());
    }
    Ok(())
}

pub fn checked_path(root: &Path, value: &str) -> Result<PathBuf, String> {
    let relative = safe_relative(value)?;
    contained(root, &relative)
}

fn safe_relative(value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(format!("public-fact path is not normalized: {value}"));
    }
    Ok(path.to_path_buf())
}

fn contained(base: &Path, relative: &Path) -> Result<PathBuf, String> {
    let canonical_base = base
        .canonicalize()
        .map_err(|error| format!("canonicalize {}: {error}", base.display()))?;
    let joined = base.join(relative);
    let canonical = joined
        .canonicalize()
        .map_err(|error| format!("canonicalize {}: {error}", joined.display()))?;
    if !canonical.starts_with(canonical_base) {
        return Err(format!(
            "public-fact path escapes authority root: {}",
            joined.display()
        ));
    }
    Ok(canonical)
}
