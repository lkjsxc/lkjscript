use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path};

use super::model::{ContentReference, WorkState};

pub fn validate(root: &Path, state: &WorkState) -> Result<(), String> {
    capsules(root, &state.selected_capsule_scope)?;
    for reference in &state.accepted_decisions {
        if !reference.path.starts_with("docs/decisions/")
            || !super::git::tracked(root, &reference.path)?
        {
            return Err(format!(
                "accepted decision is not a tracked decision: {}",
                reference.path
            ));
        }
        content(root, reference)?;
    }
    let evidence: BTreeSet<_> = state
        .evidence_references
        .iter()
        .map(|item| item.path.as_str())
        .collect();
    for reference in state
        .artifact_references
        .iter()
        .chain(&state.evidence_references)
    {
        content(root, reference)?;
    }
    for path in state
        .completed_actions
        .iter()
        .flat_map(|item| &item.evidence)
        .chain(state.command_results.iter().flat_map(|item| &item.evidence))
    {
        if !evidence.contains(path.as_str()) {
            return Err(format!("history evidence is not declared: {path}"));
        }
    }
    Ok(())
}

pub fn shape(state: &WorkState) -> Result<(), String> {
    let mut paths = BTreeSet::new();
    for reference in state
        .accepted_decisions
        .iter()
        .chain(&state.artifact_references)
        .chain(&state.evidence_references)
    {
        if !safe_path(&reference.path)
            || reference.sha256.len() != 64
            || !reference
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(format!("invalid content reference: {}", reference.path));
        }
        if !paths.insert((&reference.path, &reference.sha256)) {
            return Err(format!("duplicate content reference: {}", reference.path));
        }
    }
    Ok(())
}

fn capsules(root: &Path, selected: &[String]) -> Result<(), String> {
    let mut known = BTreeSet::new();
    for path in super::git::capsule_paths(root)? {
        let bytes = super::json::read_bounded(&root.join(&path), 32_768)?;
        let capsule: crate::model::Capsule = serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse capsule {path}: {error}"))?;
        known.insert(capsule.id);
    }
    for id in selected {
        if !known.contains(id) {
            return Err(format!("selected capsule does not exist: {id}"));
        }
    }
    Ok(())
}

fn content(root: &Path, reference: &ContentReference) -> Result<(), String> {
    let path = root.join(&reference.path);
    let lexical_metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("inspect {}: {error}", reference.path))?;
    if lexical_metadata.file_type().is_symlink() {
        return Err(format!("reference is a symbolic link: {}", reference.path));
    }
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("canonicalize repository root: {error}"))?;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("canonicalize {}: {error}", reference.path))?;
    if !canonical.starts_with(&canonical_root) {
        return Err(format!("reference escapes repository: {}", reference.path));
    }
    let metadata = canonical
        .metadata()
        .map_err(|error| format!("inspect {}: {error}", reference.path))?;
    if !metadata.is_file() || metadata.len() > super::bounds::REFERENCE_BYTES {
        return Err(format!(
            "reference is not a bounded regular file: {}",
            reference.path
        ));
    }
    let bytes = super::json::read_bounded(&canonical, super::bounds::REFERENCE_BYTES as usize)?;
    if crate::sha256::digest(&bytes) != reference.sha256 {
        return Err(format!("content hash mismatch: {}", reference.path));
    }
    Ok(())
}

fn safe_path(value: &str) -> bool {
    !value.is_empty()
        && !value.contains('\\')
        && value.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && Path::new(part)
                    .components()
                    .all(|item| matches!(item, Component::Normal(_)))
        })
}
