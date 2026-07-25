use std::fs::{File, OpenOptions};
use std::path::{Component, Path, PathBuf};

use crate::source::{SourceDiagnostic, SourceOrigin, SourceResult};

pub(crate) fn ensure_source_path(path: &Path) -> SourceResult<()> {
    if path.extension().and_then(|extension| extension.to_str()) == Some(crate::SOURCE_EXTENSION) {
        return Ok(());
    }
    Err(SourceDiagnostic::loading(
        host_diagnostic_origin(path),
        format!(
            "source path must end in .{}: {path:?}",
            crate::SOURCE_EXTENSION
        ),
    ))
}

pub(super) fn host_diagnostic_origin(path: &Path) -> SourceOrigin {
    path.to_str().map_or_else(
        || SourceOrigin::in_memory("source.lkjscript"),
        SourceOrigin::in_memory,
    )
}

pub(super) fn find_package_root(entry: &Path) -> PathBuf {
    let entry_parent = entry.parent().unwrap_or_else(|| Path::new("."));
    let mut current = entry_parent.to_path_buf();
    loop {
        if current.join("src").join("std").is_dir() {
            return current;
        }
        if !current.pop() {
            return entry_parent.to_path_buf();
        }
    }
}

pub(crate) fn source_origin(
    path: &Path,
    package_root: &Path,
    installed_root: Option<&Path>,
) -> SourceResult<SourceOrigin> {
    let relative = path
        .strip_prefix(package_root)
        .ok()
        .or_else(|| installed_root.and_then(|root| path.strip_prefix(root).ok()))
        .ok_or_else(|| {
            SourceDiagnostic::loading(
                SourceOrigin::in_memory("source.lkjscript"),
                format!("source path is outside canonical roots: {path:?}"),
            )
        })?;
    let mut pieces = Vec::new();
    for component in relative.components() {
        let Component::Normal(piece) = component else {
            return Err(SourceDiagnostic::loading(
                SourceOrigin::in_memory("source.lkjscript"),
                format!("host source path is not canonical: {path:?}"),
            ));
        };
        let piece = piece.to_str().ok_or_else(|| {
            SourceDiagnostic::loading(
                SourceOrigin::in_memory("source.lkjscript"),
                format!("host source path is not valid UTF-8: {path:?}"),
            )
        })?;
        pieces.push(piece);
    }
    if pieces.is_empty() {
        return Err(SourceDiagnostic::loading(
            SourceOrigin::in_memory("source.lkjscript"),
            format!("host source path has no logical source name: {path:?}"),
        ));
    }
    Ok(SourceOrigin {
        logical_path: pieces.join("/"),
        host_containment_path: Some(path.to_path_buf()),
    })
}

#[cfg(target_os = "linux")]
pub(super) fn reject_obvious_non_regular(_path: &Path, _origin: &SourceOrigin) -> SourceResult<()> {
    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub(super) fn reject_obvious_non_regular(path: &Path, origin: &SourceOrigin) -> SourceResult<()> {
    let metadata = std::fs::metadata(path).map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("inspect requested source {path:?}: {error}"),
        )
    })?;
    if metadata.is_file() {
        return Ok(());
    }
    Err(SourceDiagnostic::loading(
        origin.clone(),
        format!("source is not a regular file: {path:?}"),
    ))
}

#[cfg(target_os = "linux")]
pub(crate) fn open_source_file(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;

    const O_NONBLOCK: i32 = 0x800;
    OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK)
        .open(path)
}

#[cfg(not(target_os = "linux"))]
fn open_source_file(path: &Path) -> std::io::Result<File> {
    OpenOptions::new().read(true).open(path)
}

#[cfg(target_os = "linux")]
pub(crate) fn opened_source_path(
    file: &File,
    requested: &Path,
    package_root: &Path,
    installed_root: Option<&Path>,
    origin: &SourceOrigin,
) -> SourceResult<PathBuf> {
    use std::os::fd::AsRawFd;

    let descriptor_path = PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
    let canonical = descriptor_path.canonicalize().map_err(|error| {
        SourceDiagnostic::loading(
            origin.clone(),
            format!("resolve opened source descriptor for requested path {requested:?}: {error}"),
        )
    })?;
    let inside_package = canonical.starts_with(package_root);
    let inside_install = installed_root.is_some_and(|root| canonical.starts_with(root));
    if !inside_package && !inside_install {
        return Err(SourceDiagnostic::loading(
            origin.clone(),
            format!(
                "opened source escapes package roots: requested={requested:?}; actual={canonical:?}"
            ),
        ));
    }
    Ok(canonical)
}

#[cfg(not(target_os = "linux"))]
fn opened_source_path(
    _file: &File,
    requested: &Path,
    _package_root: &Path,
    _installed_root: Option<&Path>,
    origin: &SourceOrigin,
) -> SourceResult<PathBuf> {
    Err(SourceDiagnostic::loading(
        origin.clone(),
        format!(
            "host source loading requires descriptor-derived containment on the Current Linux acceptance target: {requested:?}"
        ),
    ))
}
