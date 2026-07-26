use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

mod scan;
#[cfg(test)]
mod tests;

const SCHEMA: &str = "lkjscript.capability-status";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub(super) enum Status {
    Current,
    AcceptedTarget,
    AcceptedContract,
    AcceptedSelection,
    Experimental,
    Deferred,
    Rejected,
    Superseded,
    Historical,
}

impl Status {
    fn name(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::AcceptedTarget => "accepted-target",
            Self::AcceptedContract => "accepted-contract",
            Self::AcceptedSelection => "accepted-selection",
            Self::Experimental => "experimental",
            Self::Deferred => "deferred",
            Self::Rejected => "rejected",
            Self::Superseded => "superseded",
            Self::Historical => "historical",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    schema: String,
    contract: String,
    capabilities: Vec<Capability>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capability {
    id: String,
    status: Status,
    interface: String,
    authority: String,
    evidence: String,
    claimants: Vec<String>,
}

pub(crate) fn check(root: &Path) -> usize {
    let path = root.join("meta/config/capability-status.json");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("read capability status registry: {error}");
            return 1;
        }
    };
    let registry: Registry = match serde_json::from_slice(&bytes) {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("decode capability status registry: {error}");
            return 1;
        }
    };
    let mut failures = validate_registry(root, &registry);
    let expected_contract = lkjscript_contracts::current_contracts()
        .ok()
        .and_then(|contracts| {
            contracts
                .get(lkjscript_contracts::CAPABILITY_STATUS)
                .map(lkjscript_contracts::RegisteredContract::digest)
        });
    if registry.schema != SCHEMA
        || lkjscript_contracts::ContractDigest::from_hex(&registry.contract) != expected_contract
    {
        eprintln!("capability status contract mismatch; update the registry");
        failures += 1;
    }
    let expected = expected_claims(&registry);
    let (found, scan_failures) = scan::claims(root);
    failures += scan_failures;
    for ((path, id), status) in &expected {
        match found.get(&(path.clone(), id.clone())) {
            Some(actual) if actual == status => {}
            Some(actual) => {
                eprintln!(
                    "status mismatch {path}: {id}; expected {}, found {}",
                    status.name(),
                    actual.name()
                );
                failures += 1;
            }
            None => {
                eprintln!("missing status claim {path}: {id}={}", status.name());
                failures += 1;
            }
        }
    }
    for ((path, id), _) in found {
        if !expected.contains_key(&(path.clone(), id.clone())) {
            eprintln!("unregistered status claim {path}: {id}");
            failures += 1;
        }
    }
    failures
}

fn validate_registry(root: &Path, registry: &Registry) -> usize {
    let mut failures = 0;
    let mut prior = "";
    let mut ids = BTreeSet::new();
    for capability in &registry.capabilities {
        if capability.id.as_str() <= prior || !ids.insert(&capability.id) {
            eprintln!(
                "capability IDs must be unique and sorted: {}",
                capability.id
            );
            failures += 1;
        }
        prior = &capability.id;
        if !capability
            .id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            || capability.id.contains('/')
            || capability
                .id
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_digit)
        {
            eprintln!(
                "capability ID must be a stable unnumbered name: {}",
                capability.id
            );
            failures += 1;
        }
        if capability.interface.is_empty() || capability.claimants.is_empty() {
            eprintln!("incomplete capability status record: {}", capability.id);
            failures += 1;
        }
        for linked in [&capability.authority, &capability.evidence] {
            if !root.join(linked).is_file() {
                eprintln!(
                    "missing capability status link for {}: {linked}",
                    capability.id
                );
                failures += 1;
            }
        }
        if !strictly_sorted(&capability.claimants) {
            eprintln!(
                "capability claimants must be unique and sorted: {}",
                capability.id
            );
            failures += 1;
        }
        for claimant in &capability.claimants {
            if !root.join(claimant).is_file() {
                eprintln!("missing capability status claimant: {claimant}");
                failures += 1;
            }
        }
    }
    failures
}

fn strictly_sorted(values: &[String]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

fn expected_claims(registry: &Registry) -> BTreeMap<(String, String), Status> {
    let mut result = BTreeMap::new();
    for capability in &registry.capabilities {
        for path in &capability.claimants {
            result.insert((path.clone(), capability.id.clone()), capability.status);
        }
    }
    result
}
