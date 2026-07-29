mod epoch;
mod model;
mod release;

use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub use model::{NoLiveLoanProof, OwnerMetrics, RemoteRelease};

use crate::{DataOwnerId, ResourceError, ResourceResult, WorkerId};

#[derive(Clone, Debug)]
pub struct OwnerHomeTable {
    homes: BTreeMap<DataOwnerId, WorkerId>,
    epochs: BTreeMap<DataOwnerId, u64>,
    live_loans: BTreeSet<DataOwnerId>,
    releases: BTreeMap<WorkerId, VecDeque<RemoteRelease>>,
    pending_releases: BTreeSet<DataOwnerId>,
    owner_limit: usize,
    release_limit: usize,
    max_epoch: u64,
    metrics: OwnerMetrics,
}

impl OwnerHomeTable {
    pub fn new(owner_limit: usize, release_limit: usize) -> Self {
        Self {
            homes: BTreeMap::new(),
            epochs: BTreeMap::new(),
            live_loans: BTreeSet::new(),
            releases: BTreeMap::new(),
            pending_releases: BTreeSet::new(),
            owner_limit,
            release_limit,
            max_epoch: u64::MAX,
            metrics: OwnerMetrics::default(),
        }
    }

    pub fn with_max_epoch(
        owner_limit: usize,
        release_limit: usize,
        max_epoch: u64,
    ) -> ResourceResult<Self> {
        if max_epoch == 0 {
            return Err(ResourceError::new(
                "owner-epoch-limit",
                "maximum owner epoch must be nonzero",
            ));
        }
        let mut table = Self::new(owner_limit, release_limit);
        table.max_epoch = max_epoch;
        Ok(table)
    }

    pub fn insert(&mut self, owner: DataOwnerId, home: WorkerId) -> ResourceResult<()> {
        if self.homes.len() >= self.owner_limit || self.homes.contains_key(&owner) {
            return Err(ResourceError::new(
                "owner-capacity",
                "owner table full or duplicate",
            ));
        }
        self.homes.insert(owner, home);
        self.epochs.insert(owner, 1);
        Ok(())
    }

    pub fn home(&self, owner: DataOwnerId) -> ResourceResult<WorkerId> {
        self.homes
            .get(&owner)
            .copied()
            .ok_or_else(|| ResourceError::new("owner-stale", "owner is unknown"))
    }

    pub fn begin_loan(&mut self, owner: DataOwnerId) -> ResourceResult<()> {
        if !self.homes.contains_key(&owner)
            || self.live_loans.contains(&owner)
            || self.pending_releases.contains(&owner)
        {
            return Err(ResourceError::new(
                "owner-loan",
                "owner unknown, already loaned, or pending release",
            ));
        }
        let next = self.next_epoch(owner)?;
        self.live_loans.insert(owner);
        self.epochs.insert(owner, next);
        Ok(())
    }

    pub fn end_loan(&mut self, owner: DataOwnerId) -> ResourceResult<()> {
        if !self.live_loans.contains(&owner) {
            return Err(ResourceError::new("owner-loan", "no live loan"));
        }
        let next = self.next_epoch(owner)?;
        self.live_loans.remove(&owner);
        self.epochs.insert(owner, next);
        Ok(())
    }

    pub fn prove_no_live_loan(&self, owner: DataOwnerId) -> ResourceResult<NoLiveLoanProof> {
        if !self.homes.contains_key(&owner)
            || self.live_loans.contains(&owner)
            || self.pending_releases.contains(&owner)
        {
            return Err(ResourceError::new(
                "owner-proof",
                "owner is loaned, pending release, or unknown",
            ));
        }
        Ok(NoLiveLoanProof {
            owner,
            epoch: self.epoch(owner)?,
        })
    }

    pub fn move_owner(
        &mut self,
        owner: DataOwnerId,
        home: WorkerId,
        proof: NoLiveLoanProof,
    ) -> ResourceResult<()> {
        self.check_proof(owner, proof)?;
        let next = self.next_epoch(owner)?;
        let current = self
            .homes
            .get_mut(&owner)
            .ok_or_else(|| ResourceError::new("owner-stale", "owner unknown"))?;
        *current = home;
        self.epochs.insert(owner, next);
        self.metrics.transfers = self.metrics.transfers.saturating_add(1);
        Ok(())
    }

    pub fn remove(&mut self, owner: DataOwnerId, proof: NoLiveLoanProof) -> ResourceResult<()> {
        self.check_proof(owner, proof)?;
        if self.pending_releases.contains(&owner) || self.homes.remove(&owner).is_none() {
            return Err(ResourceError::new(
                "owner-release",
                "owner pending or unknown",
            ));
        }
        self.epochs.remove(&owner);
        Ok(())
    }

    pub fn metrics(&self) -> OwnerMetrics {
        self.metrics
    }

    fn check_proof(&self, owner: DataOwnerId, proof: NoLiveLoanProof) -> ResourceResult<()> {
        if proof.owner != owner
            || self.epoch(owner)? != proof.epoch
            || self.live_loans.contains(&owner)
            || self.pending_releases.contains(&owner)
        {
            return Err(ResourceError::new(
                "owner-proof",
                "stale or releasing no-live-loan proof",
            ));
        }
        Ok(())
    }
}
