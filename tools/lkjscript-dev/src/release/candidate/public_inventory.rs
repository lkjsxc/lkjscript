//! Full source semantic inventory and the explicit final-byte boundary supplement.
use crate::error::DevError;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[path = "public_inventory_fold.rs"]
mod folds;

#[path = "public_inventory_incremental.rs"]
mod incremental;

#[path = "public_inventory_fifo.rs"]
mod fifo;

#[path = "public_inventory_sequences.rs"]
mod sequences;

#[path = "public_inventory_nominals.rs"]
mod nominals;

#[path = "public_inventory_storage.rs"]
mod storage;

#[path = "public_inventory_associated.rs"]
mod associated;

#[path = "public_inventory_read_views.rs"]
mod read_views;

const REQUIRED_FAMILIES: [(&str, Option<&str>); 7] = [
    ("native_owned_", None),
    ("native_byte_buffer_", None),
    ("native_byte_ranges_", None),
    ("resident_policy", None),
    ("native_parallel_", Some("parallel")),
    ("native_parallel_reads_", Some("native_parallel_reads")),
    ("native_refresh", None),
];
const TOPOLOGY: &str = "copied_binary_authors_builds_and_serves_interactive_topology_from_minimal";
const MAP_MATRIX: &str = "native_map_entries::native_map_entries_preserve_results_and_linear_projection_work_after_detachment";
const MAXIMUM_TESTS: usize = 4096;

pub(super) const BOUNDARIES: [&str; 7] = [
    "review_refresh",
    "cache_read_fifo",
    "cache_write_fifo",
    "publication_lock",
    "detached_foreground",
    "deployment_batch",
    "shared_parallel_http",
];

pub(super) fn boundary_name(boundary: &str) -> String {
    format!("native_boundary_smokes::{boundary}")
}

#[derive(Debug, serde::Serialize)]
pub(super) struct Inventory {
    pub(super) all: BTreeSet<String>,
    pub(super) selected: BTreeSet<String>,
}

/// This separately invoked scale experiment is deliberately outside ordinary
/// source acceptance. No other current or future public case may be ignored.
pub(super) const OPT_IN_SCALE: &str =
    "reviewed_change_plan_scale_streams_detail_beyond_stdout_envelope";

fn require(condition: bool, message: &str) -> Result<(), DevError> {
    super::require(condition, message)
}

fn required_family(name: &str, family: &str, namespace: Option<&str>) -> bool {
    let (modules, test) = name.rsplit_once("::").unwrap_or(("", name));
    // Prefix families identify test functions, not their enclosing namespaces.
    let marker_matches = if family.ends_with('_') {
        test.starts_with(family)
    } else {
        name.contains(family)
    };
    marker_matches
        && namespace.is_none_or(|namespace| modules.split("::").any(|module| module == namespace))
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
    for (family, namespace) in REQUIRED_FAMILIES {
        require(
            all.iter()
                .any(|name| required_family(name, family, namespace)),
            &format!("required public harness family is absent: {family}"),
        )?;
    }
    require(
        all.contains(TOPOLOGY),
        "required interactive topology witness is absent",
    )?;
    require(
        all.contains(MAP_MATRIX),
        "required complete detached Map execution matrix is absent",
    )?;
    folds::admit(&all)?;
    incremental::admit(&all)?;
    fifo::admit(&all)?;
    sequences::admit(&all)?;
    nominals::admit(&all)?;
    storage::admit(&all)?;
    associated::admit(&all)?;
    read_views::admit(&all)?;
    let selected = BOUNDARIES
        .map(boundary_name)
        .into_iter()
        .collect::<BTreeSet<_>>();
    require(
        selected.is_subset(&all),
        "required exact native boundary smoke is absent",
    )?;
    Ok(Inventory { all, selected })
}

pub(super) fn passed(output: &str, inventory: &Inventory) -> Result<(), DevError> {
    let mut observed = BTreeSet::new();
    let mut notices = BTreeSet::new();
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
            .and_then(|line| line.strip_suffix(" has been running for over 60 seconds"))
        {
            // Libtest notices are observations only. They never count as a
            // completed test or replace the required full passing summary.
            require(
                headers == 1
                    && summaries == 0
                    && inventory.selected.contains(name)
                    && !observed.contains(name)
                    && notices.insert(name.to_owned()),
                "public harness long-running notice is not one pending selected test",
            )?;
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
pub(super) mod tests;
