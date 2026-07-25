use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode};

pub(crate) struct PublicationGuard {
    path: PathBuf,
    _file: File,
}

impl Drop for PublicationGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

pub(crate) fn acquire(source: &Path) -> Result<Option<PublicationGuard>, ProtocolError> {
    let Some(workspace) = workspace_root(source) else {
        return Ok(None);
    };
    let staging = staging_root(&workspace);
    fs::create_dir_all(&staging).map_err(|failure| io_error("create staging root", failure))?;
    let path = staging.join("publication.lock");
    for attempt in 0..2 {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                writeln!(file, "{}", std::process::id())
                    .and_then(|()| file.sync_all())
                    .map_err(|failure| io_error("write publication lock", failure))?;
                if let Err(failure) = super::recovery::recover_all(&workspace, &staging) {
                    let _ = fs::remove_file(&path);
                    return Err(failure);
                }
                return Ok(Some(PublicationGuard { path, _file: file }));
            }
            Err(failure)
                if failure.kind() == std::io::ErrorKind::AlreadyExists
                    && attempt == 0
                    && remove_stale(&path)? =>
            {
                continue;
            }
            Err(failure) => return Err(io_error("concurrent semantic publication", failure)),
        }
    }
    Err(error(
        ProtocolErrorCode::PublicationFailed,
        "concurrent semantic publication",
    ))
}

pub(crate) fn require_workspace(source: &Path) -> Result<PathBuf, ProtocolError> {
    workspace_root(source).ok_or_else(|| {
        error(
            ProtocolErrorCode::PublicationFailed,
            "publication root is not contained by a repository workspace",
        )
    })
}

pub(crate) fn staging_root(workspace: &Path) -> PathBuf {
    workspace.join("target/lkjscript/semantic-staging")
}

fn workspace_root(source: &Path) -> Option<PathBuf> {
    let canonical = source.canonicalize().ok();
    let parent = source.parent()?.canonicalize().ok();
    let start = match canonical.as_deref() {
        Some(path) if path.is_dir() => path,
        Some(path) => path.parent()?,
        None => parent.as_deref()?,
    };
    start
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .map(Path::to_path_buf)
}

#[cfg(target_os = "linux")]
fn remove_stale(path: &Path) -> Result<bool, ProtocolError> {
    let bytes = fs::read(path).map_err(|failure| io_error("read publication lock", failure))?;
    if bytes.len() > 32 {
        return Err(error(
            ProtocolErrorCode::PublicationFailed,
            "publication lock exceeds byte bound",
        ));
    }
    let pid = std::str::from_utf8(&bytes)
        .ok()
        .and_then(|text| text.trim().parse::<u32>().ok());
    let Some(pid) = pid else {
        return Ok(false);
    };
    if Path::new("/proc").join(pid.to_string()).exists() {
        return Ok(false);
    }
    fs::remove_file(path).map_err(|failure| io_error("remove stale publication lock", failure))?;
    Ok(true)
}

#[cfg(not(target_os = "linux"))]
fn remove_stale(_path: &Path) -> Result<bool, ProtocolError> {
    Ok(false)
}

fn io_error(action: &str, failure: std::io::Error) -> ProtocolError {
    error(
        ProtocolErrorCode::PublicationFailed,
        format!("{action}: {failure}"),
    )
}
