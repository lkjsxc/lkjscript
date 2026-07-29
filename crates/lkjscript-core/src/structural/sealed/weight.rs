use super::{SealedBuilder, SealedRegionStore};
use crate::structural::{DomainClass, DomainKey, StructuralError, StructuralLimit};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub(super) fn release_weights(
        &self,
        builders: &[SealedBuilder<T, D>],
    ) -> Result<Vec<(DomainKey, u32)>, StructuralError> {
        let mut pending = Vec::new();
        let mut weights = Vec::new();
        pending
            .try_reserve_exact(builders.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        weights
            .try_reserve_exact(builders.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        pending.extend(builders.iter().map(|builder| builder.key));
        pending.sort_unstable();
        while !pending.is_empty() {
            let mut ready = None;
            for (index, &key) in pending.iter().enumerate() {
                if let Some(weight) = self.release_weight(key, &weights)? {
                    ready = Some((index, key, weight));
                    break;
                }
            }
            let Some((index, key, weight)) = ready else {
                return Err(StructuralError::UnsupportedDependency);
            };
            pending.remove(index);
            weights.push((key, weight));
        }
        Ok(weights)
    }

    fn release_weight(
        &self,
        key: DomainKey,
        weights: &[(DomainKey, u32)],
    ) -> Result<Option<u32>, StructuralError> {
        let record = &self.records[self.record_index(key)?].1;
        let mut total = 1_u32;
        for &dependency in record.dependencies.as_slice() {
            let weight = if dependency.class() == DomainClass::RegionBuilding {
                let Some((_, weight)) = weights.iter().find(|(key, _)| *key == dependency) else {
                    return Ok(None);
                };
                *weight
            } else {
                self.records[self.record_index(dependency)?].1.release_work
            };
            total = total
                .checked_add(weight)
                .ok_or(StructuralError::LimitExceeded(StructuralLimit::ReleaseWork))?;
            if total > self.limits.max_release_work {
                return Err(StructuralError::LimitExceeded(StructuralLimit::ReleaseWork));
            }
        }
        Ok(Some(total))
    }
}
