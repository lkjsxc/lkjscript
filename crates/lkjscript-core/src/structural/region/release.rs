use super::{RegionOwner, RegionReleaseReport, RegionStore};
use crate::structural::{DomainKey, StructuralError, StructuralRuntime};

impl<T: Copy, D: Copy> RegionStore<T, D> {
    pub fn release<E, F>(
        &mut self,
        runtime: &mut StructuralRuntime,
        owner: &RegionOwner<T, D>,
        mut execute_drop: F,
    ) -> Result<RegionReleaseReport<E>, StructuralError>
    where
        F: FnMut(D) -> Result<(), E>,
    {
        self.require_runtime(runtime)?;
        let root = self.record_index(owner.key)?;
        if self.records[root].1.parent.is_some() {
            return Err(StructuralError::UnsupportedDependency);
        }
        let work = self.release_order(owner.key)?;
        self.preflight_release(&work)?;
        runtime.preflight_release(&work)?;
        let failures = self.failure_buffer(&work, None)?;
        let mut report = RegionReleaseReport {
            domains_released: 0,
            chunks_released: 0,
            large_objects_released: 0,
            objects_released: 0,
            drop_failures: failures,
        };
        for key in work {
            let mut record = self.take_record(key)?;
            for drop in record.drops.drain_reverse() {
                if let Err(error) = execute_drop(drop) {
                    report.drop_failures.push(error);
                }
            }
            report.domains_released += 1;
            report.chunks_released += record.chunks.len() as u64;
            report.large_objects_released += record.large.len() as u64;
            report.objects_released += record.roots.len() as u64;
            runtime.release(key)?;
        }
        self.metrics.regions_destroyed = self
            .metrics
            .regions_destroyed
            .saturating_add(report.domains_released);
        self.metrics.release_work = self
            .metrics
            .release_work
            .saturating_add(report.domains_released);
        Ok(report)
    }

    pub fn reset<E, F>(
        &mut self,
        runtime: &mut StructuralRuntime,
        owner: &RegionOwner<T, D>,
        mut execute_drop: F,
    ) -> Result<RegionReleaseReport<E>, StructuralError>
    where
        F: FnMut(D) -> Result<(), E>,
    {
        self.require_runtime(runtime)?;
        let index = self.record_index(owner.key)?;
        let epoch = self.records[index].1.epoch.get();
        if epoch == u64::MAX {
            return Err(StructuralError::GenerationExhausted);
        }
        if self.records[index].1.loans != 0 {
            return Err(StructuralError::LiveLoan);
        }
        let release_work = usize::try_from(self.records[index].1.release_work)
            .map_err(|_| StructuralError::ArithmeticOverflow)?;
        let mut children = Vec::new();
        children
            .try_reserve_exact(self.records[index].1.children.as_slice().len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        children.extend_from_slice(self.records[index].1.children.as_slice());
        let mut work = Vec::new();
        work.try_reserve_exact(release_work.saturating_sub(1))
            .map_err(|_| StructuralError::AllocationFailed)?;
        for child in children.into_iter().rev() {
            work.extend(self.release_order(child)?);
        }
        if work.len().checked_add(1) != Some(release_work) {
            return Err(StructuralError::ArithmeticOverflow);
        }
        self.preflight_release(&work)?;
        runtime.preflight_release(&work)?;
        let failures = self.failure_buffer(&work, Some(owner.key))?;
        let mut report = RegionReleaseReport {
            domains_released: 0,
            chunks_released: 0,
            large_objects_released: 0,
            objects_released: 0,
            drop_failures: failures,
        };
        for key in work {
            self.release_record(runtime, key, &mut execute_drop, &mut report)?;
        }
        let index = self.record_index(owner.key)?;
        let record = &mut self.records[index].1;
        for drop in record.drops.drain_reverse() {
            if let Err(error) = execute_drop(drop) {
                report.drop_failures.push(error);
            }
        }
        report.chunks_released += record.chunks.len() as u64;
        report.large_objects_released += record.large.len() as u64;
        report.objects_released += record.roots.len() as u64;
        record.chunks.clear();
        record.large.clear();
        record.roots.clear();
        record.internal_edges.clear();
        record.children.clear();
        record.release_work = 1;
        record.bytes = 0;
        record.epoch =
            std::num::NonZeroU64::new(epoch + 1).ok_or(StructuralError::GenerationExhausted)?;
        self.metrics.regions_reset = self.metrics.regions_reset.saturating_add(1);
        Ok(report)
    }

    fn release_record<E, F>(
        &mut self,
        runtime: &mut StructuralRuntime,
        key: DomainKey,
        execute_drop: &mut F,
        report: &mut RegionReleaseReport<E>,
    ) -> Result<(), StructuralError>
    where
        F: FnMut(D) -> Result<(), E>,
    {
        let mut record = self.take_record(key)?;
        for drop in record.drops.drain_reverse() {
            if let Err(error) = execute_drop(drop) {
                report.drop_failures.push(error);
            }
        }
        report.domains_released += 1;
        report.chunks_released += record.chunks.len() as u64;
        report.large_objects_released += record.large.len() as u64;
        report.objects_released += record.roots.len() as u64;
        runtime.release(key)
    }

    fn release_order(&self, root: DomainKey) -> Result<Vec<DomainKey>, StructuralError> {
        let capacity = usize::try_from(self.records[self.record_index(root)?].1.release_work)
            .map_err(|_| StructuralError::ArithmeticOverflow)?;
        let mut pending = Vec::new();
        let mut order = Vec::new();
        pending
            .try_reserve_exact(capacity)
            .map_err(|_| StructuralError::AllocationFailed)?;
        order
            .try_reserve_exact(capacity)
            .map_err(|_| StructuralError::AllocationFailed)?;
        pending.push(root);
        while let Some(key) = pending.pop() {
            let index = self.record_index(key)?;
            order.push(key);
            pending.extend(
                self.records[index]
                    .1
                    .children
                    .as_slice()
                    .iter()
                    .rev()
                    .copied(),
            );
        }
        order.reverse();
        Ok(order)
    }
}
