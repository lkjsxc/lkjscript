//! Pinned PostgreSQL process support shared by live acceptance workflows.

use crate::error::DevError;
use crate::process;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Read;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

pub(crate) const LOCAL_POSTGRES_ROOT_ENVIRONMENT: &str = "LKJSCRIPT_POSTGRES_ROOT";
pub(crate) const LOCAL_POSTGRES_VERSION: &str = "16.15";
const LOCAL_POSTGRES_ROOT_SHA256: &str =
    "1201a77990ef1f03b6bfa8826a05e8b4664ce7c51c3c341c6a764947b5395d30";
const MAXIMUM_ROOT_ENTRIES: usize = 4096;
const MAXIMUM_ROOT_BYTES: u64 = 256 * 1024 * 1024;
const MAXIMUM_ROOT_ENTRY_BYTES: u64 = 192 * 1024 * 1024;
const MAXIMUM_ROOT_PATH_BYTES: usize = 4096;
const ROOT_INVENTORY_DOMAIN: &[u8] = b"lkjscript-local-postgres-root-1\0";
const REQUIRED_TOOLS: &[&str] = &[
    "createdb",
    "initdb",
    "pg_ctl",
    "pg_dump",
    "pg_isready",
    "pg_restore",
    "postgres",
    "psql",
];

#[derive(Clone, Debug)]
pub(crate) struct LocalPostgresTools {
    bin: PathBuf,
    library: PathBuf,
    share: PathBuf,
}

impl LocalPostgresTools {
    pub(crate) fn resolve(requested_root: &Path) -> Result<Self, DevError> {
        let root = requested_root.canonicalize().map_err(|error| {
            DevError::usage(format!(
                "canonicalize local PostgreSQL root '{}': {error}",
                requested_root.display()
            ))
        })?;
        let bin = root.join("usr/lib/postgresql/16/bin");
        let library = root.join("usr/lib/x86_64-linux-gnu");
        let share = root.join("usr/share/postgresql/16");
        for path in [&bin, &library, &share] {
            let metadata = fs::symlink_metadata(path).map_err(|error| {
                DevError::usage(format!(
                    "inspect local PostgreSQL path '{}': {error}",
                    path.display()
                ))
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(DevError::usage(format!(
                    "local PostgreSQL root omitted '{}'",
                    path.display()
                )));
            }
        }
        for tool in REQUIRED_TOOLS {
            let path = bin.join(tool);
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                DevError::usage(format!(
                    "inspect local PostgreSQL tool '{}': {error}",
                    path.display()
                ))
            })?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(DevError::usage(format!(
                    "local PostgreSQL tool '{}' is not a regular file",
                    path.display()
                )));
            }
            #[cfg(unix)]
            if metadata.permissions().mode() & 0o111 == 0 {
                return Err(DevError::usage(format!(
                    "local PostgreSQL tool '{}' is not executable",
                    path.display()
                )));
            }
        }
        let digest = root_inventory_sha256(&root)?;
        if digest != LOCAL_POSTGRES_ROOT_SHA256 {
            return Err(DevError::usage(format!(
                "local PostgreSQL root digest is {digest}, expected pinned {LOCAL_POSTGRES_ROOT_SHA256}"
            )));
        }
        Ok(Self {
            bin,
            library,
            share,
        })
    }

    pub(crate) fn environment(&self) -> BTreeMap<String, String> {
        let mut environment = process::environment();
        environment.insert("LANG".to_owned(), "C".to_owned());
        environment.insert(
            "LD_LIBRARY_PATH".to_owned(),
            self.library.display().to_string(),
        );
        environment
    }

    pub(crate) fn command(&self, tool: &str, arguments: &[String]) -> Vec<String> {
        let mut command = vec![self.bin.join(tool).display().to_string()];
        command.extend(arguments.iter().cloned());
        command
    }

    pub(crate) fn version_command(&self) -> Vec<String> {
        self.command("postgres", &["--version".to_owned()])
    }

    pub(crate) fn initdb_command(&self, data: &Path) -> Vec<String> {
        self.command(
            "initdb",
            &[
                "-D".to_owned(),
                data.display().to_string(),
                "-L".to_owned(),
                self.share.display().to_string(),
                "--no-locale".to_owned(),
                "--encoding=UTF8".to_owned(),
                "--username=postgres".to_owned(),
                "--auth-local=trust".to_owned(),
                "--auth-host=trust".to_owned(),
            ],
        )
    }

    pub(crate) fn start_command(
        &self,
        data: &Path,
        log: &Path,
        socket: &Path,
        port: u16,
        maximum_connections: u16,
    ) -> Vec<String> {
        self.command(
            "pg_ctl",
            &[
                "-D".to_owned(),
                data.display().to_string(),
                "-l".to_owned(),
                log.display().to_string(),
                "-o".to_owned(),
                format!(
                    "-h 127.0.0.1 -p {port} -k {} -c jit=off -c max_connections={maximum_connections}",
                    socket.display()
                ),
                "-w".to_owned(),
                "start".to_owned(),
            ],
        )
    }

    pub(crate) fn stop_command(&self, data: &Path) -> Vec<String> {
        self.command(
            "pg_ctl",
            &[
                "-D".to_owned(),
                data.display().to_string(),
                "-m".to_owned(),
                "fast".to_owned(),
                "-w".to_owned(),
                "stop".to_owned(),
            ],
        )
    }

    pub(crate) fn client_command(
        &self,
        tool: &str,
        port: u16,
        arguments: &[String],
    ) -> Vec<String> {
        let mut values = vec![
            "-h".to_owned(),
            "127.0.0.1".to_owned(),
            "-p".to_owned(),
            port.to_string(),
        ];
        values.extend(arguments.iter().cloned());
        self.command(tool, &values)
    }

    pub(crate) fn validate_version(&self, bytes: &[u8]) -> Result<String, DevError> {
        let version = std::str::from_utf8(bytes)
            .map_err(|_| DevError::corrupt("local PostgreSQL version was not UTF-8"))?
            .trim()
            .to_owned();
        let expected = format!("postgres (PostgreSQL) {LOCAL_POSTGRES_VERSION}");
        if !version.starts_with(&expected) {
            return Err(DevError::usage(format!(
                "local PostgreSQL is '{version}', expected pinned {LOCAL_POSTGRES_VERSION}"
            )));
        }
        Ok(version)
    }
}

