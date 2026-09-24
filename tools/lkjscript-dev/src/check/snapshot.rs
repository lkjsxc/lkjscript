mod executable;

use super::model::{
    ExecutableProof, InputEntry, InputSnapshot, InputSource, PlatformIdentity, RuntimeIdentity,
};
use super::registry;
use crate::error::DevError;
use crate::evidence::{self, FileKind, VerificationDigest};
use crate::process;
pub(crate) use executable::{observe as observe_executable, proof as executable_proof};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::path::{Component, Path};
use std::time::Duration;

const MAXIMUM_COMMAND_OUTPUT_BYTES: usize = 64 * 1024 * 1024;
const MAXIMUM_COMMAND_ERROR_BYTES: usize = 1024 * 1024;

pub(crate) fn capture(repository: &Path) -> Result<InputSnapshot, DevError> {
    let tracked = listed_paths(repository, &["ls-files", "--cached", "-z"])?;
    let untracked = listed_paths(
        repository,
        &["ls-files", "--others", "--exclude-standard", "-z"],
    )?;
    let mut paths = BTreeMap::new();
    for path in tracked {
        paths.insert(path, InputSource::Tracked);
    }
    for path in untracked
        .into_iter()
        .filter(|path| relevant_untracked(path))
    {
        paths.entry(path).or_insert(InputSource::Untracked);
    }

    let mut entries = Vec::with_capacity(paths.len());
    let mut total_bytes = 0_u64;
    for (label, source) in paths {
        let path = repository.join(&label);
        let proof = evidence::proof(&path, label.clone())?;
        total_bytes = total_bytes
            .checked_add(proof.bytes.unwrap_or(0))
            .ok_or_else(|| DevError::infrastructure("input snapshot byte count overflow"))?;
        let gitlink_head = if proof.kind == FileKind::Directory {
            let path_text = path.to_string_lossy().into_owned();
            Some(checked_text(
                repository,
                &["git", "-C", &path_text, "rev-parse", "HEAD"],
            )?)
        } else {
            None
        };
        entries.push(InputEntry {
            source,
            proof,
            gitlink_head,
        });
    }
    let cargo_lock = entries
        .iter()
        .find(|entry| entry.proof.path == "Cargo.lock")
        .and_then(|entry| entry.proof.digest.clone())
        .ok_or_else(|| DevError::infrastructure("Cargo.lock is absent from the input snapshot"))?;
    let git_head = checked_text(repository, &["git", "rev-parse", "HEAD"])?;
    #[derive(Serialize)]
    struct SnapshotIdentity<'a> {
        git_head: &'a str,
        entries: &'a [InputEntry],
    }
    let identity = serde_json::to_vec(&SnapshotIdentity {
        git_head: &git_head,
        entries: &entries,
    })
    .map_err(|error| DevError::infrastructure(format!("encode input snapshot: {error}")))?;
    Ok(InputSnapshot {
        digest: VerificationDigest::of(&identity),
        git_head,
        cargo_lock_digest: cargo_lock,
        file_count: entries.len(),
        total_bytes,
        entries,
    })
}

pub(crate) fn repository_paths(repository: &Path) -> Result<Vec<String>, DevError> {
    let mut paths = BTreeSet::new();
    paths.extend(listed_paths(repository, &["ls-files", "--cached", "-z"])?);
    paths.extend(
        listed_paths(
            repository,
            &["ls-files", "--others", "--exclude-standard", "-z"],
        )?
        .into_iter()
        .filter(|path| relevant_untracked(path)),
    );
    Ok(paths.into_iter().collect())
}

pub(crate) fn runtime_identity(
    repository: &Path,
    commands: impl IntoIterator<Item = (String, String)>,
) -> Result<RuntimeIdentity, DevError> {
    let (environment_digest, environment_names) = environment_identity()?;
    let current = env::current_exe().map_err(|error| {
        DevError::infrastructure(format!("resolve harness executable: {error}"))
    })?;
    let harness = evidence::proof(&current, current.to_string_lossy().into_owned())?;
    let mut command_executables = BTreeMap::new();
    for (command, identity) in commands {
        let mut proof = executable_proof(repository, &command)?;
        proof.relabel(&identity);
        command_executables.insert(identity, proof);
    }
    let rustc = checked_text(repository, &["rustc", "-Vv"])?;
    let cargo = checked_text(repository, &["cargo", "-V"])?;
    let platform = PlatformIdentity {
        operating_system: env::consts::OS.to_owned(),
        architecture: env::consts::ARCH.to_owned(),
        family: env::consts::FAMILY.to_owned(),
        child_process_control: if cfg!(target_os = "linux") {
            "linux_waitable_child_nonblocking_streams_owned_tree".to_owned()
        } else {
            "unsupported".to_owned()
        },
    };
    let mut identity = RuntimeIdentity {
        digest: VerificationDigest::of(&[]),
        rustc,
        cargo,
        platform,
        environment_digest,
        environment_names,
        harness,
        command_executables,
    };
    identity.digest = runtime_digest(&identity)?;
    Ok(identity)
}

