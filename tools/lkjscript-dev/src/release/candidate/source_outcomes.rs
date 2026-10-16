//! Bind complete ordinary public semantics to their original workspace execution.
use super::{public_inventory, require};
use crate::error::DevError;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) fn harness(stderr: &str, repository: &Path, selected: &Path) -> Result<(), DevError> {
    let mut observed = Vec::new();
    for line in stderr.lines() {
        if let Some(value) = line.trim().strip_prefix("Running tests/public_cli.rs (") {
            let path = value
                .strip_suffix(')')
                .ok_or_else(|| DevError::corrupt("incomplete original public harness identity"))?;
            let path = PathBuf::from(path);
            observed.push(if path.is_absolute() {
                path
            } else {
                repository.join(path)
            });
        }
    }
    require(
        observed.len() == 1 && observed[0] == selected,
        "original source public harness differs from Cargo-selected executable",
    )
}

fn outcome(line: &str) -> Option<(&str, &str)> {
    line.strip_prefix("test ")?.split_once(" ... ")
}

fn block(lines: &[&str], all: &BTreeSet<String>) -> Result<bool, DevError> {
    let mut observed = BTreeMap::new();
    let mut notices = BTreeSet::new();
    let mut passed = 0;
    let mut ignored = 0;
    for line in lines.iter().copied().filter(|line| !line.is_empty()) {
        if let Some((name, status)) = outcome(line) {
            if !all.contains(name) || observed.insert(name.to_owned(), status).is_some() {
                return Ok(false);
            }
            if status == "ok" {
                passed += 1;
            } else if name == public_inventory::OPT_IN_SCALE
                && (status == "ignored" || status.starts_with("ignored, "))
            {
                ignored += 1;
            } else {
                return Ok(false);
            }
        } else if let Some(name) = line
            .strip_prefix("test ")
            .and_then(|line| line.strip_suffix(" has been running for over 60 seconds"))
        {
            if !all.contains(name) || observed.contains_key(name) || !notices.insert(name) {
                return Ok(false);
            }
        } else if let Some(summary) = line.strip_prefix("test result: ok. ") {
            let Some((counts, duration)) = summary.split_once("; finished in ") else {
                return Ok(false);
            };
            return Ok(observed.keys().cloned().collect::<BTreeSet<_>>() == *all
                && counts
                    == format!(
                        "{passed} passed; 0 failed; {ignored} ignored; 0 measured; 0 filtered out"
                    )
                && !duration.is_empty());
        } else {
            return Ok(false);
        }
    }
    Ok(false)
}

pub(super) fn passed(
    stdout: &str,
    inventory: &public_inventory::Inventory,
) -> Result<(), DevError> {
    let lines = stdout.lines().collect::<Vec<_>>();
    let header = format!("running {} tests", inventory.all.len());
    let mut complete = 0;
    for (index, line) in lines.iter().enumerate() {
        if *line == header && block(&lines[index + 1..], &inventory.all)? {
            complete += 1;
        }
    }
    require(
        complete == 1,
        "original source public harness lacks one complete passing ordinary inventory",
    )?;
    // A same-named result from another binary cannot fill a missing public case
    // or provide a second passing observation for the selected source harness.
    let mut counts = BTreeMap::new();
    for line in lines {
        if let Some((name, _)) = outcome(line)
            && inventory.all.contains(name)
        {
            *counts.entry(name).or_insert(0) += 1;
        }
    }
    require(
        inventory
            .all
            .iter()
            .all(|name| counts.get(name.as_str()) == Some(&1)),
        "source public outcomes are repeated across executable blocks",
    )
}

#[cfg(test)]
#[path = "source_outcomes_tests.rs"]
mod tests;
