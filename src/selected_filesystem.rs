//! Linux selected-root filesystem authority for interactive applications.
//!
//! A grant pins one directory file descriptor. Every semantic path is a validated sequence of
//! UTF-8 components and every descendant is opened relative to that descriptor with `openat2`
//! confinement. Symlinks, magic links, and mount crossings are rejected. Reads bind both content
//! and host version facts; writes use explicit expected-content semantics and never retry after a
//! point where replacement may have become visible.

use crate::error::{ErrorCode, LkError, Result};
use rustix::fd::OwnedFd;
use rustix::fs::{
    AtFlags, Dir, FileType, Mode, OFlags, RenameFlags, ResolveFlags, Stat, fstat, fsync, openat,
    openat2, renameat_with, statat, unlinkat,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub const SELECTED_FILESYSTEM_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_PATH_DEPTH: usize = 32;
pub const MAXIMUM_PATH_COMPONENT_BYTES: usize = 255;
pub const MAXIMUM_PATH_BYTES: usize = 4_096;
pub const MAXIMUM_DIRECTORY_ENTRIES: usize = 4_096;
pub const MAXIMUM_DIRECTORY_PAGE_ENTRIES: usize = 256;
pub const MAXIMUM_FILE_BYTES: usize = 8 * 1024 * 1024;
pub const MAXIMUM_ACTION_ID_BYTES: usize = 64;

const FILE_DIGEST_DOMAIN: &[u8] = b"lkjscript-selected-file-v1\0";
const DIRECTORY_DIGEST_DOMAIN: &[u8] = b"lkjscript-selected-directory-v1\0";
const TEMPORARY_PREFIX: &str = ".lkjstudio-save-";
const RESOLVE_POLICY: ResolveFlags = ResolveFlags::BENEATH
    .union(ResolveFlags::NO_MAGICLINKS)
    .union(ResolveFlags::NO_SYMLINKS)
    .union(ResolveFlags::NO_XDEV);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemLimits {
    pub maximum_path_depth: usize,
    pub maximum_path_component_bytes: usize,
    pub maximum_path_bytes: usize,
    pub maximum_directory_entries: usize,
    pub maximum_directory_page_entries: usize,
    pub maximum_file_bytes: usize,
}

impl Default for FilesystemLimits {
    fn default() -> Self {
        Self {
            maximum_path_depth: MAXIMUM_PATH_DEPTH,
            maximum_path_component_bytes: MAXIMUM_PATH_COMPONENT_BYTES,
            maximum_path_bytes: MAXIMUM_PATH_BYTES,
            maximum_directory_entries: MAXIMUM_DIRECTORY_ENTRIES,
            maximum_directory_page_entries: MAXIMUM_DIRECTORY_PAGE_ENTRIES,
            maximum_file_bytes: MAXIMUM_FILE_BYTES,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemOperations {
    pub list: bool,
    pub read: bool,
    pub write: bool,
}

impl FilesystemOperations {
    pub const READ_WRITE: Self = Self {
        list: true,
        read: true,
        write: true,
    };
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct RelativePath(Vec<String>);

impl RelativePath {
    pub fn root() -> Self {
        Self(Vec::new())
    }

    pub fn new(components: Vec<String>) -> Result<Self> {
        validate_components(&components, &FilesystemLimits::default())?;
        Ok(Self(components))
    }

    pub fn components(&self) -> &[String] {
        &self.0
    }

    pub fn display(&self) -> String {
        if self.0.is_empty() {
            ".".to_owned()
        } else {
            self.0.join("/")
        }
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct FileDigest([u8; 32]);

impl FileDigest {
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(FILE_DIGEST_DOMAIN);
        hasher.update(bytes);
        Self(*hasher.finalize().as_bytes())
    }

    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for FileDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_hex(formatter, &self.0)
    }
}

impl fmt::Debug for FileDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl FromStr for FileDigest {
    type Err = DigestParseError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        parse_digest(value).map(Self)
    }
}

impl Serialize for FileDigest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for FileDigest {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl de::Visitor<'_> for Visitor {
            type Value = FileDigest;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a lowercase 64-digit file digest")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map_err(E::custom)
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DigestParseError;

impl fmt::Display for DigestParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("digest must contain exactly 64 lowercase hexadecimal digits")
    }
}

impl std::error::Error for DigestParseError {}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DirectoryEntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryEntry {
    pub name: String,
    pub kind: DirectoryEntryKind,
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryCursor {
    pub digest: String,
    pub offset: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DirectoryPage {
    pub contract_version: u16,
    pub path: RelativePath,
    pub digest: String,
    pub entries: Vec<DirectoryEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<DirectoryCursor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileVersion {
    pub digest: FileDigest,
    pub size: u64,
    pub device: u64,
    pub inode: u64,
    pub modified_seconds: i64,
    pub modified_nanoseconds: u64,
    pub changed_seconds: i64,
    pub changed_nanoseconds: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileRead {
    pub contract_version: u16,
    pub path: RelativePath,
    pub content: Vec<u8>,
    pub version: FileVersion,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum FileReadOutcome {
    Opened(FileRead),
    NotFound,
    WrongType,
    Changed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum SaveMode {
    Create,
    Replace { expected: FileDigest },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FileSaveRequest {
    pub contract_version: u16,
    pub action_id: String,
    pub path: RelativePath,
    pub content: Vec<u8>,
    pub mode: SaveMode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReconciliationToken {
    pub contract_version: u16,
    pub action_id: String,
    pub path: RelativePath,
    pub expected: Option<FileDigest>,
    pub intended: FileDigest,
    pub temporary_name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum FileSaveOutcome {
    Published {
        version: FileVersion,
        cleanup_pending: bool,
    },
    Unchanged {
        version: FileVersion,
    },
    Conflict {
        observed: Option<FileVersion>,
    },
    NotFound,
    WrongType,
    KnownFailure {
        message: String,
    },
    UnknownVisibility {
        token: ReconciliationToken,
        message: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ReconciliationOutcome {
    Present {
        version: FileVersion,
        cleanup_pending: bool,
    },
    Absent {
        observed: Option<FileVersion>,
    },
    Conflicting {
        observed: Option<FileVersion>,
    },
    Indeterminate {
        message: String,
    },
}

#[derive(Debug)]
pub struct SelectedFilesystem {
    grant_id: String,
    root_path: PathBuf,
    root: OwnedFd,
    operations: FilesystemOperations,
    limits: FilesystemLimits,
}

impl SelectedFilesystem {
    pub fn select(
        grant_id: String,
        root: &Path,
        operations: FilesystemOperations,
        limits: FilesystemLimits,
    ) -> Result<Self> {
        validate_action_id(&grant_id, "filesystem grant identity")?;
        validate_limits(&limits)?;
        let metadata = std::fs::symlink_metadata(root)
            .map_err(|error| filesystem_error(error, "cannot inspect selected filesystem root"))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LkError::new(
                ErrorCode::FilesystemDenied,
                "selected filesystem root must be a regular non-symlink directory",
            ));
        }
        let root_path = std::fs::canonicalize(root).map_err(|error| {
            filesystem_error(error, "cannot canonicalize selected filesystem root")
        })?;
        let root = openat(
            rustix::fs::CWD,
            &root_path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|error| errno_error(error, "cannot pin selected filesystem root"))?;
        if FileType::from_raw_mode(
            fstat(&root)
                .map_err(|error| errno_error(error, "cannot inspect pinned filesystem root"))?
                .st_mode,
        ) != FileType::Directory
        {
            return Err(LkError::new(
                ErrorCode::FilesystemDenied,
                "selected filesystem root changed type while it was selected",
            ));
        }
        Ok(Self {
            grant_id,
            root_path,
            root,
            operations,
            limits,
        })
    }

    pub fn grant_id(&self) -> &str {
        &self.grant_id
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn list(
        &self,
        path: &RelativePath,
        limit: usize,
        cursor: Option<&DirectoryCursor>,
    ) -> Result<DirectoryPage> {
        self.require(self.operations.list, "directory listing")?;
        self.validate_path(path)?;
        if limit == 0 || limit > self.limits.maximum_directory_page_entries {
            return Err(LkError::new(
                ErrorCode::FilesystemInputTooLarge,
                "directory page limit is outside the selected grant policy",
            ));
        }
        let directory = self.open_directory(path.components())?;
        let mut stream = Dir::new(directory)
            .map_err(|error| errno_error(error, "cannot create directory stream"))?;
        let directory_fd = rustix::io::dup(
            stream
                .fd()
                .map_err(|error| errno_error(error, "cannot inspect directory stream"))?,
        )
        .map_err(|error| errno_error(error, "cannot duplicate directory stream descriptor"))?;
        let mut entries = Vec::new();
        while let Some(entry) = stream.read() {
            let entry = entry.map_err(|error| errno_error(error, "directory listing failed"))?;
            let bytes = entry.file_name().to_bytes();
            if bytes == b"." || bytes == b".." {
                continue;
            }
            if entries.len() >= self.limits.maximum_directory_entries {
                return Err(LkError::new(
                    ErrorCode::FilesystemInputTooLarge,
                    "directory contains more entries than the selected grant permits",
                ));
            }
            let name = std::str::from_utf8(bytes).map_err(|_| {
                LkError::new(
                    ErrorCode::FilesystemDenied,
                    "directory contains a non-UTF-8 name that cannot be a semantic path",
                )
            })?;
            validate_component(name, &self.limits)?;
            let stat = statat(&directory_fd, name, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|error| errno_error(error, "directory entry changed during listing"))?;
            entries.push(DirectoryEntry {
                name: name.to_owned(),
                kind: directory_kind(FileType::from_raw_mode(stat.st_mode)),
                size: u64::try_from(stat.st_size).map_err(|_| {
                    LkError::new(
                        ErrorCode::FilesystemConflict,
                        "directory entry reported a negative size",
                    )
                })?,
            });
        }
        entries.sort_by(|left, right| left.name.as_bytes().cmp(right.name.as_bytes()));
        let digest = directory_digest(&entries)?;
        let offset = cursor.map_or(0, |cursor| cursor.offset);
        if let Some(cursor) = cursor
            && cursor.digest != digest
        {
            return Err(LkError::new(
                ErrorCode::FilesystemConflict,
                "directory changed before the continuation was consumed",
            ));
        }
        if offset > entries.len() {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "directory continuation offset lies beyond the observed entry set",
            ));
        }
        let end = offset.saturating_add(limit).min(entries.len());
        let page = entries[offset..end].to_vec();
        let continuation = (end < entries.len()).then(|| DirectoryCursor {
            digest: digest.clone(),
            offset: end,
        });
        Ok(DirectoryPage {
            contract_version: SELECTED_FILESYSTEM_CONTRACT_VERSION,
            path: path.clone(),
            digest,
            entries: page,
            continuation,
        })
    }

    pub fn read(&self, path: &RelativePath) -> Result<FileReadOutcome> {
        self.require(self.operations.read, "file read")?;
        self.validate_path(path)?;
        let (parent, name) = self.open_parent(path)?;
        read_at(&parent, name, path, self.limits.maximum_file_bytes)
    }

    pub fn save(&self, request: &FileSaveRequest) -> Result<FileSaveOutcome> {
        self.save_inner(request, None)
    }

    pub fn reconcile(&self, token: &ReconciliationToken) -> Result<ReconciliationOutcome> {
        self.require(self.operations.write, "file reconciliation")?;
        if token.contract_version != SELECTED_FILESYSTEM_CONTRACT_VERSION {
            return Err(LkError::new(
                ErrorCode::ProtocolVersion,
                "filesystem reconciliation token version is unsupported",
            ));
        }
        validate_action_id(&token.action_id, "filesystem action identity")?;
        self.validate_path(&token.path)?;
        let (parent, name) = self.open_parent(&token.path)?;
        let observed = match read_at(&parent, name, &token.path, self.limits.maximum_file_bytes) {
            Ok(FileReadOutcome::Opened(file)) => Some(file.version),
            Ok(FileReadOutcome::NotFound) => None,
            Ok(FileReadOutcome::WrongType | FileReadOutcome::Changed) => {
                return Ok(ReconciliationOutcome::Indeterminate {
                    message: "filesystem target cannot be observed as one stable regular file"
                        .to_owned(),
                });
            }
            Err(error) => {
                return Ok(ReconciliationOutcome::Indeterminate {
                    message: error.to_string(),
                });
            }
        };
        if observed
            .as_ref()
            .is_some_and(|value| value.digest == token.intended)
        {
            let cleanup_pending = self.cleanup_temporary(token).is_err();
            return Ok(ReconciliationOutcome::Present {
                version: observed.ok_or_else(|| {
                    LkError::new(
                        ErrorCode::FilesystemReconciliationIndeterminate,
                        "present reconciliation lost its observed version",
                    )
                })?,
                cleanup_pending,
            });
        }
        match (token.expected, observed) {
            (None, None) => Ok(ReconciliationOutcome::Absent { observed: None }),
            (Some(expected), Some(observed)) if observed.digest == expected => {
                Ok(ReconciliationOutcome::Absent {
                    observed: Some(observed),
                })
            }
            (_, Some(observed)) => Ok(ReconciliationOutcome::Conflicting {
                observed: Some(observed),
            }),
            (Some(_), None) => Ok(ReconciliationOutcome::Conflicting { observed: None }),
        }
    }

    fn save_inner(
        &self,
        request: &FileSaveRequest,
        fault: Option<SaveFault>,
    ) -> Result<FileSaveOutcome> {
        self.require(self.operations.write, "file save")?;
        if request.contract_version != SELECTED_FILESYSTEM_CONTRACT_VERSION {
            return Err(LkError::new(
                ErrorCode::ProtocolVersion,
                "filesystem save request version is unsupported",
            ));
        }
        validate_action_id(&request.action_id, "filesystem action identity")?;
        self.validate_path(&request.path)?;
        if request.content.len() > self.limits.maximum_file_bytes {
            return Err(LkError::new(
                ErrorCode::FilesystemInputTooLarge,
                "file save content exceeds the selected grant policy",
            ));
        }
        let intended = FileDigest::of(&request.content);
        let (parent, name) = self.open_parent(&request.path)?;
        let observed = read_at(&parent, name, &request.path, self.limits.maximum_file_bytes)?;
        let expected = match (&request.mode, observed) {
            (SaveMode::Create, FileReadOutcome::NotFound) => None,
            (SaveMode::Create, FileReadOutcome::Opened(file)) => {
                return Ok(FileSaveOutcome::Conflict {
                    observed: Some(file.version),
                });
            }
            (SaveMode::Create, FileReadOutcome::WrongType) => {
                return Ok(FileSaveOutcome::WrongType);
            }
            (SaveMode::Create, FileReadOutcome::Changed) => {
                return Ok(FileSaveOutcome::Conflict { observed: None });
            }
            (SaveMode::Replace { expected: _ }, FileReadOutcome::NotFound) => {
                return Ok(FileSaveOutcome::NotFound);
            }
            (SaveMode::Replace { .. }, FileReadOutcome::WrongType) => {
                return Ok(FileSaveOutcome::WrongType);
            }
            (SaveMode::Replace { .. }, FileReadOutcome::Changed) => {
                return Ok(FileSaveOutcome::Conflict { observed: None });
            }
            (SaveMode::Replace { expected }, FileReadOutcome::Opened(file))
                if file.version.digest != *expected =>
            {
                return Ok(FileSaveOutcome::Conflict {
                    observed: Some(file.version),
                });
            }
            (SaveMode::Replace { .. }, FileReadOutcome::Opened(file))
                if file.version.digest == intended =>
            {
                return Ok(FileSaveOutcome::Unchanged {
                    version: file.version,
                });
            }
            (SaveMode::Replace { expected }, FileReadOutcome::Opened(_)) => Some(*expected),
        };

        let temporary_name = temporary_name(&request.action_id);
        let token = ReconciliationToken {
            contract_version: SELECTED_FILESYSTEM_CONTRACT_VERSION,
            action_id: request.action_id.clone(),
            path: request.path.clone(),
            expected,
            intended,
            temporary_name: temporary_name.clone(),
        };
        let temporary = match openat(
            &parent,
            temporary_name.as_str(),
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::RUSR | Mode::WUSR,
        ) {
            Ok(file) => file,
            Err(rustix::io::Errno::EXIST) => {
                return Ok(FileSaveOutcome::KnownFailure {
                    message: "filesystem action identity already has a temporary publication"
                        .to_owned(),
                });
            }
            Err(error) => {
                return Ok(FileSaveOutcome::KnownFailure {
                    message: errno_error(error, "cannot create save temporary").to_string(),
                });
            }
        };
        let mut temporary_file = File::from(temporary);
        if let Err(error) = temporary_file
            .write_all(&request.content)
            .and_then(|()| temporary_file.sync_all())
        {
            let _ = unlinkat(&parent, temporary_name.as_str(), AtFlags::empty());
            return Ok(FileSaveOutcome::KnownFailure {
                message: format!("file write failed before visibility: {error}"),
            });
        }
        drop(temporary_file);
        if fault == Some(SaveFault::BeforePublication) {
            let _ = unlinkat(&parent, temporary_name.as_str(), AtFlags::empty());
            return Ok(FileSaveOutcome::KnownFailure {
                message: "injected failure before filesystem publication".to_owned(),
            });
        }

        let flags = if expected.is_some() {
            RenameFlags::EXCHANGE
        } else {
            RenameFlags::NOREPLACE
        };
        if let Err(error) = renameat_with(&parent, temporary_name.as_str(), &parent, name, flags) {
            let _ = unlinkat(&parent, temporary_name.as_str(), AtFlags::empty());
            if error == rustix::io::Errno::EXIST {
                return Ok(FileSaveOutcome::Conflict { observed: None });
            }
            return Ok(FileSaveOutcome::KnownFailure {
                message: errno_error(error, "atomic file publication failed").to_string(),
            });
        }
        if fault == Some(SaveFault::AfterPublication) {
            return Ok(FileSaveOutcome::UnknownVisibility {
                token,
                message: "injected failure after filesystem publication may be visible".to_owned(),
            });
        }

        if let Some(expected) = expected {
            let temporary_path = RelativePath::new(vec![temporary_name.clone()])?;
            match read_at(
                &parent,
                temporary_name.as_str(),
                &temporary_path,
                self.limits.maximum_file_bytes,
            )? {
                FileReadOutcome::Opened(old) if old.version.digest == expected => {}
                _ => {
                    return Ok(FileSaveOutcome::UnknownVisibility {
                        token,
                        message: "the replaced base changed at the atomic publication boundary"
                            .to_owned(),
                    });
                }
            }
        }
        if fault == Some(SaveFault::BeforeDirectorySync) || fsync(&parent).is_err() {
            return Ok(FileSaveOutcome::UnknownVisibility {
                token,
                message:
                    "filesystem publication may be visible but directory durability is unknown"
                        .to_owned(),
            });
        }

        let cleanup_pending = if expected.is_some() {
            unlinkat(&parent, temporary_name.as_str(), AtFlags::empty()).is_err()
        } else {
            false
        };
        if !cleanup_pending && expected.is_some() && fsync(&parent).is_err() {
            return Ok(FileSaveOutcome::UnknownVisibility {
                token,
                message: "filesystem publication is visible but cleanup durability is unknown"
                    .to_owned(),
            });
        }
        match read_at(&parent, name, &request.path, self.limits.maximum_file_bytes)? {
            FileReadOutcome::Opened(file) if file.version.digest == intended => {
                Ok(FileSaveOutcome::Published {
                    version: file.version,
                    cleanup_pending,
                })
            }
            _ => Ok(FileSaveOutcome::UnknownVisibility {
                token,
                message: "filesystem target changed before publication confirmation".to_owned(),
            }),
        }
    }

    fn cleanup_temporary(&self, token: &ReconciliationToken) -> Result<()> {
        let (parent, _) = self.open_parent(&token.path)?;
        match unlinkat(&parent, token.temporary_name.as_str(), AtFlags::empty()) {
            Ok(()) | Err(rustix::io::Errno::NOENT) => {}
            Err(error) => {
                return Err(errno_error(
                    error,
                    "cannot clean reconciled filesystem temporary",
                ));
            }
        }
        fsync(&parent)
            .map_err(|error| errno_error(error, "cannot synchronize reconciled temporary cleanup"))
    }

    fn validate_path(&self, path: &RelativePath) -> Result<()> {
        validate_components(path.components(), &self.limits)
    }

    fn open_directory(&self, components: &[String]) -> Result<OwnedFd> {
        let mut current = openat(
            &self.root,
            ".",
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|error| errno_error(error, "cannot reopen selected root descriptor"))?;
        for component in components {
            current = openat2(
                &current,
                component.as_str(),
                OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
                Mode::empty(),
                RESOLVE_POLICY,
            )
            .map_err(|error| errno_error(error, "cannot resolve selected directory component"))?;
        }
        Ok(current)
    }

    fn open_parent<'a>(&self, path: &'a RelativePath) -> Result<(OwnedFd, &'a str)> {
        let (name, parents) = path.components().split_last().ok_or_else(|| {
            LkError::new(
                ErrorCode::FilesystemWrongType,
                "selected filesystem root is a directory, not a regular file",
            )
        })?;
        Ok((self.open_directory(parents)?, name.as_str()))
    }

    fn require(&self, allowed: bool, operation: &str) -> Result<()> {
        if allowed {
            Ok(())
        } else {
            Err(LkError::new(
                ErrorCode::FilesystemDenied,
                format!("selected filesystem grant does not allow {operation}"),
            ))
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SaveFault {
    BeforePublication,
    AfterPublication,
    BeforeDirectorySync,
}

fn read_at(
    parent: &OwnedFd,
    name: &str,
    path: &RelativePath,
    maximum_file_bytes: usize,
) -> Result<FileReadOutcome> {
    let descriptor = match openat2(
        parent,
        name,
        OFlags::RDONLY | OFlags::CLOEXEC,
        Mode::empty(),
        RESOLVE_POLICY,
    ) {
        Ok(descriptor) => descriptor,
        Err(rustix::io::Errno::NOENT) => return Ok(FileReadOutcome::NotFound),
        Err(rustix::io::Errno::ISDIR | rustix::io::Errno::NOTDIR) => {
            return Ok(FileReadOutcome::WrongType);
        }
        Err(error) => return Err(errno_error(error, "cannot open selected file")),
    };
    let before =
        fstat(&descriptor).map_err(|error| errno_error(error, "cannot inspect selected file"))?;
    if FileType::from_raw_mode(before.st_mode) != FileType::RegularFile {
        return Ok(FileReadOutcome::WrongType);
    }
    let maximum = i64::try_from(maximum_file_bytes).map_err(|_| {
        LkError::new(
            ErrorCode::FilesystemInputTooLarge,
            "selected file byte policy cannot be represented",
        )
    })?;
    if before.st_size < 0 || before.st_size > maximum {
        return Err(LkError::new(
            ErrorCode::FilesystemInputTooLarge,
            "selected file exceeds the grant byte policy",
        ));
    }
    let mut file = File::from(descriptor);
    let mut content = Vec::with_capacity(usize::try_from(before.st_size).map_err(|_| {
        LkError::new(
            ErrorCode::FilesystemInputTooLarge,
            "selected file size cannot be represented",
        )
    })?);
    Read::by_ref(&mut file)
        .take(
            u64::try_from(maximum_file_bytes)
                .unwrap_or(u64::MAX)
                .saturating_add(1),
        )
        .read_to_end(&mut content)
        .map_err(|error| filesystem_error(error, "cannot read selected file"))?;
    if content.len() > maximum_file_bytes {
        return Err(LkError::new(
            ErrorCode::FilesystemInputTooLarge,
            "selected file grew beyond the grant byte policy",
        ));
    }
    let after =
        fstat(&file).map_err(|error| errno_error(error, "cannot revalidate selected file"))?;
    if !same_stat(&before, &after) || usize::try_from(after.st_size).ok() != Some(content.len()) {
        return Ok(FileReadOutcome::Changed);
    }
    Ok(FileReadOutcome::Opened(FileRead {
        contract_version: SELECTED_FILESYSTEM_CONTRACT_VERSION,
        path: path.clone(),
        version: file_version(&after, &content),
        content,
    }))
}

fn validate_limits(limits: &FilesystemLimits) -> Result<()> {
    if limits.maximum_path_depth == 0
        || limits.maximum_path_depth > MAXIMUM_PATH_DEPTH
        || limits.maximum_path_component_bytes == 0
        || limits.maximum_path_component_bytes > MAXIMUM_PATH_COMPONENT_BYTES
        || limits.maximum_path_bytes == 0
        || limits.maximum_path_bytes > MAXIMUM_PATH_BYTES
        || limits.maximum_directory_entries == 0
        || limits.maximum_directory_entries > MAXIMUM_DIRECTORY_ENTRIES
        || limits.maximum_directory_page_entries == 0
        || limits.maximum_directory_page_entries > MAXIMUM_DIRECTORY_PAGE_ENTRIES
        || limits.maximum_file_bytes == 0
        || limits.maximum_file_bytes > MAXIMUM_FILE_BYTES
    {
        return Err(LkError::new(
            ErrorCode::FilesystemInputTooLarge,
            "selected filesystem limits exceed the contract maxima",
        ));
    }
    Ok(())
}

fn validate_components(components: &[String], limits: &FilesystemLimits) -> Result<()> {
    if components.len() > limits.maximum_path_depth {
        return Err(LkError::new(
            ErrorCode::FilesystemInputTooLarge,
            "semantic filesystem path exceeds the grant depth policy",
        ));
    }
    let mut bytes = 0_usize;
    for component in components {
        validate_component(component, limits)?;
        bytes = bytes
            .checked_add(component.len().saturating_add(1))
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::FilesystemInputTooLarge,
                    "semantic filesystem path byte count overflowed",
                )
            })?;
    }
    if bytes > limits.maximum_path_bytes {
        return Err(LkError::new(
            ErrorCode::FilesystemInputTooLarge,
            "semantic filesystem path exceeds the grant byte policy",
        ));
    }
    Ok(())
}

fn validate_component(component: &str, limits: &FilesystemLimits) -> Result<()> {
    if component.is_empty()
        || component == "."
        || component == ".."
        || component.len() > limits.maximum_path_component_bytes
        || component.as_bytes().contains(&0)
        || component.contains('/')
        || component.contains('\\')
    {
        return Err(LkError::new(
            ErrorCode::FilesystemDenied,
            "semantic filesystem path contains an invalid component",
        ));
    }
    Ok(())
}

fn validate_action_id(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAXIMUM_ACTION_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{label} must be 1..={MAXIMUM_ACTION_ID_BYTES} ASCII identifier bytes"),
        ));
    }
    Ok(())
}

fn directory_kind(kind: FileType) -> DirectoryEntryKind {
    match kind {
        FileType::RegularFile => DirectoryEntryKind::File,
        FileType::Directory => DirectoryEntryKind::Directory,
        FileType::Symlink => DirectoryEntryKind::Symlink,
        _ => DirectoryEntryKind::Other,
    }
}

fn file_version(stat: &Stat, content: &[u8]) -> FileVersion {
    FileVersion {
        digest: FileDigest::of(content),
        size: u64::try_from(stat.st_size).unwrap_or(0),
        device: stat.st_dev,
        inode: stat.st_ino,
        modified_seconds: stat.st_mtime,
        modified_nanoseconds: stat.st_mtime_nsec,
        changed_seconds: stat.st_ctime,
        changed_nanoseconds: stat.st_ctime_nsec,
    }
}

fn same_stat(left: &Stat, right: &Stat) -> bool {
    left.st_dev == right.st_dev
        && left.st_ino == right.st_ino
        && left.st_mode == right.st_mode
        && left.st_size == right.st_size
        && left.st_mtime == right.st_mtime
        && left.st_mtime_nsec == right.st_mtime_nsec
        && left.st_ctime == right.st_ctime
        && left.st_ctime_nsec == right.st_ctime_nsec
}

fn directory_digest(entries: &[DirectoryEntry]) -> Result<String> {
    let encoded = serde_json::to_vec(entries).map_err(|error| {
        LkError::new(
            ErrorCode::FilesystemKnownFailure,
            format!("cannot encode directory observation: {error}"),
        )
    })?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(DIRECTORY_DIGEST_DOMAIN);
    hasher.update(&encoded);
    Ok(hasher.finalize().to_hex().to_string())
}

fn temporary_name(action_id: &str) -> String {
    let digest = blake3::hash(action_id.as_bytes()).to_hex();
    format!("{TEMPORARY_PREFIX}{}", &digest.as_str()[..24])
}

fn filesystem_error(error: std::io::Error, context: &str) -> LkError {
    let code = match error.kind() {
        std::io::ErrorKind::PermissionDenied => ErrorCode::FilesystemDenied,
        std::io::ErrorKind::NotFound => ErrorCode::FilesystemNotFound,
        _ => ErrorCode::FilesystemKnownFailure,
    };
    LkError::new(code, format!("{context}: {error}"))
}

fn errno_error(error: rustix::io::Errno, context: &str) -> LkError {
    let code = match error {
        rustix::io::Errno::ACCESS | rustix::io::Errno::PERM => ErrorCode::FilesystemDenied,
        rustix::io::Errno::NOENT => ErrorCode::FilesystemNotFound,
        rustix::io::Errno::NOTDIR
        | rustix::io::Errno::ISDIR
        | rustix::io::Errno::LOOP
        | rustix::io::Errno::XDEV => ErrorCode::FilesystemWrongType,
        _ => ErrorCode::FilesystemKnownFailure,
    };
    LkError::new(code, format!("{context}: {error}"))
}

fn write_hex(formatter: &mut fmt::Formatter<'_>, bytes: &[u8]) -> fmt::Result {
    for byte in bytes {
        write!(formatter, "{byte:02x}")?;
    }
    Ok(())
}

fn parse_digest(value: &str) -> std::result::Result<[u8; 32], DigestParseError> {
    if value.len() != 64
        || value
            .bytes()
            .any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
    {
        return Err(DigestParseError);
    }
    let mut output = [0_u8; 32];
    for (offset, slot) in output.iter_mut().enumerate() {
        let start = offset * 2;
        *slot = u8::from_str_radix(&value[start..start + 2], 16).map_err(|_| DigestParseError)?;
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn selected(root: &Path) -> SelectedFilesystem {
        SelectedFilesystem::select(
            "test-grant".to_owned(),
            root,
            FilesystemOperations::READ_WRITE,
            FilesystemLimits::default(),
        )
        .expect("select filesystem")
    }

    #[test]
    fn list_read_create_replace_conflict_and_reconcile_are_exact() {
        let root = tempfile::tempdir().expect("root");
        fs::write(root.path().join("b.txt"), b"base").expect("base");
        fs::create_dir(root.path().join("a")).expect("directory");
        let filesystem = selected(root.path());

        let first = filesystem
            .list(&RelativePath::root(), 1, None)
            .expect("first page");
        assert_eq!(first.entries[0].name, "a");
        let second = filesystem
            .list(&RelativePath::root(), 1, first.continuation.as_ref())
            .expect("second page");
        assert_eq!(second.entries[0].name, "b.txt");

        let base = match filesystem
            .read(&RelativePath::new(vec!["b.txt".to_owned()]).expect("path"))
            .expect("read")
        {
            FileReadOutcome::Opened(file) => file,
            other => panic!("unexpected read outcome: {other:?}"),
        };
        assert_eq!(base.content, b"base");

        let replace = FileSaveRequest {
            contract_version: SELECTED_FILESYSTEM_CONTRACT_VERSION,
            action_id: "replace-1".to_owned(),
            path: base.path.clone(),
            content: b"next".to_vec(),
            mode: SaveMode::Replace {
                expected: base.version.digest,
            },
        };
        assert!(matches!(
            filesystem.save(&replace).expect("replace"),
            FileSaveOutcome::Published { .. }
        ));
        assert_eq!(
            fs::read(root.path().join("b.txt")).expect("direct read"),
            b"next"
        );
        assert!(matches!(
            filesystem.save(&replace).expect("stale replace"),
            FileSaveOutcome::Conflict { .. }
        ));

        let create = FileSaveRequest {
            contract_version: SELECTED_FILESYSTEM_CONTRACT_VERSION,
            action_id: "create-1".to_owned(),
            path: RelativePath::new(vec!["new.txt".to_owned()]).expect("new path"),
            content: b"created".to_vec(),
            mode: SaveMode::Create,
        };
        assert!(matches!(
            filesystem.save(&create).expect("create"),
            FileSaveOutcome::Published { .. }
        ));

        let unknown = FileSaveRequest {
            action_id: "unknown-1".to_owned(),
            content: b"unknown-new".to_vec(),
            mode: SaveMode::Replace {
                expected: FileDigest::of(b"next"),
            },
            ..replace
        };
        let token = match filesystem
            .save_inner(&unknown, Some(SaveFault::AfterPublication))
            .expect("faulted save")
        {
            FileSaveOutcome::UnknownVisibility { token, .. } => token,
            other => panic!("unexpected save outcome: {other:?}"),
        };
        assert!(matches!(
            filesystem.reconcile(&token).expect("reconcile"),
            ReconciliationOutcome::Present { .. }
        ));
        assert_eq!(
            fs::read(root.path().join("b.txt")).expect("direct reconciled read"),
            b"unknown-new"
        );
    }

    #[test]
    fn semantic_paths_and_symlink_escapes_reject() {
        let root = tempfile::tempdir().expect("root");
        let outside = tempfile::tempdir().expect("outside");
        fs::write(outside.path().join("secret"), b"secret").expect("secret");
        std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).expect("symlink");
        let filesystem = selected(root.path());
        let escape = RelativePath::new(vec!["escape".to_owned(), "secret".to_owned()])
            .expect("escape path syntax");
        let error = filesystem.read(&escape).expect_err("symlink must reject");
        assert!(matches!(
            error.code,
            ErrorCode::FilesystemWrongType | ErrorCode::FilesystemDenied
        ));
        assert!(RelativePath::new(vec!["..".to_owned()]).is_err());
        assert!(RelativePath::new(vec!["a/b".to_owned()]).is_err());
    }

    #[test]
    fn exact_file_and_paste_style_bounds_reject_before_work() {
        let root = tempfile::tempdir().expect("root");
        let limits = FilesystemLimits {
            maximum_file_bytes: 4,
            ..FilesystemLimits::default()
        };
        fs::write(root.path().join("large"), b"12345").expect("large");
        let filesystem = SelectedFilesystem::select(
            "bounded".to_owned(),
            root.path(),
            FilesystemOperations::READ_WRITE,
            limits,
        )
        .expect("bounded filesystem");
        let path = RelativePath::new(vec!["large".to_owned()]).expect("path");
        assert_eq!(
            filesystem.read(&path).expect_err("one-over read").code,
            ErrorCode::FilesystemInputTooLarge
        );
        let request = FileSaveRequest {
            contract_version: SELECTED_FILESYSTEM_CONTRACT_VERSION,
            action_id: "large-save".to_owned(),
            path,
            content: b"12345".to_vec(),
            mode: SaveMode::Create,
        };
        assert_eq!(
            filesystem.save(&request).expect_err("one-over save").code,
            ErrorCode::FilesystemInputTooLarge
        );
    }
}
