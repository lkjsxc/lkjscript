use std::collections::BTreeSet;

use super::SealedRegionStore;
use crate::structural::{DomainClass, DomainKey, StructuralError, StructuralRuntime};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub fn validate(&self, runtime: &StructuralRuntime) -> Result<(), StructuralError> {
        self.require_runtime(runtime)?;
        let mut keys = BTreeSet::new();
        for (key, record) in &self.records {
            runtime.require_live(*key)?;
            if !keys.insert(*key) {
                return Err(StructuralError::DuplicateDependency);
            }
            match key.class() {
                DomainClass::RegionBuilding if record.owners == 0 => {}
                DomainClass::RegionSealed if record.owners > 0 => {}
                _ => return Err(StructuralError::IncompleteSeal),
            }
            if record.roots.len() > self.limits.max_objects_per_domain as usize
                || record.dependencies.as_slice().len() > self.limits.max_dependencies as usize
                || record.drops.as_slice().len() > self.limits.max_drop_entries as usize
                || record.bytes > self.limits.max_bytes_per_domain
            {
                return Err(StructuralError::ArithmeticOverflow);
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
        for &(key, _) in &self.records {
            if key.class() == DomainClass::RegionSealed {
                self.validate_path(key, &mut Vec::new(), &mut BTreeSet::new())?;
            }
        }
        Ok(())
    }

    fn validate_path(
        &self,
        key: DomainKey,
        path: &mut Vec<DomainKey>,
        complete: &mut BTreeSet<DomainKey>,
    ) -> Result<(), StructuralError> {
        if complete.contains(&key) {
            return Ok(());
        }
        if let Some(start) = path.iter().position(|candidate| *candidate == key) {
            let mut witness = path[start..].to_vec();
            witness.push(key);
            return Err(StructuralError::DependencyCycle(witness));
        }
        if path.len() >= self.limits.max_release_work as usize {
            return Err(StructuralError::ArithmeticOverflow);
        }
        path.push(key);
        let record = &self.records[self.record_index(key)?].1;
        for &target in record.dependencies.as_slice() {
            if target.class() == DomainClass::RegionSealed {
                self.validate_path(target, path, complete)?;
            }
        }
        path.pop();
        complete.insert(key);
        Ok(())
    }
}
