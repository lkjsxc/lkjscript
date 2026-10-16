//! Bounded, forkable project authority without filesystem handles or temporary paths.

use super::object::{ObjectKey, StoreError, StoreErrorClass};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// One capacity shared by a project, its forks, and all retained revision views.
#[derive(Clone, Debug)]
pub(crate) struct MemoryArena(Arc<ArenaInner>);

#[derive(Debug)]
struct ArenaInner {
    maximum: usize,
    used: AtomicUsize,
}

#[derive(Debug)]
struct Reservation {
    arena: MemoryArena,
    bytes: usize,
}

impl MemoryArena {
    fn new(maximum: usize) -> Self {
        Self(Arc::new(ArenaInner {
            maximum,
            used: AtomicUsize::new(0),
        }))
    }

    fn reserve(&self, bytes: usize) -> Result<Reservation, StoreError> {
        self.0.used.fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
            used.checked_add(bytes).filter(|next| *next <= self.0.maximum)
        }).map_err(|_| memory_error(StoreErrorClass::Resource, "memory_project_capacity",
            "project arena capacity exhausted; release retained views or use a larger arena"))?;
        Ok(Reservation {
            arena: self.clone(),
            bytes,
        })
    }
}

impl Reservation {
    fn grow(&mut self, bytes: usize) -> Result<(), StoreError> {
        let mut added = self.arena.reserve(bytes)?;
        self.bytes += added.bytes;
        added.bytes = 0;
        Ok(())
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        self.arena.0.used.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}

/// Immutable bytes charge their allocation exactly once even when many forks retain them.
pub(crate) struct MemoryBytes {
    bytes: Vec<u8>,
    verified: OnceLock<ObjectKey>,
    _reservation: Reservation,
}

impl std::fmt::Debug for MemoryBytes {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MemoryBytes")
            .field("length", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

impl MemoryBytes {
    pub(crate) fn copy(arena: &MemoryArena, bytes: &[u8]) -> Result<Arc<Self>, StoreError> {
        let size = bytes.len().checked_add(128).ok_or_else(capacity_overflow)?;
        let reservation = arena.reserve(size)?;
        Ok(Arc::new(Self {
            bytes: bytes.to_vec(),
            verified: OnceLock::new(),
            _reservation: reservation,
        }))
    }

    pub(crate) fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    /// Only immutable private payloads can carry this successful content proof.
    /// A replacement allocation starts unverified, including corruption fixtures.
    pub(crate) fn verify(&self, key: ObjectKey) -> Result<(), StoreError> {
        if self.verified.get() != Some(&key) {
            key.verify(&self.bytes)?;
            let _ = self.verified.set(key);
        }
        Ok(())
    }
}

#[derive(Debug)]
struct MemoryIndex<K> {
    values: BTreeMap<K, Arc<MemoryBytes>>,
    reservation: Reservation,
}

/// Indexes themselves are copy-on-write and accounted separately from shared payloads.
#[derive(Clone, Debug)]
pub(crate) struct MemoryMap<K> {
    index: Arc<MemoryIndex<K>>,
}

impl<K: Ord + Clone> MemoryMap<K> {
    pub(crate) fn new(arena: &MemoryArena) -> Result<Self, StoreError> {
        Ok(Self {
            index: Arc::new(MemoryIndex {
                values: BTreeMap::new(),
                reservation: arena.reserve(256)?,
            }),
        })
    }

    pub(crate) fn get(&self, key: &K) -> Option<&Arc<MemoryBytes>> {
        self.index.values.get(key)
    }

    pub(crate) fn len(&self) -> usize {
        self.index.values.len()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&K, &Arc<MemoryBytes>)> {
        self.index.values.iter()
    }

    pub(crate) fn insert(&mut self, key: K, bytes: &[u8]) -> Result<(), StoreError> {
        if self.get(&key).is_some_and(|old| old.as_slice() == bytes) {
            return Ok(());
        }
        let arena = self.index.reservation.arena.clone();
        let value = MemoryBytes::copy(&arena, bytes)?;
        if Arc::get_mut(&mut self.index).is_none() {
            let reservation = arena.reserve(self.index.reservation.bytes)?;
            self.index = Arc::new(MemoryIndex {
                values: self.index.values.clone(),
                reservation,
            });
        }
        let index = Arc::get_mut(&mut self.index).ok_or_else(|| {
            memory_error(
                StoreErrorClass::Io,
                "memory_project_index",
                "private memory index remained shared",
            )
        })?;
        if !index.values.contains_key(&key) {
            // Conservatively includes the B-tree node, key, pointer, and allocator overhead.
            index
                .reservation
                .grow(std::mem::size_of::<K>().saturating_mul(2) + 192)?;
        }
        index.values.insert(key, value);
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct MemoryObjectStore {
    pub(crate) arena: MemoryArena,
    pub(crate) objects: MemoryMap<ObjectKey>,
    pub(crate) readiness: Option<Arc<MemoryBytes>>,
    pub(crate) staged: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct MemoryGraphState {
    pub(crate) store: MemoryObjectStore,
    pub(crate) head: Option<Arc<MemoryBytes>>,
    pub(crate) compilation_head: Option<Arc<MemoryBytes>>,
}

#[derive(Clone, Debug)]
pub(crate) struct MemoryDataState {
    pub(crate) store_id: Option<[u8; 32]>,
    pub(crate) head: Option<[u8; 32]>,
    pub(crate) revisions: MemoryMap<[u8; 32]>,
}

/// Program meaning and application data keep independent atomic publication heads.
/// Forking shares immutable bytes and indexes; mutations copy only the selected index.
#[derive(Clone, Debug)]
pub struct MemProjectStore {
    pub(crate) arena: MemoryArena,
    pub(crate) graph: Arc<Mutex<MemoryGraphState>>,
    pub(crate) data: Arc<Mutex<MemoryDataState>>,
    _reservation: Arc<Reservation>,
}

/// Independent byte commitments for assertions that read-only operations did not mutate authority.
#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MemoryProjectInventory {
    graph_head: Option<Vec<u8>>,
    readiness: Option<Vec<u8>>,
    compilation_head: Option<Vec<u8>>,
    objects: BTreeMap<ObjectKey, [u8; 32]>,
    staged: usize,
    data_identity: Option<[u8; 32]>,
    data_head: Option<[u8; 32]>,
    data_revisions: BTreeMap<[u8; 32], [u8; 32]>,
}

impl MemProjectStore {
    pub fn new(maximum_bytes: usize) -> Result<Self, crate::platform::diagnostic::Diagnostic> {
        let arena = MemoryArena::new(maximum_bytes);
        let create = || -> Result<Self, StoreError> {
            let reservation = Arc::new(arena.reserve(512)?);
            let store = MemoryObjectStore {
                arena: arena.clone(),
                objects: MemoryMap::new(&arena)?,
                readiness: None,
                staged: 0,
            };
            let data = MemoryDataState {
                store_id: None,
                head: None,
                revisions: MemoryMap::new(&arena)?,
            };
            Ok(Self {
                arena: arena.clone(),
                graph: Arc::new(Mutex::new(MemoryGraphState {
                    store,
                    head: None,
                    compilation_head: None,
                })),
                data: Arc::new(Mutex::new(data)),
                _reservation: reservation,
            })
        };
        create().map_err(diagnostic)
    }

    pub fn fork(&self) -> Result<Self, crate::platform::diagnostic::Diagnostic> {
        let reservation = Arc::new(self.arena.reserve(512).map_err(diagnostic)?);
        let graph = lock(&self.graph)?.clone();
        let data = lock(&self.data)?.clone();
        Ok(Self {
            arena: self.arena.clone(),
            graph: Arc::new(Mutex::new(graph)),
            data: Arc::new(Mutex::new(data)),
            _reservation: reservation,
        })
    }

    pub fn used_bytes(&self) -> usize {
        self.arena.0.used.load(Ordering::Acquire)
    }

    pub fn maximum_bytes(&self) -> usize {
        self.arena.0.maximum
    }

    #[cfg(test)]
    pub(crate) fn test_inventory(
        &self,
    ) -> Result<MemoryProjectInventory, crate::platform::diagnostic::Diagnostic> {
        let graph = lock(&self.graph)?.clone();
        let data = lock(&self.data)?.clone();
        let bytes = |value: &Option<Arc<MemoryBytes>>| {
            value.as_ref().map(|value| value.as_slice().to_vec())
        };
        Ok(MemoryProjectInventory {
            graph_head: bytes(&graph.head),
            readiness: bytes(&graph.store.readiness),
            compilation_head: bytes(&graph.compilation_head),
            objects: graph
                .store
                .objects
                .iter()
                .map(|(key, value)| (*key, *blake3::hash(value.as_slice()).as_bytes()))
                .collect(),
            staged: graph.store.staged,
            data_identity: data.store_id,
            data_head: data.head,
            data_revisions: data
                .revisions
                .iter()
                .map(|(key, value)| (*key, *blake3::hash(value.as_slice()).as_bytes()))
                .collect(),
        })
    }

    pub fn data_store(
        &self,
        namespace: impl Into<String>,
        limits: crate::platform::data::DataLimits,
    ) -> Result<crate::platform::data::DataStore, crate::platform::diagnostic::Diagnostic> {
        crate::platform::data::DataStore::open_memory(self, namespace.into(), limits)
    }
}

pub(crate) fn lock<T>(
    mutex: &Mutex<T>,
) -> Result<MutexGuard<'_, T>, crate::platform::diagnostic::Diagnostic> {
    mutex.lock().map_err(|_| {
        diagnostic(memory_error(
            StoreErrorClass::Io,
            "memory_project_lock",
            "memory project publication lock was poisoned",
        ))
    })
}

fn capacity_overflow() -> StoreError {
    memory_error(
        StoreErrorClass::Resource,
        "memory_project_capacity",
        "memory allocation size overflow",
    )
}

pub(crate) fn memory_error(
    class: StoreErrorClass,
    code: &'static str,
    message: &str,
) -> StoreError {
    StoreError {
        class,
        code,
        message: message.to_owned(),
    }
}

pub(crate) fn diagnostic(error: StoreError) -> crate::platform::diagnostic::Diagnostic {
    use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
    Diagnostic::new(
        match error.class {
            StoreErrorClass::Input => DiagnosticClass::Source,
            StoreErrorClass::Resource => DiagnosticClass::Resource,
            StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
            StoreErrorClass::Io => DiagnosticClass::Infrastructure,
        },
        error.code,
        error.message,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::storage::object::ObjectDomain;

    #[test]
    fn immutable_payload_proofs_bind_the_key_and_release_their_arena() {
        let arena = MemoryArena::new(1024);
        let bytes = b"exact immutable input";
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, bytes);
        let value = MemoryBytes::copy(&arena, bytes).unwrap();
        value.verify(key).unwrap();
        value.verify(key).unwrap();
        assert_eq!(value.verified.get(), Some(&key));
        let foreign = ObjectKey::from_digest(ObjectDomain::Owner, key.digest.bytes());
        assert!(value.verify(foreign).is_err());
        assert_eq!(value.verified.get(), Some(&key));
        let replaced = MemoryBytes::copy(&arena, b"changed input").unwrap();
        assert!(replaced.verify(key).is_err());
        assert!(replaced.verified.get().is_none());
        assert!(arena.0.used.load(Ordering::Acquire) > bytes.len());
        drop(value);
        drop(replaced);
        assert_eq!(arena.0.used.load(Ordering::Acquire), 0);
    }
}
