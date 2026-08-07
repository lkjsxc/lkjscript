use super::{SealedBuilder, SealedRegionStore};
use crate::structural::{DomainClass, DomainKey, StructuralError};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub(super) fn release_weights(
        &self,
        builders: &[SealedBuilder<T, D>],
    ) -> Result<Vec<(DomainKey, u64)>, StructuralError> {
        let mut pending = Vec::new();
        let mut weights = Vec::new();
        let mut computed = Vec::new();
        pending
            .try_reserve_exact(builders.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        weights
            .try_reserve_exact(builders.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        computed
            .try_reserve_exact(self.records.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        computed.resize(self.records.len(), None);
        pending.extend(builders.iter().map(|builder| builder.key));
        pending.sort_unstable();
        while !pending.is_empty() {
            let mut ready = None;
            for (index, &key) in pending.iter().enumerate() {
                if let Some(weight) = self.release_weight(key, &computed)? {
                    ready = Some((index, key, weight));
                    break;
                }
            }
            let Some((index, key, weight)) = ready else {
                return Err(StructuralError::UnsupportedDependency);
            };
            pending.remove(index);
            let record = self.record_index(key)?;
            computed[record] = Some(weight);
            weights.push((key, weight));
        }
        Ok(weights)
    }

    fn release_weight(
        &self,
        key: DomainKey,
        weights: &[Option<u64>],
    ) -> Result<Option<u64>, StructuralError> {
        let record = &self.records[self.record_index(key)?].1;
        let mut total = 1_u64;
        for &dependency in record.dependencies.as_slice() {
            let weight = if dependency.class() == DomainClass::RegionBuilding {
                let Some(weight) = weights
                    .get(self.record_index(dependency)?)
                    .and_then(|weight| *weight)
                else {
                    return Ok(None);
                };
                weight
            } else {
                self.records[self.record_index(dependency)?].1.release_work
            };
            total = total
                .checked_add(weight)
                .ok_or(StructuralError::ArithmeticOverflow)?;
        }
        Ok(Some(total))
    }
}
