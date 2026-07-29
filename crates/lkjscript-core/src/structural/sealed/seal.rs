use std::marker::PhantomData;

use super::{SealFailure, SealedBuilder, SealedOwner, SealedRegionStore};
use crate::structural::{DomainClass, DomainKey, StructuralError, StructuralRuntime};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub fn discard_batch<E, F>(
        &mut self,
        runtime: &mut StructuralRuntime,
        builders: &[SealedBuilder<T, D>],
        mut execute_drop: F,
    ) -> Result<Vec<E>, StructuralError>
    where
        F: FnMut(D) -> Result<(), E>,
    {
        self.require_runtime(runtime)?;
        if builders.len() > self.limits.max_release_work as usize {
            return Err(StructuralError::LimitExceeded(
                crate::structural::StructuralLimit::ReleaseWork,
            ));
        }
        let mut keys = Vec::new();
        keys.try_reserve_exact(builders.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        keys.extend(builders.iter().map(|builder| builder.key));
        for &key in &keys {
            let index = self.record_index(key)?;
            if key.class() != DomainClass::RegionBuilding || self.records[index].1.loans != 0 {
                return Err(StructuralError::LiveLoan);
            }
        }
        runtime.preflight_release(&keys)?;
        let mut failures = self.failure_buffer(&keys)?;
        for key in keys {
            let index = self.record_index(key)?;
            let mut record = self.records.swap_remove(index).1;
            for drop in record.drops.drain_reverse() {
                if let Err(error) = execute_drop(drop) {
                    failures.push(error);
                }
            }
            runtime.release(key)?;
        }
        Ok(failures)
    }

    pub fn seal_batch(
        &mut self,
        runtime: &mut StructuralRuntime,
        builders: Vec<SealedBuilder<T, D>>,
    ) -> Result<Vec<SealedOwner<T, D>>, SealFailure<T, D>> {
        if let Err(error) = self.require_runtime(runtime) {
            return Err(SealFailure { error, builders });
        }
        if let Err(error) = self.validate_graph(&builders) {
            return Err(SealFailure { error, builders });
        }
        let weights = match self.release_weights(&builders) {
            Ok(weights) => weights,
            Err(error) => return Err(SealFailure { error, builders }),
        };
        let mut incoming = Vec::new();
        if incoming.try_reserve_exact(builders.len()).is_err() {
            return Err(allocation_failure(builders));
        }
        incoming.extend(builders.iter().map(|builder| (builder.key, 0_u32)));
        let mut existing = Vec::<(DomainKey, u32)>::new();
        for builder in &builders {
            let index = match self.record_index(builder.key) {
                Ok(index) => index,
                Err(error) => return Err(SealFailure { error, builders }),
            };
            let record = &self.records[index].1;
            if record.loans != 0 {
                return Err(SealFailure {
                    error: StructuralError::LiveLoan,
                    builders,
                });
            }
            for &target in record.dependencies.as_slice() {
                let entries = if target.class() == DomainClass::RegionBuilding {
                    &mut incoming
                } else {
                    &mut existing
                };
                if let Some((_, count)) = entries.iter_mut().find(|(key, _)| *key == target) {
                    let Some(next) = count.checked_add(1) else {
                        return Err(owner_failure(builders));
                    };
                    *count = next;
                } else if entries.try_reserve(1).is_err() {
                    return Err(allocation_failure(builders));
                } else {
                    entries.push((target, 1));
                }
            }
        }
        if let Err(error) = self.preflight_owner_counts(&incoming, &existing) {
            return Err(SealFailure { error, builders });
        }
        let mut mapping = Vec::new();
        let mut old_keys = Vec::new();
        let mut owners = Vec::new();
        let mut existing_indices = Vec::new();
        if mapping.try_reserve_exact(builders.len()).is_err()
            || old_keys.try_reserve_exact(builders.len()).is_err()
            || owners.try_reserve_exact(builders.len()).is_err()
            || existing_indices.try_reserve_exact(existing.len()).is_err()
        {
            return Err(allocation_failure(builders));
        }
        for &(old, count) in &incoming {
            let index = match self.record_index(old) {
                Ok(index) => index,
                Err(error) => return Err(SealFailure { error, builders }),
            };
            let weight = weights
                .iter()
                .find(|(key, _)| *key == old)
                .map_or(1, |(_, weight)| *weight);
            let new = self.sealed_key(old);
            mapping.push((old, new, index, count + 1, weight));
            old_keys.push(old);
            owners.push(SealedOwner {
                key: new,
                marker: PhantomData,
            });
        }
        for &(key, count) in &existing {
            match self.record_index(key) {
                Ok(index) => existing_indices.push((index, count)),
                Err(error) => return Err(SealFailure { error, builders }),
            }
        }
        if let Err(error) = runtime.transition_batch(
            &old_keys,
            DomainClass::RegionBuilding,
            DomainClass::RegionSealed,
        ) {
            return Err(SealFailure { error, builders });
        }
        for &(_, new, index, owners, weight) in &mapping {
            self.records[index].0 = new;
            self.records[index].1.owners = owners;
            self.records[index].1.release_work = weight;
            self.metrics.roots_published = self
                .metrics
                .roots_published
                .saturating_add(self.records[index].1.roots.len() as u64);
        }
        for (_, record) in &mut self.records {
            for dependency in record.dependencies.entries_mut() {
                if let Some((_, new, ..)) = mapping.iter().find(|(old, ..)| *old == *dependency) {
                    *dependency = *new;
                }
            }
        }
        for (index, count) in existing_indices {
            self.records[index].1.owners += count;
        }
        self.metrics.regions_sealed = self
            .metrics
            .regions_sealed
            .saturating_add(builders.len() as u64);
        Ok(owners)
    }
}

fn allocation_failure<T, D>(builders: Vec<SealedBuilder<T, D>>) -> SealFailure<T, D> {
    SealFailure {
        error: StructuralError::AllocationFailed,
        builders,
    }
}

fn owner_failure<T, D>(builders: Vec<SealedBuilder<T, D>>) -> SealFailure<T, D> {
    SealFailure {
        error: StructuralError::OwnerOverflow,
        builders,
    }
}
