use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct CheckpointLock {
    path: PathBuf,
    _file: File,
}

impl Drop for CheckpointLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub fn acquire(root: &Path, task_id: &str) -> Result<CheckpointLock, String> {
    let directory = super::storage::state_dir(root);
    fs::create_dir_all(&directory).map_err(|error| format!("create state directory: {error}"))?;
    let path = directory.join(format!("{task_id}.checkpoint.lock"));
    for attempt in 0..2 {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                writeln!(file, "{}", std::process::id())
                    .map_err(|error| format!("write checkpoint lock: {error}"))?;
                file.sync_all()
                    .map_err(|error| format!("sync checkpoint lock: {error}"))?;
                return Ok(CheckpointLock { path, _file: file });
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::AlreadyExists
                    && attempt == 0
                    && remove_stale(&path)? =>
            {
                continue;
            }
            Err(error) => return Err(format!("concurrent checkpoint for {task_id}: {error}")),
        }
    }
    Err(format!("concurrent checkpoint for {task_id}"))
}

#[cfg(target_os = "linux")]
fn remove_stale(path: &Path) -> Result<bool, String> {
    let metadata = path
        .symlink_metadata()
        .map_err(|error| format!("inspect checkpoint lock: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("checkpoint lock is not a regular file".into());
    }
    let bytes = super::json::read_bounded(path, 32)?;
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return Err("checkpoint lock is not UTF-8".into());
    };
    let Ok(pid) = text.trim().parse::<u32>() else {
        return Ok(false);
    };
    if Path::new("/proc").join(pid.to_string()).exists() {
        return Ok(false);
    }
    fs::remove_file(path).map_err(|error| format!("remove stale checkpoint lock: {error}"))?;
    Ok(true)
}

#[cfg(not(target_os = "linux"))]
fn remove_stale(_path: &Path) -> Result<bool, String> {
    Ok(false)
}
