use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{DataOwnerId, ResourceError, ResourceResult, TaskId, WorkerId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NoLiveLoanProof {
    owner: DataOwnerId,
    epoch: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OwnerMetrics {
    pub transfers: u64,
    pub remote_releases: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RemoteRelease {
    pub owner: DataOwnerId,
    pub from_task: TaskId,
}

#[derive(Clone, Debug)]
pub struct OwnerHomeTable {
    homes: BTreeMap<DataOwnerId, WorkerId>,
    epochs: BTreeMap<DataOwnerId, u64>,
    live_loans: BTreeSet<DataOwnerId>,
    releases: BTreeMap<WorkerId, VecDeque<RemoteRelease>>,
    pending_releases: BTreeSet<DataOwnerId>,
    owner_limit: usize,
    release_limit: usize,
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
            metrics: OwnerMetrics::default(),
        }
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
        if !self.homes.contains_key(&owner) || !self.live_loans.insert(owner) {
            return Err(ResourceError::new(
                "owner-loan",
                "owner unknown or already loaned",
            ));
        }
        self.bump_epoch(owner)
    }

    pub fn end_loan(&mut self, owner: DataOwnerId) -> ResourceResult<()> {
        if !self.live_loans.remove(&owner) {
            return Err(ResourceError::new("owner-loan", "no live loan"));
        }
        self.bump_epoch(owner)
    }

    pub fn prove_no_live_loan(&self, owner: DataOwnerId) -> ResourceResult<NoLiveLoanProof> {
        if !self.homes.contains_key(&owner) || self.live_loans.contains(&owner) {
            return Err(ResourceError::new(
                "owner-proof",
                "owner has a live loan or is unknown",
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
        let current = self
            .homes
            .get_mut(&owner)
            .ok_or_else(|| ResourceError::new("owner-stale", "owner unknown"))?;
        *current = home;
        self.bump_epoch(owner)?;
        self.metrics.transfers = self.metrics.transfers.saturating_add(1);
        Ok(())
    }

    pub fn remote_release(&mut self, home: WorkerId, release: RemoteRelease) -> ResourceResult<()> {
        if self.home(release.owner)? != home {
            return Err(ResourceError::new("release-home", "wrong owner home"));
        }
        let queue = self.releases.entry(home).or_default();
        if queue.len() >= self.release_limit || !self.pending_releases.insert(release.owner) {
            return Err(ResourceError::new(
                "release-capacity",
                "remote release queue full or owner already pending",
            ));
        }
        queue.push_back(release);
        self.metrics.remote_releases = self.metrics.remote_releases.saturating_add(1);
        Ok(())
    }

    pub fn drain_releases(&mut self, home: WorkerId, limit: usize) -> Vec<RemoteRelease> {
        let Some(queue) = self.releases.get_mut(&home) else {
            return Vec::new();
        };
        let releases: Vec<_> = (0..limit).filter_map(|_| queue.pop_front()).collect();
        for release in &releases {
            self.pending_releases.remove(&release.owner);
        }
        releases
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
        {
            return Err(ResourceError::new(
                "owner-proof",
                "stale no-live-loan proof",
            ));
        }
        Ok(())
    }

    fn epoch(&self, owner: DataOwnerId) -> ResourceResult<u64> {
        self.epochs
            .get(&owner)
            .copied()
            .ok_or_else(|| ResourceError::new("owner-stale", "owner epoch is unknown"))
    }

    fn bump_epoch(&mut self, owner: DataOwnerId) -> ResourceResult<()> {
        let epoch = self
            .epochs
            .get_mut(&owner)
            .ok_or_else(|| ResourceError::new("owner-stale", "owner epoch is unknown"))?;
        *epoch = epoch.saturating_add(1);
        Ok(())
    }
}
