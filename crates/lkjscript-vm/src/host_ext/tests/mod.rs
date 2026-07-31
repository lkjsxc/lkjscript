#![allow(clippy::panic)]

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use lkjscript_core::Value;

use super::ResourceTable;

static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

struct TempFile(PathBuf);

impl TempFile {
    fn new() -> std::io::Result<Self> {
        let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("lkjscript-handle-{}-{id}", std::process::id()));
        fs::write(&path, b"x")?;
        Ok(Self(path))
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

mod files;
mod sockets;
mod sqlite;
