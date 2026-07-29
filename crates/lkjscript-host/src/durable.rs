use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{HostError, HostResult};

/// Named durable byte objects. `sync` is the append durability boundary.
/// `replace` durably installs all bytes atomically or reports failure.
pub trait DurableStorage: Send + Sync {
    fn read(&self, name: &str) -> HostResult<Option<Vec<u8>>>;
    fn append(&self, name: &str, bytes: &[u8]) -> HostResult<usize>;
    fn sync(&self, name: &str) -> HostResult<()>;
    fn replace(&self, name: &str, bytes: &[u8]) -> HostResult<()>;
}

#[derive(Clone, Debug)]
pub struct PortableDurableStorage {
    root: PathBuf,
}

impl PortableDurableStorage {
    pub fn new(root: impl Into<PathBuf>) -> HostResult<Self> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| HostError::from_io("create root", error))?;
        Ok(Self { root })
    }

    fn path(&self, name: &str) -> HostResult<PathBuf> {
        validate_name(name)?;
        Ok(self.root.join(name))
    }

    fn sync_parent(&self) -> HostResult<()> {
        File::open(&self.root)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| HostError::from_io("sync storage directory", error))
    }
}

impl DurableStorage for PortableDurableStorage {
    fn read(&self, name: &str) -> HostResult<Option<Vec<u8>>> {
        let path = self.path(name)?;
        match fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(HostError::from_io(format!("read {name}"), error)),
        }
    }

    fn append(&self, name: &str, bytes: &[u8]) -> HostResult<usize> {
        let path = self.path(name)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write(bytes))
            .map_err(|error| HostError::from_io(format!("append {name}"), error))
    }

    fn sync(&self, name: &str) -> HostResult<()> {
        let path = self.path(name)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|file| file.sync_all())
            .map_err(|error| HostError::SyncFailed(format!("{name}: {error}")))
    }

    fn replace(&self, name: &str, bytes: &[u8]) -> HostResult<()> {
        let path = self.path(name)?;
        let replacement = replacement_path(&path);
        let result = (|| {
            let mut file = File::create(&replacement)
                .map_err(|error| HostError::from_io(format!("create {name} replacement"), error))?;
            file.write_all(bytes)
                .map_err(|error| HostError::from_io(format!("write {name} replacement"), error))?;
            file.sync_all()
                .map_err(|error| HostError::SyncFailed(format!("{name}: {error}")))?;
            fs::rename(&replacement, &path)
                .map_err(|error| HostError::from_io(format!("install {name}"), error))?;
            self.sync_parent()
        })();
        if result.is_err() {
            let _ignored = fs::remove_file(replacement);
        }
        result
    }
}

fn validate_name(name: &str) -> HostResult<()> {
    if name.is_empty()
        || name.len() > 128
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
        || name == "."
        || name == ".."
    {
        return Err(HostError::InvalidName(name.to_owned()));
    }
    Ok(())
}

fn replacement_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_owned();
    value.push(".replacement");
    PathBuf::from(value)
}
