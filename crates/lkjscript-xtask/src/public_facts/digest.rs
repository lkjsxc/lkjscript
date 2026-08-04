use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::load::checked_path;
use super::model::{Authority, Fact};

const MAX_ANCHOR_BYTES: u64 = 4 * 1024 * 1024;

pub fn closure(
    root: &Path,
    contract: &str,
    shard: &str,
    fact: &Fact,
    cache: &mut BTreeMap<String, String>,
    charged: &mut u64,
    limit: u64,
) -> Result<(String, Vec<(String, String)>), String> {
    let mut paths = BTreeSet::new();
    if let Authority::RepositoryPath { path } = &fact.authority {
        paths.insert(path.clone());
    }
    paths.extend(fact.implementation_anchors.iter().cloned());
    paths.extend(fact.evidence.iter().map(|item| item.path.clone()));
    let mut content = Vec::new();
    for path in paths {
        let digest = if let Some(value) = cache.get(&path) {
            value.clone()
        } else {
            let checked = checked_path(root, &path)?;
            let remaining = limit
                .checked_sub(*charged)
                .ok_or("public-fact content digest aggregate limit exceeded")?;
            let (value, bytes) = content_digest(&checked, &path, remaining.min(MAX_ANCHOR_BYTES))?;
            charge_content(charged, bytes, limit)?;
            cache.insert(path.clone(), value.clone());
            value
        };
        content.push((path, digest));
    }
    let mut bytes = Vec::new();
    append(&mut bytes, contract.as_bytes())?;
    append(&mut bytes, shard.as_bytes())?;
    append(
        &mut bytes,
        &serde_json::to_vec(fact)
            .map_err(|error| format!("encode public fact {}: {error}", fact.id))?,
    )?;
    for (path, digest) in &content {
        append(&mut bytes, path.as_bytes())?;
        append(&mut bytes, digest.as_bytes())?;
    }
    Ok((crate::sha256::digest(&bytes), content))
}

fn charge_content(charged: &mut u64, bytes: u64, limit: u64) -> Result<(), String> {
    *charged = charged
        .checked_add(bytes)
        .ok_or("public-fact content digest byte overflow")?;
    if *charged > limit {
        return Err("public-fact content digest aggregate limit exceeded".into());
    }
    Ok(())
}

fn content_digest(path: &Path, label: &str, limit: u64) -> Result<(String, u64), String> {
    let bytes = crate::structure::repository_support::read_bounded(path, limit)
        .map_err(|error| format!("read public-fact input {label}: {error}"))?;
    let mut normalized = Vec::with_capacity(bytes.len());
    for line in bytes.split_inclusive(|byte| *byte == b'\n') {
        let body = line
            .strip_suffix(b"\n")
            .unwrap_or(line)
            .strip_suffix(b"\r")
            .unwrap_or_else(|| line.strip_suffix(b"\n").unwrap_or(line));
        let marker = std::str::from_utf8(body)
            .ok()
            .is_some_and(exact_projection_marker);
        if !marker {
            normalized.extend_from_slice(line);
        }
    }
    Ok((
        crate::sha256::digest(&normalized),
        u64::try_from(bytes.len()).map_err(|_| "public-fact content size overflow")?,
    ))
}

fn exact_projection_marker(line: &str) -> bool {
    let Some(body) = line
        .strip_prefix("<!-- LKJ-F ")
        .and_then(|value| value.strip_suffix(" -->"))
    else {
        return false;
    };
    let fields: Vec<_> = body.split_whitespace().collect();
    let [id, status, digest] = fields.as_slice() else {
        return false;
    };
    !id.is_empty()
        && matches!(
            *status,
            "current"
                | "accepted-target"
                | "accepted-contract"
                | "accepted-selection"
                | "experimental"
                | "deferred"
                | "rejected"
                | "superseded"
                | "historical"
        )
        && digest.len() == 43
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub fn projection(hex: &str) -> Result<String, String> {
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err("public-fact digest is not SHA-256".into());
    }
    let mut bytes = Vec::with_capacity(32);
    for index in (0..hex.len()).step_by(2) {
        bytes.push(
            u8::from_str_radix(&hex[index..index + 2], 16)
                .map_err(|_| "public-fact digest is not lowercase hexadecimal")?,
        );
    }
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::with_capacity(43);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        output.push(ALPHABET[usize::from(first >> 2)] as char);
        let second = chunk.get(1).copied().unwrap_or_default();
        output.push(ALPHABET[usize::from((first & 3) << 4 | second >> 4)] as char);
        if chunk.len() > 1 {
            let third = chunk.get(2).copied().unwrap_or_default();
            output.push(ALPHABET[usize::from((second & 15) << 2 | third >> 6)] as char);
            if chunk.len() > 2 {
                output.push(ALPHABET[usize::from(third & 63)] as char);
            }
        }
    }
    Ok(output)
}

fn append(output: &mut Vec<u8>, value: &[u8]) -> Result<(), String> {
    let length = u64::try_from(value.len()).map_err(|_| "public-fact digest length overflow")?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_exact_compact_projection_markers_are_normalized() {
        let digest = "a".repeat(43);
        assert!(super::exact_projection_marker(&format!(
            "<!-- LKJ-F test-fact current {digest} -->"
        )));
        assert!(!super::exact_projection_marker(
            "<!-- LKJ-FOO unrelated -->"
        ));
        assert!(super::projection(&"A".repeat(64)).is_err());
        let mut charged = 5;
        assert!(super::charge_content(&mut charged, 6, 10).is_err());
    }
}
