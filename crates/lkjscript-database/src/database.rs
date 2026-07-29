use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};

use lkjscript_host::DurableStorage;

use crate::checkpoint;
use crate::persistence::{append_all, apply, names, ready};
use crate::read::ReadTransaction;
use crate::types::{
    fresh_database_id, transaction_id, DatabaseId, NamespacedKey, Operation, Value,
};
use crate::wal;
use crate::{DatabaseError, DatabaseLimits, DatabaseResult, RecoveryReport, WriteTransaction};

pub(crate) struct State {
    pub index: Arc<BTreeMap<NamespacedKey, Value>>,
    pub sequence: u64,
    pub next_transaction: u64,
    pub writer_active: bool,
    pub closed: bool,
    pub needs_reopen: bool,
}

pub(crate) struct Inner {
    pub id: DatabaseId,
    pub storage: Arc<dyn DurableStorage>,
    pub checkpoint_name: String,
    pub wal_name: String,
    pub limits: DatabaseLimits,
    pub state: Mutex<State>,
}

pub struct Database {
    pub(crate) inner: Arc<Inner>,
    recovery: RecoveryReport,
}

impl Database {
    pub fn create(
        storage: Arc<dyn DurableStorage>,
        name: &str,
        limits: DatabaseLimits,
    ) -> DatabaseResult<Self> {
        let (checkpoint_name, wal_name) = names(name)?;
        if storage.read(&checkpoint_name)?.is_some() || storage.read(&wal_name)?.is_some() {
            return Err(DatabaseError::AlreadyExists);
        }
        storage.replace(&checkpoint_name, &checkpoint::encode(&BTreeMap::new(), 0))?;
        storage.replace(&wal_name, &[])?;
        Self::open(storage, name, limits)
    }

    pub fn open(
        storage: Arc<dyn DurableStorage>,
        name: &str,
        limits: DatabaseLimits,
    ) -> DatabaseResult<Self> {
        let (checkpoint_name, wal_name) = names(name)?;
        let checkpoint_bytes = storage
            .read(&checkpoint_name)?
            .ok_or(DatabaseError::NotFound)?;
        let (mut index, checkpoint_sequence) = checkpoint::decode(&checkpoint_bytes)?;
        let replay = wal::replay(&storage.read(&wal_name)?.unwrap_or_default())?;
        let mut sequence = checkpoint_sequence.max(replay.max_sequence);
        for (commit_sequence, operations) in replay.commits {
            if commit_sequence > checkpoint_sequence {
                apply(&mut index, operations);
            }
            sequence = sequence.max(commit_sequence);
        }
        let inner = Inner {
            id: fresh_database_id(),
            storage,
            checkpoint_name,
            wal_name,
            limits,
            state: Mutex::new(State {
                index: Arc::new(index),
                sequence,
                next_transaction: 1,
                writer_active: false,
                closed: false,
                needs_reopen: false,
            }),
        };
        Ok(Self {
            inner: Arc::new(inner),
            recovery: replay.report,
        })
    }

    pub fn id(&self) -> DatabaseId {
        self.inner.id
    }

    pub fn recovery_report(&self) -> RecoveryReport {
        self.recovery
    }

    pub fn begin_read(&self) -> DatabaseResult<ReadTransaction> {
        let mut state = self.inner.lock();
        ready(&state)?;
        let id = transaction_id(self.inner.id, state.next_transaction);
        state.next_transaction = state
            .next_transaction
            .checked_add(1)
            .ok_or(DatabaseError::Closed)?;
        Ok(ReadTransaction::new(id, Arc::clone(&state.index)))
    }

    pub fn begin_write(&self) -> DatabaseResult<WriteTransaction> {
        let mut state = self.inner.lock();
        ready(&state)?;
        if state.writer_active {
            return Err(DatabaseError::WriterActive);
        }
        let id = transaction_id(self.inner.id, state.next_transaction);
        state.next_transaction = state
            .next_transaction
            .checked_add(1)
            .ok_or(DatabaseError::Closed)?;
        state.writer_active = true;
        Ok(WriteTransaction::new(
            Arc::clone(&self.inner),
            id,
            state.index.as_ref().clone(),
            self.inner.limits,
        ))
    }

    /// Atomically checkpoints the logical index, clears the WAL, and closes.
    pub fn close(self) -> DatabaseResult<()> {
        let mut state = self.inner.lock();
        ready(&state)?;
        if state.writer_active {
            return Err(DatabaseError::WriterActive);
        }
        let checkpoint = checkpoint::encode(&state.index, state.sequence);
        self.inner
            .storage
            .replace(&self.inner.checkpoint_name, &checkpoint)?;
        self.inner.storage.replace(&self.inner.wal_name, &[])?;
        state.closed = true;
        Ok(())
    }
}

impl Inner {
    pub(crate) fn lock(&self) -> MutexGuard<'_, State> {
        match self.state.lock() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    pub(crate) fn commit(
        &self,
        operations: Vec<Operation>,
        index: BTreeMap<NamespacedKey, Value>,
    ) -> DatabaseResult<()> {
        let mut state = self.lock();
        ready(&state)?;
        if !state.writer_active {
            return Err(DatabaseError::TransactionClosed);
        }
        let sequence = state
            .sequence
            .checked_add(1)
            .ok_or(DatabaseError::NeedsReopen)?;
        let mut bytes = Vec::new();
        for operation in &operations {
            bytes.extend_from_slice(&wal::operation_frame(sequence, operation));
        }
        bytes.extend_from_slice(&wal::commit_frame(sequence));
        if let Err(error) =
            append_all(self.storage.as_ref(), &self.wal_name, &bytes).and_then(|()| {
                self.storage
                    .sync(&self.wal_name)
                    .map_err(DatabaseError::from)
            })
        {
            state.writer_active = false;
            state.needs_reopen = true;
            return Err(error);
        }
        state.index = Arc::new(index);
        state.sequence = sequence;
        state.writer_active = false;
        Ok(())
    }
}
