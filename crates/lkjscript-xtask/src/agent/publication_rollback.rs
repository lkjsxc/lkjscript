use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn restore(
    directory: &Path,
    destination: &Path,
    task_id: &str,
    previous: Option<&[u8]>,
) -> Result<(), String> {
    let Some(bytes) = previous else {
        fs::remove_file(destination).map_err(|error| format!("remove published state: {error}"))?;
        return super::storage::sync_parent(directory);
    };
    let process = std::process::id();
    for attempt in 0..64u8 {
        let path = directory.join(format!(".{task_id}.{process}.{attempt}.rollback.tmp"));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("create state rollback: {error}")),
        };
        let restored = (|| {
            file.write_all(bytes)
                .map_err(|error| format!("write state rollback: {error}"))?;
            super::storage::sync(&file)?;
            drop(file);
            fs::rename(&path, destination)
                .map_err(|error| format!("restore state atomically: {error}"))?;
            super::storage::sync_parent(directory)
        })();
        if restored.is_err() {
            let _ = fs::remove_file(path);
        }
        return restored;
    }
    Err("state rollback temporary namespace exhausted".into())
}
