//! Forkable project authority with optional exact capacity and no filesystem handles.

use super::object::{ObjectKey, StoreError, StoreErrorClass};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// One optional capacity shared by a project, its forks, and retained revision views.
#[derive(Clone, Debug)]
pub(crate) struct MemoryArena(Arc<ArenaInner>);

#[derive(Debug)]
struct ArenaInner {
    maximum: Option<usize>,
    used: AtomicUsize,
}

#[derive(Debug)]
struct Reservation {
    arena: MemoryArena,
    bytes: usize,
}

impl MemoryArena {
    fn new(maximum: Option<usize>) -> Self {
        Self(Arc::new(ArenaInner {
            maximum,
            used: AtomicUsize::new(0),
        }))
    }

    fn reserve(&self, bytes: usize) -> Result<Reservation, StoreError> {
        self.0
            .used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(bytes)
                    .filter(|next| self.0.maximum.is_none_or(|maximum| *next <= maximum))
            })
            .map_err(|used| {
                if used.checked_add(bytes).is_none() {
                    capacity_overflow()
                } else {
                    memory_error(
                        StoreErrorClass::Resource,
                        "memory_project_capacity",
                        "explicit project arena capacity exhausted",
                    )
                }
            })?;
        Ok(Reservation {
            arena: self.clone(),
            bytes,
        })
    }
}

