use std::collections::{BTreeMap, BTreeSet};

use super::{SealedBuilder, SealedRegionStore};
use crate::structural::{DomainClass, StructuralError};

impl<T: Copy, D: Copy> SealedRegionStore<T, D> {
    pub(super) fn validate_graph(
        &mut self,
        builders: &[SealedBuilder<T, D>],
    ) -> Result<(), StructuralError> {
        let nodes = builders
            .iter()
            .map(|builder| builder.key)
            .collect::<BTreeSet<_>>();
        if nodes.len() != builders.len() {
            return Err(StructuralError::DuplicateDependency);
        }
        let mut graph = BTreeMap::new();
        for &key in &nodes {
            let record = &self.records[self.record_index(key)?].1;
            let mut outgoing = Vec::new();
            outgoing
                .try_reserve(record.dependencies.as_slice().len())
                .map_err(|_| StructuralError::AllocationFailed)?;
            for &target in record.dependencies.as_slice() {
                match target.class() {
                    DomainClass::RegionBuilding if nodes.contains(&target) => outgoing.push(target),
                    DomainClass::RegionBuilding => {
                        return Err(StructuralError::UnsupportedDependency);
                    }
                    DomainClass::RegionSealed => {
                        self.record_index(target)?;
                    }
                    _ => return Err(StructuralError::UnsupportedDependency),
                }
            }
            outgoing.sort_unstable();
            graph.insert(key, outgoing);
        }
        let mut colors = BTreeMap::new();
        let mut path = Vec::new();
        let mut frames = Vec::new();
        path.try_reserve(nodes.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        frames
            .try_reserve(nodes.len())
            .map_err(|_| StructuralError::AllocationFailed)?;
        for &start in &nodes {
            if colors.get(&start).copied().unwrap_or(0) != 0 {
                continue;
            }
            colors.insert(start, 1_u8);
            path.push(start);
            frames.push((start, 0_usize));
            while let Some((node, next)) = frames.last_mut() {
                let targets = graph.get(node).map(Vec::as_slice).unwrap_or(&[]);
                if *next == targets.len() {
                    let finished = *node;
                    frames.pop();
                    path.pop();
                    colors.insert(finished, 2);
                    continue;
                }
                let target = targets[*next];
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
                        self.metrics.rejected_cycles =
                            self.metrics.rejected_cycles.saturating_add(1);
                        return Err(StructuralError::DependencyCycle(witness));
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
