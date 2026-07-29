use std::cell::Cell;
use std::fs;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use super::error::{HostResult, LinuxHostError};

const MAX_FILE_BYTES: u64 = 1_048_576;
const MAX_TOTAL_BYTES: usize = 8 * 1_048_576;
const MAX_FILES: usize = 65_536;
const MAX_DIRECTORY_ENTRIES: usize = 8_192;

pub(crate) struct AnchoredRoot {
    root: PathBuf,
    files: Cell<usize>,
    bytes: Cell<usize>,
    fixture: bool,
}

impl AnchoredRoot {
    pub(crate) fn new(path: &Path) -> HostResult<Self> {
        let root =
            fs::canonicalize(path).map_err(|error| LinuxHostError::io("root", path, &error))?;
        if !root.is_dir() {
            return Err(LinuxHostError::new(
                "root",
                "discovery root is not a directory",
            ));
        }
        Ok(Self {
            fixture: root != Path::new("/"),
            root,
            files: Cell::new(0),
            bytes: Cell::new(0),
        })
    }

    pub(crate) fn is_fixture(&self) -> bool {
        self.fixture
    }

    pub(crate) fn exists(&self, relative: &str) -> HostResult<bool> {
        let path = self.relative(relative)?;
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                self.secure_existing(path)?;
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(LinuxHostError::io("metadata", &path, &error)),
        }
    }

    pub(crate) fn is_readable(&self, relative: &str) -> HostResult<bool> {
        let path = self.relative(relative)?;
        let path = match fs::symlink_metadata(&path) {
            Ok(_) => self.secure_existing(path)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(LinuxHostError::io("metadata", &path, &error)),
        };
        match fs::File::open(&path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => Ok(false),
            Err(error) => Err(LinuxHostError::io("read", &path, &error)),
        }
    }

    pub(crate) fn read(&self, relative: &str) -> HostResult<String> {
        self.read_optional(relative)?
            .ok_or_else(|| LinuxHostError::new("missing", format!("required host file {relative}")))
    }

    pub(crate) fn read_optional(&self, relative: &str) -> HostResult<Option<String>> {
        let path = self.relative(relative)?;
        let path = match fs::symlink_metadata(&path) {
            Ok(_) => self.secure_existing(path)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => return Ok(None),
            Err(error) => return Err(LinuxHostError::io("metadata", &path, &error)),
        };
        let metadata =
            fs::metadata(&path).map_err(|error| LinuxHostError::io("metadata", &path, &error))?;
        if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
            return Err(LinuxHostError::new(
                "file-bound",
                path.display().to_string(),
            ));
        }
        let files = self.files.get().saturating_add(1);
        if files > MAX_FILES {
            return Err(LinuxHostError::new("file-count", "host file cap exceeded"));
        }
        let file =
            fs::File::open(&path).map_err(|error| LinuxHostError::io("read", &path, &error))?;
        let mut bytes = Vec::new();
        file.take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| LinuxHostError::io("read", &path, &error))?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err(LinuxHostError::new(
                "file-bound",
                path.display().to_string(),
            ));
        }
        let total = self.bytes.get().saturating_add(bytes.len());
        if total > MAX_TOTAL_BYTES {
            return Err(LinuxHostError::new("byte-count", "host byte cap exceeded"));
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| LinuxHostError::new("host-utf8", path.display().to_string()))?;
        self.files.set(files);
        self.bytes.set(total);
        Ok(Some(text.trim().to_owned()))
    }

    pub(crate) fn entries(&self, relative: &str) -> HostResult<Vec<String>> {
        let path = self.relative(relative)?;
        let path = self.secure_existing(path)?;
        let iterator =
            fs::read_dir(&path).map_err(|error| LinuxHostError::io("read-dir", &path, &error))?;
        let mut names = Vec::new();
        for entry in iterator {
            if names.len() == MAX_DIRECTORY_ENTRIES {
                return Err(LinuxHostError::new("dir-bound", path.display().to_string()));
            }
            let entry = entry.map_err(|error| LinuxHostError::io("read-dir", &path, &error))?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| LinuxHostError::new("host-name", "host path name is not UTF-8"))?;
            names.push(name);
        }
        names.sort();
        Ok(names)
    }

    fn relative(&self, relative: &str) -> HostResult<PathBuf> {
        let path = Path::new(relative);
        if path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(LinuxHostError::new("host-path", relative));
        }
        Ok(self.root.join(path))
    }

    fn secure_existing(&self, path: PathBuf) -> HostResult<PathBuf> {
        let canonical = fs::canonicalize(&path)
            .map_err(|error| LinuxHostError::io("canonical", &path, &error))?;
        if !canonical.starts_with(&self.root) {
            return Err(LinuxHostError::new(
                "symlink-escape",
                path.display().to_string(),
            ));
        }
        Ok(canonical)
    }
}
