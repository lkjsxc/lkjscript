//! Finite immutable admission reuse, bound to the complete bytes read for every call.

use lkjscript::platform::{Diagnostic, DiagnosticClass, ExecutionControl, native_tool::PureTool};
use sha2::{Digest, Sha256};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

type DigestBytes = [u8; 32];
type FixtureResult<T> = Result<T, Box<Diagnostic>>;

const MAXIMUM_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;
const MAXIMUM_SHARED_PROGRAMS: usize = 8;
const MAXIMUM_SHARED_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone)]
pub struct Binding {
    pub digest: DigestBytes,
    pub tool: Arc<PureTool>,
}

struct Admitted<T> {
    digest: DigestBytes,
    bytes: Arc<[u8]>,
    value: Arc<T>,
}

impl Admitted<PureTool> {
    fn binding(&self) -> Binding {
        Binding {
            digest: self.digest,
            tool: Arc::clone(&self.value),
        }
    }
}

struct PathBinding {
    path: PathBuf,
    admitted: Arc<Admitted<PureTool>>,
    fresh_detached: bool,
}

#[derive(Default)]
pub struct ArtifactPool {
    // A fixture retains only its current binding. Shared retention has separate finite
    // limits; invocation Arcs remain owned by their joined callers, never another cache.
    current: RefCell<Option<PathBinding>>,
    preparations: Cell<u64>,
    content_hits: Cell<u64>,
    binding_hits: Cell<u64>,
    fresh_preparations: Cell<u64>,
}

impl ArtifactPool {
    pub fn bind(&self, path: &Path) -> FixtureResult<Binding> {
        let bytes = match read_artifact(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.current.borrow_mut().take();
                return Err(error);
            }
        };
        let digest = fingerprint(&bytes);
        if let Some(current) = self.current.borrow().as_ref()
            && current.path == path
            && current.admitted.digest == digest
            && current.admitted.bytes.as_ref() == bytes
        {
            self.binding_hits.set(self.binding_hits.get() + 1);
            return Ok(current.admitted.binding());
        }
        // A failed readmission cannot leave this path authorized by its previous bytes.
        self.current.borrow_mut().take();
        let (admitted, prepared) = shared_programs().lock().unwrap().admit(bytes, |bytes| {
            PureTool::load(bytes, &ExecutionControl::uncancelled()).map_err(Box::new)
        })?;
        if prepared {
            self.preparations.set(self.preparations.get() + 1);
        } else {
            self.content_hits.set(self.content_hits.get() + 1);
        }
        let binding = admitted.binding();
        self.current.replace(Some(PathBinding {
            path: path.to_owned(),
            admitted,
            fresh_detached: false,
        }));
        Ok(binding)
    }

    /// A successful public build invalidates the existing artifact binding.
    pub fn built(&self, path: &Path) -> FixtureResult<()> {
        self.current.borrow_mut().take();
        self.bind(path).map(|_| ())
    }

    /// Source removal gets a new strict admission, retained only for this exact binding.
    /// Every later call still rereads the artifact; mutation invalidates detached freshness.
    pub fn fresh_after_detachment(&self, path: &Path) -> FixtureResult<Arc<PureTool>> {
        let binding = self.bind(path)?;
        let mut current = self.current.borrow_mut();
        let current = current
            .as_mut()
            .expect("successful current artifact binding");
        assert_eq!(current.path, path);
        if current.fresh_detached {
            return Ok(binding.tool);
        }
        let tool = Arc::new(PureTool::load(
            &current.admitted.bytes,
            &ExecutionControl::uncancelled(),
        )?);
        assert!(!Arc::ptr_eq(&tool, &binding.tool));
        current.admitted = Arc::new(Admitted {
            digest: binding.digest,
            bytes: Arc::clone(&current.admitted.bytes),
            value: Arc::clone(&tool),
        });
        current.fresh_detached = true;
        self.fresh_preparations
            .set(self.fresh_preparations.get() + 1);
        Ok(tool)
    }

    pub fn observation(&self) -> serde_json::Value {
        let current = self.current.borrow();
        let shared = shared_programs().lock().unwrap();
        serde_json::json!({
            "bound_artifacts": usize::from(current.is_some()),
            "maximum_bound_artifacts": 1,
            "program_preparations": self.preparations.get(),
            "content_cache_hits": self.content_hits.get(),
            "binding_cache_hits": self.binding_hits.get(),
            "fresh_detached_preparations": self.fresh_preparations.get(),
            "shared_programs": shared.entries.len(),
            "maximum_shared_programs": MAXIMUM_SHARED_PROGRAMS,
            // This measures exact encoded inputs, not Rust heap allocations.
            "shared_artifact_bytes": shared.artifact_bytes,
            "maximum_shared_artifact_bytes": MAXIMUM_SHARED_ARTIFACT_BYTES,
        })
    }
}

