use std::collections::BTreeSet;

use super::RegionStore;
use crate::structural::{StructuralError, StructuralRuntime};

impl<T: Copy, D: Copy> RegionStore<T, D> {
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
            for root in &record.roots {
                if root.generation != record.epoch {
                    return Err(StructuralError::GenerationExhausted);
                }
            }
            for child in record.children.as_slice() {
                let child_index = self.record_index(*child)?;
                if self.records[child_index].1.parent != Some(*key) {
                    return Err(StructuralError::UnsupportedDependency);
                }
            }
            for &(from, to) in record.internal_edges.as_slice() {
                let from =
                    usize::try_from(from).map_err(|_| StructuralError::ArithmeticOverflow)?;
                let to = usize::try_from(to).map_err(|_| StructuralError::ArithmeticOverflow)?;
                if from >= record.roots.len() || to >= record.roots.len() {
                    return Err(StructuralError::SlotVacant);
                }
            }
        }
        Ok(())
    }
}
