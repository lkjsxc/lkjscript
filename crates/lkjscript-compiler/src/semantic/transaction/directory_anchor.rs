use std::fs::File;
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;

use crate::semantic::schema::ProtocolError;

pub(super) struct AnchoredDirectory {
    descriptor: PathBuf,
    _file: File,
}

impl AnchoredDirectory {
    pub fn open(expected: &Path) -> Result<Self, ProtocolError> {
        let file = File::open(expected)
            .map_err(|cause| super::journal::io_failure("open source parent", cause))?;
        let descriptor = descriptor_path(&file)?;
        let resolved = descriptor.canonicalize().map_err(|cause| {
            super::journal::io_failure("resolve source parent descriptor", cause)
        })?;
        if resolved != expected {
            return Err(super::journal::failure(
                "source ancestor changed or is a symbolic-link alias",
            ));
        }
        Ok(Self {
            descriptor,
            _file: file,
        })
    }

    pub fn join(&self, leaf: impl AsRef<Path>) -> PathBuf {
        self.descriptor.join(leaf)
    }
}

#[cfg(target_os = "linux")]
fn descriptor_path(directory: &File) -> Result<PathBuf, ProtocolError> {
    Ok(PathBuf::from(format!(
        "/proc/self/fd/{}",
        directory.as_raw_fd()
    )))
}

#[cfg(not(target_os = "linux"))]
fn descriptor_path(_directory: &File) -> Result<PathBuf, ProtocolError> {
    Err(super::journal::failure(
        "descriptor-anchored publication is unavailable on this host",
    ))
}
