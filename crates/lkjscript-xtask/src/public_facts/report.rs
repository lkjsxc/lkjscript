use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::Serialize;

use super::model::{Authority, Registry};

const MAX_REPORT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Serialize)]
struct Inventory<'a> {
    schema: &'static str,
    contract: &'a str,
    platform_revision: u64,
    facts: Vec<FactReport<'a>>,
    expected_markers: Vec<MarkerReport<'a>>,
}

#[derive(Serialize)]
struct FactReport<'a> {
    id: &'a str,
    status: &'a str,
    digest: &'a str,
    interface: &'a str,
    exclusions: Vec<&'a str>,
    authority: &'a Authority,
    content_digests: &'a [(String, String)],
    projections: &'a [String],
}

#[derive(Serialize)]
struct MarkerReport<'a> {
    projection: &'a str,
    fact: &'a str,
    marker: String,
}

pub fn write(root: &Path, registry: &Registry) -> Result<(), String> {
    let facts = registry
        .facts
        .values()
        .map(|item| FactReport {
            id: &item.fact.id,
            status: item.fact.status.name(),
            digest: &item.digest,
            interface: &item.fact.interface,
            exclusions: item
                .fact
                .exclusions
                .iter()
                .map(|value| value.interface.as_str())
                .collect(),
            authority: &item.fact.authority,
            content_digests: &item.content_digests,
            projections: &item.fact.projections,
        })
        .collect();
    let mut expected_markers = Vec::new();
    for item in registry.facts.values() {
        let digest = super::digest::projection(&item.digest)?;
        for projection in &item.fact.projections {
            expected_markers.push(MarkerReport {
                projection,
                fact: &item.fact.id,
                marker: format!(
                    "<!-- LKJ-F {} {} {digest} -->",
                    item.fact.id,
                    item.fact.status.name(),
                ),
            });
        }
    }
    let inventory = Inventory {
        schema: "lkjscript.public-fact-inventory",
        contract: &registry.manifest.contract,
        platform_revision: registry.manifest.platform_revision,
        facts,
        expected_markers,
    };
    let mut json = BoundedOutput::new(MAX_REPORT_BYTES);
    serde_json::to_writer_pretty(&mut json, &inventory)
        .map_err(|error| format!("encode public-fact report: {error}"))?;
    json.write_all(b"\n")
        .map_err(|error| format!("encode public-fact report: {error}"))?;
    let output = root.join("target/lkjscript/documentation");
    fs::create_dir_all(&output)
        .map_err(|error| format!("create documentation report directory: {error}"))?;
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("canonicalize repository root: {error}"))?;
    let output = output
        .canonicalize()
        .map_err(|error| format!("canonicalize documentation report directory: {error}"))?;
    if !output.starts_with(canonical_root) {
        return Err("documentation report directory escapes repository".into());
    }
    let old = output.join("expected-projections.md");
    if old.exists() {
        fs::remove_file(&old)
            .map_err(|error| format!("remove obsolete {}: {error}", old.display()))?;
    }
    write_atomic(&output.join("facts.json"), &json.bytes)
}

struct BoundedOutput {
    bytes: Vec<u8>,
    limit: usize,
}

impl BoundedOutput {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }
}

impl Write for BoundedOutput {
    fn write(&mut self, input: &[u8]) -> std::io::Result<usize> {
        let next = self
            .bytes
            .len()
            .checked_add(input.len())
            .ok_or_else(|| std::io::Error::other("report size overflow"))?;
        if next > self.limit {
            return Err(std::io::Error::other("report byte limit exceeded"));
        }
        self.bytes.extend_from_slice(input);
        Ok(input.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("create {}: {error}", temporary.display()))?;
    let result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(&temporary, path));
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(format!("publish {}: {error}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    #[test]
    fn report_writer_rejects_the_first_excess_byte() {
        let mut output = super::BoundedOutput::new(3);
        assert!(output.write_all(b"abc").is_ok());
        assert!(output.write_all(b"d").is_err());
    }
}
