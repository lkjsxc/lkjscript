//! Independently checked inventory and libtest outcomes for final-byte acceptance.
use crate::error::DevError;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const FAMILIES: [&str; 5] = [
    "native_owned_",
    "native_byte_buffer_",
    "native_byte_ranges_",
    "resident_policy",
    "native_parallel",
];
const TOPOLOGY: &str = "copied_binary_authors_builds_and_serves_interactive_topology_from_minimal";
const MAXIMUM_TESTS: usize = 4096;

#[derive(Debug, serde::Serialize)]
pub(super) struct Inventory {
    pub(super) all: BTreeSet<String>,
    pub(super) selected: BTreeSet<String>,
}

fn require(condition: bool, message: &str) -> Result<(), DevError> {
    super::require(condition, message)
}

pub(super) fn cargo_harness(output: &str, repository: &Path) -> Result<PathBuf, DevError> {
    let mut selected = None;
    let mut finished = false;
    for line in output.lines() {
        require(!finished, "Cargo emitted records after build completion")?;
        let record: Value = serde_json::from_str(line)?;
        if record["reason"] == "build-finished" {
            require(
                record["success"] == true,
                "public harness build did not succeed",
            )?;
            finished = true;
        }
        if record["reason"] == "compiler-artifact"
            && record["target"]["name"] == "public_cli"
            && record["profile"]["test"] == true
        {
            require(
                selected.is_none(),
                "ambiguous public harness build artifact",
            )?;
            let source = record["target"]["src_path"]
                .as_str()
                .ok_or_else(|| DevError::corrupt("public harness source is missing"))?;
            require(
                Path::new(source) == repository.join("tests/public_cli.rs"),
                "public harness belongs to a different source checkout",
            )?;
            let executable = record["executable"]
                .as_str()
                .ok_or_else(|| DevError::corrupt("public harness executable is missing"))?;
            require(
                Path::new(executable).is_absolute(),
                "relative public harness path",
            )?;
            selected = Some(PathBuf::from(executable));
        }
    }
    require(finished, "public harness build completion is missing")?;
    selected.ok_or_else(|| DevError::corrupt("Cargo did not produce the public test harness"))
}

pub(super) fn inventory(output: &str) -> Result<Inventory, DevError> {
    let mut all = BTreeSet::new();
    for line in output.lines() {
        let name = line
            .strip_suffix(": test")
            .ok_or_else(|| DevError::corrupt("unexpected public harness inventory record"))?;
        require(
            !name.is_empty()
                && name.len() <= 512
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b':')
                && all.len() < MAXIMUM_TESTS
                && all.insert(name.to_owned()),
            "invalid, repeated or excessive public harness test identity",
        )?;
    }
    for family in FAMILIES {
        require(
            all.iter().any(|name| name.contains(family)),
            &format!("required public harness family is absent: {family}"),
        )?;
    }
    require(
        all.contains(TOPOLOGY),
        "required interactive topology witness is absent",
    )?;
    let selected = all
        .iter()
        .filter(|name| FAMILIES.iter().any(|family| name.contains(family)) || *name == TOPOLOGY)
        .cloned()
        .collect();
    Ok(Inventory { all, selected })
}

pub(super) fn passed(output: &str, inventory: &Inventory) -> Result<(), DevError> {
    let mut observed = BTreeSet::new();
    let mut summaries = 0;
    let mut headers = 0;
    let count = inventory.selected.len();
    for line in output.lines().filter(|line| !line.is_empty()) {
        if line == format!("running {count} tests") {
            headers += 1;
        } else if let Some(result) = line.strip_prefix("test result: ok. ") {
            let (counts, duration) = result
                .split_once("; finished in ")
                .ok_or_else(|| DevError::corrupt("incomplete public harness summary"))?;
            require(
                counts
                    == format!(
                        "{count} passed; 0 failed; 0 ignored; 0 measured; {} filtered out",
                        inventory.all.len() - count
                    )
                    && !duration.is_empty(),
                "public harness counts contain missing, failed or ignored cases",
            )?;
            summaries += 1;
        } else if let Some(name) = line
            .strip_prefix("test ")
            .and_then(|line| line.strip_suffix(" ... ok"))
        {
            require(
                inventory.selected.contains(name) && observed.insert(name.to_owned()),
                "public harness ran an unexpected or repeated test",
            )?;
        } else {
            return Err(DevError::corrupt(
                "public harness output is not complete passing evidence",
            ));
        }
    }
    require(
        headers == 1 && summaries == 1 && observed == inventory.selected,
        "public harness did not pass every enumerated selected test exactly once",
    )
}

#[cfg(test)]
#[path = "public_inventory_tests.rs"]
mod tests;
