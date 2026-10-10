use super::model::Gate;
use crate::error::DevError;
use crate::evidence::{FileKind, FileProof, VerificationDigest};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// Declared Cargo outputs are executable targets of the checked workspace root.
// A regular file left at that path is not evidence that this invocation selected it.
pub(super) fn verify(
    repository: &Path,
    gate: &Gate,
    stdout_path: &Path,
    stdout: &FileProof,
) -> Result<(), DevError> {
    if gate.command.first().map(String::as_str) != Some("cargo") || gate.required_outputs.is_empty()
    {
        return Ok(());
    }
    if !matches!(
        gate.command.get(1).map(String::as_str),
        Some("build" | "test")
    ) || !gate
        .command
        .iter()
        .take_while(|argument| *argument != "--")
        .any(|argument| argument == "--message-format=json")
    {
        return Err(refusal(
            "executable producer requires Cargo JSON before test arguments",
        ));
    }
    let expected_bytes = stdout
        .bytes
        .ok_or_else(|| refusal("missing stdout length"))?;
    if stdout.kind != FileKind::File || expected_bytes > gate.maximum_stdout_bytes {
        return Err(refusal("unsafe or oversized producer record"));
    }
    let bytes = super::cache::read_bounded(stdout_path, expected_bytes)?;
    if bytes.len() as u64 != expected_bytes
        || stdout.digest.as_ref() != Some(&VerificationDigest::of(&bytes))
    {
        return Err(refusal("producer record changed after process observation"));
    }
    records(
        &bytes,
        &repository.join("Cargo.toml"),
        &gate.required_outputs,
    )
}

fn records(bytes: &[u8], manifest: &Path, outputs: &[PathBuf]) -> Result<(), DevError> {
    let mut selected = BTreeMap::new();
    for path in outputs {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| refusal("nonportable executable target name"))?;
        if selected.insert(name, (path.as_path(), false)).is_some() {
            return Err(refusal("ambiguous declared executable target"));
        }
    }
    for line in bytes.split(|byte| *byte == b'\n') {
        // Cargo documents that test/program output need not be JSON.
        if line.first() != Some(&b'{') {
            continue;
        }
        let message: Value = serde_json::from_slice(line)
            .map_err(|error| refusal(format!("malformed Cargo record: {error}")))?;
        match message.get("reason").and_then(Value::as_str) {
            Some("compiler-artifact") => admit_artifact(&message, manifest, &mut selected)?,
            Some("build-finished") => {
                if message.get("success").and_then(Value::as_bool) != Some(true) {
                    return Err(refusal("Cargo did not report successful build completion"));
                }
                if selected.values().any(|(_, found)| !found) {
                    return Err(refusal("missing exact root-package executable artifact"));
                }
                // Following stdout belongs to tests or the program, not the Cargo build.
                // It cannot supply a missing artifact or revoke an already completed build.
                return Ok(());
            }
            _ => {}
        }
    }
    Err(refusal("missing Cargo build-finished record"))
}

fn admit_artifact(
    message: &Value,
    manifest: &Path,
    selected: &mut BTreeMap<&str, (&Path, bool)>,
) -> Result<(), DevError> {
    if message
        .get("manifest_path")
        .and_then(Value::as_str)
        .map(Path::new)
        != Some(manifest)
    {
        return Ok(());
    }
    let Some(name) = message.pointer("/target/name").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some((expected, found)) = selected.get_mut(name) else {
        return Ok(());
    };
    if message
        .pointer("/target/kind")
        .and_then(Value::as_array)
        .is_none_or(|kinds| kinds.len() != 1 || kinds[0].as_str() != Some("bin"))
        || message.pointer("/profile/test").and_then(Value::as_bool) != Some(false)
    {
        return Ok(());
    }
    if *found {
        return Err(refusal("duplicate root-package executable artifact"));
    }
    let executable = message
        .get("executable")
        .and_then(Value::as_str)
        .map(Path::new);
    let listed = message
        .get("filenames")
        .and_then(Value::as_array)
        .is_some_and(|names| {
            names.iter().all(Value::is_string)
                && names
                    .iter()
                    .any(|name| name.as_str().map(Path::new) == Some(*expected))
        });
    if executable != Some(*expected) || !listed {
        return Err(refusal("Cargo selected a different executable output path"));
    }
    if message.get("fresh").and_then(Value::as_bool).is_none()
        || message
            .get("package_id")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        return Err(refusal("incomplete executable artifact identity"));
    }
    *found = true;
    Ok(())
}

fn refusal(message: impl Into<String>) -> DevError {
    DevError::corrupt(message)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod log_tests;