pub(super) fn runtime_digest(identity: &RuntimeIdentity) -> Result<VerificationDigest, DevError> {
    #[derive(Serialize)]
    struct RuntimeMaterial<'a> {
        rustc: &'a str,
        cargo: &'a str,
        platform: &'a PlatformIdentity,
        environment_digest: &'a VerificationDigest,
        harness: &'a evidence::FileProof,
        command_executables: &'a BTreeMap<String, ExecutableProof>,
    }
    let material = serde_json::to_vec(&RuntimeMaterial {
        rustc: &identity.rustc,
        cargo: &identity.cargo,
        platform: &identity.platform,
        environment_digest: &identity.environment_digest,
        harness: &identity.harness,
        command_executables: &identity.command_executables,
    })
    .map_err(|error| DevError::infrastructure(format!("encode runtime identity: {error}")))?;
    Ok(VerificationDigest::of(&material))
}

pub(super) fn validate_runtime(
    repository: &Path,
    retained: &RuntimeIdentity,
    harness_copy: &Path,
    commands: impl IntoIterator<Item = (String, String)>,
) -> Result<(), DevError> {
    let current = runtime_identity(repository, commands)?;
    let copy = evidence::proof(harness_copy, retained.harness.path.clone())?;
    let mut verifier = current.harness;
    verifier.path.clone_from(&retained.harness.path);
    if retained.digest != runtime_digest(retained)?
        || retained.rustc != current.rustc
        || retained.cargo != current.cargo
        || serde_json::to_vec(&retained.platform)? != serde_json::to_vec(&current.platform)?
        || retained.environment_digest != current.environment_digest
        || retained.environment_names != current.environment_names
        || retained.command_executables != current.command_executables
        || retained.harness != copy
        || retained.harness != verifier
    {
        return Err(DevError::corrupt(
            "source environment, toolchain or original verifier binding changed",
        ));
    }
    Ok(())
}

pub(crate) fn changed_profile(repository: &Path) -> Result<Vec<String>, DevError> {
    let output = checked_bytes(
        repository,
        &[
            "git",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
        ],
    )?;
    let records: Vec<&[u8]> = output
        .split(|value| *value == 0)
        .filter(|record| !record.is_empty())
        .collect();
    let mut paths = BTreeSet::new();
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        if record.len() < 4 {
            return Err(DevError::infrastructure(
                "git emitted malformed porcelain status",
            ));
        }
        let status = &record[..2];
        paths.insert(decode_path(&record[3..])?);
        if status.contains(&b'R') || status.contains(&b'C') {
            index += 1;
            let renamed = records
                .get(index)
                .ok_or_else(|| DevError::infrastructure("git omitted the second rename path"))?;
            paths.insert(decode_path(renamed)?);
        }
        index += 1;
    }

    let full = registry::profile("full")
        .ok_or_else(|| DevError::infrastructure("full profile is absent"))?;
    let product = registry::profile("product")
        .ok_or_else(|| DevError::infrastructure("product profile is absent"))?;
    let service = registry::profile("service")
        .ok_or_else(|| DevError::infrastructure("service profile is absent"))?;
    let mut selected = BTreeSet::from(["diff_check".to_owned(), "rust_only_tooling".to_owned()]);
    let mut widen_full = false;
    for path in paths {
        if path == "Cargo.toml"
            || path == "Cargo.lock"
            || path.starts_with("src/")
            || path.starts_with("tests/")
        {
            widen_full = true;
        } else if path.starts_with("applications/") || path.starts_with("packages/") {
            selected.extend(product.iter().cloned());
            selected.insert("service_acceptance".to_owned());
        } else if path.starts_with("tools/lkjscript-dev/src/check/")
            || path == "tools/lkjscript-dev/Cargo.toml"
        {
            selected.insert("checker_self_test".to_owned());
        } else if path.starts_with("tools/lkjscript-dev/src/service") {
            selected.extend(service.iter().cloned());
        } else if path.starts_with("tools/lkjscript-dev/src/scale")
            || path.starts_with("tools/lkjscript-dev/src/process")
            || path.starts_with("tools/lkjscript-dev/src/evidence")
            || path.starts_with("tools/lkjscript-dev/src/lib")
        {
            widen_full = true;
        } else if path.starts_with("docs/")
            || path.starts_with("prompts/")
            || matches!(path.as_str(), "README.md" | "AGENTS.md")
        {
        } else {
            widen_full = true;
        }
    }
    if widen_full {
        return Ok(full);
    }
    Ok(full
        .into_iter()
        .filter(|name| selected.contains(name))
        .collect())
}