fn read_artifact(path: &Path) -> FixtureResult<Vec<u8>> {
    let failure = |error| {
        Box::new(Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "fixture_artifact_read",
            format!("cannot read current artifact '{}': {error}", path.display()),
        ))
    };
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(failure)?
        .take(MAXIMUM_ARTIFACT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(failure)?;
    if bytes.len() > MAXIMUM_ARTIFACT_BYTES {
        return Err(Box::new(Diagnostic::new(
            DiagnosticClass::Resource,
            "fixture_artifact_bytes",
            "native fixture artifact exceeds its 64 MiB input bound",
        )));
    }
    Ok(bytes)
}

fn fingerprint(bytes: &[u8]) -> DigestBytes {
    Sha256::digest(bytes).into()
}

struct Programs<T> {
    entries: VecDeque<Arc<Admitted<T>>>,
    artifact_bytes: usize,
    maximum_entries: usize,
    maximum_artifact_bytes: usize,
}

impl<T> Programs<T> {
    fn new(maximum_entries: usize, maximum_artifact_bytes: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            artifact_bytes: 0,
            maximum_entries,
            maximum_artifact_bytes,
        }
    }

    fn admit<E>(
        &mut self,
        bytes: Vec<u8>,
        load: impl FnOnce(&[u8]) -> Result<T, E>,
    ) -> Result<(Arc<Admitted<T>>, bool), E> {
        let digest = fingerprint(&bytes);
        if let Some(entry) = self
            .entries
            .iter()
            .find(|entry| entry.digest == digest && entry.bytes.as_ref() == bytes)
        {
            return Ok((Arc::clone(entry), false));
        }
        // Only successful complete admission can publish a reusable entry. The mutex
        // serializes preparation, so concurrent identical misses cannot multiply owners.
        let value = Arc::new(load(&bytes)?);
        let entry = Arc::new(Admitted {
            digest,
            bytes: bytes.into(),
            value,
        });
        if self.maximum_entries > 0 && entry.bytes.len() <= self.maximum_artifact_bytes {
            while self.entries.len() >= self.maximum_entries
                || self.artifact_bytes > self.maximum_artifact_bytes - entry.bytes.len()
            {
                let evicted = self.entries.pop_front().unwrap();
                self.artifact_bytes -= evicted.bytes.len();
            }
            self.artifact_bytes += entry.bytes.len();
            self.entries.push_back(Arc::clone(&entry));
        }
        Ok((entry, true))
    }
}

fn shared_programs() -> &'static Mutex<Programs<PureTool>> {
    static PROGRAMS: OnceLock<Mutex<Programs<PureTool>>> = OnceLock::new();
    PROGRAMS.get_or_init(|| {
        Mutex::new(Programs::new(
            MAXIMUM_SHARED_PROGRAMS,
            MAXIMUM_SHARED_ARTIFACT_BYTES,
        ))
    })
}

#[cfg(test)]
mod tests;
