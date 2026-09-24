//! Observe a verifier command, without changing ordinary source symlink semantics.
use super::super::model::ExecutableProof;
use crate::error::DevError;
use crate::evidence::{self, FileKind, FileProof};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) fn proof(repository: &Path, command: &str) -> Result<ExecutableProof, DevError> {
    let path = env::var_os("PATH");
    proof_with_path(repository, command, path.as_deref())
}

fn proof_with_path(
    repository: &Path,
    command: &str,
    search_path: Option<&OsStr>,
) -> Result<ExecutableProof, DevError> {
    let Some(path) = resolve(repository, command, search_path)? else {
        return Ok(ExecutableProof {
            entry: FileProof {
                path: command.to_owned(),
                kind: FileKind::Missing,
                mode: None,
                bytes: None,
                digest: None,
                link_target: None,
            },
            resolved: None,
        });
    };
    let label = path.to_string_lossy().into_owned();
    let entry = evidence::proof(&path, label.clone())?;
    let resolved = if entry.kind == FileKind::Symlink {
        let target = path.canonicalize().map_err(|error| {
            DevError::infrastructure(format!(
                "resolve verification executable '{}': {error}",
                path.display()
            ))
        })?;
        let target = evidence::proof(&target, label.clone())?;
        if target.kind != FileKind::File || evidence::proof(&path, label)? != entry {
            return Err(DevError::infrastructure(
                "verification executable link changed or has no regular target",
            ));
        }
        Some(target)
    } else {
        None
    };
    Ok(ExecutableProof { entry, resolved })
}

fn resolve(
    repository: &Path,
    command: &str,
    search_path: Option<&OsStr>,
) -> Result<Option<PathBuf>, DevError> {
    if command.contains(std::path::MAIN_SEPARATOR) {
        return Ok(Some(repository.join(command)));
    }
    // Do not guess a platform's implicit exec search path and then certify it.
    let search_path = search_path.ok_or_else(|| {
        DevError::infrastructure("verification command lookup requires an explicit PATH")
    })?;
    for directory in env::split_paths(search_path) {
        // Relative and empty entries belong to the child's cwd, not this observer's cwd.
        let path = repository.join(directory).join(command);
        if executable_file(&path)? {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

#[cfg(target_os = "linux")]
fn executable_file(path: &Path) -> Result<bool, DevError> {
    use rustix::fs::{Access, AtFlags, CWD, accessat};
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound
                    | std::io::ErrorKind::NotADirectory
                    | std::io::ErrorKind::PermissionDenied
            ) =>
        {
            return Ok(false);
        }
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() {
        return Ok(false);
    }
    // Match execution access for the effective identity, not just any set mode bit.
    match accessat(CWD, path, Access::EXEC_OK, AtFlags::EACCESS) {
        Ok(()) => Ok(true),
        Err(rustix::io::Errno::ACCESS | rustix::io::Errno::NOENT | rustix::io::Errno::NOTDIR) => {
            Ok(false)
        }
        Err(error) => Err(DevError::infrastructure(format!(
            "inspect verification executable access '{}': {error}",
            path.display()
        ))),
    }
}

#[cfg(not(target_os = "linux"))]
fn executable_file(_path: &Path) -> Result<bool, DevError> {
    Err(DevError::infrastructure(
        "verification executable PATH admission requires Linux",
    ))
}

#[cfg(all(test, target_os = "linux"))]
#[path = "executable_tests.rs"]
mod tests;
