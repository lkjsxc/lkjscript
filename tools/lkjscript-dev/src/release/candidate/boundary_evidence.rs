//! Strict observations emitted by the trusted source-built boundary harness.
//! These bind its actual child-process observations; they are not independent
//! cryptographic attestations from an untrusted executable.
use super::{ArtifactIdentity, SchemaIdentity, archive, file, public_inventory, require};
use crate::{error::DevError, process};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const MAXIMUM_BYTES: u64 = 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    arguments: Vec<String>,
    exit_code: i32,
    joined: bool,
    environment_cleared: bool,
    executable_sha256: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    schema: SchemaIdentity,
    test: String,
    candidate_sha256: String,
    observations: Vec<Observation>,
    cleanup_complete: bool,
}

#[derive(Serialize)]
pub(super) struct Accepted {
    test: String,
    receipt: ArtifactIdentity,
    process_observations: usize,
}

fn option_values(arguments: &[&str], option: &str, expected: usize) -> bool {
    let mut count = 0;
    for (index, argument) in arguments.iter().enumerate() {
        if *argument == option {
            count += 1;
            if !arguments
                .get(index + 1)
                .is_some_and(|value| !value.is_empty() && !value.starts_with("--"))
            {
                return false;
            }
        }
    }
    count == expected
}

fn operation(boundary: &str, arguments: &[String]) -> bool {
    let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    let project =
        arguments.first() == Some(&"--project") && option_values(&arguments, "--project", 1);
    match boundary {
        "review_refresh" => project && arguments.get(2..4) == Some(&["change", "refresh"]),
        "cache_read_fifo" => project && arguments.get(2) == Some(&"check"),
        "cache_write_fifo" | "publication_lock" => {
            project && arguments.get(2..4) == Some(&["change", "apply"])
        }
        "detached_foreground" => {
            arguments.starts_with(&["run", "--deployment"])
                && option_values(&arguments, "--deployment", 1)
                && !arguments.contains(&"--batch-input-file")
                && !arguments.contains(&"--batch-result-file")
        }
        "deployment_batch" => {
            arguments.starts_with(&["run", "--deployment"])
                && option_values(&arguments, "--deployment", 1)
                && option_values(&arguments, "--batch-input-file", 1)
                && option_values(&arguments, "--batch-result-file", 1)
        }
        "shared_parallel_http" => {
            arguments.first() == Some(&"serve") && option_values(&arguments, "--deployment", 2)
        }
        _ => false,
    }
}

pub(super) fn read(root: &Path, candidate: &ArtifactIdentity) -> Result<Vec<Accepted>, DevError> {
    require(
        fs::symlink_metadata(root)?.is_dir(),
        "native boundary evidence root is not a real directory",
    )?;
    let expected = public_inventory::BOUNDARIES
        .map(|boundary| format!("{boundary}.json"))
        .into_iter()
        .collect::<BTreeSet<_>>();
    let mut actual = BTreeSet::new();
    for entry in fs::read_dir(root)? {
        let name = entry?
            .file_name()
            .into_string()
            .map_err(|_| DevError::corrupt("native boundary evidence filename is not UTF-8"))?;
        require(
            actual.len() < expected.len() && expected.contains(&name) && actual.insert(name),
            "native boundary evidence has an unexpected or repeated file",
        )?;
    }
    require(actual == expected, "native boundary evidence is incomplete")?;
    public_inventory::BOUNDARIES
        .into_iter()
        .map(|boundary| {
            let name = format!("{boundary}.json");
            let path = root.join(&name);
            let length = archive::ensure_regular(&path, "native boundary observation")?.len();
            require(
                length > 0 && length <= MAXIMUM_BYTES,
                "native boundary observation exceeds its byte bound",
            )?;
            let before = file(&path, &name)?;
            let marker: Marker =
                serde_json::from_slice(&process::read_bounded(&path, MAXIMUM_BYTES)?)?;
            let test = public_inventory::boundary_name(boundary);
            require(
                marker.schema.identity == "lkjscript-native-boundary-smoke"
                    && marker.schema.version == 1
                    && marker.test == test
                    && marker.candidate_sha256 == candidate.sha256.as_str()
                    && marker.cleanup_complete
                    && marker.observations.len() == 1,
                "native boundary observation has different bytes, identity or cleanup",
            )?;
            for observation in &marker.observations {
                require(
                    observation.joined
                        && observation.environment_cleared
                        && observation.executable_sha256 == candidate.sha256.as_str()
                        && (0..=255).contains(&observation.exit_code)
                        && !observation.arguments.is_empty()
                        && observation.arguments.len() <= 256
                        && observation.arguments.iter().all(|argument| {
                            argument.len() <= 16 * 1024 && !argument.contains('\0')
                        })
                        && observation.arguments.iter().map(String::len).sum::<usize>()
                            <= 64 * 1024,
                    "native boundary observation is substituted, unjoined or unbounded",
                )?;
            }
            require(
                marker
                    .observations
                    .iter()
                    .any(|observation| operation(boundary, &observation.arguments)),
                "native boundary marker omits its required actual operation",
            )?;
            require(
                before == file(&path, &name)?,
                "native boundary observation changed",
            )?;
            Ok(Accepted {
                test,
                receipt: before,
                process_observations: marker.observations.len(),
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "boundary_evidence_tests.rs"]
pub(super) mod tests;
