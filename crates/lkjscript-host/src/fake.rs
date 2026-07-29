use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::{DurableStorage, HostError, HostResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StorageFault {
    ShortWrite(usize),
    SyncFailure,
    DiskFull,
}

#[derive(Clone, Debug, Default)]
pub struct FakeDurableStorage {
    state: Arc<Mutex<State>>,
}

#[derive(Clone, Debug, Default)]
struct FileState {
    volatile: Vec<u8>,
    durable: Vec<u8>,
}

#[derive(Debug, Default)]
struct State {
    files: BTreeMap<String, FileState>,
    faults: VecDeque<StorageFault>,
}

impl FakeDurableStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn inject(&self, fault: StorageFault) {
        self.lock().faults.push_back(fault);
    }

    /// Discards every write not completed by a successful sync.
    pub fn crash(&self) {
        for file in self.lock().files.values_mut() {
            file.volatile.clone_from(&file.durable);
        }
    }

    pub fn truncate_durable(&self, name: &str, bytes: usize) {
        if let Some(file) = self.lock().files.get_mut(name) {
            let length = file.durable.len().saturating_sub(bytes);
            file.durable.truncate(length);
            file.volatile.clone_from(&file.durable);
        }
    }

    pub fn corrupt_durable_tail(&self, name: &str) {
        if let Some(file) = self.lock().files.get_mut(name) {
            if let Some(byte) = file.durable.last_mut() {
                *byte ^= 0x80;
            }
            file.volatile.clone_from(&file.durable);
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl DurableStorage for FakeDurableStorage {
    fn read(&self, name: &str) -> HostResult<Option<Vec<u8>>> {
        Ok(self
            .lock()
            .files
            .get(name)
            .map(|file| file.volatile.clone()))
    }

    fn append(&self, name: &str, bytes: &[u8]) -> HostResult<usize> {
        let mut state = self.lock();
        match take_append_fault(&mut state.faults) {
            Some(StorageFault::DiskFull) => return Err(HostError::DiskFull(name.to_owned())),
            Some(StorageFault::ShortWrite(limit)) => {
                let written = limit.min(bytes.len());
                state
                    .files
                    .entry(name.to_owned())
                    .or_default()
                    .volatile
                    .extend_from_slice(&bytes[..written]);
                return Ok(written);
            }
            _ => {}
        }
        state
            .files
            .entry(name.to_owned())
            .or_default()
            .volatile
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn sync(&self, name: &str) -> HostResult<()> {
        let mut state = self.lock();
        if matches!(state.faults.front(), Some(StorageFault::SyncFailure)) {
            state.faults.pop_front();
            return Err(HostError::SyncFailed(name.to_owned()));
        }
        let file = state.files.entry(name.to_owned()).or_default();
        file.durable.clone_from(&file.volatile);
        Ok(())
    }

    fn replace(&self, name: &str, bytes: &[u8]) -> HostResult<()> {
        let mut state = self.lock();
        match state.faults.front().copied() {
            Some(StorageFault::DiskFull) => {
                state.faults.pop_front();
                return Err(HostError::DiskFull(name.to_owned()));
            }
            Some(StorageFault::SyncFailure) => {
                state.faults.pop_front();
                return Err(HostError::SyncFailed(name.to_owned()));
            }
            _ => {}
        }
        state.files.insert(
            name.to_owned(),
            FileState {
                volatile: bytes.to_vec(),
                durable: bytes.to_vec(),
            },
        );
        Ok(())
    }
}

fn take_append_fault(faults: &mut VecDeque<StorageFault>) -> Option<StorageFault> {
    match faults.front().copied() {
        Some(fault @ (StorageFault::ShortWrite(_) | StorageFault::DiskFull)) => {
            faults.pop_front();
            Some(fault)
        }
        _ => None,
    }
}
