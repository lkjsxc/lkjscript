use std::fs;
use std::path::PathBuf;

use crate::{ApplicationPath, DirectoryProvider, HostError, HostResult};

#[derive(Clone, Debug)]
pub struct PortableDirectory {
    root: PathBuf,
}

impl PortableDirectory {
    pub fn open(root: impl Into<PathBuf>) -> HostResult<Self> {
        let root = root.into();
        let root = fs::canonicalize(&root)
            .map_err(|error| HostError::from_io("open directory capability", error))?;
        if !root.is_dir() {
            return Err(HostError::InvalidName(
                "directory capability root".to_string(),
            ));
        }
        Ok(Self { root })
    }

    fn candidate(&self, path: &ApplicationPath) -> PathBuf {
        path.segments()
            .fold(self.root.clone(), |current, segment| current.join(segment))
    }

    fn existing(&self, path: &ApplicationPath) -> HostResult<PathBuf> {
        let canonical = fs::canonicalize(self.candidate(path))
            .map_err(|error| HostError::from_io("resolve application path", error))?;
        if !canonical.starts_with(&self.root) {
            return Err(HostError::PermissionDenied(path.as_str().to_string()));
        }
        Ok(canonical)
    }

    fn writable(&self, path: &ApplicationPath) -> HostResult<PathBuf> {
        let candidate = self.candidate(path);
        let parent = candidate
            .parent()
            .ok_or_else(|| HostError::InvalidName(path.as_str().to_string()))?;
        let canonical_parent = fs::canonicalize(parent)
            .map_err(|error| HostError::from_io("resolve application path parent", error))?;
        if !canonical_parent.starts_with(&self.root) {
            return Err(HostError::PermissionDenied(path.as_str().to_string()));
        }
        let name = candidate
            .file_name()
            .ok_or_else(|| HostError::InvalidName(path.as_str().to_string()))?;
        let target = canonical_parent.join(name);
        match fs::symlink_metadata(&target) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                Err(HostError::PermissionDenied(path.as_str().to_string()))
            }
            Ok(_) => Ok(target),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(target),
            Err(error) => Err(HostError::from_io("inspect application path", error)),
        }
    }
}

#[cfg(target_os = "linux")]
fn write_file(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    const O_NOFOLLOW: i32 = 0o400_000;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .custom_flags(O_NOFOLLOW)
        .open(path)?;
    file.write_all(bytes)
}

#[cfg(not(target_os = "linux"))]
fn write_file(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    fs::write(path, bytes)
}

impl DirectoryProvider for PortableDirectory {
    fn read(&self, path: &ApplicationPath) -> HostResult<Vec<u8>> {
        fs::read(self.existing(path)?)
            .map_err(|error| HostError::from_io("read application file", error))
    }

    fn write(&self, path: &ApplicationPath, bytes: &[u8]) -> HostResult<()> {
        write_file(&self.writable(path)?, bytes)
            .map_err(|error| HostError::from_io("write application file", error))
    }

    fn remove(&self, path: &ApplicationPath) -> HostResult<()> {
        fs::remove_file(self.writable(path)?)
            .map_err(|error| HostError::from_io("remove application file", error))
    }

    fn list(&self, path: Option<&ApplicationPath>) -> HostResult<Vec<String>> {
        let directory = match path {
            Some(path) => self.existing(path)?,
            None => self.root.clone(),
        };
        let mut names = Vec::new();
        for entry in fs::read_dir(directory)
            .map_err(|error| HostError::from_io("list application directory", error))?
        {
            let entry = entry.map_err(|error| HostError::from_io("read directory entry", error))?;
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_string());
            }
        }
        names.sort();
        Ok(names)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directory_capability_contains_relative_paths() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("lkjscript-directory-{}", std::process::id()));
        let _ignored = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("assets"))?;
        let directory = PortableDirectory::open(&root)?;
        let path = ApplicationPath::parse("assets/counter.txt")?;
        directory.write(&path, b"41")?;
        assert_eq!(directory.read(&path)?, b"41");
        assert_eq!(
            directory.list(Some(&ApplicationPath::parse("assets")?))?,
            vec!["counter.txt"]
        );
        directory.remove(&path)?;
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn directory_capability_rejects_final_symlink_write_and_remove(
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::symlink;

        let base = std::env::temp_dir().join(format!(
            "lkjscript-directory-symlink-{}",
            std::process::id()
        ));
        let root = base.join("root");
        let outside = base.join("outside.txt");
        let _ignored = fs::remove_dir_all(&base);
        fs::create_dir_all(&root)?;
        fs::write(&outside, b"outside")?;
        symlink(&outside, root.join("escape"))?;
        let directory = PortableDirectory::open(&root)?;
        let path = ApplicationPath::parse("escape")?;

        assert!(matches!(
            directory.write(&path, b"replaced"),
            Err(HostError::PermissionDenied(name)) if name == "escape"
        ));
        assert!(matches!(
            directory.remove(&path),
            Err(HostError::PermissionDenied(name)) if name == "escape"
        ));
        assert_eq!(fs::read(&outside)?, b"outside");
        assert!(fs::symlink_metadata(root.join("escape"))?
            .file_type()
            .is_symlink());

        symlink(&base, root.join("parent-escape"))?;
        let nested_escape = ApplicationPath::parse("parent-escape/outside.txt")?;
        assert!(matches!(
            directory.write(&nested_escape, b"replaced"),
            Err(HostError::PermissionDenied(name)) if name == "parent-escape/outside.txt"
        ));
        assert!(matches!(
            directory.remove(&nested_escape),
            Err(HostError::PermissionDenied(name)) if name == "parent-escape/outside.txt"
        ));
        assert_eq!(fs::read(&outside)?, b"outside");

        fs::remove_dir_all(base)?;
        Ok(())
    }
}