impl Reservation {
    fn grow(&mut self, bytes: usize) -> Result<(), StoreError> {
        let total = self
            .bytes
            .checked_add(bytes)
            .ok_or_else(capacity_overflow)?;
        let mut added = self.arena.reserve(bytes)?;
        self.bytes = total;
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
            bytes: super::object::copy_object_bytes(bytes)?,
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

    pub(crate) fn retain(&mut self, keep: impl Fn(&K) -> bool) -> Result<(), StoreError> {
        let count = self.index.values.keys().filter(|key| keep(key)).count();
        let bytes = count
            .checked_mul(index_entry_bytes::<K>()?)
            .and_then(|bytes| bytes.checked_add(256))
            .ok_or_else(capacity_overflow)?;
        let reservation = self.index.reservation.arena.reserve(bytes)?;
        let values = self
            .index
            .values
            .iter()
            .filter(|(key, _)| keep(key))
            .map(|(key, bytes)| (key.clone(), bytes.clone()))
            .collect();
        self.index = Arc::new(MemoryIndex {
            values,
            reservation,
        });
        Ok(())
    }

    pub(crate) fn insert(&mut self, key: K, bytes: &[u8]) -> Result<(), StoreError> {
        if self.get(&key).is_some_and(|old| old.as_slice() == bytes) {
            return Ok(());
        }
        let arena = self.index.reservation.arena.clone();
        let value = MemoryBytes::copy(&arena, bytes)?;
        if Arc::get_mut(&mut self.index).is_none() {
            let growth = if self.index.values.contains_key(&key) {
                0
            } else {
                index_entry_bytes::<K>()?
            };
            let index_bytes = self
                .index
                .reservation
                .bytes
                .checked_add(growth)
                .ok_or_else(capacity_overflow)?;
            // Admit the complete copied index before publishing it. A finite refusal
            // must not retain a private index or consume capacity in this fork.
            let reservation = arena.reserve(index_bytes)?;
            let mut values = self.index.values.clone();
            values.insert(key, value);
            self.index = Arc::new(MemoryIndex {
                values,
                reservation,
            });
            return Ok(());
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
            index.reservation.grow(index_entry_bytes::<K>()?)?;
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
    pub fn new(
        maximum_bytes: Option<usize>,
    ) -> Result<Self, crate::platform::diagnostic::Diagnostic> {
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

    pub fn maximum_bytes(&self) -> Option<usize> {
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

fn index_entry_bytes<K>() -> Result<usize, StoreError> {
    std::mem::size_of::<K>()
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(192))
        .ok_or_else(capacity_overflow)
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
        let arena = MemoryArena::new(Some(1024));
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

    #[test]
    fn optional_arena_policy_is_exact_and_reservation_growth_is_atomic() {
        for maximum in [None, Some(0), Some(7), Some(8), Some(usize::MAX)] {
            let arena = MemoryArena::new(maximum);
            let result = arena.reserve(8);
            if maximum.is_some_and(|maximum| maximum < 8) {
                assert_eq!(result.unwrap_err().code, "memory_project_capacity");
                assert_eq!(arena.0.used.load(Ordering::Acquire), 0);
            } else {
                let mut reservation = result.unwrap();
                assert_eq!(arena.0.used.load(Ordering::Acquire), 8);
                if maximum == Some(8) {
                    assert!(reservation.grow(1).is_err());
                    assert_eq!(reservation.bytes, 8);
                    assert_eq!(arena.0.used.load(Ordering::Acquire), 8);
                } else {
                    reservation.grow(1).unwrap();
                    assert_eq!(reservation.bytes, 9);
                    assert_eq!(arena.0.used.load(Ordering::Acquire), 9);
                }
                drop(reservation);
                assert_eq!(arena.0.used.load(Ordering::Acquire), 0);
            }
            let zero = arena.reserve(0).unwrap();
            assert_eq!(arena.0.used.load(Ordering::Acquire), 0);
            drop(zero);
        }
        for maximum in [None, Some(usize::MAX)] {
            let arena = MemoryArena::new(maximum);
            let mut reservation = arena.reserve(usize::MAX).unwrap();
            assert!(reservation.grow(1).is_err());
            assert_eq!(reservation.bytes, usize::MAX);
            assert_eq!(arena.0.used.load(Ordering::Acquire), usize::MAX);
            drop(reservation);
            assert_eq!(arena.0.used.load(Ordering::Acquire), 0);
        }
    }

    #[test]
    fn optional_project_capacity_preserves_exact_constructor_and_fork_allowances() {
        let probe = MemProjectStore::new(None).unwrap();
        let initial = probe.used_bytes();
        let fork = probe.fork().unwrap();
        let forked = probe.used_bytes();
        drop(fork);
        assert_eq!(probe.used_bytes(), initial);
        drop(probe);
        for maximum in [
            None,
            Some(0),
            Some(initial - 1),
            Some(initial),
            Some(usize::MAX),
        ] {
            let created = MemProjectStore::new(maximum);
            if maximum.is_some_and(|maximum| maximum < initial) {
                assert!(created.is_err());
            } else {
                let project = created.unwrap();
                assert_eq!(project.maximum_bytes(), maximum);
                assert_eq!(project.used_bytes(), initial);
            }
        }
        for maximum in [None, Some(forked - 1), Some(forked), Some(usize::MAX)] {
            let project = MemProjectStore::new(maximum).unwrap();
            let fork = project.fork();
            if maximum.is_some_and(|maximum| maximum < forked) {
                assert!(fork.is_err());
                assert_eq!(project.used_bytes(), initial);
            } else {
                let fork = fork.unwrap();
                assert_eq!(fork.maximum_bytes(), maximum);
                assert_eq!(project.used_bytes(), forked);
                drop(fork);
                assert_eq!(project.used_bytes(), initial);
            }
        }
    }

    #[test]
    fn project_forks_and_retained_views_share_payload_capacity_until_last_release() {
        for maximum in [None, Some(4096), Some(usize::MAX)] {
            let project = MemProjectStore::new(maximum).unwrap();
            let arena = project.arena.clone();
            let bytes = b"one shared payload";
            let key = ObjectKey::for_bytes(ObjectDomain::Blob, bytes);
            lock(&project.graph)
                .unwrap()
                .store
                .objects
                .insert(key, bytes)
                .unwrap();
            let before_fork = project.used_bytes();
            let fork = project.fork().unwrap();
            let fork_cost = project.used_bytes() - before_fork;
            assert!(fork_cost > 0);
            let retained = lock(&project.graph).unwrap().clone();
            let first = lock(&project.graph)
                .unwrap()
                .store
                .objects
                .get(&key)
                .unwrap()
                .clone();
            let second = lock(&fork.graph)
                .unwrap()
                .store
                .objects
                .get(&key)
                .unwrap()
                .clone();
            assert!(Arc::ptr_eq(&first, &second));
            assert_eq!(first.as_slice(), bytes);
            drop(first);
            drop(second);
            drop(project);
            assert_eq!(fork.used_bytes(), before_fork);
            drop(fork);
            assert!(arena.0.used.load(Ordering::Acquire) > bytes.len());
            assert_eq!(retained.store.objects.get(&key).unwrap().as_slice(), bytes);
            drop(retained);
            assert_eq!(arena.0.used.load(Ordering::Acquire), 0);
        }
    }

    #[test]
    fn refused_shared_index_growth_preserves_the_original_index_and_capacity() {
        let probe_arena = MemoryArena::new(None);
        let mut probe = MemoryMap::<u8>::new(&probe_arena).unwrap();
        probe.insert(1, b"a").unwrap();
        let initial = probe_arena.0.used.load(Ordering::Acquire);
        let copied_index = probe.index.reservation.bytes;
        let value = MemoryBytes::copy(&probe_arena, b"b").unwrap();
        let payload = probe_arena.0.used.load(Ordering::Acquire) - initial;
        drop(value);
        drop(probe);
        assert_eq!(probe_arena.0.used.load(Ordering::Acquire), 0);
        let before_growth = initial + copied_index + payload;
        let exact = before_growth + index_entry_bytes::<u8>().unwrap();
        for maximum in [None, Some(before_growth), Some(exact - 1), Some(exact)] {
            let arena = MemoryArena::new(maximum);
            let mut changed = MemoryMap::<u8>::new(&arena).unwrap();
            changed.insert(1, b"a").unwrap();
            let retained = changed.clone();
            assert_eq!(arena.0.used.load(Ordering::Acquire), initial);
            let result = changed.insert(2, b"b");
            if maximum.is_some_and(|maximum| maximum < exact) {
                assert_eq!(result.unwrap_err().code, "memory_project_capacity");
                assert!(Arc::ptr_eq(&changed.index, &retained.index));
                assert!(changed.get(&2).is_none());
                assert_eq!(arena.0.used.load(Ordering::Acquire), initial);
            } else {
                result.unwrap();
                assert!(!Arc::ptr_eq(&changed.index, &retained.index));
                assert_eq!(changed.get(&2).unwrap().as_slice(), b"b");
                assert_eq!(arena.0.used.load(Ordering::Acquire), exact);
            }
            assert_eq!(changed.get(&1).unwrap().as_slice(), b"a");
            assert_eq!(retained.get(&1).unwrap().as_slice(), b"a");
            assert!(retained.get(&2).is_none());
            drop(changed);
            assert_eq!(arena.0.used.load(Ordering::Acquire), initial);
            drop(retained);
            assert_eq!(arena.0.used.load(Ordering::Acquire), 0);
        }
    }
}
