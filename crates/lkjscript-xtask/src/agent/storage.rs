use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::bounds;
use super::model::WorkState;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FailurePoint {
    None,
    AfterCreate,
    AfterWrite,
    AfterFileSync,
}

enum LiveState {
    Missing,
    Bytes(Vec<u8>),
    Oversized,
}

pub fn state_dir(root: &Path) -> PathBuf {
    root.join("target/lkjscript/agent-state")
}

pub fn state_path(root: &Path, task_id: &str) -> PathBuf {
    state_dir(root).join(format!("{task_id}.json"))
}

pub fn load(root: &Path, task_id: &str) -> Result<Option<WorkState>, String> {
    let path = state_path(root, task_id);
    let bytes = match read_live(&path)? {
        LiveState::Bytes(value) => value,
        LiveState::Missing => return Ok(None),
        LiveState::Oversized => {
            return super::quarantine::move_file(&path, task_id, "state exceeds state byte limit")
        }
    };
    match super::json::read_state_bytes(&bytes, &path) {
        Ok(state) if state.task_id != task_id => {
            super::quarantine::move_file(&path, task_id, "state task identity does not match file")
        }
        Ok(state) => match super::validate::shape(&state) {
            Ok(()) => Ok(Some(state)),
            Err(error) => super::quarantine::move_file(&path, task_id, &error),
        },
        Err(error) => super::quarantine::move_file(&path, task_id, &error),
    }
}

pub fn write_state(root: &Path, task_id: &str, bytes: &[u8]) -> Result<(), String> {
    write_state_at(root, task_id, bytes, FailurePoint::None)
}

pub fn write_state_at(
    root: &Path,
    task_id: &str,
    bytes: &[u8],
    failure: FailurePoint,
) -> Result<(), String> {
    bounds::output(bytes.len())?;
    let directory = state_dir(root);
    fs::create_dir_all(&directory).map_err(|error| format!("create state directory: {error}"))?;
    let destination = state_path(root, task_id);
    let mut temporary = None;
    let process = std::process::id();
    for attempt in 0..64u8 {
        let candidate = directory.join(format!(".{task_id}.{process}.{attempt}.tmp"));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(format!("create state temporary: {error}")),
        }
    }
    let (temporary_path, mut file) = temporary.ok_or("temporary state namespace exhausted")?;
    let result = (|| {
        fail(failure, FailurePoint::AfterCreate)?;
        file.write_all(bytes)
            .map_err(|error| format!("write state temporary: {error}"))?;
        fail(failure, FailurePoint::AfterWrite)?;
        sync(&file)?;
        fail(failure, FailurePoint::AfterFileSync)?;
        drop(file);
        fs::rename(&temporary_path, &destination)
            .map_err(|error| format!("publish state atomically: {error}"))?;
        sync_parent(&directory)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn read_live(path: &Path) -> Result<LiveState, String> {
    let metadata = match path.symlink_metadata() {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(LiveState::Missing)
        }
        Err(error) => return Err(format!("inspect {}: {error}", path.display())),
    };
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    if metadata.len() > bounds::STATE_BYTES as u64 {
        return Ok(LiveState::Oversized);
    }
    super::json::read_bounded(path, bounds::STATE_BYTES).map(LiveState::Bytes)
}

fn fail(actual: FailurePoint, expected: FailurePoint) -> Result<(), String> {
    if actual == expected {
        Err(format!("injected atomic write failure after {expected:?}"))
    } else {
        Ok(())
    }
}

fn sync(file: &File) -> Result<(), String> {
    file.sync_all()
        .map_err(|error| format!("sync state temporary: {error}"))
}

#[cfg(unix)]
fn sync_parent(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("sync state directory: {error}"))
}

#[cfg(not(unix))]
fn sync_parent(_path: &Path) -> Result<(), String> {
    Ok(())
}
