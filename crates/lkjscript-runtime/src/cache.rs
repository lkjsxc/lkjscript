use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::sync::Arc;

use lkjscript_contracts::PreparedProgramIdentity;
use lkjscript_core::ValidatedChunk;

use crate::{PackageContentId, RuntimeError};

struct CacheEntry {
    chunk: Arc<ValidatedChunk>,
    last_use: u64,
}

pub(crate) struct CodeCache {
    entries: BTreeMap<(PackageContentId, PreparedProgramIdentity), CacheEntry>,
    max_entries: NonZeroUsize,
    clock: u64,
}

impl CodeCache {
    pub(crate) fn new(max_entries: NonZeroUsize) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_entries,
            clock: 0,
        }
    }

    pub(crate) fn lease(
        &mut self,
        package: PackageContentId,
        chunk: Arc<ValidatedChunk>,
    ) -> Result<Arc<ValidatedChunk>, RuntimeError> {
        let prepared = chunk.prepared_identity();
        if !prepared.is_bound() {
            return Err(RuntimeError::InvalidManifest(
                "installed bytecode has no prepared program authority",
            ));
        }
        self.clock = self.clock.saturating_add(1);
        let key = (package, prepared);
        if let Some(entry) = self.entries.get_mut(&key) {
            entry.last_use = self.clock;
            return Ok(Arc::clone(&entry.chunk));
        }
        if self.entries.len() == self.max_entries.get() {
            let victim = self
                .entries
                .iter()
                .filter(|(_, entry)| Arc::strong_count(&entry.chunk) == 1)
                .min_by_key(|(_, entry)| entry.last_use)
                .map(|(id, _)| *id);
            match victim {
                Some(id) => {
                    self.entries.remove(&id);
                }
                None => return Err(RuntimeError::PackageCacheFull),
            }
        }
        self.entries.insert(
            key,
            CacheEntry {
                chunk: Arc::clone(&chunk),
                last_use: self.clock,
            },
        );
        Ok(chunk)
    }

    pub(crate) fn contains(&self, package: PackageContentId) -> bool {
        self.entries
            .keys()
            .any(|(candidate, _)| *candidate == package)
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }
}
