use std::collections::{BTreeMap, BTreeSet};

use super::SealedRegionStore;
use crate::structural::{DomainClass, DomainKey, StructuralError, StructuralRuntime};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub fn validate(&self, runtime: &StructuralRuntime) -> Result<(), StructuralError> {
        self.require_runtime(runtime)?;
        let mut keys = BTreeSet::new();
        for (index, (key, record)) in self.records.iter().enumerate() {
            runtime.require_live(*key)?;
            let slot =
                usize::try_from(key.slot()).map_err(|_| StructuralError::ArithmeticOverflow)?;
            if self.indices.get(slot).and_then(|entry| *entry) != Some(index) {
                return Err(StructuralError::StaleDomain(*key));
            }
            if !keys.insert(*key) {
                return Err(StructuralError::DuplicateDependency);
            }
            match key.class() {
                DomainClass::RegionBuilding if record.owners == 0 => {}
                DomainClass::RegionSealed if record.owners > 0 => {}
                _ => return Err(StructuralError::IncompleteSeal),
            }
            for root in &record.roots {
                if root.generation != key.generation() {
                    return Err(StructuralError::GenerationExhausted);
                }
            }
            for &target in record.dependencies.as_slice() {
                self.record_index(target)?;
            }
        }
        self.validate_sealed_cycles()
    }

    fn validate_sealed_cycles(&self) -> Result<(), StructuralError> {
        let mut colors = BTreeMap::<DomainKey, u8>::new();
        let mut path = Vec::new();
        let mut frames = Vec::new();
        path.try_reserve(self.records.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        frames
            .try_reserve(self.records.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        for &(start, _) in &self.records {
            if start.class() != DomainClass::RegionSealed
                || colors.get(&start).copied().unwrap_or(0) != 0
            {
                continue;
            }
            colors.insert(start, 1);
            path.push(start);
            frames.push((start, 0_usize));
            while let Some((node, next)) = frames.last_mut() {
                let index = self.record_index(*node)?;
                let dependencies = self.records[index].1.dependencies.as_slice();
                while *next < dependencies.len()
                    && dependencies[*next].class() != DomainClass::RegionSealed
                {
                    *next = next
                        .checked_add(1)
                        .ok_or(StructuralError::ArithmeticOverflow)?;
                }
                if *next == dependencies.len() {
                    let finished = *node;
                    frames.pop();
                    path.pop();
                    colors.insert(finished, 2);
                    continue;
                }
                let target = dependencies[*next];
                *next = next
                    .checked_add(1)
                    .ok_or(StructuralError::ArithmeticOverflow)?;
                match colors.get(&target).copied().unwrap_or(0) {
                    0 => {
                        colors.insert(target, 1);
                        path.push(target);
                        frames.push((target, 0));
                    }
                    1 => {
                        let start = path
                            .iter()
                            .position(|candidate| *candidate == target)
                            .ok_or(StructuralError::ArithmeticOverflow)?;
                        let mut witness = Vec::new();
                        witness
                            .try_reserve(path.len() - start + 1)
                            .map_err(|_| StructuralError::AllocationFailed)?;
                        witness.extend_from_slice(&path[start..]);
                        witness.push(target);
                        return Err(StructuralError::DependencyCycle(witness));
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
