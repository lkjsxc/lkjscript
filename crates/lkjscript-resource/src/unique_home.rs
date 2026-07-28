use std::collections::BTreeMap;
use std::sync::Mutex;

use lkjscript_core::{ByteVectorKey, UniqueStore, UniqueStoreId, UniqueStoreLimits};

use crate::{
    DataOwnerId, NoLiveLoanProof, OwnerHomeTable, RemoteRelease, ResourceError, ResourceResult,
    TaskId, WorkerId,
};

mod stats;
mod support;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HomedByteVector {
    pub owner: DataOwnerId,
    partition: usize,
    key: ByteVectorKey,
}

struct Metadata {
    homes: OwnerHomeTable,
    keys: BTreeMap<DataOwnerId, (usize, ByteVectorKey)>,
}

pub struct PartitionedUniqueStore {
    metadata: Mutex<Metadata>,
    stores: Vec<Mutex<UniqueStore>>,
}

impl PartitionedUniqueStore {
    pub fn new(
        store_id: u64,
        limits: UniqueStoreLimits,
        partitions: usize,
        owner_limit: usize,
        release_limit: usize,
    ) -> ResourceResult<Self> {
        if partitions == 0 {
            return Err(ResourceError::new(
                "unique-partitions",
                "partitions required",
            ));
        }
        let stores = (0..partitions)
            .map(|partition| {
                let raw = store_id
                    .checked_add(partition as u64)
                    .ok_or_else(|| ResourceError::new("unique-store-id", "store ID overflow"))?;
                let id = UniqueStoreId::new(raw).ok_or_else(|| {
                    ResourceError::new("unique-store-id", "store ID must be nonzero")
                })?;
                Ok(Mutex::new(UniqueStore::new(id, limits)))
            })
            .collect::<ResourceResult<Vec<_>>>()?;
        Ok(Self {
            metadata: Mutex::new(Metadata {
                homes: OwnerHomeTable::new(owner_limit, release_limit),
                keys: BTreeMap::new(),
            }),
            stores,
        })
    }

    pub fn allocate_byte_vector(
        &self,
        owner: DataOwnerId,
        home: WorkerId,
        bytes: Vec<u8>,
    ) -> ResourceResult<HomedByteVector> {
        let partition = home.slot as usize % self.stores.len();
        let mut metadata = self.metadata()?;
        if metadata.keys.contains_key(&owner) {
            return Err(ResourceError::new(
                "owner-duplicate",
                "owner already stored",
            ));
        }
        let mut store = self.store(partition)?;
        let key = store.allocate_byte_vector(bytes).map_err(unique_error)?;
        if let Err(error) = metadata.homes.insert(owner, home) {
            store.free_byte_vector(key).map_err(unique_error)?;
            return Err(error);
        }
        metadata.keys.insert(owner, (partition, key));
        Ok(HomedByteVector {
            owner,
            partition,
            key,
        })
    }

    pub fn home(&self, value: HomedByteVector) -> ResourceResult<WorkerId> {
        let metadata = self.metadata()?;
        self.require(&metadata, value)?;
        metadata.homes.home(value.owner)
    }

    pub fn begin_loan(&self, value: HomedByteVector) -> ResourceResult<()> {
        let mut metadata = self.metadata()?;
        self.require(&metadata, value)?;
        metadata.homes.begin_loan(value.owner)
    }

    pub fn end_loan(&self, value: HomedByteVector) -> ResourceResult<()> {
        let mut metadata = self.metadata()?;
        self.require(&metadata, value)?;
        metadata.homes.end_loan(value.owner)
    }

    pub fn prove_no_live_loan(&self, value: HomedByteVector) -> ResourceResult<NoLiveLoanProof> {
        let metadata = self.metadata()?;
        self.require(&metadata, value)?;
        metadata.homes.prove_no_live_loan(value.owner)
    }

    pub fn move_home(
        &self,
        value: HomedByteVector,
        destination: WorkerId,
        proof: NoLiveLoanProof,
    ) -> ResourceResult<()> {
        let mut metadata = self.metadata()?;
        self.require(&metadata, value)?;
        metadata.homes.move_owner(value.owner, destination, proof)
    }

    pub fn fill(&self, value: HomedByteVector, byte: u8) -> ResourceResult<()> {
        let metadata = self.metadata()?;
        self.require(&metadata, value)?;
        drop(metadata);
        self.store(value.partition)?
            .fill_byte_vector(value.key, byte)
            .map_err(unique_error)
    }

    pub fn checksum(&self, value: HomedByteVector) -> ResourceResult<u64> {
        let metadata = self.metadata()?;
        self.require(&metadata, value)?;
        drop(metadata);
        let mut store = self.store(value.partition)?;
        Ok(store
            .byte_vector(value.key)
            .map_err(unique_error)?
            .iter()
            .fold(0_u64, |sum, byte| sum.wrapping_add(u64::from(*byte))))
    }

    pub fn release(
        &self,
        worker: WorkerId,
        task: TaskId,
        value: HomedByteVector,
    ) -> ResourceResult<()> {
        let mut metadata = self.metadata()?;
        self.require(&metadata, value)?;
        let home = metadata.homes.home(value.owner)?;
        if home != worker {
            return metadata.homes.remote_release(
                home,
                RemoteRelease {
                    owner: value.owner,
                    from_task: task,
                },
            );
        }
        self.release_locked(&mut metadata, value)
    }

    pub fn drain_remote(&self, home: WorkerId, limit: usize) -> ResourceResult<usize> {
        let mut metadata = self.metadata()?;
        let releases = metadata.homes.drain_releases(home, limit);
        for release in &releases {
            let (partition, key) = metadata
                .keys
                .get(&release.owner)
                .copied()
                .ok_or_else(|| ResourceError::new("owner-stale", "release owner missing"))?;
            self.release_locked(
                &mut metadata,
                HomedByteVector {
                    owner: release.owner,
                    partition,
                    key,
                },
            )?;
        }
        Ok(releases.len())
    }
}

fn unique_error(error: impl std::fmt::Display) -> ResourceError {
    ResourceError::new("unique-store", error.to_string())
}
