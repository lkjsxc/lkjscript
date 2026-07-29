use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::{CoordinatorIdentity, RuntimeError};

const LEASE_NAME: &str = "lkjscriptd.lock";
const MAX_LEASE_BYTES: u64 = 128;

pub struct CoordinatorLease {
    path: PathBuf,
    file: File,
}

impl CoordinatorLease {
    pub fn acquire(root: &Path, identity: CoordinatorIdentity) -> Result<Self, RuntimeError> {
        fs::create_dir_all(root)
            .map_err(|_| RuntimeError::CoordinatorLease("create state root"))?;
        let path = root.join(LEASE_NAME);
        match create(&path, identity) {
            Ok(file) => Ok(Self { path, file }),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                if live_owner(&path)? {
                    return Err(RuntimeError::CoordinatorAlreadyActive);
                }
                fs::remove_file(&path)
                    .map_err(|_| RuntimeError::CoordinatorLease("remove stale lease"))?;
                let file = create(&path, identity)
                    .map_err(|_| RuntimeError::CoordinatorLease("replace stale lease"))?;
                Ok(Self { path, file })
            }
            Err(_) => Err(RuntimeError::CoordinatorLease("create exclusive lease")),
        }
    }

    pub fn sync(&self) -> Result<(), RuntimeError> {
        self.file
            .sync_all()
            .map_err(|_| RuntimeError::CoordinatorLease("sync lease"))
    }
}

impl Drop for CoordinatorLease {
    fn drop(&mut self) {
        let _ignored = fs::remove_file(&self.path);
    }
}

fn create(path: &Path, identity: CoordinatorIdentity) -> std::io::Result<File> {
    let mut file = OpenOptions::new().create_new(true).write(true).open(path)?;
    writeln!(file, "{} {}", std::process::id(), identity.get())?;
    file.sync_all()?;
    Ok(file)
}

fn live_owner(path: &Path) -> Result<bool, RuntimeError> {
    let mut file =
        File::open(path).map_err(|_| RuntimeError::CoordinatorLease("open existing lease"))?;
    if file
        .metadata()
        .map_err(|_| RuntimeError::CoordinatorLease("inspect existing lease"))?
        .len()
        > MAX_LEASE_BYTES
    {
        return Err(RuntimeError::CoordinatorLease("oversized existing lease"));
    }
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|_| RuntimeError::CoordinatorLease("read existing lease"))?;
    let mut fields = text.split_whitespace();
    let process = fields
        .next()
        .and_then(|value| value.parse::<u32>().ok())
        .ok_or(RuntimeError::CoordinatorLease("malformed existing lease"))?;
    if fields
        .next()
        .and_then(|value| value.parse::<u64>().ok())
        .is_none()
        || fields.next().is_some()
    {
        return Err(RuntimeError::CoordinatorLease("malformed existing lease"));
    }
    #[cfg(target_os = "linux")]
    {
        Ok(PathBuf::from("/proc").join(process.to_string()).exists())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = process;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_live_lease_fails_and_drop_releases() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("lkjscript-lease-{}", std::process::id()));
        let _ignored = fs::remove_dir_all(&root);
        let identity = CoordinatorIdentity::new(1).ok_or("identity")?;
        let first = CoordinatorLease::acquire(&root, identity)?;
        assert!(matches!(
            CoordinatorLease::acquire(&root, identity),
            Err(RuntimeError::CoordinatorAlreadyActive)
        ));
        drop(first);
        let second = CoordinatorLease::acquire(&root, identity)?;
        second.sync()?;
        drop(second);
        fs::remove_dir_all(root)?;
        Ok(())
    }
}
