use super::model::{InputEntry, InputSnapshot, InputSource, PlatformIdentity, RuntimeIdentity};
use super::registry;
use crate::error::DevError;
use crate::evidence::{self, FileKind, VerificationDigest};
use crate::process;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

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
        proof.path.clone_from(&identity);
        command_executables.insert(identity, proof);
    }
    let rustc = checked_text(repository, &["rustc", "-Vv"])?;
    let cargo = checked_text(repository, &["cargo", "-V"])?;
    let platform = PlatformIdentity {
        operating_system: env::consts::OS.to_owned(),
        architecture: env::consts::ARCH.to_owned(),
        family: env::consts::FAMILY.to_owned(),
        child_process_control: if cfg!(target_os = "linux") {
            "linux_process_group_sigkill".to_owned()
        } else {
            "unsupported".to_owned()
        },
    };
    #[derive(Serialize)]
    struct RuntimeMaterial<'a> {
        rustc: &'a str,
        cargo: &'a str,
        platform: &'a PlatformIdentity,
        environment_digest: &'a VerificationDigest,
        harness: &'a evidence::FileProof,
        command_executables: &'a BTreeMap<String, evidence::FileProof>,
    }
    let material = serde_json::to_vec(&RuntimeMaterial {
        rustc: &rustc,
        cargo: &cargo,
        platform: &platform,
        environment_digest: &environment_digest,
        harness: &harness,
        command_executables: &command_executables,
    })
    .map_err(|error| DevError::infrastructure(format!("encode runtime identity: {error}")))?;
    Ok(RuntimeIdentity {
        digest: VerificationDigest::of(&material),
        rustc,
        cargo,
        platform,
        environment_digest,
        environment_names,
        harness,
        command_executables,
    })
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
    let mut selected = BTreeSet::from(["diff_check".to_owned()]);
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
        } else if path == "tools/check"
            || path.starts_with("tools/lkjscript-dev/src/check/")
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
    let mut child = Command::new(program);
    child
        .args(&command[1..])
        .current_dir(repository)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(process::environment());
    let mut child = child.spawn().map_err(|error| {
        DevError::infrastructure(format!("run identity command '{program}': {error}"))
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| DevError::infrastructure("identity stdout pipe is unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| DevError::infrastructure("identity stderr pipe is unavailable"))?;
    let stdout_reader = thread::spawn(move || read_pipe_bounded(stdout, maximum));
    let stderr_reader =
        thread::spawn(move || read_pipe_bounded(stderr, MAXIMUM_COMMAND_ERROR_BYTES));
    let status = child.wait().map_err(|error| {
        DevError::infrastructure(format!("wait for identity command '{program}': {error}"))
    })?;
    let (stdout, stdout_exceeded) = join_pipe(stdout_reader, "stdout")?;
    let (_, stderr_exceeded) = join_pipe(stderr_reader, "stderr")?;
    if stdout_exceeded {
        return Err(DevError::infrastructure(format!(
            "identity command '{program}' exceeded {maximum} stdout bytes"
        )));
    }
    if stderr_exceeded {
        return Err(DevError::infrastructure(format!(
            "identity command '{program}' exceeded {MAXIMUM_COMMAND_ERROR_BYTES} stderr bytes"
        )));
    }
    if !status.success() {
        return Err(DevError::infrastructure(format!(
            "identity command '{program}' failed with {:?}",
            status.code()
        )));
    }
    Ok(stdout)
}

fn read_pipe_bounded(
    mut pipe: impl std::io::Read,
    maximum: usize,
) -> Result<(Vec<u8>, bool), DevError> {
    let mut retained = Vec::with_capacity(maximum.min(64 * 1024));
    let mut exceeded = false;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = pipe.read(&mut buffer).map_err(|error| {
            DevError::infrastructure(format!("read identity command pipe: {error}"))
        })?;
        if read == 0 {
            return Ok((retained, exceeded));
        }
        let remaining = maximum.saturating_sub(retained.len());
        let keep = remaining.min(read);
        retained.extend_from_slice(&buffer[..keep]);
        exceeded |= keep != read;
    }
}

fn join_pipe(
    reader: thread::JoinHandle<Result<(Vec<u8>, bool), DevError>>,
    stream: &str,
) -> Result<(Vec<u8>, bool), DevError> {
    reader
        .join()
        .map_err(|_| DevError::infrastructure(format!("identity {stream} reader panicked")))?
}

fn resolve_executable(repository: &Path, command: &str) -> Option<PathBuf> {
    if command.contains(std::path::MAIN_SEPARATOR) {
        let path = PathBuf::from(command);
        return Some(if path.is_absolute() {
            path
        } else {
            repository.join(path)
        });
    }
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|directory| directory.join(command))
        .find(|candidate| candidate.is_file())
}

pub(crate) fn executable_proof(
    repository: &Path,
    command: &str,
) -> Result<evidence::FileProof, DevError> {
    match resolve_executable(repository, command) {
        Some(path) => evidence::proof(&path, path.to_string_lossy().into_owned()),
        None => Ok(evidence::FileProof {
            path: command.to_owned(),
            kind: FileKind::Missing,
            mode: None,
            bytes: None,
            digest: None,
            link_target: None,
        }),
    }
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
