use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use lkjscript_native::{decode_image, encode_image, InstallableImage};

use crate::{ArtifactKey, CacheError, NativeArtifactCache, Publication};

impl NativeArtifactCache {
    pub fn publish(
        &self,
        key: &ArtifactKey,
        image: &InstallableImage,
    ) -> Result<Publication, CacheError> {
        let bytes = encode_image(image, key.digest(), self.codec_limits()?)
            .map_err(|error| CacheError::host("encode object", error))?;
        let length = u64::try_from(bytes.len())
            .map_err(|_| CacheError::message("cache object length overflow"))?;
        let Some(_lock) = PublicationLock::acquire(&self.root)? else {
            return Ok(Publication::SkippedBusy);
        };
        let staging = self.staging.join("publish.tmp");
        remove_if_present(&staging)?;
        let final_path = self.object_path(key);
        let existing = existing_length(&final_path)?;
        if let Some(existing_bytes) = valid_existing(self, key, &final_path)? {
            if existing_bytes == bytes {
                return Ok(Publication::Duplicate { bytes: length });
            }
            return Err(CacheError::message("cache key collision"));
        }
        let scan = self.scan()?;
        let next_count = scan.count.saturating_add(u64::from(existing.is_none()));
        let next_bytes = scan
            .bytes
            .saturating_sub(existing.unwrap_or(0))
            .saturating_add(length);
        if scan.over_limit
            || next_count > self.limits.max_objects
            || next_bytes > self.limits.max_total_bytes
        {
            return Ok(Publication::SkippedFull);
        }
        let publication = self.write_and_publish(key, &bytes, &staging, &final_path);
        if publication.is_err() {
            let _ = fs::remove_file(&staging);
        }
        publication?;
        Ok(Publication::Published { bytes: length })
    }

    fn write_and_publish(
        &self,
        key: &ArtifactKey,
        bytes: &[u8],
        staging: &Path,
        final_path: &Path,
    ) -> Result<(), CacheError> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(staging)
            .map_err(|error| CacheError::host("create staging object", error))?;
        file.write_all(bytes)
            .map_err(|error| CacheError::host("write staging object", error))?;
        file.flush()
            .map_err(|error| CacheError::host("flush staging object", error))?;
        file.sync_all()
            .map_err(|error| CacheError::host("sync staging object", error))?;
        drop(file);
        let staged =
            fs::read(staging).map_err(|error| CacheError::host("reread staging", error))?;
        decode_image(&staged, key.digest(), self.codec_limits()?)
            .map_err(|error| CacheError::host("validate staging object", error))?;
        fs::rename(staging, final_path)
            .map_err(|error| CacheError::host("publish object", error))?;
        File::open(&self.objects)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| CacheError::host("sync object directory", error))
    }
}

struct PublicationLock {
    path: PathBuf,
    _file: File,
}

impl PublicationLock {
    fn acquire(root: &Path) -> Result<Option<Self>, CacheError> {
        let path = root.join("publication.lock");
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => Ok(Some(Self { path, _file: file })),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(None),
            Err(error) => Err(CacheError::host("acquire publication lock", error)),
        }
    }
}

impl Drop for PublicationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn existing_length(path: &Path) -> Result<Option<u64>, CacheError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Ok(Some(metadata.len()))
        }
        Ok(metadata) => Ok(Some(metadata.len())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(CacheError::host("inspect existing object", error)),
    }
}

fn valid_existing(
    cache: &NativeArtifactCache,
    key: &ArtifactKey,
    path: &Path,
) -> Result<Option<Vec<u8>>, CacheError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => metadata,
        Ok(_) => return Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(CacheError::host("inspect existing object", error)),
    };
    if metadata.len() > cache.limits.max_object_bytes {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(|error| CacheError::host("read existing object", error))?;
    Ok(decode_image(&bytes, key.digest(), cache.codec_limits()?)
        .ok()
        .map(|_| bytes))
}

fn remove_if_present(path: &Path) -> Result<(), CacheError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(CacheError::host("remove stale staging object", error)),
    }
}
