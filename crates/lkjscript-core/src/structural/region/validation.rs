use std::collections::BTreeSet;

use super::RegionStore;
use crate::structural::{StructuralError, StructuralRuntime};

impl<T: Copy, D: Copy> RegionStore<T, D> {
    pub fn validate(&self, runtime: &StructuralRuntime) -> Result<(), StructuralError> {
        self.require_runtime(runtime)?;
        let mut keys = BTreeSet::new();
        for (key, record) in &self.records {
            runtime.require_live(*key)?;
            if !keys.insert(*key) {
                return Err(StructuralError::DuplicateDependency);
            }
            if record.roots.len() > self.limits.max_objects_per_domain as usize
                || record.chunks.len() > self.limits.max_chunks_per_domain as usize
                || record.children.as_slice().len() > self.limits.max_dependencies as usize
                || record.drops.as_slice().len() > self.limits.max_drop_entries as usize
                || record.bytes > self.limits.max_bytes_per_domain
            {
                return Err(StructuralError::ArithmeticOverflow);
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
                if from as usize >= record.roots.len() || to as usize >= record.roots.len() {
                    return Err(StructuralError::SlotVacant);
                }
            }
        }
        Ok(())
    }
}
