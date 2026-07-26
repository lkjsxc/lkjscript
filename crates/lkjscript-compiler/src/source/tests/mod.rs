#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use lkjscript_core::Limits;

use super::{
    load, parse as parser, validate, validate::finish_tree, validate_source_directory_tree,
    DeclarationKind, NodeKind, SourceFoundationBudget, SourceOrigin,
    FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES, FOUNDATION_MAX_SOURCE_FILE_BYTES,
    FOUNDATION_MAX_SOURCE_UNITS,
};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> std::io::Result<Self> {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "lkjscript-source-{label}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn unit_main(body: &str) -> String {
    format!("main/\nsig/\n->\nUnit\n/sig\n{body}\n/main\n")
}

fn named_def(name: &str) -> String {
    format!(
        "def/\nname/\n{name}\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\nunit\n/fn\n/def\n"
    )
}

mod diagnostics;
mod edition;
mod format;
mod identity;
mod limits;
mod linux_safety;
mod loading;
mod modules;
