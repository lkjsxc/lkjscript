use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::bounds;
use super::load::checked_path;
use super::model::{Authority, Fact, LocatedFact, Manifest, Shard, ShardSpec, Status};

const MAX_AGGREGATE_MEMBERS: usize = 8_192;

pub fn manifest(manifest: &Manifest, limit: usize) -> Result<(), String> {
    if manifest.shards.is_empty() || manifest.shards.len() > limit {
        return Err("public-fact shard count is outside bounds".into());
    }
    let mut prior_last = "";
    let mut paths = BTreeSet::new();
    for spec in &manifest.shards {
        bounds::stable_id(&spec.first, "shard first")?;
        bounds::stable_id(&spec.last, "shard last")?;
        if spec.first.as_str() <= prior_last || spec.first > spec.last || !paths.insert(&spec.path)
        {
            return Err(format!(
                "public-fact shard ranges are not canonical: {}",
                spec.path
            ));
        }
        if !spec.path.ends_with(".json") || spec.path.contains('/') {
            return Err(format!("public-fact shard path is invalid: {}", spec.path));
        }
        prior_last = &spec.last;
    }
    Ok(())
}

pub fn shard(spec: &ShardSpec, shard: &Shard) -> Result<(), String> {
    let mut prior = "";
    for fact in &shard.facts {
        if fact.id.as_str() <= prior || fact.id < spec.first || fact.id > spec.last {
            return Err(format!(
                "public facts are unsorted or in the wrong shard: {}",
                fact.id
            ));
        }
        prior = &fact.id;
    }
    if shard.facts.first().map(|fact| &fact.id) != Some(&spec.first)
        || shard.facts.last().map(|fact| &fact.id) != Some(&spec.last)
    {
        return Err(format!("public-fact shard range is stale: {}", spec.path));
    }
    Ok(())
}

pub fn registry(
    root: &Path,
    manifest: &Manifest,
    facts: &BTreeMap<String, LocatedFact>,
) -> Result<(), String> {
    let mut aggregate = 0usize;
    for located in facts.values() {
        let fact = &located.fact;
        bounds::stable_id(&fact.id, "fact")?;
        bounds::text(&fact.interface, "positive interface")?;
        if fact.platform_revision == 0 || fact.platform_revision > manifest.platform_revision {
            return Err(format!("invalid platform cut for public fact: {}", fact.id));
        }
        for collection in [
            &fact.scope,
            &fact.implementation_anchors,
            &fact.projections,
            &fact.dependencies,
            &fact.invalidated_by,
            &fact.contracts,
        ] {
            bounds::collection(collection, &fact.id)?;
            aggregate = aggregate
                .checked_add(collection.len())
                .ok_or("public-fact member count overflow")?;
        }
        if fact.scope.is_empty() || fact.exclusions.is_empty() || fact.projections.is_empty() {
            return Err(format!("incomplete public fact boundary: {}", fact.id));
        }
        if fact.status == Status::Current
            && (fact.implementation_anchors.is_empty() || fact.evidence.is_empty())
        {
            return Err(format!(
                "Current fact lacks implementation or evidence: {}",
                fact.id
            ));
        }
        validate_exclusions(fact, &mut aggregate)?;
        validate_evidence(fact, &mut aggregate)?;
        validate_paths(root, fact)?;
        for digest in fact
            .contracts
            .iter()
            .chain(fact.evidence.iter().flat_map(|item| item.contracts.iter()))
        {
            bounds::sha256(digest, &fact.id)?;
        }
        for dependency in fact.dependencies.iter().chain(&fact.invalidated_by) {
            if dependency == &fact.id || !facts.contains_key(dependency) {
                return Err(format!(
                    "unknown or self-referential public fact edge: {} -> {dependency}",
                    fact.id
                ));
            }
        }
    }
    if aggregate > MAX_AGGREGATE_MEMBERS {
        return Err("public-fact aggregate member limit exceeded".into());
    }
    bounds::acyclic(facts)
}

fn validate_exclusions(fact: &Fact, aggregate: &mut usize) -> Result<(), String> {
    if fact.exclusions.len() > bounds::MAX_MEMBERS {
        return Err(format!("too many exclusions: {}", fact.id));
    }
    let mut prior = "";
    for item in &fact.exclusions {
        bounds::stable_id(&item.id, "exclusion")?;
        bounds::text(&item.interface, "exclusion interface")?;
        if item.id.as_str() <= prior {
            return Err(format!("exclusions are not unique and sorted: {}", fact.id));
        }
        prior = &item.id;
    }
    *aggregate = aggregate
        .checked_add(fact.exclusions.len())
        .ok_or("public-fact member count overflow")?;
    Ok(())
}

fn validate_evidence(fact: &Fact, aggregate: &mut usize) -> Result<(), String> {
    if fact.evidence.len() > bounds::MAX_MEMBERS {
        return Err(format!("too much evidence: {}", fact.id));
    }
    let mut prior = "";
    for item in &fact.evidence {
        if item.path.as_str() <= prior || item.not_tested.len() > bounds::MAX_MEMBERS {
            return Err(format!(
                "evidence is not unique, sorted, or bounded: {}",
                fact.id
            ));
        }
        prior = &item.path;
        bounds::collection(&item.contracts, &fact.id)?;
        bounds::collection(&item.not_tested, &fact.id)?;
        if let Some(commit) = &item.commit {
            bounds::commit(commit, &fact.id)?;
        }
    }
    let nested = fact.evidence.iter().try_fold(0usize, |total, item| {
        total
            .checked_add(item.contracts.len())
            .and_then(|value| value.checked_add(item.not_tested.len()))
            .ok_or("public-fact member count overflow")
    })?;
    *aggregate = aggregate
        .checked_add(fact.evidence.len())
        .and_then(|value| value.checked_add(nested))
        .ok_or("public-fact member count overflow")?;
    Ok(())
}

fn validate_paths(root: &Path, fact: &Fact) -> Result<(), String> {
    if let Authority::RepositoryPath { path } = &fact.authority {
        checked_path(root, path)?;
    }
    for path in fact
        .implementation_anchors
        .iter()
        .chain(fact.projections.iter())
        .chain(fact.evidence.iter().map(|item| &item.path))
    {
        checked_path(root, path)?;
    }
    if fact.projections.iter().any(|path| !path.ends_with(".md")) {
        return Err(format!("projection is not Markdown: {}", fact.id));
    }
    Ok(())
}
