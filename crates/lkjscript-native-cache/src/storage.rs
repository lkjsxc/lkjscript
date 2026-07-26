use std::fs::{self, File};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use lkjscript_native::{decode_image, ImageCodecLimits};

use crate::{ArtifactKey, CacheError, CacheLimits, Lookup, MissReason};

pub struct NativeArtifactCache {
    pub(crate) root: PathBuf,
    pub(crate) objects: PathBuf,
    pub(crate) staging: PathBuf,
    pub(crate) limits: CacheLimits,
}

impl NativeArtifactCache {
    pub fn open(package_root: &Path, limits: CacheLimits) -> Result<Self, CacheError> {
        let package_root = package_root
            .canonicalize()
            .map_err(|error| CacheError::host("canonicalize package root", error))?;
        let target = checked_directory(&package_root, "target")?;
        let artifacts = checked_directory(&target, "lkjscript")?;
        let root = checked_directory(&artifacts, "native-cache")?;
        let objects = checked_directory(&root, "objects")?;
        let staging = checked_directory(&root, "staging")?;
        let canonical = root
            .canonicalize()
            .map_err(|error| CacheError::host("canonicalize root", error))?;
        if !canonical.starts_with(&package_root) {
            return Err(CacheError::message("native cache root escapes package"));
        }
        Ok(Self {
            root,
            objects,
            staging,
            limits,
        })
    }

    pub fn lookup(&self, key: &ArtifactKey) -> Result<Lookup, CacheError> {
        let path = self.object_path(key);
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Lookup::Miss(MissReason::NotFound));
            }
            Err(error) => return Err(CacheError::host("inspect object", error)),
        };
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > self.limits.max_object_bytes
        {
            return Ok(Lookup::Miss(MissReason::Corrupt));
        }
        if self.scan()?.over_limit {
            return Ok(Lookup::Miss(MissReason::OverLimit));
        }
        let mut file = File::open(&path).map_err(|error| CacheError::host("open object", error))?;
        let maximum = self.limits.max_object_bytes.saturating_add(1);
        let mut bytes = Vec::new();
        file.by_ref()
            .take(maximum)
            .read_to_end(&mut bytes)
            .map_err(|error| CacheError::host("read object", error))?;
        if u64::try_from(bytes.len())
            .ok()
            .is_none_or(|length| length != metadata.len())
        {
            return Ok(Lookup::Miss(MissReason::Corrupt));
        }
        let codec = self.codec_limits()?;
        match decode_image(&bytes, key.digest(), codec) {
            Ok(image) => Ok(Lookup::Hit {
                image: Box::new(image),
                bytes: metadata.len(),
            }),
            Err(_) => Ok(Lookup::Miss(MissReason::Corrupt)),
        }
    }

    pub(crate) fn object_path(&self, key: &ArtifactKey) -> PathBuf {
        self.objects.join(format!("{}.image", key.hex()))
    }

    pub(crate) fn codec_limits(&self) -> Result<ImageCodecLimits, CacheError> {
        Ok(ImageCodecLimits {
            max_encoded_bytes: usize::try_from(self.limits.max_object_bytes)
                .map_err(|_| CacheError::message("cache object limit overflow"))?,
            max_records: self.limits.max_records,
        })
    }

    pub(crate) fn scan(&self) -> Result<Scan, CacheError> {
        let mut count = 0_u64;
        let mut bytes = 0_u64;
        for entry in
            fs::read_dir(&self.objects).map_err(|error| CacheError::host("scan objects", error))?
        {
            let entry = entry.map_err(|error| CacheError::host("read object entry", error))?;
            count = count
                .checked_add(1)
                .ok_or_else(|| CacheError::message("cache object count overflow"))?;
            if count > self.limits.max_objects.saturating_add(1) {
                return Ok(Scan {
                    count,
                    bytes,
                    over_limit: true,
                });
            }
            let metadata = fs::symlink_metadata(entry.path())
                .map_err(|error| CacheError::host("inspect object entry", error))?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Ok(Scan {
                    count,
                    bytes,
                    over_limit: true,
                });
            }
            bytes = bytes
                .checked_add(metadata.len())
                .ok_or_else(|| CacheError::message("cache byte count overflow"))?;
        }
        Ok(Scan {
            count,
            bytes,
            over_limit: count > self.limits.max_objects || bytes > self.limits.max_total_bytes,
        })
    }
}

pub(crate) struct Scan {
    pub(crate) count: u64,
    pub(crate) bytes: u64,
    pub(crate) over_limit: bool,
}

fn checked_directory(parent: &Path, name: &str) -> Result<PathBuf, CacheError> {
    let path = parent.join(name);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(CacheError::message(
                "native cache directory is not contained",
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if let Err(error) = fs::create_dir(&path) {
                if error.kind() != std::io::ErrorKind::AlreadyExists {
                    return Err(CacheError::host("create directory", error));
                }
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|error| CacheError::host("inspect raced directory", error))?;
                if metadata.file_type().is_symlink() || !metadata.is_dir() {
                    return Err(CacheError::message(
                        "native cache directory is not contained",
                    ));
                }
            }
        }
        Err(error) => return Err(CacheError::host("inspect directory", error)),
    }
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
        .map_err(|error| CacheError::host("set directory permissions", error))?;
    Ok(path)
}
