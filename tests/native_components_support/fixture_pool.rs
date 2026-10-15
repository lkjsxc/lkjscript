//! Immutable artifact admission is shared by content; no invocation state enters the pool.

use lkjscript::platform::{Diagnostic, ExecutionControl, native_tool::PureTool};
use sha2::{Digest, Sha256};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

type DigestBytes = [u8; 32];
type FixtureResult<T> = Result<T, Box<Diagnostic>>;

#[derive(Clone)]
pub struct Binding {
    pub digest: DigestBytes,
    pub tool: Arc<PureTool>,
}

#[derive(Default)]
pub struct ArtifactPool {
    paths: RefCell<BTreeMap<PathBuf, Binding>>,
    preparations: Cell<u64>,
    content_hits: Cell<u64>,
    binding_hits: Cell<u64>,
    fresh_preparations: Cell<u64>,
    fresh_paths: RefCell<BTreeSet<PathBuf>>,
}

impl ArtifactPool {
    pub fn bind(&self, path: &Path) -> FixtureResult<Binding> {
        if let Some(binding) = self.paths.borrow().get(path) {
            self.binding_hits.set(self.binding_hits.get() + 1);
            return Ok(binding.clone());
        }
        let bytes = fs::read(path).unwrap();
        let (binding, prepared) = admitted(&bytes)?;
        if prepared {
            self.preparations.set(self.preparations.get() + 1);
        } else {
            self.content_hits.set(self.content_hits.get() + 1);
        }
        self.paths
            .borrow_mut()
            .insert(path.to_path_buf(), binding.clone());
        Ok(binding)
    }

    /// A successful public build invalidates the existing artifact binding.
    pub fn built(&self, path: &Path) -> FixtureResult<()> {
        self.paths.borrow_mut().remove(path);
        self.fresh_paths.borrow_mut().remove(path);
        self.bind(path).map(|_| ())
    }

    /// Source removal is checked against current artifact bytes, with a new strict admission.
    pub fn fresh_after_detachment(&self, path: &Path) -> FixtureResult<Arc<PureTool>> {
        let binding = self.bind(path)?;
        if self.fresh_paths.borrow().contains(path) {
            return Ok(binding.tool);
        }
        let bytes = fs::read(path).unwrap();
        assert_eq!(binding.digest, fingerprint(&bytes));
        let tool = Arc::new(PureTool::load(&bytes, &ExecutionControl::uncancelled())?);
        assert!(!Arc::ptr_eq(&tool, &binding.tool));
        self.paths.borrow_mut().insert(
            path.to_path_buf(),
            Binding {
                digest: binding.digest,
                tool: Arc::clone(&tool),
            },
        );
        self.fresh_paths.borrow_mut().insert(path.to_path_buf());
        self.fresh_preparations
            .set(self.fresh_preparations.get() + 1);
        Ok(tool)
    }

    pub fn observation(&self) -> serde_json::Value {
        serde_json::json!({
            "bound_artifacts": self.paths.borrow().len(),
            "program_preparations": self.preparations.get(),
            "content_cache_hits": self.content_hits.get(),
            "binding_cache_hits": self.binding_hits.get(),
            "fresh_detached_preparations": self.fresh_preparations.get(),
        })
    }
}

fn fingerprint(bytes: &[u8]) -> DigestBytes {
    Sha256::digest(bytes).into()
}

fn admitted(bytes: &[u8]) -> FixtureResult<(Binding, bool)> {
    static PROGRAMS: OnceLock<Mutex<BTreeMap<DigestBytes, Arc<PureTool>>>> = OnceLock::new();
    let digest = fingerprint(bytes);
    let mut programs = PROGRAMS.get_or_init(Mutex::default).lock().unwrap();
    let (tool, prepared) = if let Some(tool) = programs.get(&digest) {
        (Arc::clone(tool), false)
    } else {
        let tool = Arc::new(PureTool::load(bytes, &ExecutionControl::uncancelled())?);
        programs.insert(digest, Arc::clone(&tool));
        (tool, true)
    };
    Ok((Binding { digest, tool }, prepared))
}
