use std::marker::PhantomData;

use super::{SealedOwner, SealedRegionStore, SealedReleaseReport};
use crate::structural::{DomainKey, StructuralError, StructuralRuntime};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub fn release<E, F>(
        &mut self,
        runtime: &mut StructuralRuntime,
        owner: SealedOwner<T, D>,
        mut execute_drop: F,
    ) -> Result<SealedReleaseReport<E>, (StructuralError, SealedOwner<T, D>)>
    where
        F: FnMut(D) -> Result<(), E>,
    {
        let owner_domain = owner.key;
        if let Err(error) = self.require_runtime(runtime) {
            return Err((error, owner));
        }
        let (decrements, finals) = match self.release_plan(owner_domain) {
            Ok(plan) => plan,
            Err(error) => return Err((error, owner)),
        };
        if let Err(error) = runtime.preflight_release(&finals) {
            return Err((error, owner));
        }
        let failures = match self.failure_buffer(&finals) {
            Ok(failures) => failures,
            Err(error) => return Err((error, owner)),
        };
        let mut report = SealedReleaseReport {
            regions_released: 0,
            objects_released: 0,
            dependency_releases: decrements.len().saturating_sub(1) as u64,
            drop_failures: failures,
        };
        for key in decrements {
            let index = self
                .record_index(key)
                .map_err(|error| (error, owner_key(owner_domain)))?;
            self.records[index].1.owners -= 1;
        }
        for key in finals {
            let mut record = self
                .take_record(key)
                .map_err(|error| (error, owner_key(owner_domain)))?;
            for drop in record.drops.drain_reverse() {
                if let Err(error) = execute_drop(drop) {
                    report.drop_failures.push(error);
                }
            }
            report.regions_released += 1;
            report.objects_released += record.roots.len() as u64;
            runtime
                .release(key)
                .map_err(|error| (error, owner_key(owner_domain)))?;
        }
        self.metrics.releases = self.metrics.releases.saturating_add(1);
        self.metrics.regions_destroyed = self
            .metrics
            .regions_destroyed
            .saturating_add(report.regions_released);
        self.metrics.release_work = self
            .metrics
            .release_work
            .saturating_add(report.dependency_releases.saturating_add(1));
        Ok(report)
    }

    fn release_plan(
        &self,
        key: DomainKey,
    ) -> Result<(Vec<DomainKey>, Vec<DomainKey>), StructuralError> {
        let root = &self.records[self.record_index(key)?].1;
        let capacity =
            usize::try_from(root.release_work).map_err(|_| StructuralError::ArithmeticOverflow)?;
        let mut counts = Vec::new();
        counts
            .try_reserve_exact(self.records.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        counts.extend(self.records.iter().map(|(_, record)| record.owners));
        let mut pending = Vec::new();
        let mut decrements = Vec::new();
        let mut finals = Vec::new();
        pending
            .try_reserve_exact(capacity)
            .map_err(|_| StructuralError::AllocationFailed)?;
        decrements
            .try_reserve_exact(capacity)
            .map_err(|_| StructuralError::AllocationFailed)?;
        finals
            .try_reserve_exact(capacity)
            .map_err(|_| StructuralError::AllocationFailed)?;
        pending.push(key);
        while let Some(current) = pending.pop() {
            let index = self.record_index(current)?;
            let count = counts
                .get_mut(index)
                .ok_or(StructuralError::StaleDomain(current))?;
            *count = count
                .checked_sub(1)
                .ok_or(StructuralError::StaleDomain(current))?;
            decrements.push(current);
            if *count == 0 {
                let record = &self.records[self.record_index(current)?].1;
                if record.loans != 0 {
                    return Err(StructuralError::LiveLoan);
                }
                finals.push(current);
                pending.extend(record.dependencies.as_slice().iter().rev().copied());
            }
        }
        Ok((decrements, finals))
    }
}

fn owner_key<T, D>(key: DomainKey) -> SealedOwner<T, D> {
    SealedOwner {
        key,
        marker: PhantomData,
    }
}