pub(crate) fn configured_root() -> Option<PathBuf> {
    std::env::var_os(LOCAL_POSTGRES_ROOT_ENVIRONMENT).map(PathBuf::from)
}

fn root_inventory_sha256(root: &Path) -> Result<String, DevError> {
    let mut pending = vec![root.to_path_buf()];
    let mut entries = Vec::new();
    while let Some(directory) = pending.pop() {
        let reader = fs::read_dir(&directory).map_err(|error| {
            DevError::usage(format!(
                "read local PostgreSQL directory '{}': {error}",
                directory.display()
            ))
        })?;
        for item in reader {
            let path = item
                .map_err(|error| DevError::usage(format!("read PostgreSQL entry: {error}")))?
                .path();
            if entries.len() >= MAXIMUM_ROOT_ENTRIES {
                return Err(DevError::usage(format!(
                    "local PostgreSQL root exceeds {MAXIMUM_ROOT_ENTRIES} entries"
                )));
            }
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                DevError::usage(format!(
                    "inspect local PostgreSQL entry '{}': {error}",
                    path.display()
                ))
            })?;
            if metadata.is_dir() {
                pending.push(path.clone());
            } else if !metadata.is_file() && !metadata.file_type().is_symlink() {
                return Err(DevError::usage(format!(
                    "local PostgreSQL entry '{}' has an unsupported file type",
                    path.display()
                )));
            }
            let relative = path.strip_prefix(root).map_err(|_| {
                DevError::usage("local PostgreSQL inventory path escaped its exact root")
            })?;
            let relative = relative.to_str().ok_or_else(|| {
                DevError::usage("local PostgreSQL inventory contains a non-UTF-8 path")
            })?;
            if relative.is_empty() || relative.len() > MAXIMUM_ROOT_PATH_BYTES {
                return Err(DevError::usage(
                    "local PostgreSQL inventory contains an invalid path length",
                ));
            }
            entries.push((relative.to_owned(), path, metadata));
        }
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0));

    let mut hasher = Sha256::new();
    hasher.update(ROOT_INVENTORY_DOMAIN);
    let mut total_bytes = 0_u64;
    for (relative, path, metadata) in entries {
        hash_field(&mut hasher, relative.as_bytes())?;
        #[cfg(unix)]
        hasher.update(metadata.permissions().mode().to_be_bytes());
        #[cfg(not(unix))]
        hasher.update(0_u32.to_be_bytes());
        if metadata.is_dir() {
            hasher.update(b"d");
            hasher.update(0_u64.to_be_bytes());
        } else if metadata.is_file() {
            let bytes = metadata.len();
            if bytes > MAXIMUM_ROOT_ENTRY_BYTES {
                return Err(DevError::usage(format!(
                    "local PostgreSQL entry '{}' exceeds {MAXIMUM_ROOT_ENTRY_BYTES} bytes",
                    path.display()
                )));
            }
            total_bytes = total_bytes
                .checked_add(bytes)
                .ok_or_else(|| DevError::usage("local PostgreSQL root byte count overflowed"))?;
            if total_bytes > MAXIMUM_ROOT_BYTES {
                return Err(DevError::usage(format!(
                    "local PostgreSQL root exceeds {MAXIMUM_ROOT_BYTES} bytes"
                )));
            }
            hasher.update(b"f");
            hasher.update(bytes.to_be_bytes());
            let mut file = File::open(&path).map_err(|error| {
                DevError::usage(format!(
                    "open local PostgreSQL entry '{}': {error}",
                    path.display()
                ))
            })?;
            let mut buffer = [0_u8; 64 * 1024];
            let mut observed = 0_u64;
            loop {
                let read = file.read(&mut buffer).map_err(|error| {
                    DevError::usage(format!(
                        "read local PostgreSQL entry '{}': {error}",
                        path.display()
                    ))
                })?;
                if read == 0 {
                    break;
                }
                observed = observed.saturating_add(read as u64);
                hasher.update(&buffer[..read]);
            }
            if observed != bytes {
                return Err(DevError::usage(format!(
                    "local PostgreSQL entry '{}' changed during verification",
                    path.display()
                )));
            }
        } else {
            let target = fs::read_link(&path).map_err(|error| {
                DevError::usage(format!(
                    "read local PostgreSQL symlink '{}': {error}",
                    path.display()
                ))
            })?;
            let target = target.to_str().ok_or_else(|| {
                DevError::usage("local PostgreSQL inventory contains a non-UTF-8 symlink")
            })?;
            if target.len() > MAXIMUM_ROOT_PATH_BYTES {
                return Err(DevError::usage(
                    "local PostgreSQL inventory symlink target is oversized",
                ));
            }
            hasher.update(b"l");
            hash_field(&mut hasher, target.as_bytes())?;
        }
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn hash_field(hasher: &mut Sha256, bytes: &[u8]) -> Result<(), DevError> {
    let length = u64::try_from(bytes.len())
        .map_err(|_| DevError::usage("local PostgreSQL inventory field length overflowed"))?;
    hasher.update(length.to_be_bytes());
    hasher.update(bytes);
    Ok(())
}
