use std::collections::BTreeMap;
use std::path::Path;

mod bounds;
mod digest;
mod load;
mod model;
mod report;
mod scan;
mod scan_marker;
#[cfg(test)]
mod tests;
mod validate;

pub(crate) use model::{Authority, Evidence, EvidenceClass, LocatedFact, Registry};

pub(crate) fn load(root: &Path) -> Result<Registry, String> {
    load::load(root)
}

pub(crate) fn check(root: &Path) -> usize {
    let registry = match load(root) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    let mut expected = BTreeMap::new();
    let mut failures = 0;
    for located in registry.facts.values() {
        for path in &located.fact.projections {
            let key = (path.clone(), located.fact.id.clone());
            if expected.insert(key, located).is_some() {
                eprintln!("duplicate public-fact projection: {path}");
                failures += 1;
            }
        }
    }
    let (found, scan_failures) = scan::claims(root);
    failures += scan_failures;
    for ((path, id), located) in &expected {
        let marker_digest = digest::projection(&located.digest).unwrap_or_default();
        match found.get(&(path.clone(), id.clone())) {
            Some(claim) if claim.status == located.fact.status && claim.digest == marker_digest => {
            }
            Some(claim) => {
                eprintln!(
                    "stale public-fact projection {path}: {id}; expected {} {}, found {} {}",
                    located.fact.status.name(),
                    marker_digest,
                    claim.status.name(),
                    claim.digest
                );
                failures += 1;
            }
            None => {
                eprintln!("missing public-fact projection {path}: {id}");
                failures += 1;
            }
        }
    }
    for ((path, id), _) in found {
        if !expected.contains_key(&(path.clone(), id.clone())) {
            eprintln!("unregistered public-fact projection {path}: {id}");
            failures += 1;
        }
    }
    failures
}

pub(crate) fn generate(root: &Path) -> usize {
    let registry = match load(root) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return 1;
        }
    };
    report::write(root, &registry).map_or_else(
        |error| {
            eprintln!("{error}");
            1
        },
        |()| 0,
    )
}
