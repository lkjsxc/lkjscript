use std::collections::BTreeMap;
use std::sync::Mutex;

use lkjscript_core::{
    ByteVectorKey, UniqueStore, UniqueStoreId, UniqueStoreLimits, UniqueStoreStats,
};

use crate::{
    DataOwnerId, NoLiveLoanProof, OwnerHomeTable, OwnerMetrics, RemoteRelease, ResourceError,
    ResourceResult, TaskId, WorkerId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HomedByteVector {
    pub owner: DataOwnerId,
    key: ByteVectorKey,
}

struct State {
    store: UniqueStore,
    homes: OwnerHomeTable,
    keys: BTreeMap<DataOwnerId, ByteVectorKey>,
}

pub struct PartitionedUniqueStore {
    state: Mutex<State>,
}

impl PartitionedUniqueStore {
    pub fn new(
        store_id: u64,
        limits: UniqueStoreLimits,
        owner_limit: usize,
        release_limit: usize,
    ) -> ResourceResult<Self> {
        let id = UniqueStoreId::new(store_id)
            .ok_or_else(|| ResourceError::new("unique-store-id", "store ID must be nonzero"))?;
        Ok(Self {
            state: Mutex::new(State {
                store: UniqueStore::new(id, limits),
                homes: OwnerHomeTable::new(owner_limit, release_limit),
                keys: BTreeMap::new(),
            }),
        })
    }

    pub fn allocate_byte_vector(
        &self,
        owner: DataOwnerId,
        home: WorkerId,
        bytes: Vec<u8>,
    ) -> ResourceResult<HomedByteVector> {
        let mut state = self.lock()?;
        if state.keys.contains_key(&owner) {
            return Err(ResourceError::new(
                "owner-duplicate",
                "owner already stored",
            ));
        }
        let key = state
            .store
            .allocate_byte_vector(bytes)
            .map_err(unique_error)?;
        if let Err(error) = state.homes.insert(owner, home) {
            state.store.free_byte_vector(key).map_err(unique_error)?;
            return Err(error);
        }
        state.keys.insert(owner, key);
        Ok(HomedByteVector { owner, key })
    }

    pub fn home(&self, value: HomedByteVector) -> ResourceResult<WorkerId> {
        self.lock()?.homes.home(value.owner)
    }

    pub fn begin_loan(&self, value: HomedByteVector) -> ResourceResult<()> {
        self.lock()?.homes.begin_loan(value.owner)
    }

    pub fn end_loan(&self, value: HomedByteVector) -> ResourceResult<()> {
        self.lock()?.homes.end_loan(value.owner)
    }

    pub fn prove_no_live_loan(&self, value: HomedByteVector) -> ResourceResult<NoLiveLoanProof> {
        self.lock()?.homes.prove_no_live_loan(value.owner)
    }

    pub fn move_home(
        &self,
        value: HomedByteVector,
        destination: WorkerId,
        proof: NoLiveLoanProof,
    ) -> ResourceResult<()> {
        self.lock()?
            .homes
            .move_owner(value.owner, destination, proof)
    }

    pub fn fill(&self, value: HomedByteVector, byte: u8) -> ResourceResult<()> {
        let mut state = self.lock()?;
        require_key(&state, value)?;
        state
            .store
            .fill_byte_vector(value.key, byte)
            .map_err(unique_error)
    }

    pub fn checksum(&self, value: HomedByteVector) -> ResourceResult<u64> {
        let mut state = self.lock()?;
        require_key(&state, value)?;
        let bytes = state.store.byte_vector(value.key).map_err(unique_error)?;
        Ok(bytes
            .iter()
            .fold(0_u64, |sum, byte| sum.wrapping_add(u64::from(*byte))))
    }

    pub fn release(
        &self,
        worker: WorkerId,
        task: TaskId,
        value: HomedByteVector,
    ) -> ResourceResult<()> {
        let mut state = self.lock()?;
        require_key(&state, value)?;
        let home = state.homes.home(value.owner)?;
        if home != worker {
            return state.homes.remote_release(
                home,
                RemoteRelease {
                    owner: value.owner,
                    from_task: task,
                },
            );
        }
        release_now(&mut state, value)
    }

    pub fn drain_remote(&self, home: WorkerId, limit: usize) -> ResourceResult<usize> {
        let mut state = self.lock()?;
        let releases = state.homes.drain_releases(home, limit);
        for release in &releases {
            let key =
                state.keys.get(&release.owner).copied().ok_or_else(|| {
                    ResourceError::new("owner-stale", "remote release owner missing")
                })?;
            release_now(
                &mut state,
                HomedByteVector {
                    owner: release.owner,
                    key,
                },
            )?;
        }
        Ok(releases.len())
    }

    pub fn metrics(&self) -> ResourceResult<(OwnerMetrics, UniqueStoreStats)> {
        let state = self.lock()?;
        Ok((state.homes.metrics(), state.store.stats()))
    }

    pub fn verify_empty(&self) -> ResourceResult<()> {
        let state = self.lock()?;
        if !state.keys.is_empty() || state.store.assert_no_leaks().is_err() {
            return Err(ResourceError::new(
                "unique-leak",
                "session unique owner leaked",
            ));
        }
        Ok(())
    }

    fn lock(&self) -> ResourceResult<std::sync::MutexGuard<'_, State>> {
        self.state
            .lock()
            .map_err(|_| ResourceError::new("poison", "unique home state poisoned"))
    }
}

fn require_key(state: &State, value: HomedByteVector) -> ResourceResult<()> {
    if state.keys.get(&value.owner) != Some(&value.key) {
        return Err(ResourceError::new("owner-stale", "owner key is stale"));
    }
    Ok(())
}

fn release_now(state: &mut State, value: HomedByteVector) -> ResourceResult<()> {
    let proof = state.homes.prove_no_live_loan(value.owner)?;
    state
        .store
        .free_byte_vector(value.key)
        .map_err(unique_error)?;
    state.homes.remove(value.owner, proof)?;
    state.keys.remove(&value.owner);
    Ok(())
}

fn unique_error(error: impl std::fmt::Display) -> ResourceError {
    ResourceError::new("unique-store", error.to_string())
}