fn environment_identity() -> Result<(VerificationDigest, Vec<String>), DevError> {
    let mut values = BTreeMap::new();
    for (name, value) in process::environment() {
        values.insert(name, VerificationDigest::of(value.as_bytes()));
    }
    let names = values.keys().cloned().collect();
    let bytes = serde_json::to_vec(&values).map_err(|error| {
        DevError::infrastructure(format!("encode redacted environment identity: {error}"))
    })?;
    Ok((VerificationDigest::of(&bytes), names))
}

fn listed_paths(repository: &Path, arguments: &[&str]) -> Result<Vec<String>, DevError> {
    let mut command = vec!["git"];
    command.extend(arguments);
    let output = checked_bytes(repository, &command)?;
    output
        .split(|value| *value == 0)
        .filter(|item| !item.is_empty())
        .map(decode_path)
        .collect()
}

fn relevant_untracked(path: &str) -> bool {
    !Path::new(path).components().any(|component| {
        matches!(component, Component::Normal(value) if value == OsStr::new("target") || value == OsStr::new(".artifacts"))
    })
}

fn checked_text(repository: &Path, command: &[&str]) -> Result<String, DevError> {
    let bytes = checked_bytes(repository, command)?;
    String::from_utf8(bytes)
        .map(|value| value.trim().to_owned())
        .map_err(|error| {
            DevError::infrastructure(format!(
                "command '{}' returned invalid UTF-8: {error}",
                command[0]
            ))
        })
}

fn checked_bytes(repository: &Path, command: &[&str]) -> Result<Vec<u8>, DevError> {
    checked_bytes_with_limit(repository, command, MAXIMUM_COMMAND_OUTPUT_BYTES)
}

fn checked_bytes_with_limit(
    repository: &Path,
    command: &[&str],
    maximum: usize,
) -> Result<Vec<u8>, DevError> {
    let program = command
        .first()
        .ok_or_else(|| DevError::infrastructure("empty identity command"))?;
    checked_command(
        repository,
        command,
        program,
        maximum,
        Duration::from_secs(30),
    )
}

fn checked_command(
    repository: &Path,
    command: &[&str],
    program: &str,
    maximum: usize,
    timeout: Duration,
) -> Result<Vec<u8>, DevError> {
    // Outside the checkout: observing inputs must not add its own logs to that snapshot.
    let logs = tempfile::Builder::new()
        .prefix("lkjscript-identity-")
        .tempdir()?;
    let specification = process::ProcessSpec {
        command: command.iter().map(|value| (*value).to_owned()).collect(),
        cwd: repository.to_path_buf(),
        environment: process::environment(),
        timeout,
        maximum_stdout_bytes: maximum as u64,
        maximum_stderr_bytes: MAXIMUM_COMMAND_ERROR_BYTES as u64,
        stdout_path: logs.path().join("stdout"),
        stderr_path: logs.path().join("stderr"),
        unavailable_exit_code: None,
    };
    let result = process::run(&specification, repository);
    if result.status != process::ProcessStatus::Passed {
        return Err(DevError::infrastructure(format!(
            "identity command '{program}' failed: {:?}: {}",
            result.status,
            result.reason.as_deref().unwrap_or("no reason")
        )));
    }
    process::read_bounded(&specification.stdout_path, maximum as u64)
}

fn decode_path(bytes: &[u8]) -> Result<String, DevError> {
    String::from_utf8(bytes.to_vec())
        .map_err(|_| DevError::infrastructure("repository path is not portable UTF-8"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_output_is_bounded_while_the_pipe_is_drained() {
        let temporary = tempfile::tempdir().expect("temporary identity directory");
        let result =
            checked_bytes_with_limit(temporary.path(), &["/usr/bin/printf", "123456789"], 4);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod command_lifecycle_tests {
    use super::*;
    #[test]
    fn identity_commands_share_deadlines_output_errors_and_recovery() {
        let root = tempfile::tempdir().unwrap();
        for script in [
            "sleep 0.6 & printf done",
            "sleep 0.6 >/dev/null & printf error >&2",
            "exec sleep 0.6",
        ] {
            let started = std::time::Instant::now();
            let result = checked_command(
                root.path(),
                &["/bin/sh", "-c", script],
                "/bin/sh",
                1024,
                Duration::from_millis(60),
            );
            assert!(result.is_err());
            assert!(started.elapsed() < Duration::from_millis(500));
        }
        assert!(
            checked_bytes_with_limit(
                root.path(),
                &["/bin/sh", "-c", "head -c 1048577 /dev/zero >&2"],
                4
            )
            .is_err()
        );
        assert_eq!(
            checked_bytes_with_limit(root.path(), &["/usr/bin/printf", "1234"], 4).unwrap(),
            b"1234"
        );
        assert!(
            std::fs::read_dir(root.path()).unwrap().next().is_none(),
            "identity observation wrote into observed input"
        );
    }
}
