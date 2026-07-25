use std::io::Read;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::bounds;

pub fn head(root: &Path) -> Result<String, String> {
    text(root, &["rev-parse", "HEAD"])
}

pub fn require_commit(root: &Path, revision: &str) -> Result<(), String> {
    revision_text(revision)?;
    let object = format!("{revision}^{{commit}}");
    let output = run(root, &["cat-file", "-e", &object])?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("repository revision is not a commit: {revision}"))
    }
}

pub fn require_ancestor(root: &Path, ancestor: &str, descendant: &str) -> Result<(), String> {
    require_commit(root, ancestor)?;
    require_commit(root, descendant)?;
    let output = run(root, &["merge-base", "--is-ancestor", ancestor, descendant])?;
    if output.status.success() {
        Ok(())
    } else if output.status.code() == Some(1) {
        Err(format!(
            "repository revision {ancestor} is not an ancestor of {descendant}"
        ))
    } else {
        Err("git merge-base failed".into())
    }
}

pub fn capsule_paths(root: &Path) -> Result<Vec<String>, String> {
    let output = run(root, &["ls-files", "-z", "*capsule.json"])?;
    if !output.status.success() {
        return Err("git ls-files for capsules failed".into());
    }
    let mut paths = Vec::new();
    for bytes in output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
    {
        let path = std::str::from_utf8(bytes).map_err(|_| "capsule path is not UTF-8")?;
        paths.push(path.to_owned());
        if paths.len() > bounds::CAPSULE_FILES {
            return Err("capsule enumeration exceeds bound".into());
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

pub fn tracked(root: &Path, path: &str) -> Result<bool, String> {
    let output = run(root, &["ls-files", "--error-unmatch", "--", path])?;
    match output.status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!("git ls-files failed for {path}")),
    }
}

pub fn revision_text(value: &str) -> Result<(), String> {
    if !matches!(value.len(), 40 | 64)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("invalid repository revision: {value}"));
    }
    Ok(())
}

fn text(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = run(root, args)?;
    if !output.status.success() {
        return Err(format!("git {} failed", args.join(" ")));
    }
    let value = std::str::from_utf8(&output.stdout).map_err(|_| "git output is not UTF-8")?;
    Ok(value.trim().to_owned())
}

fn run(root: &Path, args: &[&str]) -> Result<Output, String> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("run git {}: {error}", args.join(" ")))?;
    let stdout = child.stdout.take().ok_or("git stdout pipe is missing")?;
    let stderr = child.stderr.take().ok_or("git stderr pipe is missing")?;
    let retained = Arc::new(AtomicUsize::new(0));
    let stdout_reader = read_pipe(stdout, Arc::clone(&retained));
    let stderr_reader = read_pipe(stderr, retained);
    let status = child
        .wait()
        .map_err(|error| format!("wait for git {}: {error}", args.join(" ")))?;
    let (stdout, stdout_overflow) = stdout_reader
        .join()
        .map_err(|_| "git stdout reader panicked")??;
    let (stderr, stderr_overflow) = stderr_reader
        .join()
        .map_err(|_| "git stderr reader panicked")??;
    if stdout_overflow || stderr_overflow {
        return Err("git output exceeds bound".into());
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_pipe(
    mut pipe: impl Read + Send + 'static,
    retained: Arc<AtomicUsize>,
) -> std::thread::JoinHandle<Result<(Vec<u8>, bool), String>> {
    std::thread::spawn(move || {
        let mut output = Vec::new();
        let mut buffer = [0u8; 8192];
        let mut overflow = false;
        loop {
            let count = pipe
                .read(&mut buffer)
                .map_err(|error| format!("read git output: {error}"))?;
            if count == 0 {
                return Ok((output, overflow));
            }
            let before = retained
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                    Some(value.saturating_add(count))
                })
                .unwrap_or(usize::MAX);
            let available = bounds::GIT_OUTPUT_BYTES.saturating_sub(before);
            let keep = available.min(count);
            output.extend_from_slice(&buffer[..keep]);
            overflow |= keep != count;
        }
    })
}
