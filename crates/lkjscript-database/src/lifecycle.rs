use crate::database::{Database, State};
use crate::persistence::ready;
use crate::{checkpoint, DatabaseError, DatabaseResult};

impl Database {
    /// Atomically checkpoints the logical index and clears the WAL.
    pub fn checkpoint(&self) -> DatabaseResult<()> {
        let state = self.inner.lock();
        ready(&state)?;
        if state.writer_active {
            return Err(DatabaseError::WriterActive);
        }
        self.checkpoint_locked(&state)
    }

    /// Atomically checkpoints the logical index, clears the WAL, and closes.
    pub fn close(self) -> DatabaseResult<()> {
        let mut state = self.inner.lock();
        ready(&state)?;
        if state.writer_active {
            return Err(DatabaseError::WriterActive);
        }
        self.checkpoint_locked(&state)?;
        state.closed = true;
        Ok(())
    }

    fn checkpoint_locked(&self, state: &State) -> DatabaseResult<()> {
        let checkpoint = checkpoint::encode(&state.index, state.sequence);
        self.inner
            .storage
            .replace(&self.inner.checkpoint_name, &checkpoint)?;
        self.inner.storage.replace(&self.inner.wal_name, &[])?;
        Ok(())
    }
}
