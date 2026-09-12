//! Descriptor-relative installation namespace access. No checked path is reopened for writes.
use super::{Diagnostic, corrupt, io_error, source};
use rustix::fs::{self, AtFlags, Mode, OFlags};
use std::ffi::OsStr;
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path};

pub(super) fn absolute(path: &Path) -> Result<(), Diagnostic> {
    if !path.is_absolute()
        || path.as_os_str().len() > 4096
        || path.components().count() > 64
        || path
            .components()
            .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
    {
        return Err(source(
            "runtime_prefix",
            "prefix must be an absolute path of at most 4096 bytes and 64 components without parent traversal",
        ));
    }
    Ok(())
}

pub(super) fn open_absolute(path: &Path, create: bool) -> Result<Option<File>, Diagnostic> {
    absolute(path)?;
    let mut directory = File::from(
        fs::open(
            "/",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io_error)?,
    );
    for part in path.components() {
        if let Component::Normal(name) = part {
            match directory_at(&directory, name)? {
                Some(next) => directory = next,
                None if create => {
                    mkdir(&directory, name, 0o755)?;
                    directory = required_directory(&directory, name)?;
                }
                None => return Ok(None),
            }
        }
    }
    Ok(Some(directory))
}

pub(super) fn stat(
    directory: &File,
    name: impl AsRef<OsStr>,
) -> Result<Option<fs::Stat>, Diagnostic> {
    match fs::statat(directory, name.as_ref(), AtFlags::SYMLINK_NOFOLLOW) {
        Ok(value) => Ok(Some(value)),
        Err(rustix::io::Errno::NOENT) => Ok(None),
        Err(error) => Err(io_error(error)),
    }
}

pub(super) fn directory_at(
    directory: &File,
    name: impl AsRef<OsStr>,
) -> Result<Option<File>, Diagnostic> {
    match fs::openat(
        directory,
        name.as_ref(),
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => Ok(Some(File::from(fd))),
        Err(rustix::io::Errno::NOENT) => Ok(None),
        Err(error) => Err(source(
            "runtime_path",
            format!(
                "directory '{}' is unavailable or traverses a symlink: {error}; choose a different owned prefix",
                name.as_ref().to_string_lossy()
            ),
        )),
    }
}

pub(super) fn required_directory(
    directory: &File,
    name: impl AsRef<OsStr>,
) -> Result<File, Diagnostic> {
    directory_at(directory, name)?.ok_or_else(|| corrupt("installation directory is missing"))
}

pub(super) fn owned(file: &File) -> Result<(), Diagnostic> {
    let metadata = file.metadata().map_err(io_error)?;
    if metadata.uid() != rustix::process::geteuid().as_raw() || metadata.mode() & 0o022 != 0 {
        return Err(source(
            "runtime_ownership",
            "installation paths must belong to the invoking user and may not be writable by group or others; choose a different prefix",
        ));
    }
    Ok(())
}

pub(super) fn same(left: &File, right: &File) -> Result<bool, Diagnostic> {
    let left = left.metadata().map_err(io_error)?;
    let right = right.metadata().map_err(io_error)?;
    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

pub(super) fn mkdir(
    directory: &File,
    name: impl AsRef<OsStr>,
    mode: u32,
) -> Result<(), Diagnostic> {
    match fs::mkdirat(directory, name.as_ref(), Mode::from_raw_mode(mode)) {
        Ok(()) => sync(directory),
        Err(rustix::io::Errno::EXIST) => {
            required_directory(directory, name)?;
            Ok(())
        }
        Err(error) => Err(io_error(error)),
    }
}

pub(super) fn private_directory(directory: &File, name: &str) -> Result<File, Diagnostic> {
    fs::mkdirat(directory, name, Mode::from_raw_mode(0o700)).map_err(io_error)?;
    required_directory(directory, name)
}

pub(super) fn file_at(directory: &File, name: impl AsRef<OsStr>) -> Result<File, Diagnostic> {
    let file = File::from(
        fs::openat(
            directory,
            name.as_ref(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(io_error)?,
    );
    let metadata = file.metadata().map_err(io_error)?;
    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(corrupt(
            "installation input must be a regular nonlinked file",
        ));
    }
    Ok(file)
}

pub(super) fn create_file(directory: &File, name: &str, mode: u32) -> Result<File, Diagnostic> {
    Ok(File::from(
        fs::openat(
            directory,
            name,
            OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::from_raw_mode(mode),
        )
        .map_err(io_error)?,
    ))
}

pub(super) fn write_new(
    directory: &File,
    name: &str,
    bytes: &[u8],
    mode: u32,
) -> Result<(), Diagnostic> {
    let mut file = create_file(directory, name, mode)?;
    reserve(&file, bytes.len() as u64)?;
    file.write_all(bytes).map_err(io_error)?;
    fs::fchmod(&file, Mode::from_raw_mode(mode)).map_err(io_error)?;
    sync(&file)
}

pub(super) fn reserve(file: &File, bytes: u64) -> Result<(), Diagnostic> {
    if bytes > 0 {
        fs::fallocate(file, fs::FallocateFlags::empty(), 0, bytes).map_err(io_error)?;
    }
    Ok(())
}

pub(super) fn sync(file: &File) -> Result<(), Diagnostic> {
    file.sync_all().map_err(io_error)
}

pub(super) fn entries(directory: &File, maximum: usize) -> Result<Vec<String>, Diagnostic> {
    let reader = fs::Dir::read_from(directory).map_err(io_error)?;
    let mut names = Vec::new();
    for item in reader {
        let item = item.map_err(io_error)?;
        let name = item
            .file_name()
            .to_str()
            .map_err(|_| corrupt("installation entry has a non-UTF-8 name"))?;
        if matches!(name, "." | "..") {
            continue;
        }
        if names.len() >= maximum || name.len() > 255 {
            return Err(super::capacity(
                "installation inventory exceeds its entry bound",
            ));
        }
        names.push(name.to_owned());
    }
    names.sort();
    Ok(names)
}

pub(super) fn remove_file(directory: &File, name: &str) -> Result<(), Diagnostic> {
    fs::unlinkat(directory, name, AtFlags::empty()).map_err(io_error)
}
pub(super) fn remove_directory(directory: &File, name: &str) -> Result<(), Diagnostic> {
    fs::unlinkat(directory, name, AtFlags::REMOVEDIR).map_err(io_error)
}
pub(super) fn publish(
    from: &File,
    name: &str,
    to: &File,
    destination: &str,
) -> Result<(), rustix::io::Errno> {
    fs::renameat_with(from, name, to, destination, fs::RenameFlags::NOREPLACE)
}
