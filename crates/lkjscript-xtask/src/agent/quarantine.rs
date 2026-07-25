use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

pub fn move_file<T>(path: &Path, task_id: &str, reason: &str) -> Result<T, String> {
    let hash = crate::sha256::digest_file(path, super::bounds::QUARANTINE_BYTES)
        .map_err(|error| format!("hash corrupt state {}: {error}", path.display()))?;
    let directory = path
        .parent()
        .ok_or("state path has no parent")?
        .join("quarantine");
    fs::create_dir_all(&directory).map_err(|error| format!("create quarantine: {error}"))?;
    let destination = directory.join(format!("{task_id}-{hash}.json"));
    if destination.exists() {
        if !equal_files(path, &destination)? {
            return Err("quarantine hash collision; existing file was not overwritten".into());
        }
        fs::remove_file(path)
            .map_err(|error| format!("remove duplicate corrupt state: {error}"))?;
    } else {
        fs::rename(path, &destination)
            .map_err(|error| format!("quarantine corrupt state: {error}"))?;
    }
    Err(format!(
        "quarantined corrupt state as {}: {reason}",
        destination.display()
    ))
}

fn equal_files(left: &Path, right: &Path) -> Result<bool, String> {
    for path in [left, right] {
        let bytes = path
            .metadata()
            .map_err(|error| format!("inspect quarantine candidate: {error}"))?
            .len();
        if bytes > super::bounds::QUARANTINE_BYTES {
            return Err("quarantine comparison exceeds byte limit".into());
        }
    }
    let mut left = File::open(left).map_err(|error| format!("open corrupt state: {error}"))?;
    let mut right = File::open(right).map_err(|error| format!("open quarantine: {error}"))?;
    let mut left_bytes = [0u8; 8192];
    let mut right_bytes = [0u8; 8192];
    let mut total = 0u64;
    loop {
        let left_count = left
            .read(&mut left_bytes)
            .map_err(|error| format!("read corrupt state: {error}"))?;
        let right_count = right
            .read(&mut right_bytes)
            .map_err(|error| format!("read quarantine: {error}"))?;
        if left_count != right_count || left_bytes[..left_count] != right_bytes[..right_count] {
            return Ok(false);
        }
        if left_count == 0 {
            return Ok(true);
        }
        total = total
            .checked_add(u64::try_from(left_count).map_err(|_| "quarantine size overflow")?)
            .ok_or("quarantine size overflow")?;
        if total > super::bounds::QUARANTINE_BYTES {
            return Err("quarantine comparison exceeds byte limit".into());
        }
    }
}
