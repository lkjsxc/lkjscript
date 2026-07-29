use super::{NoLiveLoanProof, OwnerHomeTable, RemoteRelease};
use crate::{ResourceError, ResourceResult, WorkerId};

impl OwnerHomeTable {
    pub fn remote_release(
        &mut self,
        home: WorkerId,
        mut release: RemoteRelease,
        proof: NoLiveLoanProof,
    ) -> ResourceResult<()> {
        self.check_proof(release.owner, proof)?;
        if self.home(release.owner)? != home {
            return Err(ResourceError::new("release-home", "wrong owner home"));
        }
        if self.release_limit == 0
            || self.pending_releases.contains(&release.owner)
            || self
                .releases
                .get(&home)
                .is_some_and(|queue| queue.len() >= self.release_limit)
        {
            return Err(ResourceError::new(
                "release-capacity",
                "remote release queue full or owner already pending",
            ));
        }
        let next = self.next_epoch(release.owner)?;
        let new_home = !self.releases.contains_key(&home);
        if self
            .releases
            .entry(home)
            .or_default()
            .try_reserve(1)
            .is_err()
        {
            if new_home {
                self.releases.remove(&home);
            }
            return Err(ResourceError::new(
                "release-allocation",
                "release queue allocation",
            ));
        }
        self.pending_releases.insert(release.owner);
        self.epochs.insert(release.owner, next);
        release.epoch = next;
        self.releases.entry(home).or_default().push_back(release);
        self.metrics.remote_releases = self.metrics.remote_releases.saturating_add(1);
        Ok(())
    }

    pub fn drain_releases(
        &mut self,
        home: WorkerId,
        limit: usize,
    ) -> ResourceResult<Vec<RemoteRelease>> {
        let Some(queue) = self.releases.get_mut(&home) else {
            return Ok(Vec::new());
        };
        let count = limit.min(queue.len());
        let mut releases = Vec::new();
        releases
            .try_reserve_exact(count)
            .map_err(|_| ResourceError::new("release-allocation", "release drain allocation"))?;
        releases.extend((0..count).filter_map(|_| queue.pop_front()));
        let empty = queue.is_empty();
        if empty {
            self.releases.remove(&home);
        }
        Ok(releases)
    }

    pub fn release_queue_count(&self) -> usize {
        self.releases.len()
    }

    pub fn pending_release_count(&self) -> usize {
        self.pending_releases.len()
    }

    pub fn complete_release<F>(&mut self, release: RemoteRelease, teardown: F) -> ResourceResult<()>
    where
        F: FnOnce() -> ResourceResult<()>,
    {
        self.validate_release(release)?;
        teardown()?;
        self.finish_release(release);
        Ok(())
    }

    pub fn process_releases<F>(
        &mut self,
        home: WorkerId,
        limit: usize,
        mut teardown: F,
    ) -> ResourceResult<usize>
    where
        F: FnMut(RemoteRelease) -> ResourceResult<()>,
    {
        let mut completed = 0_usize;
        while completed < limit {
            let Some(release) = self
                .releases
                .get(&home)
                .and_then(|queue| queue.front())
                .copied()
            else {
                break;
            };
            self.validate_release(release)?;
            teardown(release)?;
            let queue = self.releases.get_mut(&home).ok_or_else(|| {
                ResourceError::new("release-authority", "release queue disappeared")
            })?;
            if queue.pop_front() != Some(release) {
                return Err(ResourceError::new(
                    "release-authority",
                    "release queue order changed",
                ));
            }
            let empty = queue.is_empty();
            if empty {
                self.releases.remove(&home);
            }
            self.finish_release(release);
            completed += 1;
        }
        Ok(completed)
    }

    fn validate_release(&self, release: RemoteRelease) -> ResourceResult<()> {
        if !self.pending_releases.contains(&release.owner)
            || self.epoch(release.owner)? != release.epoch
            || self.live_loans.contains(&release.owner)
        {
            return Err(ResourceError::new(
                "release-authority",
                "remote release authority is stale",
            ));
        }
        Ok(())
    }

    fn finish_release(&mut self, release: RemoteRelease) {
        self.pending_releases.remove(&release.owner);
        self.homes.remove(&release.owner);
        self.epochs.remove(&release.owner);
    }
}
