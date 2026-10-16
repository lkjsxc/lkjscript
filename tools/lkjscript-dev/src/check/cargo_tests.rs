//! Direct execution of an exact test artifact from an admitted Cargo producer.
use super::cargo_origin_units::Unit;
use super::model::{CargoTestInput, Gate, GateReceipt};
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};

pub(super) const COMMAND: &str = "$CARGO_TEST";
pub(super) const LIFECYCLE: &str =
    "copied_binary_completes_normalized_standard_dependent_command_lifecycle";
const MAXIMUM_ORIGIN_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TestExecutable {
    pub(super) source: String,
    pub(super) executable: FileProof,
}

pub(super) fn inventory(
    repository: &Path,
    units: &[Unit],
) -> Result<BTreeMap<String, TestExecutable>, DevError> {
    let mut selected = BTreeMap::new();
    for unit in units {
        if Path::new(&unit.manifest_path) != repository.join("Cargo.toml")
            || unit.target.get("kind") != Some(&serde_json::json!(["test"]))
            || unit.profile.get("test") != Some(&Value::Bool(true))
            || unit.profile.get("opt_level").and_then(Value::as_str) != Some("3")
        {
            continue;
        }
        let name = unit
            .target
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| DevError::corrupt("Cargo test target has no name"))?;
        let source = unit
            .target
            .get("src_path")
            .and_then(Value::as_str)
            .ok_or_else(|| DevError::corrupt("Cargo test target has no source"))?;
        let executable = executable(unit)?;
        let test = TestExecutable {
            source: source.to_owned(),
            executable,
        };
        validate(repository, name, &test)?;
        if selected.insert(name.to_owned(), test).is_some() {
            return Err(DevError::corrupt(
                "Cargo test target was selected more than once",
            ));
        }
    }
    Ok(selected)
}

pub(super) fn executable(unit: &Unit) -> Result<FileProof, DevError> {
    // Cargo-origin admission attaches the exact schema outputs and build log
    // to every root unit. They remain part of its origin, not test executables.
    let attached = unit
        .build_script
        .as_ref()
        .and_then(|script| script.get("out_dir"))
        .and_then(Value::as_str)
        .map(|directory| {
            let directory = Path::new(directory);
            let mut paths = lkjscript::platform::canonical_schema::generated_outputs()
                .map(|(name, _)| directory.join(name))
                .into_iter()
                .collect::<BTreeSet<_>>();
            if let Some(parent) = directory.parent() {
                paths.insert(parent.join("output"));
            }
            paths
        })
        .unwrap_or_default();
    let mut artifacts = unit
        .filenames
        .iter()
        .filter(|file| !attached.contains(Path::new(&file.path)));
    let executable = artifacts
        .next()
        .ok_or_else(|| DevError::corrupt("Cargo test executable is absent"))?;
    if artifacts.next().is_some() {
        return Err(DevError::corrupt("Cargo test executable is ambiguous"));
    }
    Ok(executable.clone())
}

fn validate(repository: &Path, name: &str, test: &TestExecutable) -> Result<(), DevError> {
    let path = Path::new(&test.executable.path);
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        || Path::new(&test.source) != repository.join("tests").join(format!("{name}.rs"))
        || path.parent() != Some(repository.join("target/release/deps").as_path())
        || path
            .components()
            .any(|part| matches!(part, Component::CurDir | Component::ParentDir))
        || !path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|file| file.starts_with(&format!("{name}-")))
        || test.executable.kind != FileKind::File
        || test.executable.bytes.is_none_or(|bytes| bytes == 0)
        || test.executable.digest.is_none()
        || test.executable.mode.is_none_or(|mode| mode & 0o111 == 0)
        || test.executable.link_target.is_some()
    {
        return Err(DevError::corrupt(
            "Cargo test selection is not an exact release executable",
        ));
    }
    Ok(())
}

pub(super) fn origin_value(
    repository: &Path,
    input: &CargoTestInput,
    dependencies: &BTreeMap<String, GateReceipt>,
) -> Result<Value, DevError> {
    let producer = dependencies
        .get(&input.producer)
        .filter(|producer| producer.passed())
        .ok_or_else(|| DevError::corrupt("selected Cargo test has no passed producer"))?;
    let origin = producer
        .cargo_origin
        .as_ref()
        .ok_or_else(|| DevError::corrupt("selected Cargo test has no origin inventory"))?;
    let path = repository.join(&origin.path);
    if origin.kind != FileKind::File || evidence::proof(&path, origin.path.clone())? != *origin {
        return Err(DevError::corrupt("selected Cargo test origin changed"));
    }
    let bytes = super::cache::read_bounded(&path, MAXIMUM_ORIGIN_BYTES)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub(super) fn input_proofs(
    repository: &Path,
    input: &CargoTestInput,
    dependencies: &BTreeMap<String, GateReceipt>,
) -> Result<Vec<FileProof>, DevError> {
    if input.target == super::workspace_tests::TARGET {
        super::workspace_tests::input_proofs(
            repository,
            &origin_value(repository, input, dependencies)?,
        )
    } else {
        Ok(vec![input_proof(repository, input, dependencies)?])
    }
}

fn input_proof(
    repository: &Path,
    input: &CargoTestInput,
    dependencies: &BTreeMap<String, GateReceipt>,
) -> Result<FileProof, DevError> {
    let value = origin_value(repository, input, dependencies)?;
    let tests =
        if value.get("schema").and_then(Value::as_str) == Some("lkjscript-live-cargo-1") {
            serde_json::from_value::<BTreeMap<String, TestExecutable>>(
                value
                    .get("test_executables")
                    .cloned()
                    .ok_or_else(|| DevError::corrupt("live Cargo test inventory is absent"))?,
            )?
        } else {
            let units: Vec<Unit> =
                serde_json::from_value(value.get("units").cloned().ok_or_else(|| {
                    DevError::corrupt("archived Cargo unit inventory is absent")
                })?)?;
            inventory(repository, &units)?
        };
    let selected = tests
        .get(&input.target)
        .ok_or_else(|| DevError::corrupt("exact Cargo test executable was not produced"))?;
    validate(repository, &input.target, selected)?;
    let mut proof = selected.executable.clone();
    proof.path = evidence::relative(repository, Path::new(&proof.path));
    Ok(proof)
}

pub(super) fn verify_result(
    repository: &Path,
    gate: &Gate,
    stdout: &Path,
    inputs: &[FileProof],
) -> Result<(), DevError> {
    if gate.cargo_test.is_none() {
        return Ok(());
    }
    if super::workspace_tests::selected(gate) {
        return super::workspace_tests::verify_result(repository, stdout, inputs);
    }
    let bytes = super::cache::read_bounded(stdout, gate.maximum_stdout_bytes)?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|_| DevError::corrupt("selected test output is not UTF-8"))?;
    let expected = format!("test {LIFECYCLE} ... ok");
    if !text.lines().any(|line| line == "running 1 test")
        || !text.lines().any(|line| line == expected)
        || !text.lines().any(|line| {
            line.starts_with("test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured;")
        })
    {
        return Err(DevError::corrupt(
            "exact candidate lifecycle test did not pass once",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
